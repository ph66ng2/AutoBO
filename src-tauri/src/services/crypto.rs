//! AES-256-GCM encryption for sensitive configuration values.
//! Used for SMTP password, WhatsApp token, and Sicredi API key.

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand::Rng;
use std::env;

#[derive(Debug)]
pub struct CryptoError(String);

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CryptoError: {}", self.0)
    }
}
impl std::error::Error for CryptoError {}

fn get_encryption_key() -> Result<Key<Aes256Gcm>, CryptoError> {
    let key_b64 = env::var("AUTOBO_ENCRYPTION_KEY")
        .map_err(|_| CryptoError("AUTOBO_ENCRYPTION_KEY env var not set".into()))?;
    let key_bytes = BASE64
        .decode(&key_b64)
        .map_err(|e| CryptoError(format!("Invalid base64 key: {}", e)))?;
    if key_bytes.len() != 32 {
        return Err(CryptoError(format!(
            "Key must be 32 bytes, got {}",
            key_bytes.len()
        )));
    }
    Ok(*Key::<Aes256Gcm>::from_slice(&key_bytes))
}

/// Encrypt plaintext with AES-256-GCM. Returns base64(nonce || ciphertext).
pub fn encrypt(plaintext: &str) -> Result<String, CryptoError> {
    let key = get_encryption_key()?;
    let cipher = Aes256Gcm::new(&key);

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| CryptoError(format!("Encryption failed: {}", e)))?;

    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(BASE64.encode(&combined))
}

/// Decrypt base64-encoded ciphertext (format: nonce || ciphertext).
pub fn decrypt(encoded: &str) -> Result<String, CryptoError> {
    let key = get_encryption_key()?;
    let cipher = Aes256Gcm::new(&key);

    let combined = BASE64
        .decode(encoded)
        .map_err(|e| CryptoError(format!("Invalid base64 input: {}", e)))?;

    if combined.len() < 12 {
        return Err(CryptoError("Ciphertext too short (must be >= 12 bytes)".into()));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError(format!("Decryption failed: {}", e)))?;

    String::from_utf8(plaintext)
        .map_err(|e| CryptoError(format!("Invalid UTF-8 in decrypted data: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn set_test_key() {
        // 32 bytes base64-encoded: "abcdefghijklmnopqrstuvwxyz012345"
        let key = "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXowMTIzNDU=";
        env::set_var("AUTOBO_ENCRYPTION_KEY", key);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        set_test_key();
        let plaintext = "Hello, AutoBO!";
        let encrypted = encrypt(plaintext).expect("encrypt failed");
        let decrypted = decrypt(&encrypted).expect("decrypt failed");
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_different_outputs() {
        set_test_key();
        let e1 = encrypt("test").expect("encrypt 1 failed");
        let e2 = encrypt("test").expect("encrypt 2 failed");
        assert_ne!(
            e1, e2,
            "Same plaintext should produce different ciphertext"
        );
    }

    #[test]
    fn test_invalid_ciphertext_rejected() {
        set_test_key();
        assert!(decrypt("not-valid-base64!!!").is_err());
        assert!(decrypt("").is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        set_test_key();
        let encrypted = encrypt("").expect("encrypt empty failed");
        let decrypted = decrypt(&encrypted).expect("decrypt empty failed");
        assert_eq!("", decrypted);
    }

    #[test]
    fn test_wrong_key_fails() {
        set_test_key();
        let encrypted = encrypt("secret").expect("encrypt failed");
        env::set_var(
            "AUTOBO_ENCRYPTION_KEY",
            "QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVowMTIzNDU2Nzg=",
        );
        assert!(
            decrypt(&encrypted).is_err(),
            "Decryption with wrong key should fail"
        );
    }
}
