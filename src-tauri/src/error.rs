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

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Permission denied: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Result type alias for application operations
pub type AppResult<T> = Result<T, AppError>;
