//! Persistência dos PDFs da oficina nos mesmos diretórios do AutoOS.

use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{error, info};

const AUTOOS_ORDERS_DIR: &str = "Ordens de Servico";
const AUTOOS_QUOTES_DIR: &str = "Orcamentos";

fn default_documents_directory() -> Result<PathBuf, String> {
    let base_dir = std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join("Documents"))
        .or_else(|| {
            directories::UserDirs::new()
                .and_then(|dirs| dirs.document_dir().map(|path| path.to_path_buf()))
        })
        .ok_or_else(|| "Não foi possível localizar a pasta Documents".to_string())?;
    fs::create_dir_all(&base_dir)
        .map_err(|e| format!("Erro ao preparar diretório Documents: {e}"))?;
    Ok(base_dir)
}

fn default_orders_directory() -> Result<PathBuf, String> {
    let dir = default_documents_directory()?.join(AUTOOS_ORDERS_DIR);
    fs::create_dir_all(&dir).map_err(|e| format!("Erro ao preparar diretório de OS: {e}"))?;
    Ok(dir)
}

fn default_quotes_directory() -> Result<PathBuf, String> {
    let dir = default_documents_directory()?.join(AUTOOS_QUOTES_DIR);
    fs::create_dir_all(&dir).map_err(|e| format!("Erro ao preparar diretório de orçamentos: {e}"))?;
    Ok(dir)
}

fn sanitize_filename_component(value: &str) -> String {
    let sanitized: String = value
        .trim()
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' => character,
            _ => '_',
        })
        .collect();
    sanitized.trim_matches('_').chars().take(80).collect()
}

fn reveal_file_in_manager(file_path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg("/select,")
            .arg(file_path)
            .spawn()
            .map_err(|e| format!("Arquivo salvo, mas não foi possível abrir o Explorer: {e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(file_path)
            .spawn()
            .map_err(|e| format!("Arquivo salvo, mas não foi possível abrir o Finder: {e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        let file_path_string = file_path.to_string_lossy().to_string();
        for (command_name, args) in [
            ("nautilus", vec!["--select".to_string(), file_path_string.clone()]),
            ("dolphin", vec!["--select".to_string(), file_path_string.clone()]),
            ("nemo", vec!["--no-desktop".to_string(), file_path_string.clone()]),
        ] {
            if Command::new(command_name).args(&args).spawn().is_ok() {
                return Ok(());
            }
        }
        let parent = file_path.parent().unwrap_or(file_path);
        Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| format!("Arquivo salvo, mas não foi possível abrir a pasta: {e}"))?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    Ok(())
}

fn write_pdf(dir: PathBuf, file_name: String, bytes: &[u8]) -> Result<String, String> {
    let file_path = dir.join(&file_name);
    fs::write(&file_path, bytes).map_err(|e| format!("Erro ao salvar documento: {e}"))?;
    if let Err(reveal_error) = reveal_file_in_manager(&file_path) {
        error!("{reveal_error}");
        return Err(reveal_error);
    }
    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn salvar_ordem_servico_pdf(
    bytes: Vec<u8>,
    empresa_nome: Option<String>,
    nome_arquivo: Option<String>,
) -> Result<String, String> {
    let file_name = if let Some(nome) = nome_arquivo.filter(|n| !n.is_empty()) {
        nome
    } else {
        let empresa = sanitize_filename_component(empresa_nome.as_deref().unwrap_or("Empresa"));
        let empresa = if empresa.is_empty() {
            "Empresa".to_string()
        } else {
            empresa
        };
        format!(
            "OrdemServico_{}_{}.pdf",
            empresa,
            Utc::now().format("%Y-%m-%d_%H-%M-%S")
        )
    };
    let path = write_pdf(default_orders_directory()?, file_name, &bytes)?;
    info!("Ordem de serviço salva em {path}");
    Ok(path)
}

#[tauri::command]
pub async fn salvar_orcamento_pdf(
    bytes: Vec<u8>,
    empresa_nome: Option<String>,
    nome_arquivo: Option<String>,
) -> Result<String, String> {
    let file_name = if let Some(nome) = nome_arquivo.filter(|n| !n.is_empty()) {
        nome
    } else {
        let empresa = sanitize_filename_component(empresa_nome.as_deref().unwrap_or("Cliente"));
        let empresa = if empresa.is_empty() {
            "Cliente".to_string()
        } else {
            empresa
        };
        format!(
            "Orcamento_{}_{}.pdf",
            empresa,
            Utc::now().format("%Y-%m-%d_%H-%M-%S")
        )
    };
    let path = write_pdf(default_quotes_directory()?, file_name, &bytes)?;
    info!("Orçamento salvo em {path}");
    Ok(path)
}
