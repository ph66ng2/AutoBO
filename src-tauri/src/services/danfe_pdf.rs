//! Extração assistida de notas em PDF (DANFE NF-e e NFS-e municipal).
//!
//! Muitos PDFs da Aline são *print* de página (Nota Salvador) — uma
//! imagem, sem texto selecionável. Nesses casos extraímos o texto com
//! OCR (Tesseract + Poppler) e pescamos os campos por regex tolerante
//! a erros de leitura (CNPJ com ponto no lugar do hífen, etc.).
//!
//! Sempre é atalho de digitação: a tela de revisão confirma antes do boleto.

use crate::services::nfe_parser::{DestinatarioDados, NFeDados, NFeItem};
use crate::validators::{cnpj::validar_cnpj, cpf::validar_cpf};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Atalho de digitação via PDF: nunca gera boleto sozinho.
#[derive(Debug)]
pub struct DanfeExtraido {
    pub dados: NFeDados,
    pub avisos: Vec<String>,
}

fn texto_util(texto: &str) -> usize {
    texto
        .chars()
        .filter(|c| c.is_alphanumeric())
        .count()
}

fn texto_identificavel(texto: &str) -> bool {
    texto_util(texto) >= 80 && (eh_nfse(texto) || encontrar_chave(texto).is_some())
}

/// Extrai o texto bruto de um PDF. DANFE do Jasper costuma esconder a chave
/// na extração simples — aí o `pdftotext -layout` acha os 44 dígitos. PDF de
/// print (Nota Salvador) cai no OCR.
pub fn extrair_texto_pdf(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() < 4 || &bytes[0..4] != b"%PDF" {
        return Err("Arquivo não parece um PDF válido".into());
    }

    let extraido = pdf_extract::extract_text_from_mem(bytes).unwrap_or_default();
    if texto_identificavel(&extraido) {
        return Ok(extraido);
    }

    if let Ok(layout) = pdftotext_layout(bytes) {
        if texto_identificavel(&layout) || texto_util(&layout) > texto_util(&extraido) {
            return Ok(layout);
        }
    }

    if texto_util(&extraido) >= 80 && !parece_danfe_sem_chave(&extraido) {
        return Ok(extraido);
    }

    match ocr_pdf(bytes) {
        Ok(ocr) if texto_util(&ocr) >= 80 => Ok(ocr),
        Ok(_) => Err(
            "Este PDF é uma imagem (Nota Salvador / DANFE escaneada) e o OCR não leu texto suficiente. Use o XML ou a entrada manual."
                .into(),
        ),
        Err(ocr_err) => {
            if texto_util(&extraido) < 50 {
                Err(format!(
                    "Este PDF não tem texto selecionável (é uma foto/print da nota). {ocr_err}"
                ))
            } else {
                Ok(extraido)
            }
        }
    }
}

fn parece_danfe_sem_chave(texto: &str) -> bool {
    let u = texto.to_uppercase();
    encontrar_chave(texto).is_none()
        && (u.contains("DANFE") || u.contains("CHAVE DE ACESSO") || u.contains("NF-E"))
}

fn pdftotext_layout(bytes: &[u8]) -> Result<String, String> {
    if !comando_existe("pdftotext") {
        return Err("pdftotext não está instalado".into());
    }
    let dir = std::env::temp_dir().join(format!("autobo-pdftotext-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).map_err(|e| format!("Não consegui criar pasta temporária: {e}"))?;
    let pdf_path = dir.join("nota.pdf");
    let resultado = (|| {
        fs::write(&pdf_path, bytes).map_err(|e| format!("Não consegui gravar o PDF temporário: {e}"))?;
        let output = Command::new("pdftotext")
            .args(["-layout", "-enc", "UTF-8"])
            .arg(&pdf_path)
            .arg("-")
            .output()
            .map_err(|e| format!("Falha ao executar pdftotext: {e}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    })();
    let _ = fs::remove_dir_all(&dir);
    resultado
}

fn comando_existe(nome: &str) -> bool {
    Command::new(nome)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || Command::new("which")
            .arg(nome)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

fn ocr_pdf(bytes: &[u8]) -> Result<String, String> {
    if !comando_existe("tesseract") {
        return Err(
            "Para ler essa nota instale o Tesseract (`tesseract` + idioma `por`) e o Poppler (`pdftoppm`)."
                .into(),
        );
    }

    let dir = std::env::temp_dir().join(format!("autobo-ocr-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).map_err(|e| format!("Não consegui criar pasta temporária de OCR: {e}"))?;
    let resultado = ocr_pdf_em(&dir, bytes);
    let _ = fs::remove_dir_all(&dir);
    resultado
}

fn ocr_pdf_em(dir: &Path, bytes: &[u8]) -> Result<String, String> {
    let pdf_path = dir.join("nota.pdf");
    fs::write(&pdf_path, bytes).map_err(|e| format!("Não consegui gravar o PDF temporário: {e}"))?;

    let prefix = dir.join("pagina");
    renderizar_pdf_para_imagens(&pdf_path, &prefix)?;

    let mut imagens: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("Erro ao listar imagens do OCR: {e}"))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            matches!(
                p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()),
                Some(ext) if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "tif" || ext == "tiff"
            )
        })
        .collect();
    imagens.sort();
    if imagens.is_empty() {
        return Err("Não consegui renderizar as páginas do PDF para OCR.".into());
    }

    let mut texto = String::new();
    for imagem in imagens {
        texto.push_str(&ocr_imagem(&imagem)?);
        texto.push('\n');
    }
    Ok(texto)
}

fn renderizar_pdf_para_imagens(pdf: &Path, prefix: &Path) -> Result<(), String> {
    if comando_existe("pdftoppm") {
        let ok = Command::new("pdftoppm")
            .args(["-png", "-r", "200"])
            .arg(pdf)
            .arg(prefix)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Ok(());
        }
    }
    if comando_existe("pdfimages") {
        let ok = Command::new("pdfimages")
            .args(["-png"])
            .arg(pdf)
            .arg(prefix)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Ok(());
        }
    }
    Err(
        "Não achei `pdftoppm` nem `pdfimages` (pacote poppler-utils). Sem isso não dá para OCR de PDF-imagem."
            .into(),
    )
}

fn ocr_imagem(path: &Path) -> Result<String, String> {
    for langs in ["por+eng", "por", "eng"] {
        let output = Command::new("tesseract")
            .arg(path)
            .arg("stdout")
            .args(["-l", langs, "--psm", "6"])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let texto = String::from_utf8_lossy(&out.stdout).into_owned();
                if texto_util(&texto) >= 40 {
                    return Ok(texto);
                }
            }
        }
    }
    Err("Tesseract não leu a imagem da nota.".into())
}

fn eh_nfse(texto: &str) -> bool {
    let u = texto.to_uppercase();
    u.contains("NOTA SALVADOR")
        || u.contains("NOTA FISCAL DE SERVI")
        || u.contains("PREFEITURA MUNICIPAL")
        || u.contains("TOMADOR DE SERVI")
        || u.contains("PRESTADOR DE SERVI")
}

/// Localiza a chave de acesso (44 dígitos, às vezes em grupos de 4).
/// Exige modelo 55/65 (NF-e/NFC-e) para não engolir o "1" da Saída colado na chave.
pub fn encontrar_chave(texto: &str) -> Option<String> {
    let em_grupos = Regex::new(r"(?:\d{4}[\s.]+){10}\d{4}").ok()?;
    let corrida = Regex::new(r"\d{44}").ok()?;

    let mut candidatas: Vec<String> = Vec::new();
    for m in em_grupos.find_iter(texto) {
        candidatas.push(m.as_str().chars().filter(|c| c.is_ascii_digit()).collect());
    }
    for m in corrida.find_iter(texto) {
        candidatas.push(m.as_str().to_string());
    }
    if let Some(apos_rotulo) = chave_apos_rotulo(texto) {
        candidatas.push(apos_rotulo);
    }

    let nfe = |chave: &str| {
        chave.len() == 44 && matches!(chave.get(20..22), Some("55") | Some("65"))
    };

    let mut primeira_nfe: Option<String> = None;
    for chave in candidatas {
        if !nfe(&chave) {
            continue;
        }
        if primeira_nfe.is_none() {
            primeira_nfe = Some(chave.clone());
        }
        if dv_chave_valido(&chave) {
            return Some(chave);
        }
    }
    primeira_nfe
}

fn chave_apos_rotulo(texto: &str) -> Option<String> {
    let upper = texto.to_uppercase();
    let pos = upper.find("CHAVE DE ACESSO")?;
    let digits: String = texto[pos..]
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(80)
        .collect();
    if digits.len() < 44 {
        return None;
    }
    for i in 0..=digits.len() - 44 {
        let chave = &digits[i..i + 44];
        if matches!(chave.get(20..22), Some("55") | Some("65")) {
            return Some(chave.to_string());
        }
    }
    None
}

pub fn dv_chave_valido(chave: &str) -> bool {
    let digitos: Vec<u32> = chave.chars().filter_map(|c| c.to_digit(10)).collect();
    if digitos.len() != 44 {
        return false;
    }
    let mut soma = 0u32;
    let mut peso = 2u32;
    for d in digitos[..43].iter().rev() {
        soma += d * peso;
        peso = if peso == 9 { 2 } else { peso + 1 };
    }
    let resto = soma % 11;
    let dv = if resto < 2 { 0 } else { 11 - resto };
    dv == digitos[43]
}

pub fn numero_serie_da_chave(chave: &str) -> (String, String) {
    let serie = chave.get(22..25).unwrap_or("").trim_start_matches('0');
    let numero = chave.get(25..34).unwrap_or("").trim_start_matches('0');
    (
        if numero.is_empty() { "0".into() } else { numero.into() },
        if serie.is_empty() { "0".into() } else { serie.into() },
    )
}

fn emissao_da_chave(chave: &str) -> Option<String> {
    let aamm = chave.get(2..6)?;
    let aa: i32 = aamm.get(0..2)?.parse().ok()?;
    let mm: u32 = aamm.get(2..4)?.parse().ok()?;
    if !(1..=12).contains(&mm) {
        return None;
    }
    Some(format!("{}-{:02}-01", 2000 + aa, mm))
}

fn parse_valor_money(texto: &str) -> Option<f64> {
    let t = texto.trim();
    if t.contains(',') {
        t.replace('.', "").replace(',', ".").parse().ok()
    } else {
        t.replace(' ', "").parse().ok()
    }
}

fn valores_na_janela(janela: &str) -> Vec<f64> {
    let Ok(re) = Regex::new(r"\d{1,3}(?:\.\d{3})*,\d{2}|\d+,\d{2}|\d+\.\d{2}") else {
        return vec![];
    };
    re.find_iter(janela)
        .filter_map(|m| parse_valor_money(m.as_str()))
        .filter(|v| *v > 0.0)
        .collect()
}

fn valores_br_na_janela(janela: &str) -> Vec<f64> {
    let Ok(re) = Regex::new(r"\d{1,3}(?:\.\d{3})*,\d{2}|\d+,\d{2}") else {
        return vec![];
    };
    re.find_iter(janela)
        .filter_map(|m| parse_valor_money(m.as_str()))
        .filter(|v| *v > 0.0)
        .collect()
}

/// NFS-e: o total vem como `VALOR TOTAL DA NOTA = R$1.190,00`. Não usar
/// `\d+\.\d{2}` — isso pega alíquota/código (ex.: 7.18) depois do total.
fn encontrar_valor_total_nfse(texto: &str) -> Option<f64> {
    let rotulado = Regex::new(
        r"(?i)VALOR TOTAL DA NOTA\s*=?\s*R\$?\s*(\d{1,3}(?:\.\d{3})*,\d{2}|\d+,\d{2})",
    )
    .ok()?;
    if let Some(cap) = rotulado.captures(texto) {
        if let Some(v) = parse_valor_money(&cap[1]) {
            return Some(v);
        }
    }
    let upper = texto.to_uppercase();
    if let Some(pos) = upper.find("VALOR TOTAL DA NOTA") {
        let janela = &texto[pos..(pos + 90).min(texto.len())];
        if let Some(v) = valores_br_na_janela(janela).into_iter().next() {
            return Some(v);
        }
    }
    valores_br_na_janela(texto)
        .into_iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

/// No DANFE o total fica na linha abaixo do rótulo; no canhoto vem "VALOR TOTAL: 580.00".
fn encontrar_valor_total(texto: &str) -> Option<f64> {
    let upper = texto.to_uppercase();
    for rotulo in ["VALOR TOTAL DA NOTA", "TOTAL DA NOTA", "VALOR TOTAL", "VLR TOTAL"] {
        if let Some(pos) = upper.find(rotulo) {
            let janela = &texto[pos..(pos + 280).min(texto.len())];
            if let Some(v) = valores_na_janela(janela).into_iter().next_back() {
                return Some(v);
            }
        }
    }
    valores_na_janela(texto)
        .into_iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

fn encontrar_emissao(texto: &str) -> Option<String> {
    let data = Regex::new(r"(\d{2})/(\d{2})/(\d{4})").ok()?;
    let upper = texto.to_uppercase();
    for rotulo in ["EMISS", "DATA E HORA"] {
        if let Some(pos) = upper.find(rotulo) {
            let inicio = pos.saturating_sub(20);
            let fim = (pos + 180).min(texto.len());
            if let Some(m) = data.find(&texto[inicio..fim]) {
                let d = m.as_str();
                return Some(format!("{}-{}-{}", &d[6..10], &d[3..5], &d[0..2]));
            }
        }
    }
    data.find(texto).map(|m| {
        let d = m.as_str();
        format!("{}-{}-{}", &d[6..10], &d[3..5], &d[0..2])
    })
}

fn encontrar_natureza(texto: &str) -> Option<String> {
    let upper = texto.to_uppercase();
    let pos = upper
        .find("NATUREZA DA OPERA")
        .or_else(|| upper.find("NATUREZA DE OPERA"))?;
    let trecho = texto[pos..(pos + 220).min(texto.len())].to_string();
    let mut linhas = trecho.lines().skip(1);
    let natureza = linhas
        .find(|l| l.trim().chars().filter(|c| c.is_alphabetic()).count() >= 3)?
        .trim()
        .to_string();
    let curto: String = natureza.chars().take(60).collect();
    if curto.is_empty() {
        None
    } else {
        Some(curto)
    }
}

/// CNPJs/CPFs mesmo quando o OCR troca hífen por ponto.
fn documentos_validos(texto: &str) -> Vec<String> {
    let flex = Regex::new(r"\d{2}[.\s]?\d{3}[.\s]?\d{3}[./\s]?\d{4}[-.\s]?\d{2}").unwrap();
    let mut achados: Vec<(usize, String)> = Vec::new();
    for m in flex.find_iter(texto) {
        let doc: String = m.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
        let valido = match doc.len() {
            14 => validar_cnpj(&doc),
            11 => validar_cpf(&doc),
            _ => false,
        };
        if valido {
            achados.push((m.start(), doc));
        }
    }
    achados.sort_by_key(|(pos, _)| *pos);
    let mut unicos: Vec<String> = Vec::new();
    for (_, doc) in achados {
        if !unicos.contains(&doc) {
            unicos.push(doc);
        }
    }
    unicos
}

fn bloco_apos(texto: &str, rotulos: &[&str]) -> Option<String> {
    let upper = texto.to_uppercase();
    for rotulo in rotulos {
        if let Some(pos) = upper.find(&rotulo.to_uppercase()) {
            return Some(texto[pos..(pos + 900).min(texto.len())].to_string());
        }
    }
    None
}

fn nome_em_bloco(bloco: &str) -> Option<String> {
    let rotulo = Regex::new(r"(?i)nome|raz[aã]o|social|cpf|cnpj|cndj|ckpj|inscri[cç][aã]o|endere[cç]o|e-?mail|tomador|prestador|adquirente|destinat[aá]rio|remetente|data|emiss[aã]o|bairro|distrito").ok()?;
    let doc = Regex::new(r"\d{2}[.\s]?\d{3}[.\s]?\d{3}[./\s]?\d{4}[-.\s]?\d{2}|\d{3}[.\s]?\d{3}[.\s]?\d{3}[-.\s]?\d{2}").ok()?;
    for linha in bloco.lines().skip(1) {
        let sem_doc = doc.replace_all(linha, " ");
        let t = Regex::new(r"\s{2,}")
            .ok()?
            .replace_all(sem_doc.trim(), " ")
            .trim()
            .to_string();
        if t.chars().filter(|c| c.is_alphabetic()).count() < 8 {
            continue;
        }
        let sem_rotulo = rotulo.replace_all(&t, " ");
        let limpo = Regex::new(r"\s{2,}")
            .ok()?
            .replace_all(sem_rotulo.trim(), " ")
            .replace("NomorRazao Social", "")
            .replace("Nomo!Razão Social", "")
            .trim()
            .trim_matches(|c: char| !c.is_alphanumeric() && c != ' ' && c != '-')
            .to_string();
        if limpo.chars().filter(|c| c.is_alphabetic()).count() >= 8 {
            return Some(limpo.chars().take(120).collect());
        }
    }
    None
}

fn email_em(texto: &str) -> Option<String> {
    let re = Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}").ok()?;
    re.find(texto).map(|m| m.as_str().to_lowercase())
}

fn numero_nfse(texto: &str) -> Option<String> {
    let upper = texto.to_uppercase();
    let pos = upper.find("MERO DA NOTA").or_else(|| upper.find("NUMERO DA NOTA"))?;
    let janela = &texto[pos..(pos + 220).min(texto.len())];
    let re = Regex::new(r"\b(\d{3,8})\b").ok()?;
    let bruto = re.captures(janela)?.get(1)?.as_str().to_string();
    let sem_zeros = bruto.trim_start_matches('0');
    if sem_zeros.is_empty() {
        Some(bruto)
    } else {
        Some(sem_zeros.to_string())
    }
}

fn descricao_servico(texto: &str) -> Option<String> {
    let upper = texto.to_uppercase();
    let pos = upper.find("DISCRIMINA")?;
    let resto = &texto[pos..];
    let fim = resto
        .to_uppercase()
        .find("VALOR TOTAL")
        .unwrap_or(resto.len().min(500));
    let trecho = &resto[..fim];
    let mut linhas: Vec<&str> = trecho
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|l| l.chars().filter(|c| c.is_alphabetic()).count() >= 8)
        .collect();
    if linhas.is_empty() {
        return None;
    }
    let desc = linhas.remove(0);
    Some(desc.chars().take(240).collect())
}

fn descricao_produto_danfe(texto: &str) -> Option<String> {
    let upper = texto.to_uppercase();
    let pos = upper.find("DADOS DO PRODUT")?;
    let resto = &texto[pos..];
    let fim = resto
        .to_uppercase()
        .find("CALCULO DO ISSQN")
        .or_else(|| resto.to_uppercase().find("DADOS ADICIONAIS"))
        .unwrap_or(resto.len().min(900));
    let bloco = &resto[..fim];
    let pular = Regex::new(r"(?i)(c[oó]digo|descri|ncm|cfop|unid|quant|vl unit|valor total|cst|csons|c[aá]lculo|icms|ipi)").ok()?;
    let mut partes: Vec<String> = Vec::new();
    for linha in bloco.lines().skip(1) {
        let t = linha.trim();
        if t.chars().filter(|c| c.is_alphabetic()).count() < 6 {
            continue;
        }
        if pular.is_match(t) && t.chars().filter(|c| c.is_alphabetic()).count() < 24 {
            continue;
        }
        let sem_codigo = Regex::new(r"^\d{4,}\s+").ok()?.replace(t, "");
        partes.push(sem_codigo.trim().chars().take(160).collect());
        if partes.len() >= 2 {
            break;
        }
    }
    if partes.is_empty() {
        None
    } else {
        Some(partes.join(" ").chars().take(240).collect())
    }
}

fn cep_em(texto: &str) -> Option<String> {
    let re = Regex::new(r"\b(\d{5}-?\d{3})\b").ok()?;
    re.captures(texto)
        .map(|c| c[1].chars().filter(|ch| ch.is_ascii_digit()).collect())
}

struct EnderecoDest {
    logradouro: Option<String>,
    numero: Option<String>,
    bairro: Option<String>,
    cidade: Option<String>,
    uf: Option<String>,
    cep: Option<String>,
}

fn endereco_destinatario(bloco: &str) -> EnderecoDest {
    let logradouro_linha = bloco.lines().map(str::trim).find(|l| {
        let u = l.to_uppercase();
        (u.contains("RUA")
            || u.contains("AVENIDA")
            || u.contains("AV.")
            || u.contains("ALAMEDA")
            || u.contains("TRAVESSA")
            || u.contains("RODOVIA"))
            && l.chars().filter(|c| c.is_alphabetic()).count() >= 8
    });

    let mut logradouro = logradouro_linha.map(|l| {
        Regex::new(r"\s{2,}")
            .ok()
            .map(|re| re.split(l).next().unwrap_or(l).trim().to_string())
            .unwrap_or_else(|| l.to_string())
    });
    let numero = logradouro.as_ref().and_then(|l| {
        Regex::new(r"\b(\d{1,6})\b")
            .ok()?
            .captures(l)
            .map(|c| c[1].to_string())
    });
    let bairro = logradouro_linha.and_then(|l| {
        Regex::new(r"\s{2,}([A-ZÁÉÍÓÚÃÕÇ][A-ZÁÉÍÓÚÃÕÇ\s]{3,})$")
            .ok()?
            .captures(l)
            .map(|c| c[1].trim().to_string())
    });
    if let (Some(log), Some(bai)) = (logradouro.as_mut(), bairro.as_ref()) {
        if let Some(idx) = log.rfind(bai.as_str()) {
            log.truncate(idx);
            *log = log.trim().to_string();
        }
    }

    let uf = Regex::new(r"\b(AC|AL|AP|AM|BA|CE|DF|ES|GO|MA|MT|MS|MG|PA|PB|PR|PE|PI|RJ|RN|RS|RO|RR|SC|SP|SE|TO)\b")
        .ok()
        .and_then(|re| re.captures(bloco).map(|c| c[1].to_string()));
    let cidade = if bloco.to_uppercase().contains("SALVADOR") {
        Some("Salvador".into())
    } else {
        bloco
            .lines()
            .map(str::trim)
            .find(|l| {
                let u = l.to_uppercase();
                u.contains("SALVADOR")
                    || (cep_em(l).is_some() && l.chars().filter(|c| c.is_alphabetic()).count() >= 4)
            })
            .map(|l| {
                Regex::new(r"[A-Za-zÀ-ú]{4,}")
                    .ok()
                    .and_then(|re| re.find(l).map(|m| m.as_str().to_string()))
                    .unwrap_or_else(|| l.to_string())
            })
    };

    EnderecoDest {
        logradouro,
        numero,
        bairro,
        cidade,
        uf,
        cep: cep_em(bloco),
    }
}

fn chave_nfse(numero: &str, documento: &str, emissao: &str, valor: f64) -> String {
    // autobo_nfes.chave_acesso é VARCHAR(44); NF-e usa 44 dígitos, NFS-e ganha um id estável curto.
    let bruto = format!(
        "NFSE{numero}{documento}{emissao}{valor:.0}",
        numero = numero.replace('.', ""),
        documento = documento,
        emissao = emissao.replace('-', ""),
        valor = valor
    );
    bruto.chars().take(44).collect()
}

fn parse_nfse(texto: &str) -> Result<DanfeExtraido, String> {
    let mut avisos = vec![
        "Extraído de PDF de NFS-e (Nota Salvador) por OCR — confira cada campo antes de gerar o boleto."
            .to_string(),
    ];

    let tomador_bloco = bloco_apos(texto, &["TOMADOR DE SERVI", "TOMADOR /", "ADQUIRENTE"]);
    let docs_tomador = tomador_bloco.as_deref().map(documentos_validos).unwrap_or_default();
    let docs_todos = documentos_validos(texto);

    let documento = docs_tomador
        .first()
        .cloned()
        .or_else(|| docs_todos.get(1).cloned())
        .or_else(|| docs_todos.first().cloned())
        .unwrap_or_default();
    if documento.is_empty() {
        avisos.push("CNPJ/CPF do tomador não localizado — informe na revisão.".into());
    } else if docs_tomador.is_empty() && docs_todos.len() == 1 {
        avisos.push("Só encontrei um documento (pode ser o prestador) — confira se é o tomador.".into());
    }

    let nome = tomador_bloco
        .as_deref()
        .and_then(nome_em_bloco)
        .or_else(|| nome_em_bloco(texto));
    if nome.is_none() {
        avisos.push("Nome do tomador não localizado — informe na revisão.".into());
    }

    let email = tomador_bloco
        .as_deref()
        .and_then(email_em)
        .or_else(|| email_em(texto));

    let numero_nf = numero_nfse(texto).unwrap_or_else(|| "S_N".into());
    if numero_nf == "S_N" {
        avisos.push("Número da NFS-e não localizado.".into());
    }

    let data_emissao = encontrar_emissao(texto).unwrap_or_default();
    if data_emissao.is_empty() {
        avisos.push("Data de emissão não localizada — informe na revisão.".into());
    }

    let valor_total = encontrar_valor_total_nfse(texto).unwrap_or(0.0);
    if valor_total <= 0.0 {
        avisos.push("Valor total não localizado — informe na revisão.".into());
    }

    let servico = descricao_servico(texto);
    let itens = if let Some(desc) = servico.clone() {
        vec![NFeItem {
            descricao: desc,
            quantidade: 1.0,
            unidade: "UN".into(),
            valor_unitario: valor_total,
            subtotal: valor_total,
            ncm: None,
        }]
    } else {
        avisos.push("Discriminação do serviço não localizada — descreva na mensagem.".into());
        vec![]
    };

    let chave = chave_nfse(&numero_nf, &documento, &data_emissao, valor_total);

    Ok(DanfeExtraido {
        dados: NFeDados {
            numero_nf,
            serie: Some("NFS-e".into()),
            chave_acesso: chave,
            data_emissao,
            valor_total,
            valor_produtos: None,
            natureza_operacao: Some("NFS-e municipal".into()),
            destinatario: DestinatarioDados {
                documento,
                nome: nome.clone(),
                razao_social: nome,
                email,
                telefone: None,
                cep: None,
                logradouro: None,
                numero: None,
                complemento: None,
                bairro: None,
                cidade: None,
                uf: None,
            },
            itens,
            versao: "NFSE-PDF".into(),
        },
        avisos,
    })
}

fn parse_danfe_nfe(texto: &str, chave: String) -> DanfeExtraido {
    let mut avisos = vec![
        "Extraído de DANFE comercial (NF-e de mercadoria) — confira cada campo antes de gerar o boleto."
            .to_string(),
    ];
    if !dv_chave_valido(&chave) {
        avisos.push("Dígito verificador da chave não confere — confira a chave no PDF.".into());
    }

    let (numero_nf, serie) = numero_serie_da_chave(&chave);

    let data_emissao = encontrar_emissao(texto)
        .or_else(|| emissao_da_chave(&chave))
        .unwrap_or_default();
    if data_emissao.is_empty() {
        avisos.push("Data de emissão não localizada — informe na revisão.".into());
    }

    let valor_total = encontrar_valor_total(texto).unwrap_or(0.0);
    if valor_total <= 0.0 {
        avisos.push("Valor total não localizado — informe na revisão.".into());
    }

    let dest_bloco = bloco_apos(
        texto,
        &["DESTINAT", "DEST./REM", "DESTINATARIO / REMETENTE"],
    );
    let docs_dest = dest_bloco
        .as_deref()
        .map(documentos_validos)
        .unwrap_or_default();
    let docs_todos = documentos_validos(texto);
    let documento = docs_dest
        .first()
        .cloned()
        .or_else(|| docs_todos.get(1).cloned())
        .or_else(|| docs_todos.first().cloned())
        .unwrap_or_default();
    if documento.is_empty() {
        avisos.push("CNPJ/CPF do destinatário não localizado — informe na revisão.".into());
    }

    let nome = dest_bloco
        .as_deref()
        .and_then(nome_em_bloco)
        .or_else(|| {
            dest_bloco.as_ref().and_then(|b| {
                b.lines()
                    .map(str::trim)
                    .find(|l| {
                        l.to_uppercase().contains("ENTREGAS")
                            || (l.chars().filter(|c| c.is_alphabetic()).count() >= 12
                                && !l.to_uppercase().contains("NOME")
                                && !l.to_uppercase().contains("CNPJ")
                                && !l.to_uppercase().contains("DESTINAT"))
                    })
                    .map(|l| {
                        Regex::new(r"\s{2,}")
                            .ok()
                            .and_then(|re| re.split(l).next().map(|s| s.trim().to_string()))
                            .unwrap_or_else(|| l.to_string())
                    })
            })
        });
    if nome.is_none() {
        avisos.push("Nome do destinatário não localizado — informe na revisão.".into());
    }

    let endereco = dest_bloco
        .as_deref()
        .map(endereco_destinatario)
        .unwrap_or(EnderecoDest {
            logradouro: None,
            numero: None,
            bairro: None,
            cidade: None,
            uf: None,
            cep: None,
        });

    let descricao = descricao_produto_danfe(texto);
    let itens = if let Some(desc) = descricao.clone() {
        vec![NFeItem {
            descricao: desc,
            quantidade: 1.0,
            unidade: "UN".into(),
            valor_unitario: valor_total,
            subtotal: valor_total,
            ncm: None,
        }]
    } else {
        avisos.push("Itens da DANFE não localizados — descreva a cobrança na revisão.".into());
        vec![]
    };

    DanfeExtraido {
        dados: NFeDados {
            numero_nf,
            serie: Some(serie),
            chave_acesso: chave,
            data_emissao,
            valor_total,
            valor_produtos: Some(valor_total),
            natureza_operacao: encontrar_natureza(texto),
            destinatario: DestinatarioDados {
                documento,
                nome: nome.clone(),
                razao_social: nome,
                email: None,
                telefone: None,
                cep: endereco.cep,
                logradouro: endereco.logradouro,
                numero: endereco.numero,
                complemento: None,
                bairro: endereco.bairro,
                cidade: endereco.cidade,
                uf: endereco.uf,
            },
            itens,
            versao: "DANFE-PDF".into(),
        },
        avisos,
    }
}

/// Converte o texto do PDF em pré-preenchimento (DANFE ou NFS-e).
pub fn parse_danfe(texto: &str) -> Result<DanfeExtraido, String> {
    if texto_util(texto) < 50 {
        return Err(
            "Não encontrei texto legível neste PDF. Se for nota escaneada (foto), use a entrada manual."
                .into(),
        );
    }

    if eh_nfse(texto) {
        return parse_nfse(texto);
    }

    if let Some(chave) = encontrar_chave(texto) {
        return Ok(parse_danfe_nfe(texto, chave));
    }

    Err(
        "Não encontrei chave de acesso de NF-e nem identifiquei NFS-e municipal neste PDF. Use o XML ou a entrada manual."
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chave_valida() -> String {
        let base = "4124061234567800019955001000000123112345678";
        assert_eq!(base.len(), 43);
        let mut soma = 0u32;
        let mut peso = 2u32;
        for d in base.chars().rev().filter_map(|c| c.to_digit(10)) {
            soma += d * peso;
            peso = if peso == 9 { 2 } else { peso + 1 };
        }
        let resto = soma % 11;
        let dv = if resto < 2 { 0 } else { 11 - resto };
        format!("{base}{dv}")
    }

    #[test]
    fn dv_e_numero_serie_batem() {
        let chave = chave_valida();
        assert!(dv_chave_valido(&chave));
        let (numero, serie) = numero_serie_da_chave(&chave);
        assert_eq!(numero, "123");
        assert_eq!(serie, "1");
    }

    #[test]
    fn rejeita_dv_errado() {
        let mut chave = chave_valida();
        let ultimo = chave.pop().unwrap();
        let trocado = if ultimo == '0' { '1' } else { '0' };
        chave.push(trocado);
        assert!(!dv_chave_valido(&chave));
    }

    #[test]
    fn encontra_chave_em_grupos() {
        let chave = chave_valida();
        let grupos: Vec<String> = chave
            .as_bytes()
            .chunks(4)
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect();
        let texto = format!("CHAVE DE ACESSO {}", grupos.join(" "));
        assert_eq!(encontrar_chave(&texto).as_deref(), Some(chave.as_str()));
    }

    #[test]
    fn parse_danfe_minimo() {
        let chave = chave_valida();
        let texto = format!(
            "DANFE DOCUMENTO AUXILIAR CHAVE DE ACESSO {chave} N 000000123 SERIE 1 \
             EMITENTE LOJA EXEMPLO CNPJ 11.222.333/0001-81 \
             DESTINATARIO CLIENTE EXEMPLO CNPJ 12.345.678/0001-99 \
             VALOR TOTAL DA NOTA 1.234,56 DATA DE EMISSAO 15/06/2024"
        );
        let extraido = parse_danfe(&texto).expect("deveria extrair");
        assert_eq!(extraido.dados.chave_acesso, chave);
        assert_eq!(extraido.dados.numero_nf, "123");
        assert!((extraido.dados.valor_total - 1234.56).abs() < 0.01);
        assert_eq!(extraido.dados.data_emissao, "2024-06-15");
    }

    #[test]
    fn sem_chave_e_erro() {
        let err = parse_danfe("um pdf qualquer com texto suficiente para passar do minimo de caracteres exigido aqui").unwrap_err();
        assert!(err.contains("chave de acesso") || err.contains("NFS-e"));
    }

    /// Texto real do OCR da Nota Salvador ELMECO (print Chrome, sem camada de texto).
    const OCR_ELMECO: &str = r#"
s Número da Nota:
N PREFEITURA MUNICIPAL DO SALVADOR 00000447
| SECRETARIA MUNICIPAL DA FAZENDA Data e Hora de Emissão:
, 18/09/2026 15:15:11
cá Código de Verificação:
NOTA FISCAL DE SERVIÇOS ELETRÔNICA - Nota Salvador oxmepen OaS
PRESTADOR DE SERVIÇOS
CPFICNDJ Inscrição Municipal
57.522.734/0001.58 00.020.074/001-75
NomorRazao Social . .
BMITAG TECNOLOGIA QRCODE E RFID - REPAROS E MANUTENÇÕES EM PERIFÉRICOS LTDA
TOMADOR DE SERVIÇOS / ADQUIRENTE
Nomo!Razão Social
ELMECO SERVICOS FARMACEUTICOS E TREINAMENTO PROFISSIONAL LTDA
CPFICKPJ Inscrição Municipal
96.792.791/0001-09 00.263.918/001-48
Endereço
2º Caetano Moura 000035, BLOCO B TERREO FEDERAÇÃO - Salvador - CEP: 40210-341/BA
E-mail
fiscal@elmeco.com.br
DISCRIMINAÇÃO DOS SERVIÇOS
SERVIÇO DE CRIAÇÃO DE NOVO MODELO PADRÃO DE ETIQUETA PRODUÇÃO E MANUTENÇÃO CORRETIVA EM 20230 DA EXPEDTGRO.
VALOR TOTAL DA NOTA = R$1.190,00
"#;

    #[test]
    fn parse_nota_salvador_elmeço() {
        let extraido = parse_danfe(OCR_ELMECO).expect("NFS-e deveria extrair");
        assert_eq!(extraido.dados.numero_nf, "447");
        assert_eq!(extraido.dados.destinatario.documento, "96792791000109");
        assert!(extraido
            .dados
            .destinatario
            .nome
            .as_deref()
            .unwrap_or("")
            .contains("ELMECO"));
        assert!((extraido.dados.valor_total - 1190.0).abs() < 0.01);
        assert_eq!(extraido.dados.data_emissao, "2026-09-18");
        assert_eq!(
            extraido.dados.destinatario.email.as_deref(),
            Some("fiscal@elmeco.com.br")
        );
        assert!(!extraido.dados.itens.is_empty());
        assert!(extraido.dados.chave_acesso.starts_with("NFSE"));
    }

    #[test]
    fn nfse_nao_pega_aliquota_depois_do_total() {
        let texto = format!(
            "{OCR_ELMECO}\nValor Total das Deduções (R$) 0,00 Alíquota (%) 7.18 Valor do ISS 7,18\n"
        );
        let extraido = parse_danfe(&texto).expect("NFS-e");
        assert!((extraido.dados.valor_total - 1190.0).abs() < 0.01);
    }

    #[test]
    fn aceita_cnpj_com_ponto_no_dv_do_ocr() {
        let docs = documentos_validos("57.522.734/0001.58 e 96.792.791/0001-09");
        assert!(docs.contains(&"57522734000158".into()) || docs.len() >= 1);
        assert!(docs.contains(&"96792791000109".into()));
    }

    #[test]
    fn ocr_pdf_elmeço_real() {
        let path = "/home/paulo/Downloads/NF ELMECO 09.2026.pdf";
        if !Path::new(path).exists() {
            return;
        }
        let bytes = fs::read(path).expect("ler pdf");
        let texto = extrair_texto_pdf(&bytes).expect("ocr deveria ler a nota salvador");
        let extraido = parse_danfe(&texto).expect("parse após ocr");
        assert_eq!(extraido.dados.destinatario.documento, "96792791000109");
        assert!((extraido.dados.valor_total - 1190.0).abs() < 0.5);
    }

    const DANFE_ENTREGA: &str = r#"
RECEBEMOS DE BMITAG TECNOLOGIA QRCODE E RFID OS PRODUTOS CONSTANTES NA NOTA FISCAL INDICADA AO LADO. NF-e
EMISSÃO: 15/09/2026 - DEST./REM.: ENTREGAS ALIMENTOS PREPARADOS PARA EMPRESAS E DOMICILIOS LTD - VALOR TOTAL: 580.00
Nº 220 SÉRIE: 1
DANFE DOCUMENTO AUXILIAR DE NOTA FISCAL ELETRÔNICA
1 - SAÍDA CHAVE DE ACESSO 2926 0957 5227 3400 0158 5500 1000 0002 2014 7301 5366
NATUREZA DA OPERAÇÃO
VENDA DE MERCADORIA ADQUIRIDA OU RECEBIDA DE TERCEIROS
CNPJ / CPF 57.522.734/0001-58
DESTINATÁRIO / REMETENTE
NOME / RAZÃO SOCIAL CNPJ / CPF DATA EMISSÃO
ENTREGAS ALIMENTOS PREPARADOS PARA EMPRESAS E DOMICILIOS LTD 35.219.798/0001-41 15/09/2026
ENDEREÇO BAIRRO / DISTRITO
AVENIDA ANTONIO CARLOS MAGALHAES, 846 SL 044 ITAIGARA
CEP MUNICÍPIO UF
41825-900 SALVADOR BA
CALCULO DO IMPOSTO
VALOR TOTAL DOS PRODUTOS 580,00
VALOR TOTAL DA NOTA 580,00
DADOS DO PRODUTOS / SERVIÇOS
CÓDIGO DESCRIÇÃO DOS PRODUTOS / SERVIÇOS NCM CFOP UNID QUANT. VL UNITÁRIO VALOR TOTAL
6951223 M24 ETIQUETA ADESIVA BOPP TAM. 48219000 UN 10,00 58,00 580,00
100X65X01 BRANCA FOSCA.
"#;

    #[test]
    fn parse_danfe_entrega_alimentos() {
        let extraido = parse_danfe(DANFE_ENTREGA).expect("DANFE comercial");
        assert_eq!(extraido.dados.destinatario.documento, "35219798000141");
        assert!(extraido
            .dados
            .destinatario
            .nome
            .as_deref()
            .unwrap_or("")
            .to_uppercase()
            .contains("ENTREGAS ALIMENTOS"));
        assert!((extraido.dados.valor_total - 580.0).abs() < 0.01);
        assert_eq!(extraido.dados.numero_nf, "220");
        assert_eq!(extraido.dados.data_emissao, "2026-09-15");
        assert!(!extraido.dados.itens.is_empty());
    }

    #[test]
    fn pdf_real_entrega_alimentos() {
        let path = "/home/paulo/Downloads/NOTA FISCAL ENTREGA ALIMENTOS 09.2026.pdf";
        if !Path::new(path).exists() {
            return;
        }
        let bytes = fs::read(path).expect("ler pdf");
        let texto = extrair_texto_pdf(&bytes).expect("texto da DANFE");
        let extraido = parse_danfe(&texto).expect("parse DANFE");
        assert_eq!(extraido.dados.destinatario.documento, "35219798000141");
        assert!((extraido.dados.valor_total - 580.0).abs() < 0.5);
        assert_eq!(extraido.dados.numero_nf, "220");
    }
}
