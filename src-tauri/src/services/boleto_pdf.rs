use printpdf::{BuiltinFont, Mm, PdfDocument};
use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;

pub struct BoletoPdfDados<'a> {
    pub seu_numero: &'a str,
    pub pagador: &'a str,
    pub documento: &'a str,
    pub valor: f64,
    pub vencimento: &'a str,
    pub origem: &'a str,
    pub mensagem: Option<&'a str>,
}

pub fn gerar_pdf_boleto(dados: BoletoPdfDados<'_>) -> Result<PathBuf, String> {
    let dir = boletos_dir()?;
    fs::create_dir_all(&dir).map_err(|e| format!("Não foi possível criar pasta de boletos: {e}"))?;

    let path = dir.join(nome_arquivo_boleto(dados.seu_numero));

    let (doc, page1, layer1) = PdfDocument::new("AutoBO Boleto", Mm(210.0), Mm(297.0), "Camada");
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Fonte do PDF: {e}"))?;
    let current_layer = doc.get_page(page1).get_layer(layer1);

    current_layer.use_text("BMITAG  AutoBO", 16.0, Mm(20.0), Mm(270.0), &font);
    current_layer.use_text(
        "Comprovante de cobranca (PDF local)",
        11.0,
        Mm(20.0),
        Mm(258.0),
        &font,
    );
    current_layer.use_text(
        format!("Numero: {}", dados.seu_numero),
        12.0,
        Mm(20.0),
        Mm(240.0),
        &font,
    );
    current_layer.use_text(
        format!("Pagador: {}", dados.pagador),
        11.0,
        Mm(20.0),
        Mm(230.0),
        &font,
    );
    current_layer.use_text(
        format!("Documento: {}", dados.documento),
        11.0,
        Mm(20.0),
        Mm(220.0),
        &font,
    );
    current_layer.use_text(
        format!("Valor: R$ {:.2}", dados.valor),
        11.0,
        Mm(20.0),
        Mm(210.0),
        &font,
    );
    current_layer.use_text(
        format!("Vencimento: {}", dados.vencimento),
        11.0,
        Mm(20.0),
        Mm(200.0),
        &font,
    );
    current_layer.use_text(
        format!("Origem: {}", dados.origem),
        11.0,
        Mm(20.0),
        Mm(190.0),
        &font,
    );
    if let Some(msg) = dados.mensagem.filter(|m| !m.is_empty()) {
        current_layer.use_text(format!("Mensagem: {msg}"), 10.0, Mm(20.0), Mm(176.0), &font);
    }
    current_layer.use_text(
        "Registro Sicredi ainda nao aplicado. PDF para revisao e envio manual.",
        9.0,
        Mm(20.0),
        Mm(155.0),
        &font,
    );

    let file = fs::File::create(&path).map_err(|e| format!("Erro ao gravar PDF: {e}"))?;
    doc.save(&mut BufWriter::new(file))
        .map_err(|e| format!("Erro ao salvar PDF: {e}"))?;

    Ok(path)
}

pub fn nome_arquivo_boleto(seu_numero: &str) -> String {
    let safe_numero: String = seu_numero
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("boleto-{safe_numero}.pdf")
}

pub fn caminho_pdf_boleto(seu_numero: &str) -> Result<PathBuf, String> {
    Ok(boletos_dir()?.join(nome_arquivo_boleto(seu_numero)))
}

pub fn boletos_dir() -> Result<PathBuf, String> {
    let base = directories::UserDirs::new()
        .and_then(|d| d.document_dir().map(|p| p.to_path_buf()))
        .or_else(|| {
            directories::ProjectDirs::from("com", "bmitag", "AutoBO")
                .map(|d| d.data_local_dir().to_path_buf())
        })
        .ok_or_else(|| "Não foi possível resolver a pasta de documentos".to_string())?;
    Ok(base.join("AutoBO").join("Boletos"))
}

pub fn abrir_arquivo(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", path])
            .spawn()
            .map_err(|e| format!("Não foi possível abrir o arquivo: {e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Não foi possível abrir o arquivo: {e}"))?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Não foi possível abrir o arquivo: {e}"))?;
        Ok(())
    }
}
