use axum::{
    extract::State,
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    Json,
    extract::ConnectInfo,
};
use std::net::SocketAddr;
use serde_json::json;
use crate::models::{LoginDto, RegisterDto, UserResponse};
use crate::services::{AuthResponse, AuthSettings};
use super::AppState;

// Helper to extract IP
pub fn extract_ip(headers: &HeaderMap, addr: SocketAddr) -> String {
    if let Some(forwarded) = headers.get("X-Forwarded-For") {
        if let Ok(s) = forwarded.to_str() {
            return s.split(',').next().unwrap_or(s).trim().to_string();
        }
    }
    addr.ip().to_string()
}

// Helper to map AppError to Axum Response
impl IntoResponse for crate::error::AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            crate::error::AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            crate::error::AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()),
            crate::error::AppError::UserNotFound => (StatusCode::NOT_FOUND, "User not found".to_string()),
            crate::error::AppError::UserAlreadyExists => (StatusCode::CONFLICT, "User already exists".to_string()),
            crate::error::AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            crate::error::AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token".to_string()),
            crate::error::AppError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token expired".to_string()),
            crate::error::AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            crate::error::AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            crate::error::AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Unknown error".to_string()),
        };

        let body = Json(json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

pub async fn get_auth_settings(
    State(state): State<AppState>,
) -> Result<Json<AuthSettings>, crate::error::AppError> {
    let settings = state.auth_service.get_auth_settings().await;
    Ok(Json(settings))
}

pub async fn get_current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<UserResponse>, crate::error::AppError> {
    let auth_header = headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    // Pass tenant_id from claims to get enriched data (role, permissions)
    let user_response = state.auth_service.get_enriched_user(&claims.sub, claims.tenant_id).await?;
    
    Ok(Json(user_response))
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<LoginDto>,
) -> Result<Json<AuthResponse>, crate::error::AppError> {
    // Validate payload (validator crate usage)
    use validator::Validate;
    if let Err(e) = payload.validate() {
        return Err(crate::error::AppError::Validation(format!("Validation error: {}", e)));
    }

    let ip = extract_ip(&headers, addr);
    let response = state.auth_service.login(payload, Some(ip)).await?;
    Ok(Json(response))
}

pub async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<RegisterDto>,
) -> Result<Json<AuthResponse>, crate::error::AppError> {
    use validator::Validate;
    if let Err(e) = payload.validate() {
        return Err(crate::error::AppError::Validation(format!("Validation error: {}", e)));
    }

    let ip = extract_ip(&headers, addr);
    let response = state.auth_service.register(payload, Some(ip)).await?;
    Ok(Json(response))
}

#[derive(serde::Deserialize)]
pub struct VerifyEmailDto {
    token: String,
}

pub async fn verify_email(
    State(state): State<AppState>,
    Json(payload): Json<VerifyEmailDto>,
) -> Result<Json<AuthResponse>, crate::error::AppError> {
    let response = state.auth_service.verify_email(&payload.token).await?;
    Ok(Json(response))
}

#[derive(serde::Deserialize)]
pub struct ForgotPasswordDto {
    email: String,
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    state.auth_service.forgot_password(&payload.email).await?;
    Ok(Json(json!({"message": "Password reset link sent"})))
}

#[derive(serde::Deserialize)]
pub struct ResetPasswordDto {
    token: String,
    password: String,
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    state.auth_service.reset_password(&payload.token, &payload.password).await?;
    Ok(Json(json!({"message": "Password reset successfully"})))
}

#[derive(serde::Deserialize)]
pub struct ValidateTokenDto {
    token: String,
}

pub async fn validate_token(
    State(state): State<AppState>,
    Json(payload): Json<ValidateTokenDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    state.auth_service.validate_token(&payload.token).await?;
    Ok(Json(json!({"valid": true})))
}