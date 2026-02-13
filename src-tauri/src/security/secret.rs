use crate::error::{AppError, AppResult};
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose, Engine as _};
use rand_core::RngCore;
use sha2::{Digest, Sha256};

const PREFIX: &str = "enc:v1:";

fn get_master_secret() -> AppResult<String> {
    // Prefer a single app-wide master secret (can be used for other purposes too).
    // Keep backward compatibility with the old env var.
    let raw = std::env::var("APP_SECRET")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            std::env::var("MIKROTIK_CRED_KEY")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
        .ok_or_else(|| {
            AppError::Internal("Missing env APP_SECRET (or legacy MIKROTIK_CRED_KEY)".into())
        })?;

    Ok(raw)
}

fn derive_key_for(purpose: &str) -> AppResult<[u8; 32]> {
    // Domain-separated derivation from the master secret.
    // This prevents key reuse across different crypto purposes.
    let master = get_master_secret()?;
    let raw = format!("{master}:{purpose}:v1");
    let digest = Sha256::digest(raw.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest[..32]);
    Ok(out)
}

pub fn encrypt_secret(plaintext: &str) -> AppResult<String> {
    // Backward compatible: this function remains dedicated for MikroTik router credentials.
    encrypt_secret_for("mikrotik_credentials", plaintext)
}

pub fn encrypt_secret_for(purpose: &str, plaintext: &str) -> AppResult<String> {
    if plaintext.trim().is_empty() {
        return Ok(String::new());
    }
    if plaintext.starts_with(PREFIX) {
        return Ok(plaintext.to_string());
    }

    let key = derive_key_for(purpose)?;
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
    // Backward compatible: this function remains dedicated for MikroTik router credentials.
    decrypt_secret_for("mikrotik_credentials", stored)
}

pub fn decrypt_secret_for(purpose: &str, stored: &str) -> AppResult<String> {
    if stored.trim().is_empty() {
        return Ok(String::new());
    }
    if !stored.starts_with(PREFIX) {
        // Backward compatibility: plaintext stored in DB
        return Ok(stored.to_string());
    }

    let key = derive_key_for(purpose)?;
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

pub fn decrypt_secret_opt_for(purpose: &str, stored: &str) -> AppResult<Option<String>> {
    let s = decrypt_secret_for(purpose, stored)?;
    if s.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}
