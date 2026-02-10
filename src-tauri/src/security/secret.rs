use crate::error::{AppError, AppResult};
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose, Engine as _};
use rand_core::RngCore;
use sha2::{Digest, Sha256};

const PREFIX: &str = "enc:v1:";

fn derive_key() -> AppResult<[u8; 32]> {
    // We intentionally allow passing a passphrase-like string.
    // It is hashed into a fixed 32-byte key.
    let raw = std::env::var("MIKROTIK_CRED_KEY")
        .map_err(|_| AppError::Internal("Missing env MIKROTIK_CRED_KEY".into()))?;
    if raw.trim().is_empty() {
        return Err(AppError::Internal("Missing env MIKROTIK_CRED_KEY".into()));
    }
    let digest = Sha256::digest(raw.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest[..32]);
    Ok(out)
}

pub fn encrypt_secret(plaintext: &str) -> AppResult<String> {
    if plaintext.trim().is_empty() {
        return Ok(String::new());
    }
    if plaintext.starts_with(PREFIX) {
        return Ok(plaintext.to_string());
    }

    let key = derive_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| AppError::Internal("Invalid key".into()))?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| AppError::Internal("Failed to encrypt secret".into()))?;

    let mut blob = Vec::with_capacity(12 + ciphertext.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);

    Ok(format!(
        "{}{}",
        PREFIX,
        general_purpose::STANDARD_NO_PAD.encode(blob)
    ))
}

pub fn decrypt_secret(stored: &str) -> AppResult<String> {
    if stored.trim().is_empty() {
        return Ok(String::new());
    }
    if !stored.starts_with(PREFIX) {
        // Backward compatibility: plaintext stored in DB
        return Ok(stored.to_string());
    }

    let key = derive_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| AppError::Internal("Invalid key".into()))?;

    let b64 = &stored[PREFIX.len()..];
    let blob = general_purpose::STANDARD_NO_PAD
        .decode(b64)
        .map_err(|_| AppError::Internal("Invalid encrypted secret".into()))?;
    if blob.len() < 12 {
        return Err(AppError::Internal("Invalid encrypted secret".into()));
    }
    let (nonce_bytes, ciphertext) = blob.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| AppError::Internal("Failed to decrypt secret".into()))?;

    String::from_utf8(plaintext).map_err(|_| AppError::Internal("Invalid decrypted secret".into()))
}

pub fn decrypt_secret_opt(stored: &str) -> AppResult<Option<String>> {
    let s = decrypt_secret(stored)?;
    if s.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}
