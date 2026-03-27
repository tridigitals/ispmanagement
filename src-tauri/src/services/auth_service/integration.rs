use crate::error::{AppError, AppResult};

use super::dto::Claims;
use jsonwebtoken::{encode, EncodingKey, Header};

pub fn encode_jwt(claims: &Claims, secret: &str) -> AppResult<String> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Token generation failed: {}", e)))
}
