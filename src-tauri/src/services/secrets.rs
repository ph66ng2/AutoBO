//! OS keyring for Sicredi secrets (api_key, codigo_acesso).
//! Never return decrypted secrets to the frontend.

use keyring::{Entry, Error as KeyringError};

const SERVICE: &str = "com.bmitag.autobo";

#[derive(Debug)]
pub enum SecretsError {
    Unavailable(String),
    NotFound,
    Other(String),
}

impl std::fmt::Display for SecretsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(m) => write!(f, "Cofre do sistema indisponível: {m}"),
            Self::NotFound => write!(f, "Segredo não encontrado no cofre"),
            Self::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for SecretsError {}

fn map_err(e: KeyringError) -> SecretsError {
    match e {
        KeyringError::NoEntry => SecretsError::NotFound,
        KeyringError::PlatformFailure(inner) => {
            SecretsError::Unavailable(inner.to_string())
        }
        KeyringError::NoStorageAccess(inner) => {
            SecretsError::Unavailable(inner.to_string())
        }
        other => SecretsError::Other(other.to_string()),
    }
}

fn entry(chave: &str) -> Result<Entry, SecretsError> {
    Entry::new(SERVICE, chave).map_err(map_err)
}

/// Probe whether the OS secret service is usable.
pub fn probe_keyring() -> Result<(), SecretsError> {
    let e = entry("sicredi._probe")?;
    e.set_password("ok").map_err(map_err)?;
    let _ = e.delete_credential();
    Ok(())
}

pub fn set_secret(chave: &str, valor: &str) -> Result<(), SecretsError> {
    if valor.is_empty() {
        return delete_secret(chave);
    }
    let e = entry(chave)?;
    e.set_password(valor).map_err(map_err)
}

pub fn get_secret(chave: &str) -> Result<String, SecretsError> {
    let e = entry(chave)?;
    e.get_password().map_err(map_err)
}

pub fn delete_secret(chave: &str) -> Result<(), SecretsError> {
    let e = entry(chave)?;
    match e.delete_credential() {
        Ok(()) => Ok(()),
        Err(KeyringError::NoEntry) => Ok(()),
        Err(other) => Err(map_err(other)),
    }
}

pub fn secret_configured(chave: &str) -> bool {
    matches!(get_secret(chave), Ok(v) if !v.is_empty())
}

pub const SICREDI_API_KEY: &str = "sicredi.api_key";
pub const SICREDI_CODIGO_ACESSO: &str = "sicredi.codigo_acesso";
