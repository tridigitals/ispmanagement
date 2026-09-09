//! SaaS Boilerplate - Error Handling Module

use thiserror::Error;

/// Application-wide error types
#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("User not found")]
    UserNotFound,

    #[error("User already exists")]
    UserAlreadyExists,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not authorized")]
    Unauthorized,

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Permission denied: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimited(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Account pending approval")]
    AccountPendingApproval,
}

impl AppError {
    /// Kode mesin stabil untuk frontend (lihat FE extractApiErrorCode).
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Database(_) => "DATABASE",
            AppError::Authentication(_) => "AUTHENTICATION",
            AppError::InvalidCredentials => "INVALID_CREDENTIALS",
            AppError::UserNotFound => "USER_NOT_FOUND",
            AppError::UserAlreadyExists => "USER_ALREADY_EXISTS",
            AppError::TokenExpired => "TOKEN_EXPIRED",
            AppError::InvalidToken => "INVALID_TOKEN",
            AppError::Validation(_) => "VALIDATION",
            AppError::Unauthorized => "UNAUTHORIZED",
            AppError::Configuration(_) => "CONFIGURATION",
            AppError::Internal(_) => "INTERNAL",
            AppError::Forbidden(_) => "FORBIDDEN",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Cache(_) => "CACHE",
            AppError::RateLimited(_) => "RATE_LIMITED",
            AppError::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
            AppError::Conflict(_) => "CONFLICT",
            AppError::AccountPendingApproval => "ACCOUNT_PENDING_APPROVAL",
        }
    }

    /// Pesan yang boleh diperlihatkan ke user. Variant yang membungkus
    /// internals (sqlx chain, file path, config) digeneralisasi — sama seperti
    /// yang sudah dilakukan `IntoResponse` di jalur HTTP (A-08: jalur Tauri
    /// kini memakai kontrak yang sama).
    pub fn public_message(&self) -> String {
        match self {
            AppError::Validation(msg)
            | AppError::NotFound(msg)
            | AppError::Conflict(msg)
            | AppError::RateLimited(msg) => msg.clone(),
            AppError::Forbidden(_) => "Anda tidak memiliki izin untuk aksi ini.".to_string(),
            AppError::Authentication(_) => "Autentikasi gagal.".to_string(),
            AppError::InvalidCredentials => "Kredensial tidak valid.".to_string(),
            AppError::UserAlreadyExists => "Pengguna sudah terdaftar.".to_string(),
            AppError::UserNotFound => "Pengguna tidak ditemukan.".to_string(),
            AppError::TokenExpired => "Sesi berakhir, silakan masuk kembali.".to_string(),
            AppError::InvalidToken | AppError::Unauthorized => "Tidak terautentikasi.".to_string(),
            AppError::Database(_)
            | AppError::Internal(_)
            | AppError::Cache(_)
            | AppError::Configuration(_)
            | AppError::ServiceUnavailable(_) => {
                "Terjadi kesalahan pada server. Coba lagi.".to_string()
            }
            AppError::AccountPendingApproval => "Akun menunggu persetujuan admin.".to_string(),
        }
    }
}

impl serde::Serialize for AppError {
    /// A-08: envelope bertipe untuk SEMUA jalur (Tauri reject maupun HTTP).
    /// `message` = detail untuk log/developer; `public_message` = aman tampil;
    /// `code` = mesin-stabil. FE (`core.ts`) menegakkan pemakaian publik.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 4)?;
        s.serialize_field("kind", self.code())?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.serialize_field("public_message", &self.public_message())?;
        s.end()
    }
}

/// Result type alias for application operations
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serialize_memisahkan_publik_dan_interna() {
        let e = AppError::Database(sqlx::Error::RowNotFound);
        let v: serde_json::Value = serde_json::to_value(&e).unwrap();
        assert_eq!(v["kind"], "DATABASE");
        assert_eq!(v["code"], "DATABASE");
        // detail internal TETAP ada untuk log developer...
        assert!(v["message"].as_str().unwrap().contains("Database error"));
        // ...tapi public_message tidak memuat internals.
        let pub_msg = v["public_message"].as_str().unwrap();
        assert!(!pub_msg.to_lowercase().contains("database"));
        assert!(!pub_msg.is_empty());

        let val = AppError::Validation("Nama wajib diisi".into());
        let v: serde_json::Value = serde_json::to_value(&val).unwrap();
        assert_eq!(v["public_message"], "Nama wajib diisi");
        assert_eq!(v["code"], "VALIDATION");
    }

    #[test]
    fn forbidden_tidak_membocorkan_alasan_detail() {
        let e = AppError::Forbidden("user 42 lacks pppoe:manage on tenant x".into());
        let pub_msg = e.public_message();
        assert!(!pub_msg.contains("user 42"));
        assert!(!pub_msg.contains("tenant"));
        assert_eq!(e.code(), "FORBIDDEN");
    }
}
