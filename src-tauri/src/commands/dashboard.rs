//! Dashboard commands: metrics and top debtors.

use crate::db::AppState;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow)]
pub struct DashboardMetricas {
    pub boletos_gerados: i64,
    pub boletos_pagos: i64,
    pub valor_a_receber: Option<f64>,
    pub valor_recebido: Option<f64>,
    pub boletos_vencidos: i64,
}

#[tauri::command]
pub async fn metricas_dashboard(
    state: tauri::State<'_, AppState>,
) -> Result<DashboardMetricas, String> {
    sqlx::query_as::<_, DashboardMetricas>(
        r#"SELECT
            COUNT(*) FILTER (WHERE status IN ('RASCUNHO','REGISTRADO','VENCIDO')) as boletos_gerados,
            COUNT(*) FILTER (WHERE status = 'PAGO') as boletos_pagos,
            COALESCE(SUM(valor_nominal) FILTER (WHERE status IN ('RASCUNHO','REGISTRADO','VENCIDO')), 0)::FLOAT8 as valor_a_receber,
            COALESCE(SUM(valor_pago) FILTER (WHERE status = 'PAGO'), 0)::FLOAT8 as valor_recebido,
            COUNT(*) FILTER (WHERE status = 'VENCIDO') as boletos_vencidos
        FROM autobo_boletos"#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("Erro ao carregar métricas: {}", e))
}

#[derive(Debug, Serialize, FromRow)]
pub struct TopDevedor {
    pub pagador_id: i64,
    pub documento: String,
    pub nome: Option<String>,
    pub razao_social: Option<String>,
    pub total_em_aberto: Option<f64>,
    pub quantidade_boletos: i64,
}

#[tauri::command]
pub async fn top_devedores(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<TopDevedor>, String> {
    sqlx::query_as::<_, TopDevedor>(
        r#"SELECT
            p.id as pagador_id,
            p.documento,
            p.nome,
            p.razao_social,
            SUM(b.valor_nominal)::FLOAT8 as total_em_aberto,
            COUNT(b.id) as quantidade_boletos
        FROM autobo_boletos b
        JOIN autobo_pagadores p ON p.id = b.pagador_id
        WHERE b.status IN ('REGISTRADO', 'VENCIDO')
        GROUP BY p.id, p.documento, p.nome, p.razao_social
        ORDER BY total_em_aberto DESC
        LIMIT 5"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Erro ao carregar top devedores: {}", e))
}
