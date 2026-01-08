use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use crate::models::{LoginDto, RegisterDto};
use crate::services::AuthResponse;
use super::AppState;

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
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Unknown error".to_string()),
        };

        let body = Json(json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginDto>,
) -> Result<Json<AuthResponse>, crate::error::AppError> {
    // Validate payload (validator crate usage)
    use validator::Validate;
    if let Err(e) = payload.validate() {
        return Err(crate::error::AppError::Validation(format!("Validation error: {}", e)));
    }

    let response = state.auth_service.login(payload).await?;
    Ok(Json(response))
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterDto>,
) -> Result<Json<AuthResponse>, crate::error::AppError> {
    use validator::Validate;
    if let Err(e) = payload.validate() {
        return Err(crate::error::AppError::Validation(format!("Validation error: {}", e)));
    }

    let response = state.auth_service.register(payload).await?;
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
