//! Boleto commands — generate (local PDF), list, query, cancel.
//!
//! Sicredi registration remains a later wave. The PDF is the shareable artifact
//! for the Aline MVP.

use crate::commands::pagador::{upsert_pagador, PagadorInput, PAGADOR_SELECT};
use crate::db::AppState;
use crate::services::boleto_pdf::{abrir_arquivo, caminho_pdf_boleto, gerar_pdf_boleto, BoletoPdfDados};
use crate::services::boleto_sicredi_flow::{self, PagadorDados};
use crate::services::sicredi::load_config;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub const BOLETO_SELECT: &str = r#"
    id, pagador_id, nfe_id, seu_numero, nosso_numero,
    valor_nominal::FLOAT8 as valor_nominal,
    valor_pago::FLOAT8 as valor_pago,
    data_emissao, data_vencimento, data_pagamento,
    tipo_cobranca, especie_documento, tipo_juros,
    percentual_juros_mes::FLOAT8 as percentual_juros_mes,
    tipo_multa,
    percentual_multa::FLOAT8 as percentual_multa,
    dias_protesto, mensagem, linha_digitavel, codigo_barras,
    txid, qr_code, status, email_enviado, whatsapp_enviado,
    origem, gerado_por, criado_em, atualizado_em,
    sicredi_ambiente, sicredi_codigo_beneficiario, sicredi_seu_numero,
    sicredi_id_titulo_empresa, pdf_pendente, pdf_oficial_path,
    status_anterior, operacao_pendente, tentativas_operacao,
    ultimo_erro_codigo, ultimo_erro_mensagem, ultimo_http_status
"#;

pub const ITEM_BOLETO_SELECT: &str = r#"
    id, boleto_id, descricao,
    quantidade::FLOAT8 as quantidade,
    COALESCE(unidade, 'UN') as unidade,
    valor_unitario::FLOAT8 as valor_unitario,
    subtotal::FLOAT8 as subtotal,
    produto_autoos_id
"#;

// ─── Input Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BoletoInput {
    pub pagador_id: Option<i64>,
    pub pagador: Option<PagadorInput>,
    pub nfe_id: Option<i64>,
    pub nfe: Option<NfeResumoInput>,
    pub origem: String,
    pub seu_numero: Option<String>,
    pub valor_nominal: f64,
    pub data_vencimento: String,
    pub tipo_cobranca: Option<String>,
    pub especie_documento: Option<String>,
    pub tipo_juros: Option<String>,
    pub percentual_juros_mes: Option<f64>,
    pub tipo_multa: Option<String>,
    pub percentual_multa: Option<f64>,
    pub dias_protesto: Option<i32>,
    pub mensagem: Option<String>,
    /// When true (default), attempt Sicredi registration if configured.
    #[serde(default = "default_true")]
    pub registrar_sicredi: bool,
    #[serde(default)]
    pub itens: Vec<ItemInput>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NfeResumoInput {
    pub numero_nf: String,
    pub serie: Option<String>,
    pub chave_acesso: String,
    pub data_emissao: String,
    pub valor_total: f64,
    pub natureza_operacao: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemInput {
    pub descricao: String,
    pub quantidade: f64,
    pub unidade: Option<String>,
    pub valor_unitario: f64,
    pub subtotal: f64,
    pub produto_autoos_id: Option<i64>,
}

// ─── Row Types (matching DB tables) ──────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct BoletoRow {
    pub id: i64,
    pub pagador_id: i64,
    pub nfe_id: Option<i64>,
    pub seu_numero: String,
    pub nosso_numero: Option<String>,
    pub valor_nominal: f64,
    pub valor_pago: Option<f64>,
    pub data_emissao: chrono::NaiveDate,
    pub data_vencimento: chrono::NaiveDate,
    pub data_pagamento: Option<chrono::NaiveDate>,
    pub tipo_cobranca: String,
    pub especie_documento: String,
    pub tipo_juros: Option<String>,
    pub percentual_juros_mes: Option<f64>,
    pub tipo_multa: Option<String>,
    pub percentual_multa: Option<f64>,
    pub dias_protesto: Option<i32>,
    pub mensagem: Option<String>,
    pub linha_digitavel: Option<String>,
    pub codigo_barras: Option<String>,
    pub txid: Option<String>,
    pub qr_code: Option<String>,
    pub status: String,
    pub email_enviado: bool,
    pub whatsapp_enviado: bool,
    pub origem: String,
    pub gerado_por: Option<String>,
    pub criado_em: chrono::NaiveDateTime,
    pub atualizado_em: chrono::NaiveDateTime,
    pub sicredi_ambiente: Option<String>,
    pub sicredi_codigo_beneficiario: Option<String>,
    pub sicredi_seu_numero: Option<String>,
    pub sicredi_id_titulo_empresa: Option<String>,
    pub pdf_pendente: bool,
    pub pdf_oficial_path: Option<String>,
    pub status_anterior: Option<String>,
    pub operacao_pendente: Option<String>,
    pub tentativas_operacao: i32,
    pub ultimo_erro_codigo: Option<String>,
    pub ultimo_erro_mensagem: Option<String>,
    pub ultimo_http_status: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ItemBoletoRow {
    pub id: i64,
    pub boleto_id: i64,
    pub descricao: String,
    pub quantidade: f64,
    pub unidade: String,
    pub valor_unitario: f64,
    pub subtotal: f64,
    pub produto_autoos_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct BoletoListaRow {
    pub id: i64,
    pub pagador_id: i64,
    pub nfe_id: Option<i64>,
    pub seu_numero: String,
    pub nosso_numero: Option<String>,
    pub valor_nominal: f64,
    pub data_emissao: chrono::NaiveDate,
    pub data_vencimento: chrono::NaiveDate,
    pub data_pagamento: Option<chrono::NaiveDate>,
    pub tipo_cobranca: String,
    pub status: String,
    pub origem: String,
    pub mensagem: Option<String>,
    pub pagador_nome: Option<String>,
    pub pagador_documento: Option<String>,
}

// ─── Commands ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn listar_boletos(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<BoletoListaRow>, String> {
    sqlx::query_as::<_, BoletoListaRow>(
        r#"
        SELECT
            b.id, b.pagador_id, b.nfe_id, b.seu_numero, b.nosso_numero,
            b.valor_nominal::FLOAT8 as valor_nominal,
            b.data_emissao, b.data_vencimento, b.data_pagamento,
            b.tipo_cobranca, b.status, b.origem, b.mensagem,
            COALESCE(NULLIF(p.nome, ''), NULLIF(p.razao_social, ''), p.nome_fantasia) as pagador_nome,
            p.documento as pagador_documento
        FROM autobo_boletos b
        LEFT JOIN autobo_pagadores p ON p.id = b.pagador_id
        ORDER BY b.criado_em DESC
        LIMIT 500
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Erro ao listar boletos: {}", e))
}

#[tauri::command]
pub async fn buscar_boleto(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let boleto = sqlx::query_as::<_, BoletoRow>(&format!(
        "SELECT {BOLETO_SELECT} FROM autobo_boletos WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| format!("Erro: {}", e))?
    .ok_or("Boleto não encontrado")?;

    let itens = sqlx::query_as::<_, ItemBoletoRow>(&format!(
        "SELECT {ITEM_BOLETO_SELECT} FROM autobo_itens_boleto WHERE boleto_id = $1"
    ))
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| format!("Erro ao buscar itens: {}", e))?;

    Ok(serde_json::json!({
        "boleto": boleto,
        "itens": itens
    }))
}

#[tauri::command]
pub async fn cancelar_boleto(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let status: String = sqlx::query_scalar("SELECT status FROM autobo_boletos WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("Erro: {e}"))?
        .ok_or("Boleto não encontrado")?;

    // Local-only cancel for drafts / payload errors without nosso_numero
    let nosso: Option<String> =
        sqlx::query_scalar("SELECT nosso_numero FROM autobo_boletos WHERE id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| e.to_string())?;

    if nosso.is_none()
        && matches!(
            status.as_str(),
            "RASCUNHO" | "ERRO_PAYLOAD" | "REGISTRO_INDETERMINADO"
        )
    {
        sqlx::query(
            "UPDATE autobo_boletos SET status = 'CANCELADO', atualizado_em = NOW()
             WHERE id = $1 AND status = $2",
        )
        .bind(id)
        .bind(&status)
        .execute(&state.db)
        .await
        .map_err(|e| format!("Erro ao cancelar: {e}"))?;
        return Ok(());
    }

    boleto_sicredi_flow::pedir_baixa_sicredi(&state.db, id).await
}

/// Remove o boleto (e a NF-e ligada) para poder testar o import de novo.
#[tauri::command]
pub async fn excluir_boleto(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| format!("Erro ao iniciar exclusão: {e}"))?;

    let nfe_id: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT nfe_id FROM autobo_boletos WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Erro ao localizar boleto: {e}"))?
    .flatten();

    sqlx::query("UPDATE autobo_nfes SET boleto_id = NULL WHERE boleto_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Erro ao desvincular NF-e: {e}"))?;

    let deleted = sqlx::query("DELETE FROM autobo_boletos WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Erro ao excluir boleto: {e}"))?;
    if deleted.rows_affected() == 0 {
        return Err("Boleto não encontrado".into());
    }

    if let Some(nfe_id) = nfe_id {
        sqlx::query("DELETE FROM autobo_nfes WHERE id = $1")
            .bind(nfe_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Erro ao excluir NF-e ligada: {e}"))?;
    }

    tx.commit()
        .await
        .map_err(|e| format!("Erro ao confirmar exclusão: {e}"))?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct GerarBoletoResult {
    pub boleto: BoletoRow,
    pub pdf_path: String,
    #[serde(default)]
    pub avisos: Vec<String>,
}

fn digits_only(value: &str) -> String {
    value.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn normalize_origem(origem: &str) -> String {
    match origem.trim().to_uppercase().as_str() {
        "NFE" | "NF-E" | "NF" => "NFE".into(),
        _ => "MANUAL".into(),
    }
}

fn normalize_tipo_cobranca(tipo: Option<&str>) -> String {
    match tipo.map(|t| t.trim().to_uppercase()).as_deref() {
        Some("RECORRENTE") => "RECORRENTE".into(),
        Some("UNICA") | Some("ÚNICA") => "UNICA".into(),
        _ => "SIMPLES".into(),
    }
}

fn validar_rascunho_boleto(valor_nominal: f64, data_vencimento: &str) -> Result<(), String> {
    if valor_nominal <= 0.0 {
        return Err("Valor nominal deve ser maior que zero".into());
    }
    if data_vencimento.trim().is_empty() {
        return Err("Informe o vencimento.".into());
    }
    Ok(())
}

fn validar_documento_pagador(documento: &str) -> Result<(), String> {
    if documento.is_empty() {
        return Err("Documento do pagador é obrigatório".into());
    }
    Ok(())
}

fn tipo_pessoa_from_doc(documento: &str) -> String {
    if documento.len() <= 11 {
        "PF".into()
    } else {
        "PJ".into()
    }
}

#[tauri::command]
pub async fn gerar_boleto(
    input: BoletoInput,
    state: tauri::State<'_, AppState>,
) -> Result<GerarBoletoResult, String> {
    validar_rascunho_boleto(input.valor_nominal, &input.data_vencimento)?;

    let origem = normalize_origem(&input.origem);
    let mut itens = input.itens.clone();
    if itens.is_empty() {
        itens.push(ItemInput {
            descricao: input
                .mensagem
                .clone()
                .filter(|m| !m.trim().is_empty())
                .unwrap_or_else(|| "Cobrança".into()),
            quantidade: 1.0,
            unidade: Some("UN".into()),
            valor_unitario: input.valor_nominal,
            subtotal: input.valor_nominal,
            produto_autoos_id: None,
        });
    }

    let pagador = if let Some(id) = input.pagador_id {
        sqlx::query_as::<_, crate::commands::pagador::PagadorRow>(&format!(
            "SELECT {PAGADOR_SELECT} FROM autobo_pagadores WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("Erro ao buscar pagador: {e}"))?
        .ok_or("Pagador não encontrado")?
    } else {
        let dados = input
            .pagador
            .clone()
            .ok_or("Informe o pagador (documento e nome)")?;
        let mut dados = dados;
        dados.documento = digits_only(&dados.documento);
        validar_documento_pagador(&dados.documento)?;
        if dados.tipo_pessoa.trim().is_empty() {
            dados.tipo_pessoa = tipo_pessoa_from_doc(&dados.documento);
        }
        upsert_pagador(&state.db, dados).await?
    };

    if pagador.bloqueado {
        return Err("Pagador bloqueado — não é possível gerar boleto".into());
    }

    let mut nfe_id = input.nfe_id;
    if origem == "NFE" {
        if nfe_id.is_none() {
            if let Some(nfe) = &input.nfe {
                nfe_id = Some(
                    sqlx::query_scalar::<_, i64>(
                        r#"INSERT INTO autobo_nfes (
                            pagador_id, numero_nf, serie, chave_acesso, data_emissao,
                            valor_total, natureza_operacao, status
                        ) VALUES ($1,$2,$3,$4,$5::date,$6,$7,'IMPORTADA')
                        ON CONFLICT (chave_acesso) DO UPDATE SET
                            pagador_id = EXCLUDED.pagador_id,
                            status = autobo_nfes.status
                        RETURNING id"#,
                    )
                    .bind(pagador.id)
                    .bind(&nfe.numero_nf)
                    .bind(&nfe.serie)
                    .bind(&nfe.chave_acesso)
                    .bind(&nfe.data_emissao)
                    .bind(nfe.valor_total)
                    .bind(&nfe.natureza_operacao)
                    .fetch_one(&state.db)
                    .await
                    .map_err(|e| format!("Erro ao gravar NF-e: {e}"))?,
                );
            }
        }
        if let Some(id) = nfe_id {
            let nfe_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM autobo_nfes WHERE id = $1)",
            )
            .bind(id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| format!("Erro ao verificar NF-e: {e}"))?;
            if !nfe_exists {
                return Err(format!("NF-e id={id} não encontrada"));
            }
        }
    }

    let seu_numero = input.seu_numero.unwrap_or_else(|| {
        if origem == "NFE" {
            format!("NF-{}", nfe_id.unwrap_or(0))
        } else {
            format!("MAN-{}", chrono::Local::now().format("%Y%m%d-%H%M%S"))
        }
    });

    let boleto = sqlx::query_as::<_, BoletoRow>(&format!(
        r#"INSERT INTO autobo_boletos (
            pagador_id, nfe_id, seu_numero, valor_nominal, data_vencimento,
            tipo_cobranca, especie_documento, tipo_juros, percentual_juros_mes,
            tipo_multa, percentual_multa, dias_protesto, mensagem,
            origem, status, gerado_por
        ) VALUES ($1,$2,$3,$4,$5::date,$6,$7,$8,$9,$10,$11,$12,$13,$14,'RASCUNHO','Aline')
        RETURNING {BOLETO_SELECT}"#
    ))
    .bind(pagador.id)
    .bind(nfe_id)
    .bind(&seu_numero)
    .bind(input.valor_nominal)
    .bind(&input.data_vencimento)
    .bind(normalize_tipo_cobranca(input.tipo_cobranca.as_deref()))
    .bind(
        input
            .especie_documento
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("DUPLICATA_MERCANTIL_INDICACAO"),
    )
    .bind(input.tipo_juros.as_deref())
    .bind(input.percentual_juros_mes)
    .bind(input.tipo_multa.as_deref())
    .bind(input.percentual_multa)
    .bind(input.dias_protesto)
    .bind(&input.mensagem)
    .bind(&origem)
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("Erro ao criar boleto: {e}"))?;

    for item in &itens {
        let subtotal = if item.subtotal > 0.0 {
            item.subtotal
        } else {
            item.quantidade * item.valor_unitario
        };
        sqlx::query(
            "INSERT INTO autobo_itens_boleto (boleto_id, descricao, quantidade, unidade, valor_unitario, subtotal, produto_autoos_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(boleto.id)
        .bind(&item.descricao)
        .bind(item.quantidade)
        .bind(item.unidade.as_deref().unwrap_or("UN"))
        .bind(item.valor_unitario)
        .bind(subtotal)
        .bind(item.produto_autoos_id)
        .execute(&state.db)
        .await
        .map_err(|e| format!("Erro ao inserir item: {e}"))?;
    }

    if origem == "NFE" {
        if let Some(id) = nfe_id {
            sqlx::query(
                "UPDATE autobo_nfes SET boleto_id = $1, status = 'BOLETO_GERADO', pagador_id = $2 WHERE id = $3",
            )
            .bind(boleto.id)
            .bind(pagador.id)
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(|e| format!("Erro ao atualizar NF-e: {e}"))?;
        }
    }

    let nome_pagador = pagador
        .nome
        .clone()
        .or(pagador.razao_social.clone())
        .unwrap_or_else(|| pagador.documento.clone());

    let pdf_path = gerar_pdf_boleto(BoletoPdfDados {
        seu_numero: &boleto.seu_numero,
        pagador: &nome_pagador,
        documento: &pagador.documento,
        valor: boleto.valor_nominal,
        vencimento: &boleto.data_vencimento.to_string(),
        origem: &origem,
        mensagem: input.mensagem.as_deref(),
    })?;

    // Attempt Sicredi registration when configured
    let mut boleto = boleto;
    let mut avisos: Vec<String> = vec![];
    if input.registrar_sicredi {
        match load_config(&state.db).await {
            Ok(_) => {
                let valor_dec = Decimal::from_f64_retain(input.valor_nominal)
                    .ok_or("Valor nominal inválido")?;
                let pj = input
                    .percentual_juros_mes
                    .and_then(Decimal::from_f64_retain);
                let pm = input
                    .percentual_multa
                    .and_then(Decimal::from_f64_retain);
                let dados = PagadorDados {
                    tipo_pessoa: pagador.tipo_pessoa.clone(),
                    documento: pagador.documento.clone(),
                    nome: nome_pagador.clone(),
                    logradouro: pagador.logradouro.clone().unwrap_or_default(),
                    numero: pagador.numero.clone().unwrap_or_default(),
                    cidade: pagador.cidade.clone().unwrap_or_default(),
                    uf: pagador.uf.clone().unwrap_or_default(),
                    cep: pagador.cep.clone().unwrap_or_default(),
                    telefone: pagador.telefone.clone(),
                    email: pagador.email.clone(),
                };
                match boleto_sicredi_flow::registrar_boleto_sicredi(
                    &state.db,
                    boleto.id,
                    dados,
                    valor_dec,
                    &input.data_vencimento,
                    input.especie_documento.as_deref(),
                    input.mensagem.as_deref(),
                    input.tipo_juros.as_deref(),
                    pj,
                    input.tipo_multa.as_deref(),
                    pm,
                )
                .await
                {
                    Ok(()) => {
                        boleto = sqlx::query_as::<_, BoletoRow>(&format!(
                            "SELECT {BOLETO_SELECT} FROM autobo_boletos WHERE id = $1"
                        ))
                        .bind(boleto.id)
                        .fetch_one(&state.db)
                        .await
                        .map_err(|e| e.to_string())?;
                    }
                    Err(e) => {
                        avisos.push(e);
                        boleto = sqlx::query_as::<_, BoletoRow>(&format!(
                            "SELECT {BOLETO_SELECT} FROM autobo_boletos WHERE id = $1"
                        ))
                        .bind(boleto.id)
                        .fetch_one(&state.db)
                        .await
                        .map_err(|e| e.to_string())?;
                    }
                }
            }
            Err(_) => {
                // Not configured — keep local draft PDF
            }
        }
    }

    Ok(GerarBoletoResult {
        boleto,
        pdf_path: pdf_path.to_string_lossy().into_owned(),
        avisos,
    })
}

#[tauri::command]
pub async fn abrir_pdf_boleto(path: String) -> Result<(), String> {
    abrir_arquivo(&path)
}

#[tauri::command]
pub async fn abrir_pdf_do_boleto(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let seu_numero: String = sqlx::query_scalar("SELECT seu_numero FROM autobo_boletos WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("Erro ao localizar boleto: {e}"))?
        .ok_or_else(|| "Boleto não encontrado".to_string())?;

    let path = caminho_pdf_boleto(&seu_numero)?;
    let oficial: Option<String> = sqlx::query_scalar(
        "SELECT pdf_oficial_path FROM autobo_boletos WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .flatten();
    if let Some(ref p) = oficial {
        let op = std::path::PathBuf::from(p);
        if op.exists() {
            return abrir_arquivo(p);
        }
    }
    if !path.exists() {
        return Err(format!(
            "Não achei o PDF deste boleto em {}. Gere o boleto de novo se o arquivo foi apagado.",
            path.display()
        ));
    }
    abrir_arquivo(&path.to_string_lossy())
}

#[tauri::command]
pub async fn retentar_rascunho(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<BoletoRow, String> {
    retentar_rascunho_interno(&state.db, id).await
}

pub async fn retentar_rascunho_interno(
    pool: &sqlx::PgPool,
    id: i64,
) -> Result<BoletoRow, String> {
    let boleto = sqlx::query_as::<_, BoletoRow>(&format!(
        "SELECT {BOLETO_SELECT} FROM autobo_boletos WHERE id = $1 AND status IN ('RASCUNHO', 'ERRO_PAYLOAD', 'AGUARDANDO_RETRY')"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Erro: {}", e))?
    .ok_or("Boleto não encontrado ou status não permite retentativa")?;

    // If retry, move to RASCUNHO first via CAS-ish update
    if boleto.status == "AGUARDANDO_RETRY" {
        sqlx::query(
            "UPDATE autobo_boletos SET status = 'RASCUNHO', atualizado_em = NOW()
             WHERE id = $1 AND status = 'AGUARDANDO_RETRY' AND operacao_pendente = 'CADASTRO'",
        )
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    let pagador = sqlx::query_as::<_, crate::commands::pagador::PagadorRow>(&format!(
        "SELECT {PAGADOR_SELECT} FROM autobo_pagadores WHERE id = $1"
    ))
    .bind(boleto.pagador_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let nome = pagador
        .nome
        .clone()
        .or(pagador.razao_social.clone())
        .unwrap_or_else(|| pagador.documento.clone());

    let valor = Decimal::from_f64_retain(boleto.valor_nominal).ok_or("Valor inválido")?;
    boleto_sicredi_flow::registrar_boleto_sicredi(
        pool,
        boleto.id,
        PagadorDados {
            tipo_pessoa: pagador.tipo_pessoa,
            documento: pagador.documento,
            nome,
            logradouro: pagador.logradouro.unwrap_or_default(),
            numero: pagador.numero.unwrap_or_default(),
            cidade: pagador.cidade.unwrap_or_default(),
            uf: pagador.uf.unwrap_or_default(),
            cep: pagador.cep.unwrap_or_default(),
            telefone: pagador.telefone,
            email: pagador.email,
        },
        valor,
        &boleto.data_vencimento.to_string(),
        Some(&boleto.especie_documento),
        boleto.mensagem.as_deref(),
        boleto.tipo_juros.as_deref(),
        boleto
            .percentual_juros_mes
            .and_then(Decimal::from_f64_retain),
        boleto.tipo_multa.as_deref(),
        boleto.percentual_multa.and_then(Decimal::from_f64_retain),
    )
    .await?;

    sqlx::query_as::<_, BoletoRow>(&format!(
        "SELECT {BOLETO_SELECT} FROM autobo_boletos WHERE id = $1"
    ))
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn retentar_pdf_oficial(
    id: i64,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    boleto_sicredi_flow::retentar_pdf(&state.db, id).await
}

#[tauri::command]
pub async fn sincronizar_sicredi_agora(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    crate::services::sicredi_reconcile::sincronizar_liquidacoes(&state.db).await
}

#[cfg(test)]
mod tests {
    use super::{validar_documento_pagador, validar_rascunho_boleto};

    #[test]
    fn rejeita_valor_nao_positivo_antes_de_persistir() {
        let err = validar_rascunho_boleto(0.0, "2026-10-21").unwrap_err();
        assert!(err.contains("maior que zero"));
    }

    #[test]
    fn rejeita_vencimento_vazio() {
        let err = validar_rascunho_boleto(10.0, "  ").unwrap_err();
        assert!(err.contains("vencimento"));
    }

    #[test]
    fn rejeita_documento_vazio() {
        let err = validar_documento_pagador("").unwrap_err();
        assert!(err.contains("Documento"));
    }

    #[test]
    fn aceita_rascunho_sintetico() {
        validar_rascunho_boleto(89.9, "2026-11-01").unwrap();
        validar_documento_pagador("11222333000181").unwrap();
    }
}
