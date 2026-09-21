//! CRUD commands for configuration values.
//!
//! Sicredi secrets (`api_key`, `codigo_acesso`) live in the OS keyring.
//! `get_config` never returns decrypted secrets — only configured/empty flags.
//! Other sensitive SMTP/WhatsApp values still use AES-GCM when AUTOBO_ENCRYPTION_KEY is set.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::db::AppState;
use crate::services::crypto;
use crate::services::secrets::{self, SICREDI_API_KEY, SICREDI_CODIGO_ACESSO};
use crate::services::sicredi::{load_config, SicrediClient};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ConfigRow {
    pub id: i64,
    pub chave: String,
    pub valor: String,
    pub descricao: Option<String>,
    pub atualizado_em: chrono::NaiveDateTime,
}

const SENSITIVE_AES: &[&str] = &["smtp.senha", "whatsapp.token"];

const SECRET_KEYRING: &[&str] = &[SICREDI_API_KEY, SICREDI_CODIGO_ACESSO];

fn is_aes_sensitive(chave: &str) -> bool {
    SENSITIVE_AES.contains(&chave)
}

fn is_keyring_secret(chave: &str) -> bool {
    SECRET_KEYRING.contains(&chave)
}

#[tauri::command]
pub async fn get_config(
    chave: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    if is_keyring_secret(&chave) {
        let configured = secrets::secret_configured(&chave);
        return Ok(serde_json::json!({
            "chave": chave,
            "valor": "",
            "configurado": configured,
            "descricao": null,
            "atualizado_em": null,
        }));
    }

    let row = sqlx::query_as::<_, ConfigRow>(
        "SELECT id, chave, valor, descricao, atualizado_em FROM autobo_configuracoes WHERE chave = $1",
    )
    .bind(&chave)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("Erro: {}", e))?
    .ok_or_else(|| format!("Configuração '{}' não encontrada", chave))?;

    let valor = if is_aes_sensitive(&chave) {
        // Never return decrypted secret to frontend
        let configured = !row.valor.is_empty();
        return Ok(serde_json::json!({
            "chave": row.chave,
            "valor": "",
            "configurado": configured,
            "descricao": row.descricao,
            "atualizado_em": row.atualizado_em,
        }));
    } else {
        row.valor.clone()
    };

    Ok(serde_json::json!({
        "chave": row.chave,
        "valor": valor,
        "configurado": !valor.is_empty(),
        "descricao": row.descricao,
        "atualizado_em": row.atualizado_em,
    }))
}

#[tauri::command]
pub async fn set_config(
    chave: String,
    valor: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if is_keyring_secret(&chave) {
        secrets::probe_keyring().map_err(|e| e.to_string())?;
        secrets::set_secret(&chave, &valor).map_err(|e| e.to_string())?;
        // Marker in DB so UI knows something is configured (not the secret)
        let marker = if valor.is_empty() { "" } else { "keyring" };
        sqlx::query(
            "INSERT INTO autobo_configuracoes (chave, valor) VALUES ($1, $2)
             ON CONFLICT (chave) DO UPDATE SET valor = $2, atualizado_em = NOW()",
        )
        .bind(&chave)
        .bind(marker)
        .execute(&state.db)
        .await
        .map_err(|e| format!("Erro ao salvar marker: {e}"))?;
        return Ok(());
    }

    let valor_armazenado = if is_aes_sensitive(&chave) {
        crypto::encrypt(&valor).map_err(|e| format!("Erro ao criptografar: {}", e))?
    } else {
        valor
    };

    sqlx::query(
        "INSERT INTO autobo_configuracoes (chave, valor) VALUES ($1, $2)
         ON CONFLICT (chave) DO UPDATE SET valor = $2, atualizado_em = NOW()",
    )
    .bind(&chave)
    .bind(&valor_armazenado)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Erro ao salvar: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn listar_config_sicredi(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let chaves = [
        "sicredi.ambiente",
        "sicredi.cooperativa",
        "sicredi.posto",
        "sicredi.codigo_beneficiario",
        "sicredi.especie_documento",
        "sicredi.prefixo_instalacao",
        "sicredi.data_entrada_producao",
        "sicredi.campo_mensagem_json",
    ];
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

    let mut map = serde_json::Map::new();
    for c in chaves {
        map.insert(c.to_string(), serde_json::json!(cfg(&state.db, c).await?));
    }
    let api_marker = cfg(&state.db, "sicredi.api_key").await?;
    map.insert(
        "sicredi.api_key".into(),
        serde_json::json!({
            "configurado": secrets::secret_configured(SICREDI_API_KEY) || !api_marker.is_empty()
        }),
    );
    map.insert(
        "sicredi.codigo_acesso".into(),
        serde_json::json!({
            "configurado": secrets::secret_configured(SICREDI_CODIGO_ACESSO)
        }),
    );

    let keyring_ok = secrets::probe_keyring().is_ok();
    Ok(serde_json::json!({
        "valores": map,
        "keyring_disponivel": keyring_ok,
    }))
}

#[tauri::command]
pub async fn probe_keyring() -> Result<bool, String> {
    match secrets::probe_keyring() {
        Ok(()) => Ok(true),
        Err(e) => Err(e.to_string()),
    }
}

/// Test Sicredi: OAuth + liquidados/dia.
#[tauri::command]
pub async fn testar_sicredi(
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    let config = load_config(&state.db).await?;
    let client = SicrediClient::new(config);
    client.testar_conectividade().await?;
    Ok(true)
}

#[tauri::command]
pub async fn testar_smtp(
    _state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    Ok(false)
}

#[tauri::command]
pub async fn testar_whatsapp(
    _state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    Ok(false)
}

/// Ensure installation prefix exists (for idTituloEmpresa).
pub async fn ensure_prefixo_instalacao(pool: &sqlx::PgPool) -> Result<String, String> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT valor FROM autobo_configuracoes WHERE chave = 'sicredi.prefixo_instalacao'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(p) = existing.filter(|s| !s.is_empty()) {
        return Ok(p);
    }

    let prefix: String = uuid::Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .collect();
    sqlx::query(
        "INSERT INTO autobo_configuracoes (chave, valor, descricao) VALUES ('sicredi.prefixo_instalacao', $1, 'Prefixo instalação')
         ON CONFLICT (chave) DO UPDATE SET valor = EXCLUDED.valor",
    )
    .bind(&prefix)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(prefix)
}
