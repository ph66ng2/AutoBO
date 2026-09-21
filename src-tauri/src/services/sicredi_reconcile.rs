//! Liquidation sync cursor + startup orphan recovery.

use chrono::{Duration, Local, NaiveDate};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::services::boleto_cas;
use crate::services::boleto_status::{orphan_startup_target, StatusFinanceiro};
use crate::services::sicredi::{load_config, SicrediClient};

/// Recover REGISTRANDO / BAIXA_ENVIANDO orphans left by crash.
pub async fn recuperar_orfaos_startup(pool: &PgPool) -> Result<u64, String> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, status FROM autobo_boletos WHERE status IN ('REGISTRANDO', 'BAIXA_ENVIANDO')",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut n = 0u64;
    for (id, status) in rows {
        let atual = StatusFinanceiro::parse(&status).ok_or_else(|| status.clone())?;
        if let Some(alvo) = orphan_startup_target(atual) {
            let out = boleto_cas::transicionar(pool, id, atual, alvo, false).await?;
            if out.aplicada {
                n += 1;
                info!("Orphan {} {} → {}", id, atual.as_str(), alvo.as_str());
            }
        }
    }
    Ok(n)
}

async fn ensure_cursor(
    pool: &PgPool,
    ambiente: &str,
    beneficiario: &str,
) -> Result<NaiveDate, String> {
    if let Some(d) = sqlx::query_scalar::<_, NaiveDate>(
        "SELECT ultimo_dia_fechado FROM sicredi_sincronizacao_liquidacoes
         WHERE ambiente = $1 AND codigo_beneficiario = $2",
    )
    .bind(ambiente)
    .bind(beneficiario)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    {
        return Ok(d);
    }

    // Init: day before oldest Sicredi boleto for this convenio, or config date
    let oldest: Option<NaiveDate> = sqlx::query_scalar(
        "SELECT MIN(data_emissao) FROM autobo_boletos
         WHERE sicredi_ambiente = $1 AND sicredi_codigo_beneficiario = $2
           AND nosso_numero IS NOT NULL",
    )
    .bind(ambiente)
    .bind(beneficiario)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let init = if let Some(d) = oldest {
        d - Duration::days(1)
    } else {
        let cfg: Option<String> = sqlx::query_scalar(
            "SELECT valor FROM autobo_configuracoes WHERE chave = 'sicredi.data_entrada_producao'",
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
        match cfg.as_deref().filter(|s| !s.is_empty()) {
            Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| format!("sicredi.data_entrada_producao inválida: {s}"))?
                - Duration::days(1),
            None => Local::now().date_naive() - Duration::days(1),
        }
    };

    sqlx::query(
        "INSERT INTO sicredi_sincronizacao_liquidacoes (ambiente, codigo_beneficiario, ultimo_dia_fechado)
         VALUES ($1,$2,$3)
         ON CONFLICT (ambiente, codigo_beneficiario) DO NOTHING",
    )
    .bind(ambiente)
    .bind(beneficiario)
    .bind(init)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(init)
}

async fn avancar_cursor(
    pool: &PgPool,
    ambiente: &str,
    beneficiario: &str,
    dia: NaiveDate,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE sicredi_sincronizacao_liquidacoes
         SET ultimo_dia_fechado = $3, atualizado_em = NOW()
         WHERE ambiente = $1 AND codigo_beneficiario = $2
           AND ultimo_dia_fechado < $3",
    )
    .bind(ambiente)
    .bind(beneficiario)
    .bind(dia)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

async fn aplicar_liquidacao(
    pool: &PgPool,
    ambiente: &str,
    beneficiario: &str,
    nosso_numero: &str,
    valor: Option<f64>,
    data_pag: Option<NaiveDate>,
) -> Result<(), String> {
    let row: Option<(i64, String)> = sqlx::query_as(
        "SELECT id, status FROM autobo_boletos
         WHERE sicredi_ambiente = $1 AND sicredi_codigo_beneficiario = $2 AND nosso_numero = $3",
    )
    .bind(ambiente)
    .bind(beneficiario)
    .bind(nosso_numero)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    let Some((id, status_s)) = row else {
        return Ok(());
    };
    let atual = StatusFinanceiro::parse(&status_s).ok_or(status_s)?;
    if atual == StatusFinanceiro::Pago {
        return Ok(());
    }

    let out =
        boleto_cas::transicionar(pool, id, atual, StatusFinanceiro::Pago, true).await?;
    if out.aplicada {
        sqlx::query(
            "UPDATE autobo_boletos SET valor_pago = COALESCE($2, valor_pago), data_pagamento = COALESCE($3, data_pagamento), atualizado_em = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(valor)
        .bind(data_pag)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
        info!("Liquidação aplicada boleto {id} nosso={nosso_numero}");
    }
    Ok(())
}

async fn sync_dia(
    pool: &PgPool,
    client: &SicrediClient,
    ambiente: &str,
    beneficiario: &str,
    dia: NaiveDate,
) -> Result<(), String> {
    let mut pagina = 0;
    loop {
        let page = client.liquidados_dia(dia, pagina).await?;
        for item in &page.items {
            aplicar_liquidacao(
                pool,
                ambiente,
                beneficiario,
                &item.nosso_numero,
                item.valor_liquidado,
                item.data_pagamento.or(Some(dia)),
            )
            .await?;
        }
        if !page.has_next {
            break;
        }
        pagina += 1;
        if pagina > 100 {
            return Err("liquidados/dia: excesso de páginas".into());
        }
    }
    Ok(())
}

/// Full reconciliation: orphans, today, 3-day overlap, days after cursor before today.
pub async fn sincronizar_liquidacoes(pool: &PgPool) -> Result<(), String> {
    let _ = recuperar_orfaos_startup(pool).await;

    let config = match load_config(pool).await {
        Ok(c) => c,
        Err(e) => {
            warn!("Sicredi não configurado, skip sync: {e}");
            return Ok(());
        }
    };
    if config.codigo_beneficiario.is_empty() && !config.is_sandbox() {
        warn!("codigo_beneficiario vazio, skip sync");
        return Ok(());
    }

    let client = SicrediClient::new(config.clone());
    let ambiente = client.config().ambiente.clone();
    let beneficiario = client.config().codigo_beneficiario.clone();
    let hoje = Local::now().date_naive();

    let cursor = ensure_cursor(pool, &ambiente, &beneficiario).await?;

    // 1) Always re-query today (never advance cursor to today)
    if let Err(e) = sync_dia(pool, &client, &ambiente, &beneficiario, hoje).await {
        warn!("sync hoje: {e}");
    }

    // 2) Overlap last 3 days
    for i in 1..=3 {
        let d = hoje - Duration::days(i);
        if let Err(e) = sync_dia(pool, &client, &ambiente, &beneficiario, d).await {
            warn!("sync overlap {d}: {e}");
        }
    }

    // 3) Days after cursor and before today — advance one by one
    let mut dia = cursor + Duration::days(1);
    while dia < hoje {
        match sync_dia(pool, &client, &ambiente, &beneficiario, dia).await {
            Ok(()) => {
                avancar_cursor(pool, &ambiente, &beneficiario, dia).await?;
                info!("Cursor avançado para {dia}");
            }
            Err(e) => {
                warn!("Falha ao fechar dia {dia}: {e}");
                break;
            }
        }
        dia += Duration::days(1);
    }

    Ok(())
}
