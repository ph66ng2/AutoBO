//! PDFs comerciais da oficina (OS e orçamento) para a Aline consultar
//! o mesmo conteúdo que o AutoOS gera no balcão.

use printpdf::{BuiltinFont, Mm, PdfDocument};
use serde::Serialize;
use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoDocumentoOficina {
    OrdemServico,
    Orcamento,
}

impl TipoDocumentoOficina {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ordem_servico" | "ordemservico" | "os" => Ok(Self::OrdemServico),
            "orcamento" | "orçamento" => Ok(Self::Orcamento),
            other => Err(format!("Tipo de documento inválido: {other}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::OrdemServico => "ordem_servico",
            Self::Orcamento => "orcamento",
        }
    }

    fn titulo(self) -> &'static str {
        match self {
            Self::OrdemServico => "ORDEM DE SERVICO",
            Self::Orcamento => "ORCAMENTO TECNICO",
        }
    }

    fn filename_prefix(self) -> &'static str {
        match self {
            Self::OrdemServico => "OrdemServico",
            Self::Orcamento => "Orcamento",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinhaOrcamento {
    pub descricao: String,
    pub quantidade: String,
    pub valor: f64,
}

#[derive(Debug, Clone)]
pub struct DocumentoOficinaDados {
    pub equipamento_id: i32,
    pub serial_number: String,
    pub marca: String,
    pub modelo: String,
    pub tipo: String,
    pub status_label: String,
    pub cliente_nome: String,
    pub cliente_documento: String,
    pub contato: String,
    pub responsavel: String,
    pub data_entrada: String,
    pub patrimonio: String,
    pub defeito: String,
    pub diagnostico: String,
    pub acessorios: String,
    pub acessorios_outros: String,
    pub observacoes: String,
    pub tecnico: String,
    pub linhas: Vec<LinhaOrcamento>,
    pub total: f64,
}

#[derive(Debug, Serialize)]
pub struct DocumentoOficinaGerado {
    pub tipo: String,
    pub filename: String,
    pub path: String,
}

pub fn status_label(status: &str) -> String {
    match status.trim() {
        "RECEBIDO" => "Recebido",
        "EM_VERIFICACAO" => "Em Verificacao",
        "VERIFICADO" => "Verificado",
        "AGUARDANDO_APROVACAO" => "Aguardando Aprovacao",
        "APROVADO" => "Aprovado",
        "REPROVADO" => "Reprovado",
        "EM_MANUTENCAO" => "Em Manutencao",
        "AGUARDANDO_PECA" => "Aguardando Peca",
        "PRONTO" => "Pronto",
        "ENTREGUE" => "Entregue",
        "ORCAMENTO_VENCIDO" => "Orcamento Vencido",
        "ABANDONADO" => "Abandonado",
        other if other.is_empty() => "—",
        other => other,
    }
    .to_string()
}

pub fn numero_os(equipamento_id: i32) -> String {
    format!("OS-{equipamento_id:05}")
}

pub fn parse_linhas_orcamento(
    servicos_json: Option<&str>,
    pecas_json: Option<&str>,
) -> Vec<LinhaOrcamento> {
    let mut linhas = Vec::new();
    if let Some(raw) = servicos_json.filter(|value| !value.trim().is_empty()) {
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(raw) {
            for item in items {
                let descricao = item
                    .get("descricao")
                    .or_else(|| item.get("nome"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Servico")
                    .to_string();
                let valor = item.get("valor").and_then(|v| v.as_f64()).unwrap_or(0.0);
                linhas.push(LinhaOrcamento {
                    descricao,
                    quantidade: "01".into(),
                    valor,
                });
            }
        }
    }
    if let Some(raw) = pecas_json.filter(|value| !value.trim().is_empty()) {
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(raw) {
            for item in items {
                let descricao = item
                    .get("nome")
                    .or_else(|| item.get("descricao"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Peca")
                    .to_string();
                let quantidade = item.get("quantidade").and_then(|v| v.as_i64()).unwrap_or(1);
                let valor = item
                    .get("valorTotal")
                    .or_else(|| item.get("valor_total"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or_else(|| {
                        let unitario = item
                            .get("valorUnitario")
                            .or_else(|| item.get("valor_unitario"))
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        unitario * quantidade as f64
                    });
                linhas.push(LinhaOrcamento {
                    descricao,
                    quantidade: format!("{quantidade:02}"),
                    valor,
                });
            }
        }
    }
    linhas
}

pub fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(8);
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            if current.is_empty() {
                current = word.to_string();
            } else if current.len() + 1 + word.len() <= max_chars {
                current.push(' ');
                current.push_str(word);
            } else {
                lines.push(std::mem::take(&mut current));
                current = word.to_string();
            }
        }
        if current.is_empty() {
            if paragraph.is_empty() {
                lines.push(String::new());
            }
        } else {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push("—".into());
    }
    lines
}

fn pdf_safe(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            '–' | '—' => '-',
            '“' | '”' => '"',
            '‘' | '’' => '\'',
            'ç' | 'Ç' => 'c',
            'ã' | 'Ã' | 'á' | 'Á' | 'à' | 'À' | 'â' | 'Â' => 'a',
            'é' | 'É' | 'ê' | 'Ê' => 'e',
            'í' | 'Í' => 'i',
            'ó' | 'Ó' | 'ô' | 'Ô' | 'õ' | 'Õ' => 'o',
            'ú' | 'Ú' => 'u',
            other => other,
        })
        .collect()
}

fn documents_home() -> Result<PathBuf, String> {
    directories::UserDirs::new()
        .and_then(|dirs| dirs.document_dir().map(|path| path.to_path_buf()))
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join("Documents"))
        })
        .ok_or_else(|| "Não foi possível localizar a pasta Documents".to_string())
}

pub fn find_existing_document(
    tipo: TipoDocumentoOficina,
    equipamento_id: i32,
) -> Option<PathBuf> {
    let home = documents_home().ok()?;
    let dir_name = match tipo {
        TipoDocumentoOficina::Orcamento => "Orcamentos",
        TipoDocumentoOficina::OrdemServico => "Ordens de Servico",
    };
    let prefixes = [
        format!("{}_{equipamento_id}_", tipo.filename_prefix()),
        format!("{}_{equipamento_id}.", tipo.filename_prefix()),
    ];
    let mut matches: Vec<PathBuf> = fs::read_dir(home.join(dir_name))
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            name.to_ascii_lowercase().ends_with(".pdf")
                && prefixes.iter().any(|prefix| name.starts_with(prefix))
        })
        .collect();
    matches.sort();
    matches.pop()
}

fn output_dir(tipo: TipoDocumentoOficina) -> Result<PathBuf, String> {
    let dir = documents_home()?.join("AutoBO").join(match tipo {
        TipoDocumentoOficina::Orcamento => "Orcamentos",
        TipoDocumentoOficina::OrdemServico => "Ordens de Servico",
    });
    fs::create_dir_all(&dir).map_err(|e| format!("Não foi possível criar pasta de documentos: {e}"))?;
    Ok(dir)
}

struct PdfCursor {
    y: f32,
}

impl PdfCursor {
    fn write(
        &mut self,
        layer: &printpdf::PdfLayerReference,
        font: &printpdf::IndirectFontRef,
        text: &str,
        size: f32,
        bold_gap: f32,
    ) {
        if self.y < 20.0 {
            self.y = 20.0;
        }
        layer.use_text(pdf_safe(text), size, Mm(18.0), Mm(self.y), font);
        self.y -= bold_gap;
    }
}

pub fn gerar_pdf_oficina(
    tipo: TipoDocumentoOficina,
    dados: &DocumentoOficinaDados,
) -> Result<PathBuf, String> {
    if let Some(existing) = find_existing_document(tipo, dados.equipamento_id) {
        return Ok(existing);
    }

    let os = numero_os(dados.equipamento_id);
    let serial_safe: String = dados
        .serial_number
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect();
    let filename = format!(
        "{}_{}_{serial_safe}.pdf",
        tipo.filename_prefix(),
        dados.equipamento_id
    );
    let path = output_dir(tipo)?.join(&filename);

    let (doc, page1, layer1) = PdfDocument::new(tipo.titulo(), Mm(210.0), Mm(297.0), "Camada");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Fonte do PDF: {e}"))?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("Fonte do PDF: {e}"))?;
    let layer = doc.get_page(page1).get_layer(layer1);
    let mut cursor = PdfCursor { y: 278.0 };

    cursor.write(&layer, &font_bold, "BMITAG TECNOLOGIA QRCODE E RFID", 13.0, 6.0);
    cursor.write(
        &layer,
        &font,
        "Vendas e Manutencoes de Equipamentos ZEBRA",
        9.0,
        5.0,
    );
    cursor.write(
        &layer,
        &font,
        "Tel: +55 71 98223-5050 / +55 71 98165-0801",
        9.0,
        5.0,
    );
    cursor.write(
        &layer,
        &font,
        "E-mail: bmitag@bmitag.com.br | bmitag.com.br",
        9.0,
        5.0,
    );
    cursor.write(&layer, &font, "CNPJ: 57.522.734/0001-58", 9.0, 8.0);
    cursor.write(&layer, &font_bold, tipo.titulo(), 14.0, 6.0);
    cursor.write(&layer, &font_bold, &os, 11.0, 8.0);

    let equipamento = format!("{} {}", dados.marca, dados.modelo);
    match tipo {
        TipoDocumentoOficina::OrdemServico => {
            cursor.write(&layer, &font_bold, "Status atual", 9.0, 5.0);
            cursor.write(&layer, &font, &dados.status_label, 11.0, 7.0);
            cursor.write(&layer, &font_bold, "Entrada", 9.0, 5.0);
            cursor.write(&layer, &font, &dados.data_entrada, 10.0, 7.0);
            cursor.write(&layer, &font_bold, "Empresa cliente", 9.0, 5.0);
            cursor.write(
                &layer,
                &font,
                &format!("{}  {}", dados.cliente_nome, dados.cliente_documento),
                10.0,
                7.0,
            );
            cursor.write(&layer, &font_bold, "Contato", 9.0, 5.0);
            cursor.write(&layer, &font, &dados.contato, 10.0, 7.0);
            cursor.write(&layer, &font_bold, "Equipamento / Tipo", 9.0, 5.0);
            cursor.write(
                &layer,
                &font,
                &format!("{}  |  {}", equipamento, dados.tipo),
                10.0,
                7.0,
            );
            cursor.write(&layer, &font_bold, "N. de serie / Patrimonio", 9.0, 5.0);
            cursor.write(
                &layer,
                &font,
                &format!("{}  |  {}", dados.serial_number, dados.patrimonio),
                10.0,
                7.0,
            );
            cursor.write(&layer, &font_bold, "Defeito informado", 9.0, 5.0);
            for line in wrap_text(&dados.defeito, 90) {
                cursor.write(&layer, &font, &line, 10.0, 5.0);
            }
            cursor.y -= 2.0;
            cursor.write(&layer, &font_bold, "Laudo tecnico", 9.0, 5.0);
            for line in wrap_text(&dados.diagnostico, 90) {
                cursor.write(&layer, &font, &line, 10.0, 5.0);
            }
            cursor.y -= 2.0;
            cursor.write(&layer, &font_bold, "Acessorios", 9.0, 5.0);
            cursor.write(
                &layer,
                &font,
                &format!("{}  |  {}", dados.acessorios, dados.acessorios_outros),
                10.0,
                7.0,
            );
            cursor.write(&layer, &font_bold, "Observacoes", 9.0, 5.0);
            for line in wrap_text(&dados.observacoes, 90) {
                cursor.write(&layer, &font, &line, 10.0, 5.0);
            }
        }
        TipoDocumentoOficina::Orcamento => {
            cursor.write(&layer, &font_bold, "Empresa / Responsavel / Contato", 9.0, 5.0);
            cursor.write(&layer, &font, &dados.cliente_nome, 10.0, 5.0);
            cursor.write(&layer, &font, &dados.responsavel, 10.0, 5.0);
            cursor.write(&layer, &font, &dados.contato, 10.0, 8.0);
            cursor.write(&layer, &font_bold, "Planilha de valores", 9.0, 6.0);
            if dados.linhas.is_empty() && dados.total > 0.0 {
                cursor.write(
                    &layer,
                    &font,
                    &format!(
                        "Servicos tecnicos  {}  01  R$ {:.2}",
                        equipamento, dados.total
                    ),
                    10.0,
                    6.0,
                );
            }
            for linha in &dados.linhas {
                for (index, line) in wrap_text(&linha.descricao, 70).into_iter().enumerate() {
                    let text = if index == 0 {
                        format!(
                            "{}  {}  {}  R$ {:.2}",
                            line, equipamento, linha.quantidade, linha.valor
                        )
                    } else {
                        line
                    };
                    cursor.write(&layer, &font, &text, 9.0, 5.0);
                }
            }
            cursor.y -= 2.0;
            cursor.write(
                &layer,
                &font_bold,
                &format!("VALOR TOTAL: R$ {:.2}", dados.total),
                12.0,
                8.0,
            );
            cursor.write(&layer, &font_bold, "N. de serie", 9.0, 5.0);
            cursor.write(&layer, &font, &dados.serial_number, 10.0, 7.0);
            if !dados.diagnostico.is_empty() && dados.diagnostico != "—" {
                cursor.write(&layer, &font_bold, "Diagnostico", 9.0, 5.0);
                for line in wrap_text(&dados.diagnostico, 90) {
                    cursor.write(&layer, &font, &line, 10.0, 5.0);
                }
            }
            if !dados.tecnico.is_empty() {
                cursor.write(
                    &layer,
                    &font,
                    &format!("Tecnico responsavel: {}", dados.tecnico),
                    9.0,
                    5.0,
                );
            }
        }
    }

    let file = fs::File::create(&path).map_err(|e| format!("Erro ao gravar PDF: {e}"))?;
    doc.save(&mut BufWriter::new(file))
        .map_err(|e| format!("Erro ao salvar PDF: {e}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotulo_nao_repete_o_codigo_do_banco() {
        assert_eq!(status_label("AGUARDANDO_APROVACAO"), "Aguardando Aprovacao");
        assert_eq!(numero_os(41), "OS-00041");
    }

    #[test]
    fn parseia_servicos_do_autoos() {
        let linhas = parse_linhas_orcamento(
            Some(r#"[{"descricao":"Troca da cabeça de Impressão","valor":3850}]"#),
            Some("[]"),
        );
        assert_eq!(linhas.len(), 1);
        assert_eq!(linhas[0].valor, 3850.0);
    }

    #[test]
    fn quebra_linhas_longas() {
        let lines = wrap_text("um dois tres quatro", 8);
        assert!(lines.len() >= 2);
    }
}
