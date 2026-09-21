//! Compare-and-set status transitions against Postgres.

use sqlx::PgPool;

use crate::services::boleto_status::{
    pode_aplicar, StatusFinanceiro, TransicaoResultado, apos_cas_zero,
};

pub struct CasOutcome {
    pub aplicada: bool,
    pub status_atual: StatusFinanceiro,
    pub resultado: TransicaoResultado,
}

/// Attempt CAS: expected → novo. On 0 rows, re-read and apply precedence.
pub async fn transicionar(
    pool: &PgPool,
    id: i64,
    esperado: StatusFinanceiro,
    novo: StatusFinanceiro,
    liquidacao_bancaria: bool,
) -> Result<CasOutcome, String> {
    if !pode_aplicar(esperado, novo, liquidacao_bancaria) && esperado != novo {
        return Ok(CasOutcome {
            aplicada: false,
            status_atual: esperado,
            resultado: TransicaoResultado::BloqueadaPorPrecedencia {
                atual: esperado,
                tentou: novo,
            },
        });
    }

    let res = sqlx::query(
        "UPDATE autobo_boletos SET status = $1, atualizado_em = NOW() WHERE id = $2 AND status = $3",
    )
    .bind(novo.as_str())
    .bind(id)
    .bind(esperado.as_str())
    .execute(pool)
    .await
    .map_err(|e| format!("CAS update: {e}"))?;

    if res.rows_affected() > 0 {
        return Ok(CasOutcome {
            aplicada: true,
            status_atual: novo,
            resultado: TransicaoResultado::Aplicada {
                de: esperado,
                para: novo,
            },
        });
    }

    let atual_s: String = sqlx::query_scalar("SELECT status FROM autobo_boletos WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Boleto não encontrado".to_string())?;

    let atual = StatusFinanceiro::parse(&atual_s)
        .ok_or_else(|| format!("Status desconhecido: {atual_s}"))?;

    // If liquidation and we can still apply from actual status, retry once
    if liquidacao_bancaria
        && novo == StatusFinanceiro::Pago
        && pode_aplicar(atual, StatusFinanceiro::Pago, true)
        && atual != StatusFinanceiro::Pago
    {
        let res2 = sqlx::query(
            "UPDATE autobo_boletos SET status = 'PAGO', atualizado_em = NOW() WHERE id = $1 AND status = $2",
        )
        .bind(id)
        .bind(atual.as_str())
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
        if res2.rows_affected() > 0 {
            return Ok(CasOutcome {
                aplicada: true,
                status_atual: StatusFinanceiro::Pago,
                resultado: TransicaoResultado::Aplicada {
                    de: atual,
                    para: StatusFinanceiro::Pago,
                },
            });
        }
    }

    let resultado = apos_cas_zero(atual, novo, liquidacao_bancaria);
    Ok(CasOutcome {
        aplicada: matches!(resultado, TransicaoResultado::Aplicada { .. }),
        status_atual: atual,
        resultado,
    })
}

pub async fn marcar_erro(
    pool: &PgPool,
    id: i64,
    codigo: &str,
    mensagem: &str,
    http_status: Option<i32>,
) -> Result<(), String> {
    let msg: String = mensagem.chars().take(500).collect();
    sqlx::query(
        r#"UPDATE autobo_boletos SET
            ultimo_erro_codigo = $2,
            ultimo_erro_mensagem = $3,
            ultimo_http_status = $4,
            atualizado_em = NOW()
         WHERE id = $1"#,
    )
    .bind(id)
    .bind(codigo)
    .bind(&msg)
    .bind(http_status)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn set_retry(
    pool: &PgPool,
    id: i64,
    operacao: &str,
    tentativas: i32,
    proxima: chrono::NaiveDateTime,
    erro_codigo: &str,
) -> Result<(), String> {
    sqlx::query(
        r#"UPDATE autobo_boletos SET
            status = 'AGUARDANDO_RETRY',
            operacao_pendente = $2,
            tentativas_operacao = $3,
            proxima_tentativa_em = $4,
            ultima_tentativa_em = NOW(),
            ultimo_erro_codigo = $5,
            atualizado_em = NOW()
         WHERE id = $1"#,
    )
    .bind(id)
    .bind(operacao)
    .bind(tentativas)
    .bind(proxima)
    .bind(erro_codigo)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
