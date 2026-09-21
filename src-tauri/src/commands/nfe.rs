use crate::db::AppState;
use crate::services::danfe_pdf::{extrair_texto_pdf, parse_danfe};
use crate::services::nfe_parser::{parse_nfe_xml, NFeDados};
use crate::validators::{cnpj::validar_cnpj, cpf::validar_cpf};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sqlx::PgPool;

#[derive(serde::Serialize)]
pub struct NFeImportadaDTO {
    pub sucesso: bool,
    pub dados: Option<NFeDados>,
    pub erro: Option<String>,
    pub avisos: Vec<String>,
}

#[tauri::command]
pub async fn importar_nfe(
    xml_base64: String,
    state: tauri::State<'_, AppState>,
) -> Result<NFeImportadaDTO, String> {
    importar_nfe_inner(&xml_base64, &state.db).await
}

#[tauri::command]
pub async fn importar_nfe_lote(
    xmls_base64: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<NFeImportadaDTO>, String> {
    let mut resultados = Vec::new();
    for xml_b64 in xmls_base64 {
        let result = importar_nfe_inner(&xml_b64, &state.db).await;
        resultados.push(match result {
            Ok(dto) => dto,
            Err(err) => NFeImportadaDTO {
                sucesso: false,
                dados: None,
                erro: Some(err),
                avisos: vec![],
            },
        });
    }
    Ok(resultados)
}

async fn importar_nfe_inner(
    xml_base64: &str,
    db: &PgPool,
) -> Result<NFeImportadaDTO, String> {
    let xml_bytes = BASE64
        .decode(xml_base64)
        .map_err(|e| format!("Erro ao decodificar base64: {}", e))?;
    let xml_str = String::from_utf8(xml_bytes)
        .map_err(|e| format!("XML inválido (não é UTF-8): {}", e))?;

    let dados = parse_nfe_xml(&xml_str)
        .map_err(|e| format!("Erro ao processar NF-e: {}", e))?;

    let mut avisos = Vec::new();
    let doc = &dados.destinatario.documento;
    if doc.len() == 11 && !validar_cpf(doc) {
        avisos.push("CPF inválido".into());
    } else if doc.len() == 14 && !validar_cnpj(doc) {
        avisos.push("CNPJ inválido".into());
    }

    verificar_duplicidade(&dados.chave_acesso, db).await?;

    Ok(NFeImportadaDTO {
        sucesso: true,
        dados: Some(dados),
        erro: None,
        avisos,
    })
}

async fn verificar_duplicidade(chave_acesso: &str, db: &PgPool) -> Result<(), String> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM autobo_nfes WHERE chave_acesso = $1)",
    )
    .bind(chave_acesso)
    .fetch_one(db)
    .await
    .map_err(|e| format!("Erro ao verificar duplicidade: {}", e))?;

    if exists {
        return Err(format!("NF-e já importada: chave de acesso {chave_acesso}"));
    }
    Ok(())
}

/// Pré-preenchimento a partir de DANFE ou NFS-e em PDF (atalho de digitação).
/// PDFs-imagem (Nota Salvador) passam por OCR. A conferência fica na revisão.
#[tauri::command]
pub async fn importar_danfe_pdf(
    pdf_base64: String,
    state: tauri::State<'_, AppState>,
) -> Result<NFeImportadaDTO, String> {
    let pdf_bytes = BASE64
        .decode(pdf_base64)
        .map_err(|e| format!("Erro ao decodificar base64: {}", e))?;

    let extraido = tokio::task::spawn_blocking(move || {
        let texto = extrair_texto_pdf(&pdf_bytes)?;
        parse_danfe(&texto)
    })
    .await
    .map_err(|e| format!("OCR interrompido: {e}"))??;

    verificar_duplicidade(&extraido.dados.chave_acesso, &state.db).await?;

    Ok(NFeImportadaDTO {
        sucesso: true,
        dados: Some(extraido.dados),
        erro: None,
        avisos: extraido.avisos,
    })
}
