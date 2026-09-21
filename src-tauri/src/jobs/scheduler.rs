//! Scheduled background jobs.

use sqlx::PgPool;
use tracing::{info, warn};

use crate::services::sicredi_reconcile;

/// Job 1: Sicredi reconciliation (liquidations + orphan recovery).
pub async fn job_atualizar_status(pool: &PgPool) {
    match sicredi_reconcile::sincronizar_liquidacoes(pool).await {
        Ok(()) => info!("Job Sicredi sync ok"),
        Err(e) => warn!("Job Sicredi sync: {e}"),
    }
}

pub async fn job_lembrete_vencimento(pool: &PgPool) {
    let boletos: Vec<(i64, String)> = sqlx::query_as(
        "SELECT b.id, p.email FROM autobo_boletos b
         JOIN autobo_pagadores p ON p.id = b.pagador_id
         WHERE b.status = 'REGISTRADO' AND b.data_vencimento = CURRENT_DATE + 1
         AND b.email_enviado = FALSE AND p.email IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    info!("Job lembrete D-1: {} boletos to remind", boletos.len());
}

pub async fn job_aviso_vencido(pool: &PgPool) {
    let boletos: Vec<(i64, String)> = sqlx::query_as(
        "SELECT b.id, p.email FROM autobo_boletos b
         JOIN autobo_pagadores p ON p.id = b.pagador_id
         WHERE b.status = 'VENCIDO' AND b.data_vencimento = CURRENT_DATE - 1
         AND p.email IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    info!("Job aviso D+1: {} overdue boletos", boletos.len());
}

pub async fn job_marcar_vencidos(pool: &PgPool) {
    let result = sqlx::query(
        "UPDATE autobo_boletos SET status = 'VENCIDO', atualizado_em = NOW()
         WHERE status = 'REGISTRADO' AND data_vencimento < CURRENT_DATE",
    )
    .execute(pool)
    .await;

    match result {
        Ok(r) => info!("Job marcar vencidos: {} boletos updated", r.rows_affected()),
        Err(e) => warn!("Job marcar vencidos error: {}", e),
    }
}

/// Process AGUARDANDO_RETRY when proxima_tentativa_em elapsed.
pub async fn job_retries(pool: &PgPool) {
    let rows: Vec<(i64, Option<String>)> = sqlx::query_as(
        "SELECT id, operacao_pendente FROM autobo_boletos
         WHERE status = 'AGUARDANDO_RETRY'
           AND (proxima_tentativa_em IS NULL OR proxima_tentativa_em <= NOW())
         LIMIT 20",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    for (id, op) in rows {
        match op.as_deref() {
            Some("BAIXA") => {
                // Move back to BAIXA_ENVIANDO then call flow — use pedir which checks status
                let _ = sqlx::query(
                    "UPDATE autobo_boletos SET status = 'REGISTRADO', atualizado_em = NOW()
                     WHERE id = $1 AND status = 'AGUARDANDO_RETRY' AND operacao_pendente = 'BAIXA'
                       AND status_anterior = 'REGISTRADO'",
                )
                .bind(id)
                .execute(pool)
                .await;
                let _ = sqlx::query(
                    "UPDATE autobo_boletos SET status = 'VENCIDO', atualizado_em = NOW()
                     WHERE id = $1 AND status = 'AGUARDANDO_RETRY' AND operacao_pendente = 'BAIXA'
                       AND status_anterior = 'VENCIDO'",
                )
                .bind(id)
                .execute(pool)
                .await;
                if let Err(e) = crate::services::boleto_sicredi_flow::pedir_baixa_sicredi(pool, id).await
                {
                    warn!("Retry baixa {id}: {e}");
                }
            }
            Some("CADASTRO") => {
                if let Err(e) =
                    crate::commands::boleto::retentar_rascunho_interno(pool, id).await
                {
                    warn!("Retry cadastro {id}: {e}");
                }
            }
            _ => {}
        }
    }
}

pub fn iniciar_scheduler(pool: PgPool) {
    use tokio_cron_scheduler::{Job, JobScheduler};

    let pool1 = pool.clone();
    let pool2 = pool.clone();
    let pool3 = pool.clone();
    let pool4 = pool.clone();
    let pool5 = pool.clone();
    let pool_boot = pool;

    tokio::spawn(async move {
        // Startup reconciliation
        if let Err(e) = sicredi_reconcile::sincronizar_liquidacoes(&pool_boot).await {
            warn!("Startup Sicredi sync: {e}");
        }

        let sched = JobScheduler::new().await.unwrap();

        let job1 = Job::new_async("0 0 * * * *", move |_uuid, _lock| {
            let pool = pool1.clone();
            Box::pin(async move {
                job_atualizar_status(&pool).await;
            })
        })
        .unwrap();

        let job2 = Job::new_async("0 0 8 * * *", move |_uuid, _lock| {
            let pool = pool2.clone();
            Box::pin(async move {
                job_lembrete_vencimento(&pool).await;
            })
        })
        .unwrap();

        let job3 = Job::new_async("0 0 9 * * *", move |_uuid, _lock| {
            let pool = pool3.clone();
            Box::pin(async move {
                job_aviso_vencido(&pool).await;
            })
        })
        .unwrap();

        let job4 = Job::new_async("0 5 0 * * *", move |_uuid, _lock| {
            let pool = pool4.clone();
            Box::pin(async move {
                job_marcar_vencidos(&pool).await;
            })
        })
        .unwrap();

        // Retries every 5 minutes
        let job5 = Job::new_async("0 */5 * * * *", move |_uuid, _lock| {
            let pool = pool5.clone();
            Box::pin(async move {
                job_retries(&pool).await;
            })
        })
        .unwrap();

        sched.add(job1).await.unwrap();
        sched.add(job2).await.unwrap();
        sched.add(job3).await.unwrap();
        sched.add(job4).await.unwrap();
        sched.add(job5).await.unwrap();

        sched.start().await.unwrap();
        info!("Scheduler started with 5 jobs + startup sync");
    });
}
