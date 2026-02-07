use crate::error::{AppError, AppResult};
use crate::db::DbPool;
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeClaims {
    pub sub: String, // user_id
    pub tenant_id: Option<String>,
    pub category: String,
    pub channel: String,
    pub exp: usize,
}

#[cfg(feature = "postgres")]
async fn load_jwt_secret(pool: &DbPool) -> AppResult<String> {
    let secret: Option<String> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'jwt_secret' AND tenant_id IS NULL LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;
    Ok(secret.unwrap_or_else(|| "dev-secret".to_string()))
}

#[cfg(not(feature = "postgres"))]
async fn load_jwt_secret(_pool: &DbPool) -> AppResult<String> {
    Ok("dev-secret".to_string())
}

pub async fn encode_unsubscribe_token(
    pool: &DbPool,
    user_id: &str,
    tenant_id: Option<String>,
    category: &str,
    channel: &str,
    ttl_days: i64,
) -> AppResult<String> {
    let secret = load_jwt_secret(pool).await?;
    let exp = (Utc::now() + chrono::Duration::days(ttl_days.max(1))).timestamp() as usize;
    let claims = UnsubscribeClaims {
        sub: user_id.to_string(),
        tenant_id,
        category: category.to_string(),
        channel: channel.to_string(),
        exp,
    };
    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Token encode failed: {}", e)))
}

pub async fn decode_unsubscribe_token(pool: &DbPool, token: &str) -> AppResult<UnsubscribeClaims> {
    let secret = load_jwt_secret(pool).await?;
    let mut validation = Validation::default();
    validation.validate_exp = true;

    let data = jsonwebtoken::decode::<UnsubscribeClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| AppError::Validation(format!("Invalid token: {}", e)))?;

    Ok(data.claims)
}

