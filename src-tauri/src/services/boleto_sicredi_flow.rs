//! Orchestrates Sicredi register / PDF / baixa with CAS transitions.
//! HTTP always runs outside DB transactions.

use std::path::PathBuf;

use chrono::{Duration, Local};
use rust_decimal::Decimal;
use sqlx::PgPool;
use tracing::{info, warn};

use crate::commands::configuracoes::ensure_prefixo_instalacao;
use crate::services::boleto_cas::{self, marcar_erro, set_retry};
use crate::services::boleto_pdf::{caminho_pdf_boleto, nome_arquivo_boleto};
use crate::services::boleto_status::{
    classificar_baixa_422, pode_restaurar_status_anterior, Baixa422Acao, StatusFinanceiro,
};
use crate::services::sicredi::{load_config, SicrediClient, SicrediHttpResult};
use crate::services::sicredi_adapter::{
    build_cadastro_body, gerar_id_titulo_empresa, gerar_sicredi_seu_numero, BoletoSicrediInput,
    PagadorSicrediInput,
};

const MAX_RETRY: i32 = 5;

pub struct PagadorDados {
    pub tipo_pessoa: String,
    pub documento: String,
    pub nome: String,
    pub logradouro: String,
    pub numero: String,
    pub cidade: String,
    pub uf: String,
    pub cep: String,
    pub telefone: Option<String>,
    pub email: Option<String>,
}

async fn cfg(pool: &PgPool, chave: &str) -> Result<String, String> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT valor FROM autobo_configuracoes WHERE chave = $1",
    )
    .bind(chave)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .unwrap_or_default())
}

/// Register draft boleto with Sicredi (idempotent CAS flow).
pub async fn registrar_boleto_sicredi(
    pool: &PgPool,
    boleto_id: i64,
    pagador: PagadorDados,
    valor: Decimal,
    data_vencimento: &str,
    especie: Option<&str>,
    mensagem: Option<&str>,
    tipo_juros: Option<&str>,
    percentual_juros: Option<Decimal>,
    tipo_multa: Option<&str>,
    percentual_multa: Option<Decimal>,
) -> Result<(), String> {
    let status: String = sqlx::query_scalar("SELECT status FROM autobo_boletos WHERE id = $1")
        .bind(boleto_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let atual = StatusFinanceiro::parse(&status).ok_or(status)?;
    if !matches!(
        atual,
        StatusFinanceiro::Rascunho
            | StatusFinanceiro::ErroPayload
            | StatusFinanceiro::RegistroIndeterminado
    ) {
        // Already registered or in flight — don't POST again from RASCUNHO path
        if matches!(
            atual,
            StatusFinanceiro::Registrado
                | StatusFinanceiro::Registrando
                | StatusFinanceiro::Vencido
                | StatusFinanceiro::Pago
        ) {
            return Ok(());
        }
        if atual == StatusFinanceiro::RegistroIndeterminado {
            return Err(
                "Registro indeterminado — reconcilie antes de novo POST (ação humana)".into(),
            );
        }
    }

    let config = load_config(pool).await?;
    let client = SicrediClient::new(config.clone());
    let ambiente = client.config().ambiente.clone();
    let beneficiario = client.config().codigo_beneficiario.clone();
    let prefixo = ensure_prefixo_instalacao(pool).await?;

    let sicredi_seu = gerar_sicredi_seu_numero(boleto_id);
    let id_titulo = gerar_id_titulo_empresa(&prefixo, boleto_id);
    let especie = especie
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or(cfg(pool, "sicredi.especie_documento").await?);
    let campo_msg = cfg(pool, "sicredi.campo_mensagem_json").await?;

    // Reserve ids + CAS to REGISTRANDO
    sqlx::query(
        r#"UPDATE autobo_boletos SET
            sicredi_ambiente = $2,
            sicredi_codigo_beneficiario = $3,
            sicredi_seu_numero = $4,
            sicredi_id_titulo_empresa = $5,
            especie_documento = $6,
            atualizado_em = NOW()
         WHERE id = $1"#,
    )
    .bind(boleto_id)
    .bind(&ambiente)
    .bind(&beneficiario)
    .bind(&sicredi_seu)
    .bind(&id_titulo)
    .bind(&especie)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    let cas = boleto_cas::transicionar(
        pool,
        boleto_id,
        atual,
        StatusFinanceiro::Registrando,
        false,
    )
    .await?;
    if !cas.aplicada {
        return Err(format!(
            "Não foi possível iniciar registro (status={})",
            cas.status_atual.as_str()
        ));
    }

    let input = BoletoSicrediInput {
        codigo_beneficiario: beneficiario.clone(),
        data_vencimento: data_vencimento.to_string(),
        valor,
        especie_documento: especie,
        sicredi_seu_numero: sicredi_seu,
        sicredi_id_titulo_empresa: id_titulo,
        pagador: PagadorSicrediInput {
            tipo_pessoa: pagador.tipo_pessoa,
            documento: pagador.documento,
            nome: pagador.nome,
            logradouro: pagador.logradouro,
            numero: pagador.numero,
            cidade: pagador.cidade,
            uf: pagador.uf,
            cep: pagador.cep,
            telefone: pagador.telefone,
            email: pagador.email,
        },
        mensagem_livre: mensagem.map(|s| s.to_string()),
        campo_mensagem_json: if campo_msg.is_empty() {
            None
        } else {
            Some(campo_msg)
        },
        tipo_juros: tipo_juros.map(|s| s.to_string()),
        percentual_juros_mes: percentual_juros,
        tipo_multa: tipo_multa.map(|s| s.to_string()),
        percentual_multa,
    };

    let body = match build_cadastro_body(&input) {
        Ok(b) => b,
        Err(e) => {
            let _ = boleto_cas::transicionar(
                pool,
                boleto_id,
                StatusFinanceiro::Registrando,
                StatusFinanceiro::ErroPayload,
                false,
            )
            .await;
            marcar_erro(pool, boleto_id, "VALIDACAO", &e.0, None).await?;
            return Err(e.0);
        }
    };

    // HTTP outside transaction
    let result = client.criar_boleto(&body).await?;

    handle_cadastro_result(pool, boleto_id, &client, result).await
}

async fn handle_cadastro_result(
    pool: &PgPool,
    boleto_id: i64,
    client: &SicrediClient,
    result: SicrediHttpResult,
) -> Result<(), String> {
    if result.ambiguous {
        let _ = boleto_cas::transicionar(
            pool,
            boleto_id,
            StatusFinanceiro::Registrando,
            StatusFinanceiro::RegistroIndeterminado,
            false,
        )
        .await?;
        marcar_erro(
            pool,
            boleto_id,
            "AMBIGUO",
            &result.message(),
            if result.status > 0 {
                Some(result.status as i32)
            } else {
                None
            },
        )
        .await?;
        return Err(format!(
            "Registro indeterminado: {}. Não será reenviado automaticamente.",
            result.message()
        ));
    }

    match result.status {
        201 | 200 => {
            let nosso = result.nosso_numero().ok_or("201 sem nossoNumero")?;
            let linha = result.linha_digitavel();
            let barras = result.codigo_barras();
            sqlx::query(
                r#"UPDATE autobo_boletos SET
                    nosso_numero = $2,
                    linha_digitavel = $3,
                    codigo_barras = $4,
                    pdf_pendente = TRUE,
                    ultimo_http_status = $5,
                    ultimo_erro_codigo = NULL,
                    ultimo_erro_mensagem = NULL,
                    atualizado_em = NOW()
                 WHERE id = $1"#,
            )
            .bind(boleto_id)
            .bind(&nosso)
            .bind(&linha)
            .bind(&barras)
            .bind(result.status as i32)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

            let cas = boleto_cas::transicionar(
                pool,
                boleto_id,
                StatusFinanceiro::Registrando,
                StatusFinanceiro::Registrado,
                false,
            )
            .await?;
            if !cas.aplicada && cas.status_atual != StatusFinanceiro::Registrado {
                warn!(
                    "CAS REGISTRADO falhou id={} atual={}",
                    boleto_id,
                    cas.status_atual.as_str()
                );
            }

            // PDF independent
            if let Some(ref ld) = linha {
                match baixar_e_salvar_pdf(pool, client, boleto_id, ld).await {
                    Ok(()) => info!("PDF oficial salvo boleto {boleto_id}"),
                    Err(e) => {
                        warn!("PDF pendente boleto {boleto_id}: {e}");
                        sqlx::query(
                            "UPDATE autobo_boletos SET pdf_pendente = TRUE, ultimo_erro_mensagem = $2 WHERE id = $1",
                        )
                        .bind(boleto_id)
                        .bind(&e.chars().take(500).collect::<String>())
                        .execute(pool)
                        .await
                        .ok();
                    }
                }
            }
            Ok(())
        }
        429 => {
            let tentativas: i32 = sqlx::query_scalar(
                "SELECT tentativas_operacao FROM autobo_boletos WHERE id = $1",
            )
            .bind(boleto_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0)
                + 1;
            if tentativas > MAX_RETRY {
                marcar_erro(pool, boleto_id, "429", "teto de retry", Some(429)).await?;
                return Err("Rate limit: teto de tentativas".into());
            }
            let proxima = Local::now().naive_local() + Duration::seconds(30 * tentativas as i64);
            set_retry(pool, boleto_id, "CADASTRO", tentativas, proxima, "429").await?;
            Err("Rate limit — agendado retry".into())
        }
        400 | 415 | 422 => {
            let msg = result.message();
            // Duplicidade → indeterminado / reconcile, not rascunho
            if msg.to_lowercase().contains("duplic") || msg.to_lowercase().contains("já cadastr")
            {
                let _ = boleto_cas::transicionar(
                    pool,
                    boleto_id,
                    StatusFinanceiro::Registrando,
                    StatusFinanceiro::RegistroIndeterminado,
                    false,
                )
                .await?;
                marcar_erro(pool, boleto_id, "DUPLICIDADE", &msg, Some(result.status as i32))
                    .await?;
                return Err(format!("Possível duplicidade: {msg}"));
            }
            let _ = boleto_cas::transicionar(
                pool,
                boleto_id,
                StatusFinanceiro::Registrando,
                StatusFinanceiro::ErroPayload,
                false,
            )
            .await?;
            marcar_erro(
                pool,
                boleto_id,
                "PAYLOAD",
                &msg,
                Some(result.status as i32),
            )
            .await?;
            Err(msg)
        }
        other => {
            let _ = boleto_cas::transicionar(
                pool,
                boleto_id,
                StatusFinanceiro::Registrando,
                StatusFinanceiro::RegistroIndeterminado,
                false,
            )
            .await?;
            marcar_erro(
                pool,
                boleto_id,
                "HTTP",
                &result.message(),
                Some(other as i32),
            )
            .await?;
            Err(format!("Cadastro HTTP {other}: {}", result.message()))
        }
    }
}

pub async fn baixar_e_salvar_pdf(
    pool: &PgPool,
    client: &SicrediClient,
    boleto_id: i64,
    linha_digitavel: &str,
) -> Result<(), String> {
    let bytes = client.baixar_pdf(linha_digitavel).await?;
    let seu: String = sqlx::query_scalar("SELECT seu_numero FROM autobo_boletos WHERE id = $1")
        .bind(boleto_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    let dir = caminho_pdf_boleto(&seu)?
        .parent()
        .ok_or("dir PDF")?
        .to_path_buf();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let nome = format!("oficial-{}", nome_arquivo_boleto(&seu));
    let path = dir.join(nome);
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;

    sqlx::query(
        "UPDATE autobo_boletos SET pdf_pendente = FALSE, pdf_oficial_path = $2, atualizado_em = NOW() WHERE id = $1",
    )
    .bind(boleto_id)
    .bind(path.to_string_lossy().as_ref())
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn retentar_pdf(pool: &PgPool, boleto_id: i64) -> Result<(), String> {
    let row: Option<(Option<String>, bool)> = sqlx::query_as(
        "SELECT linha_digitavel, pdf_pendente FROM autobo_boletos WHERE id = $1",
    )
    .bind(boleto_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;
    let (linha, pendente) = row.ok_or("Boleto não encontrado")?;
    if !pendente {
        return Ok(());
    }
    let linha = linha.ok_or("Sem linha digitável")?;
    let client = SicrediClient::new(load_config(pool).await?);
    baixar_e_salvar_pdf(pool, &client, boleto_id, &linha).await
}

/// User-initiated baixa with CAS BAIXA_ENVIANDO before HTTP.
pub async fn pedir_baixa_sicredi(pool: &PgPool, boleto_id: i64) -> Result<(), String> {
    let row: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT status, nosso_numero, status_anterior FROM autobo_boletos WHERE id = $1",
    )
    .bind(boleto_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    let (status_s, nosso, _) = row.ok_or("Boleto não encontrado")?;
    let atual = StatusFinanceiro::parse(&status_s).ok_or(status_s)?;

    if atual.is_user_command_terminal() {
        return Err(format!(
            "Boleto em estado terminal {} — baixa não permitida",
            atual.as_str()
        ));
    }
    if matches!(
        atual,
        StatusFinanceiro::BaixaIndeterminada | StatusFinanceiro::BaixaEnviando
    ) {
        return Err(
            "Baixa indeterminada/em envio — consulte antes de novo PATCH (ação humana)".into(),
        );
    }
    if !matches!(
        atual,
        StatusFinanceiro::Registrado | StatusFinanceiro::Vencido
    ) {
        return Err(format!("Status {} não permite baixa", atual.as_str()));
    }
    let nosso = nosso.ok_or("Sem nosso_numero — boleto não registrado")?;

    sqlx::query(
        "UPDATE autobo_boletos SET status_anterior = status, atualizado_em = NOW() WHERE id = $1",
    )
    .bind(boleto_id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    let cas = boleto_cas::transicionar(
        pool,
        boleto_id,
        atual,
        StatusFinanceiro::BaixaEnviando,
        false,
    )
    .await?;
    if !cas.aplicada {
        return Err(format!(
            "CAS BAIXA_ENVIANDO falhou (atual={})",
            cas.status_atual.as_str()
        ));
    }

    let client = SicrediClient::new(load_config(pool).await?);
    let result = client.pedir_baixa(&nosso).await?;
    handle_baixa_result(pool, boleto_id, atual, result).await
}

async fn handle_baixa_result(
    pool: &PgPool,
    boleto_id: i64,
    status_antes_envio: StatusFinanceiro,
    result: SicrediHttpResult,
) -> Result<(), String> {
    if result.ambiguous {
        let _ = boleto_cas::transicionar(
            pool,
            boleto_id,
            StatusFinanceiro::BaixaEnviando,
            StatusFinanceiro::BaixaIndeterminada,
            false,
        )
        .await?;
        marcar_erro(pool, boleto_id, "AMBIGUO", &result.message(), None).await?;
        return Err("Baixa indeterminada — consulte antes de reenviar".into());
    }

    match result.status {
        202 => {
            if let Some(txid) = result.transaction_id() {
                sqlx::query(
                    "UPDATE autobo_boletos SET sicredi_transaction_id = $2 WHERE id = $1",
                )
                .bind(boleto_id)
                .bind(&txid)
                .execute(pool)
                .await
                .ok();
            }
            let cas = boleto_cas::transicionar(
                pool,
                boleto_id,
                StatusFinanceiro::BaixaEnviando,
                StatusFinanceiro::BaixaSolicitada,
                false,
            )
            .await?;
            if !cas.aplicada && cas.status_atual == StatusFinanceiro::Pago {
                info!("Baixa 202 ignorada — boleto já PAGO");
                return Ok(());
            }
            Ok(())
        }
        429 => {
            let tentativas: i32 = sqlx::query_scalar(
                "SELECT tentativas_operacao FROM autobo_boletos WHERE id = $1",
            )
            .bind(boleto_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0)
                + 1;
            let proxima = Local::now().naive_local() + Duration::seconds(30 * tentativas as i64);
            set_retry(pool, boleto_id, "BAIXA", tentativas, proxima, "429").await?;
            Err("Rate limit na baixa — retry agendado".into())
        }
        422 => {
            match classificar_baixa_422(&result.message()) {
                Baixa422Acao::ConvergirPago => {
                    let _ = boleto_cas::transicionar(
                        pool,
                        boleto_id,
                        StatusFinanceiro::BaixaEnviando,
                        StatusFinanceiro::Pago,
                        true,
                    )
                    .await?;
                    // Also try from Cancelado if somehow
                    Ok(())
                }
                Baixa422Acao::ConvergirCancelado => {
                    let _ = boleto_cas::transicionar(
                        pool,
                        boleto_id,
                        StatusFinanceiro::BaixaEnviando,
                        StatusFinanceiro::Cancelado,
                        false,
                    )
                    .await?;
                    Ok(())
                }
                Baixa422Acao::ManterSolicitadaOuIndeterminada => {
                    let alvo = if result.transaction_id().is_some() {
                        StatusFinanceiro::BaixaSolicitada
                    } else {
                        StatusFinanceiro::BaixaIndeterminada
                    };
                    let _ = boleto_cas::transicionar(
                        pool,
                        boleto_id,
                        StatusFinanceiro::BaixaEnviando,
                        alvo,
                        false,
                    )
                    .await?;
                    Ok(())
                }
                Baixa422Acao::RestaurarAnteriorComErro => {
                    if pode_restaurar_status_anterior(
                        StatusFinanceiro::BaixaEnviando,
                        status_antes_envio,
                    ) {
                        let _ = boleto_cas::transicionar(
                            pool,
                            boleto_id,
                            StatusFinanceiro::BaixaEnviando,
                            status_antes_envio,
                            false,
                        )
                        .await?;
                    }
                    marcar_erro(
                        pool,
                        boleto_id,
                        "BAIXA_422",
                        &result.message(),
                        Some(422),
                    )
                    .await?;
                    Err(result.message())
                }
                Baixa422Acao::ErroOperacional => {
                    marcar_erro(
                        pool,
                        boleto_id,
                        "BAIXA_422",
                        &result.message(),
                        Some(422),
                    )
                    .await?;
                    Err(result.message())
                }
            }
        }
        400 => {
            marcar_erro(
                pool,
                boleto_id,
                "BAIXA_400",
                &result.message(),
                Some(400),
            )
            .await?;
            if pode_restaurar_status_anterior(
                StatusFinanceiro::BaixaEnviando,
                status_antes_envio,
            ) {
                let _ = boleto_cas::transicionar(
                    pool,
                    boleto_id,
                    StatusFinanceiro::BaixaEnviando,
                    status_antes_envio,
                    false,
                )
                .await?;
            }
            Err(result.message())
        }
        other if (500..600).contains(&other) => {
            let _ = boleto_cas::transicionar(
                pool,
                boleto_id,
                StatusFinanceiro::BaixaEnviando,
                StatusFinanceiro::BaixaIndeterminada,
                false,
            )
            .await?;
            Err(format!("Baixa HTTP {other}"))
        }
        other => {
            marcar_erro(
                pool,
                boleto_id,
                "BAIXA_HTTP",
                &result.message(),
                Some(other as i32),
            )
            .await?;
            Err(format!("Baixa HTTP {other}"))
        }
    }
}
