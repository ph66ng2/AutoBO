//! Sicredi Cobrança API client (OAuth + boleto operations).
//!
//! Base: https://api-parceiro.sicredi.com.br
//! Sandbox prefix: /sb/

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::NaiveDate;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::services::boleto_status::parse_has_next;
use crate::services::secrets::{self, SICREDI_API_KEY, SICREDI_CODIGO_ACESSO};

const BASE: &str = "https://api-parceiro.sicredi.com.br";

#[derive(Debug, Clone)]
pub struct SicrediConfig {
    pub ambiente: String, // sandbox | producao
    pub cooperativa: String,
    pub posto: String,
    pub codigo_beneficiario: String,
    pub api_key: String,
    pub codigo_acesso: String,
}

impl SicrediConfig {
    pub fn is_sandbox(&self) -> bool {
        self.ambiente.eq_ignore_ascii_case("sandbox")
    }

    pub fn auth_url(&self) -> String {
        if self.is_sandbox() {
            format!("{BASE}/sb/auth/openapi/token")
        } else {
            format!("{BASE}/auth/openapi/token")
        }
    }

    pub fn boletos_url(&self) -> String {
        if self.is_sandbox() {
            format!("{BASE}/sb/cobranca/boleto/v1/boletos")
        } else {
            format!("{BASE}/cobranca/boleto/v1/boletos")
        }
    }

    pub fn oauth_username(&self) -> String {
        format!("{}{}", self.codigo_beneficiario, self.cooperativa)
    }

    /// Sandbox fixed credentials from the manual.
    pub fn apply_sandbox_defaults(&mut self) {
        if self.is_sandbox() {
            self.codigo_beneficiario = "12345".into();
            self.cooperativa = "6789".into();
            self.posto = "03".into();
            self.codigo_acesso = "teste123".into();
            // username becomes 123456789
        }
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
refresh_expires_in: Option<i64>,
    #[serde(default)]
    #[allow(dead_code)]
    token_type: Option<String>,
}

struct CachedToken {
    access_token: String,
    refresh_token: Option<String>,
    access_expires_at: Instant,
    refresh_expires_at: Option<Instant>,
}

pub struct SicrediClient {
    http: reqwest::Client,
    config: SicrediConfig,
    token: Mutex<Option<CachedToken>>,
}

impl SicrediClient {
    pub fn new(mut config: SicrediConfig) -> Self {
        if config.is_sandbox() {
            config.apply_sandbox_defaults();
        }
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
            config,
            token: Mutex::new(None),
        }
    }

    pub fn config(&self) -> &SicrediConfig {
        &self.config
    }

    async fn password_grant(&self) -> Result<CachedToken, String> {
        let username = if self.config.is_sandbox() {
            "123456789".to_string()
        } else {
            self.config.oauth_username()
        };
        let password = if self.config.is_sandbox() {
            "teste123".to_string()
        } else {
            self.config.codigo_acesso.clone()
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.config.api_key)
                .map_err(|e| format!("x-api-key inválida: {e}"))?,
        );
        headers.insert("context", HeaderValue::from_static("COBRANCA"));
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );

        let body = [
            ("grant_type", "password"),
            ("username", username.as_str()),
            ("password", password.as_str()),
            ("scope", "cobranca"),
        ];

        let resp = self
            .http
            .post(self.config.auth_url())
            .headers(headers)
            .form(&body)
            .send()
            .await
            .map_err(|e| format!("Erro de rede na autenticação: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("Auth HTTP {status}: {}", sanitize_err(&text)));
        }
        let tok: TokenResponse =
            serde_json::from_str(&text).map_err(|e| format!("Auth JSON inválido: {e}"))?;
        Ok(cache_from_response(tok))
    }

    async fn refresh_grant(&self, refresh_token: &str) -> Result<CachedToken, String> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.config.api_key)
                .map_err(|e| format!("x-api-key inválida: {e}"))?,
        );
        headers.insert("context", HeaderValue::from_static("COBRANCA"));
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );

        let body = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", "cobranca"),
        ];

        let resp = self
            .http
            .post(self.config.auth_url())
            .headers(headers)
            .form(&body)
            .send()
            .await
            .map_err(|e| format!("Erro de rede no refresh: {e}"))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("Refresh HTTP {status}: {}", sanitize_err(&text)));
        }
        let tok: TokenResponse =
            serde_json::from_str(&text).map_err(|e| format!("Refresh JSON inválido: {e}"))?;
        Ok(cache_from_response(tok))
    }

    /// Get a valid access token (password or refresh). One refresh at a time via mutex.
    pub async fn access_token(&self) -> Result<String, String> {
        let mut guard = self.token.lock().await;
        let margin = Duration::from_secs(30);

        if let Some(cached) = guard.as_ref() {
            if Instant::now() + margin < cached.access_expires_at {
                return Ok(cached.access_token.clone());
            }
            if let Some(ref rt) = cached.refresh_token {
                if cached
                    .refresh_expires_at
                    .map(|t| Instant::now() + margin < t)
                    .unwrap_or(false)
                {
                    match self.refresh_grant(rt).await {
                        Ok(new_tok) => {
                            let access = new_tok.access_token.clone();
                            *guard = Some(new_tok);
                            return Ok(access);
                        }
                        Err(e) => {
                            warn!("refresh falhou, tentando password: {e}");
                        }
                    }
                }
            }
        }

        let new_tok = self.password_grant().await?;
        let access = new_tok.access_token.clone();
        *guard = Some(new_tok);
        Ok(access)
    }

    fn cobranca_headers(&self, token: &str) -> Result<HeaderMap, String> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.config.api_key)
                .map_err(|e| format!("x-api-key: {e}"))?,
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| format!("Authorization: {e}"))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "cooperativa",
            HeaderValue::from_str(&self.config.cooperativa)
                .map_err(|e| format!("cooperativa: {e}"))?,
        );
        headers.insert(
            "posto",
            HeaderValue::from_str(&self.config.posto).map_err(|e| format!("posto: {e}"))?,
        );
        Ok(headers)
    }

    fn baixa_headers(&self, token: &str) -> Result<HeaderMap, String> {
        let mut h = self.cobranca_headers(token)?;
        h.insert(
            "codigoBeneficiario",
            HeaderValue::from_str(&self.config.codigo_beneficiario)
                .map_err(|e| format!("codigoBeneficiario: {e}"))?,
        );
        Ok(h)
    }

    async fn with_auth_retry<F, Fut>(&self, f: F) -> Result<reqwest::Response, String>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, String>>,
    {
        let token = self.access_token().await?;
        let resp = f(token).await?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            // Force password re-auth once
            {
                let mut g = self.token.lock().await;
                *g = None;
            }
            let token2 = self.access_token().await?;
            return f(token2).await;
        }
        Ok(resp)
    }

    pub async fn criar_boleto(&self, body: &Value) -> Result<SicrediHttpResult, String> {
        let url = self.config.boletos_url();
        let resp = self
            .with_auth_retry(|token| {
                let url = url.clone();
                let body = body.clone();
                let headers = self.cobranca_headers(&token);
                async move {
                    let headers = headers?;
                    self.http
                        .post(url)
                        .headers(headers)
                        .json(&body)
                        .send()
                        .await
                        .map_err(|e| format!("Rede no cadastro: {e}"))
                }
            })
            .await;

        match resp {
            Ok(r) => SicrediHttpResult::from_response(r).await,
            Err(e) if is_ambiguous_network(&e) => Ok(SicrediHttpResult::ambiguous(e)),
            Err(e) => Err(e),
        }
    }

    pub async fn baixar_pdf(&self, linha_digitavel: &str) -> Result<Vec<u8>, String> {
        let url = format!("{}/pdf", self.config.boletos_url());
        let linha = linha_digitavel.to_string();
        let resp = self
            .with_auth_retry(|token| {
                let url = url.clone();
                let linha = linha.clone();
                let headers = self.cobranca_headers(&token);
                async move {
                    let headers = headers?;
                    self.http
                        .get(url)
                        .headers(headers)
                        .query(&[("linhaDigitavel", linha.as_str())])
                        .send()
                        .await
                        .map_err(|e| format!("Rede no PDF: {e}"))
                }
            })
            .await?;

        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Ler PDF: {e}"))?;
        if !status.is_success() {
            return Err(format!(
                "PDF HTTP {status}: {}",
                sanitize_err(&String::from_utf8_lossy(&bytes))
            ));
        }
        if bytes.len() < 4 || &bytes[..4] != b"%PDF" {
            return Err("Resposta PDF sem assinatura %PDF".into());
        }
        Ok(bytes.to_vec())
    }

    pub async fn pedir_baixa(&self, nosso_numero: &str) -> Result<SicrediHttpResult, String> {
        let url = format!("{}/{}/baixa", self.config.boletos_url(), nosso_numero);
        let resp = self
            .with_auth_retry(|token| {
                let url = url.clone();
                let headers = self.baixa_headers(&token);
                async move {
                    let headers = headers?;
                    self.http
                        .patch(url)
                        .headers(headers)
                        .json(&serde_json::json!({}))
                        .send()
                        .await
                        .map_err(|e| format!("Rede na baixa: {e}"))
                }
            })
            .await;

        match resp {
            Ok(r) => SicrediHttpResult::from_response(r).await,
            Err(e) if is_ambiguous_network(&e) => Ok(SicrediHttpResult::ambiguous(e)),
            Err(e) => Err(e),
        }
    }

    pub async fn consultar_por_nosso_numero(
        &self,
        nosso_numero: &str,
    ) -> Result<SicrediHttpResult, String> {
        let url = self.config.boletos_url();
        let nn = nosso_numero.to_string();
        let ben = self.config.codigo_beneficiario.clone();
        let resp = self
            .with_auth_retry(|token| {
                let url = url.clone();
                let nn = nn.clone();
                let ben = ben.clone();
                let headers = self.cobranca_headers(&token);
                async move {
                    let headers = headers?;
                    self.http
                        .get(url)
                        .headers(headers)
                        .query(&[
                            ("codigoBeneficiario", ben.as_str()),
                            ("nossoNumero", nn.as_str()),
                        ])
                        .send()
                        .await
                        .map_err(|e| format!("Rede na consulta: {e}"))
                }
            })
            .await?;
        SicrediHttpResult::from_response(resp).await
    }

    pub async fn liquidados_dia(
        &self,
        dia: NaiveDate,
        pagina: i32,
    ) -> Result<LiquidadosDiaPage, String> {
        let url = format!("{}/liquidados/dia", self.config.boletos_url());
        let dia_s = dia.format("%d/%m/%Y").to_string();
        let ben = self.config.codigo_beneficiario.clone();
        let pagina_s = pagina.to_string();
        let resp = self
            .with_auth_retry(|token| {
                let url = url.clone();
                let dia_s = dia_s.clone();
                let ben = ben.clone();
                let pagina_s = pagina_s.clone();
                let headers = self.cobranca_headers(&token);
                async move {
                    let mut headers = headers?;
                    // Manual sometimes shows form content-type for this GET
                    headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/x-www-form-urlencoded"),
                    );
                    self.http
                        .get(url)
                        .headers(headers)
                        .query(&[
                            ("codigoBeneficiario", ben.as_str()),
                            ("dia", dia_s.as_str()),
                            ("pagina", pagina_s.as_str()),
                        ])
                        .send()
                        .await
                        .map_err(|e| format!("Rede liquidados/dia: {e}"))
                }
            })
            .await?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!(
                "liquidados/dia HTTP {status}: {}",
                sanitize_err(&text)
            ));
        }
        let v: Value =
            serde_json::from_str(&text).map_err(|e| format!("JSON liquidados: {e}"))?;
        Ok(LiquidadosDiaPage::from_json(v))
    }

    /// Connectivity test: OAuth + liquidados/dia (empty list is success).
    pub async fn testar_conectividade(&self) -> Result<(), String> {
        let _ = self.access_token().await?;
        let hoje = chrono::Local::now().date_naive();
        let _ = self.liquidados_dia(hoje, 0).await?;
        info!("Sicredi connectivity OK (auth + liquidados/dia)");
        Ok(())
    }
}

fn cache_from_response(tok: TokenResponse) -> CachedToken {
    let expires = tok.expires_in.max(60) as u64;
    let refresh_exp = tok.refresh_expires_in.unwrap_or(1800).max(60) as u64;
    CachedToken {
        access_token: tok.access_token,
        refresh_token: tok.refresh_token,
        access_expires_at: Instant::now() + Duration::from_secs(expires),
        refresh_expires_at: Some(Instant::now() + Duration::from_secs(refresh_exp)),
    }
}

fn is_ambiguous_network(err: &str) -> bool {
    let e = err.to_lowercase();
    e.contains("timeout")
        || e.contains("timed out")
        || e.contains("connection")
        || e.contains("reset")
        || e.contains("broken pipe")
}

fn sanitize_err(s: &str) -> String {
    let mut t = s.chars().take(400).collect::<String>();
    for needle in ["Bearer ", "access_token", "refresh_token", "password"] {
        if t.to_lowercase().contains(&needle.to_lowercase()) {
            t = "[sanitized]".into();
            break;
        }
    }
    t
}

#[derive(Debug, Clone)]
pub struct SicrediHttpResult {
    pub status: u16,
    pub body: Value,
    pub ambiguous: bool,
    pub network_error: Option<String>,
}

impl SicrediHttpResult {
    async fn from_response(resp: reqwest::Response) -> Result<Self, String> {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        let body = if text.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or_else(|_| json_msg(&text))
        };
        Ok(Self {
            status,
            body,
            ambiguous: (500..600).contains(&status),
            network_error: None,
        })
    }

    fn ambiguous(err: String) -> Self {
        Self {
            status: 0,
            body: Value::Null,
            ambiguous: true,
            network_error: Some(sanitize_err(&err)),
        }
    }

    pub fn message(&self) -> String {
        self.body
            .get("message")
            .or_else(|| self.body.get("mensagem"))
            .or_else(|| self.body.get("error"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| self.network_error.clone())
            .unwrap_or_else(|| format!("HTTP {}", self.status))
    }

    pub fn nosso_numero(&self) -> Option<String> {
        self.body
            .get("nossoNumero")
            .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| {
                v.as_i64().map(|n| n.to_string())
            }))
    }

    pub fn linha_digitavel(&self) -> Option<String> {
        self.body
            .get("linhaDigitavel")
            .or_else(|| self.body.get("linhaDigitável"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub fn codigo_barras(&self) -> Option<String> {
        self.body
            .get("codigoBarras")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub fn transaction_id(&self) -> Option<String> {
        self.body
            .get("transactionId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub fn situacao(&self) -> Option<String> {
        self.body
            .get("situacao")
            .or_else(|| self.body.get("situação"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

fn json_msg(text: &str) -> Value {
    serde_json::json!({ "message": sanitize_err(text) })
}

#[derive(Debug, Clone)]
pub struct LiquidadoItem {
    pub nosso_numero: String,
    pub valor_liquidado: Option<f64>,
    pub data_pagamento: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct LiquidadosDiaPage {
    pub items: Vec<LiquidadoItem>,
    pub has_next: bool,
}

impl LiquidadosDiaPage {
    pub fn from_json(v: Value) -> Self {
        let has_next = v
            .get("hasNext")
            .and_then(|x| parse_has_next(x))
            .unwrap_or(false);

        let arr = v
            .get("items")
            .or_else(|| v.get("resultado"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();

        let items = arr
            .into_iter()
            .filter_map(|item| {
                let nosso = item
                    .get("nossoNumero")
                    .and_then(|n| n.as_str().map(|s| s.to_string()).or_else(|| n.as_i64().map(|i| i.to_string())))?;
                let valor = item
                    .get("valorLiquidado")
                    .or_else(|| item.get("valor"))
                    .and_then(|x| x.as_f64());
                let data = item
                    .get("dataPagamento")
                    .and_then(|x| x.as_str())
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                Some(LiquidadoItem {
                    nosso_numero: nosso,
                    valor_liquidado: valor,
                    data_pagamento: data,
                })
            })
            .collect();

        Self { items, has_next }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn liquidados_has_next_string_and_bool() {
        let p1 = LiquidadosDiaPage::from_json(json!({
            "hasNext": false,
            "items": [{"nossoNumero": "211001292", "valorLiquidado": 10.5, "dataPagamento": "2026-09-20"}]
        }));
        assert!(!p1.has_next);
        assert_eq!(p1.items.len(), 1);

        let p2 = LiquidadosDiaPage::from_json(json!({
            "hasNext": "true",
            "items": []
        }));
        assert!(p2.has_next);
    }
}

/// Load config from DB + keyring.
pub async fn load_config(pool: &sqlx::PgPool) -> Result<SicrediConfig, String> {
    async fn cfg(pool: &sqlx::PgPool, chave: &str) -> Result<String, String> {
        let v: Option<String> = sqlx::query_scalar(
            "SELECT valor FROM autobo_configuracoes WHERE chave = $1",
        )
        .bind(chave)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(v.unwrap_or_default())
    }

    let ambiente = cfg(pool, "sicredi.ambiente").await?;
    let cooperativa = cfg(pool, "sicredi.cooperativa").await?;
    let posto = cfg(pool, "sicredi.posto").await?;
    let codigo_beneficiario = cfg(pool, "sicredi.codigo_beneficiario").await?;

    let api_key = secrets::get_secret(SICREDI_API_KEY).unwrap_or_else(|_| {
        // fallback: legacy plaintext in DB during migration
        String::new()
    });
    let mut api_key = api_key;
    if api_key.is_empty() {
        api_key = cfg(pool, "sicredi.api_key").await?;
    }

    let codigo_acesso = secrets::get_secret(SICREDI_CODIGO_ACESSO).unwrap_or_default();

    if api_key.is_empty() {
        return Err("sicredi.api_key não configurada no cofre".into());
    }

    Ok(SicrediConfig {
        ambiente: if ambiente.is_empty() {
            "sandbox".into()
        } else {
            ambiente
        },
        cooperativa,
        posto,
        codigo_beneficiario,
        api_key,
        codigo_acesso,
    })
}

pub type SharedSicredi = Arc<Mutex<Option<SicrediClient>>>;
