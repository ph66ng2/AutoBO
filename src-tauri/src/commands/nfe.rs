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

fn decodificar_e_parsear_nfe(xml_base64: &str) -> Result<NFeDados, String> {
    let xml_bytes = BASE64
        .decode(xml_base64)
        .map_err(|e| format!("Erro ao decodificar base64: {}", e))?;
    let xml_str = String::from_utf8(xml_bytes)
        .map_err(|e| format!("XML inválido (não é UTF-8): {}", e))?;
    parse_nfe_xml(&xml_str).map_err(|e| format!("Erro ao processar NF-e: {}", e))
}

fn rejeitar_chave_duplicada(ja_importada: bool, chave_acesso: &str) -> Result<(), String> {
    if ja_importada {
        Err(format!("NF-e já importada: chave de acesso {chave_acesso}"))
    } else {
        Ok(())
    }
}

async fn importar_nfe_inner(
    xml_base64: &str,
    db: &PgPool,
) -> Result<NFeImportadaDTO, String> {
    let dados = decodificar_e_parsear_nfe(xml_base64)?;

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

    rejeitar_chave_duplicada(exists, chave_acesso)
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

pub const STATUS_NOTA_IMPORTADA: &str = "IMPORTADA";

/// Grava somente nota importada. Emissão fiscal não usa `autobo_nfes`.
pub fn sql_registrar_nota_importada() -> &'static str {
    r#"INSERT INTO autobo_nfes (
                            pagador_id, numero_nf, serie, chave_acesso, data_emissao,
                            valor_total, natureza_operacao, status
                        ) VALUES ($1,$2,$3,$4,$5::date,$6,$7,'IMPORTADA')
                        ON CONFLICT (chave_acesso) DO UPDATE SET
                            pagador_id = EXCLUDED.pagador_id,
                            status = autobo_nfes.status
                        RETURNING id"#
}

#[cfg(test)]
mod tests {
    use super::{decodificar_e_parsear_nfe, rejeitar_chave_duplicada};
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    #[test]
    fn parser_invalido_nao_devolve_dados_para_persistir() {
        let xml = BASE64.encode("<nao-e-nfe>");
        let erro = decodificar_e_parsear_nfe(&xml).unwrap_err();
        assert!(erro.contains("Erro ao processar NF-e"));
    }

    #[test]
    fn chave_repetida_e_rejeitada_sem_novo_registro() {
        let chave = "35123456789012345678901234567890123456789012";
        let erro = rejeitar_chave_duplicada(true, chave).unwrap_err();
        assert_eq!(erro, format!("NF-e já importada: chave de acesso {chave}"));
        rejeitar_chave_duplicada(false, chave).unwrap();
    }

    #[test]
    fn importacao_nao_grava_emissao() {
        let sql = super::sql_registrar_nota_importada();
        assert!(sql.contains("'IMPORTADA'"));
        assert!(!sql.contains("EMITIDA"));
        assert!(!sql.contains("ISSUED"));
        assert_eq!(super::STATUS_NOTA_IMPORTADA, "IMPORTADA");
    }
}
