//! Pagador (payer) management commands.
//!
//! Uses `documento` as the unique business key (NO `cliente_id`).
//! All queries target `autobo_pagadores` directly — no JOINs with `clientes`.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::db::AppState;
use sqlx::PgPool;

pub const PAGADOR_SELECT: &str = r#"
    id, tipo_pessoa, documento, nome, razao_social, nome_fantasia,
    telefone, email, cep, logradouro, numero, complemento,
    bairro, cidade, uf, prazo_vencimento_dias, tipo_cobranca_padrao,
    percentual_juros_mes::FLOAT8 as percentual_juros_mes,
    percentual_multa::FLOAT8 as percentual_multa,
    dias_protesto, mensagem_padrao, ativo, bloqueado, motivo_bloqueio,
    criado_em, atualizado_em
"#;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PagadorInput {
    pub tipo_pessoa: String,          // "PF" or "PJ"
    pub documento: String,            // CPF/CNPJ numbers only
    pub nome: Option<String>,
    pub razao_social: Option<String>,
    pub nome_fantasia: Option<String>,
    pub telefone: Option<String>,
    pub email: Option<String>,
    pub cep: Option<String>,
    pub logradouro: Option<String>,
    pub numero: Option<String>,
    pub complemento: Option<String>,
    pub bairro: Option<String>,
    pub cidade: Option<String>,
    pub uf: Option<String>,
    pub prazo_vencimento_dias: Option<i32>,
    pub tipo_cobranca_padrao: Option<String>,
    pub percentual_juros_mes: Option<f64>,
    pub percentual_multa: Option<f64>,
    pub dias_protesto: Option<i32>,
    pub mensagem_padrao: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PagadorRow {
    pub id: i64,
    pub tipo_pessoa: String,
    pub documento: String,
    pub nome: Option<String>,
    pub razao_social: Option<String>,
    pub nome_fantasia: Option<String>,
    pub telefone: Option<String>,
    pub email: Option<String>,
    pub cep: Option<String>,
    pub logradouro: Option<String>,
    pub numero: Option<String>,
    pub complemento: Option<String>,
    pub bairro: Option<String>,
    pub cidade: Option<String>,
    pub uf: Option<String>,
    pub prazo_vencimento_dias: i32,
    pub tipo_cobranca_padrao: String,
    pub percentual_juros_mes: Option<f64>,
    pub percentual_multa: Option<f64>,
    pub dias_protesto: Option<i32>,
    pub mensagem_padrao: Option<String>,
    pub ativo: bool,
    pub bloqueado: bool,
    pub motivo_bloqueio: Option<String>,
    pub criado_em: chrono::NaiveDateTime,
    pub atualizado_em: chrono::NaiveDateTime,
}

/// List all pagadores ordered by name (PF) then razão social (PJ).
/// No JOINs — queries `autobo_pagadores` directly.
#[tauri::command]
pub async fn listar_pagadores(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PagadorRow>, String> {
    sqlx::query_as::<_, PagadorRow>(
        &format!("SELECT {PAGADOR_SELECT} FROM autobo_pagadores ORDER BY nome NULLS LAST, razao_social NULLS LAST")
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Erro ao listar pagadores: {}", e))
}

/// Find a pagador by primary key `id`.
#[tauri::command]
pub async fn buscar_pagador(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<PagadorRow, String> {
    sqlx::query_as::<_, PagadorRow>(
        &format!("SELECT {PAGADOR_SELECT} FROM autobo_pagadores WHERE id = $1")
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("Erro ao buscar pagador: {}", e))?
    .ok_or_else(|| "Pagador não encontrado".into())
}

/// Find a pagador by business key `documento` (CPF/CNPJ).
#[tauri::command]
pub async fn buscar_pagador_por_documento(
    documento: String,
    state: tauri::State<'_, AppState>,
) -> Result<PagadorRow, String> {
    sqlx::query_as::<_, PagadorRow>(
        &format!("SELECT {PAGADOR_SELECT} FROM autobo_pagadores WHERE documento = $1")
    )
    .bind(&documento)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("Erro ao buscar pagador por documento: {}", e))?
    .ok_or_else(|| "Pagador não encontrado".into())
}

pub async fn upsert_pagador(pool: &PgPool, input: PagadorInput) -> Result<PagadorRow, String> {
    let prazo = input.prazo_vencimento_dias.unwrap_or(5);
    let tipo_cob = input.tipo_cobranca_padrao.as_deref().unwrap_or("SIMPLES");
    let juros = input.percentual_juros_mes.unwrap_or(1.0);
    let multa = input.percentual_multa.unwrap_or(2.0);

    sqlx::query_as::<_, PagadorRow>(&format!(
        r#"INSERT INTO autobo_pagadores (
            tipo_pessoa, documento, nome, razao_social, nome_fantasia,
            telefone, email, cep, logradouro, numero, complemento,
            bairro, cidade, uf, prazo_vencimento_dias, tipo_cobranca_padrao,
            percentual_juros_mes, percentual_multa, dias_protesto, mensagem_padrao
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)
        ON CONFLICT (documento) DO UPDATE SET
            tipo_pessoa = EXCLUDED.tipo_pessoa,
            nome = COALESCE(EXCLUDED.nome, autobo_pagadores.nome),
            razao_social = COALESCE(EXCLUDED.razao_social, autobo_pagadores.razao_social),
            nome_fantasia = COALESCE(EXCLUDED.nome_fantasia, autobo_pagadores.nome_fantasia),
            telefone = COALESCE(EXCLUDED.telefone, autobo_pagadores.telefone),
            email = COALESCE(EXCLUDED.email, autobo_pagadores.email),
            cep = COALESCE(EXCLUDED.cep, autobo_pagadores.cep),
            logradouro = COALESCE(EXCLUDED.logradouro, autobo_pagadores.logradouro),
            numero = COALESCE(EXCLUDED.numero, autobo_pagadores.numero),
            complemento = COALESCE(EXCLUDED.complemento, autobo_pagadores.complemento),
            bairro = COALESCE(EXCLUDED.bairro, autobo_pagadores.bairro),
            cidade = COALESCE(EXCLUDED.cidade, autobo_pagadores.cidade),
            uf = COALESCE(EXCLUDED.uf, autobo_pagadores.uf),
            prazo_vencimento_dias = EXCLUDED.prazo_vencimento_dias,
            tipo_cobranca_padrao = EXCLUDED.tipo_cobranca_padrao,
            percentual_juros_mes = EXCLUDED.percentual_juros_mes,
            percentual_multa = EXCLUDED.percentual_multa,
            dias_protesto = COALESCE(EXCLUDED.dias_protesto, autobo_pagadores.dias_protesto),
            mensagem_padrao = COALESCE(EXCLUDED.mensagem_padrao, autobo_pagadores.mensagem_padrao),
            atualizado_em = NOW()
        RETURNING {PAGADOR_SELECT}"#
    ))
    .bind(&input.tipo_pessoa)
    .bind(&input.documento)
    .bind(&input.nome)
    .bind(&input.razao_social)
    .bind(&input.nome_fantasia)
    .bind(&input.telefone)
    .bind(&input.email)
    .bind(&input.cep)
    .bind(&input.logradouro)
    .bind(&input.numero)
    .bind(&input.complemento)
    .bind(&input.bairro)
    .bind(&input.cidade)
    .bind(&input.uf)
    .bind(prazo)
    .bind(tipo_cob)
    .bind(juros)
    .bind(multa)
    .bind(input.dias_protesto)
    .bind(&input.mensagem_padrao)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Erro ao salvar pagador: {}", e))
}

/// Create or update a pagador.
#[tauri::command]
pub async fn salvar_pagador(
    input: PagadorInput,
    state: tauri::State<'_, AppState>,
) -> Result<PagadorRow, String> {
    upsert_pagador(&state.db, input).await
}

/// Toggle the `bloqueado` flag on a pagador.
///
/// When blocking (`bloquear = true`), `motivo` SHOULD be provided.
/// When unblocking, `motivo` is cleared (set to NULL).
#[tauri::command]
pub async fn bloquear_pagador(
    id: i64,
    bloquear: bool,
    motivo: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE autobo_pagadores SET bloqueado = $1, motivo_bloqueio = $2, atualizado_em = NOW() WHERE id = $3"
    )
    .bind(bloquear)
    .bind(&motivo)
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(|e| format!("Erro ao bloquear/desbloquear pagador: {}", e))?;
    Ok(())
}
