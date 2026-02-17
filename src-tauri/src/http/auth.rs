use super::AppState;
use crate::models::{LoginDto, RegisterDto, UserResponse};
use crate::security::access_rules;
use crate::services::{AuthResponse, AuthSettings};
use axum::{
    extract::ConnectInfo,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::net::SocketAddr;

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
            crate::error::AppError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string())
            }
            crate::error::AppError::UserNotFound => {
                (StatusCode::NOT_FOUND, "User not found".to_string())
            }
            crate::error::AppError::UserAlreadyExists => {
                (StatusCode::CONFLICT, "User already exists".to_string())
            }
            crate::error::AppError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "Unauthorized".to_string())
            }
            crate::error::AppError::InvalidToken => {
                (StatusCode::UNAUTHORIZED, "Invalid token".to_string())
            }
            crate::error::AppError::TokenExpired => {
                (StatusCode::UNAUTHORIZED, "Token expired".to_string())
            }
            crate::error::AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            crate::error::AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            crate::error::AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            crate::error::AppError::Cache(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            crate::error::AppError::RateLimited(msg) => (StatusCode::TOO_MANY_REQUESTS, msg),
            crate::error::AppError::ServiceUnavailable(msg) => {
                (StatusCode::SERVICE_UNAVAILABLE, msg)
            }
            crate::error::AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unknown error".to_string(),
            ),
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
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    // Pass tenant_id from claims to get enriched data (role, permissions)
    let user_response = state
        .auth_service
        .get_enriched_user(&claims.sub, claims.tenant_id)
        .await?;

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
        return Err(crate::error::AppError::Validation(format!(
            "Validation error: {}",
            e
        )));
    }

    let ip = extract_ip(&headers, addr);

    // Generate device fingerprint from User-Agent + IP for trusted device check
    let user_agent = headers
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    let device_fingerprint =
        crate::services::AuthService::generate_device_fingerprint(user_agent.as_deref(), Some(&ip));

    let response = state
        .auth_service
        .login(payload, Some(ip), Some(device_fingerprint))
        .await?;
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
        return Err(crate::error::AppError::Validation(format!(
            "Validation error: {}",
            e
        )));
    }

    let ip = extract_ip(&headers, addr);
    let response = state.auth_service.register(payload, Some(ip)).await?;
    Ok(Json(response))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct ResetPasswordDto {
    token: String,
    password: String,
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    state
        .auth_service
        .reset_password(&payload.token, &payload.password)
        .await?;
    Ok(Json(json!({"message": "Password reset successfully"})))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Verify2faDto {
    temp_token: String,
    code: String,
    trust_device: Option<bool>,
    _device_fingerprint: Option<String>,
}

pub async fn verify_login_2fa(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<Verify2faDto>,
) -> Result<Json<AuthResponse>, crate::error::AppError> {
    let response = state
        .auth_service
        .verify_login_2fa(&payload.temp_token, &payload.code)
        .await?;

    // Trust device if requested
    // Trust device if requested
    if payload.trust_device.unwrap_or(false) {
        let ip = extract_ip(&headers, addr);
        let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

        // Generate fingerprint internally
        let fingerprint =
            crate::services::AuthService::generate_device_fingerprint(user_agent, Some(&ip));

        let _ = state
            .auth_service
            .trust_device(&response.user.id, &fingerprint, Some(&ip), user_agent)
            .await;
    }

    Ok(Json(response))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct RequestEmailOtpDto {
    temp_token: String,
}

pub async fn request_email_otp(
    State(state): State<AppState>,
    Json(payload): Json<RequestEmailOtpDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = state
        .auth_service
        .validate_2fa_token(&payload.temp_token)
        .await?;
    state.auth_service.generate_email_otp(&claims.sub).await?;
    Ok(Json(json!({"message": "OTP sent to email"})))
}

pub async fn verify_email_otp(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<Verify2faDto>,
) -> Result<Json<AuthResponse>, crate::error::AppError> {
    let response = state
        .auth_service
        .verify_email_otp(&payload.temp_token, &payload.code)
        .await?;

    // Trust device if requested
    // Trust device if requested
    if payload.trust_device.unwrap_or(false) {
        let ip = extract_ip(&headers, addr);
        let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

        // Generate fingerprint internally
        let fingerprint =
            crate::services::AuthService::generate_device_fingerprint(user_agent, Some(&ip));

        let _ = state
            .auth_service
            .trust_device(&response.user.id, &fingerprint, Some(&ip), user_agent)
            .await;
    }

    Ok(Json(response))
}

pub async fn get_2fa_methods(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, crate::error::AppError> {
    let methods = state.auth_service.get_available_2fa_methods().await;
    Ok(Json(methods))
}

// ==================== 2FA Setup Endpoints ====================

/// Enable 2FA: Generate Secret & QR Code
pub async fn enable_2fa(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    let (secret, qr) = state.auth_service.enable_2fa(&claims.sub).await?;

    Ok(Json(json!({
        "secret": secret,
        "qr": qr
    })))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verify2FASetupDto {
    secret: String,
    code: String,
}

/// Verify 2FA Setup: Validate code and enable 2FA
pub async fn verify_2fa_setup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Verify2FASetupDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    let recovery_codes = state
        .auth_service
        .verify_2fa_setup(&claims.sub, &payload.secret, &payload.code)
        .await?;

    Ok(Json(json!({
        "recovery_codes": recovery_codes
    })))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Disable2FADto {
    code: String,
}

/// Disable 2FA
pub async fn disable_2fa(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Disable2FADto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    state
        .auth_service
        .disable_2fa(&claims.sub, &payload.code)
        .await?;

    Ok(Json(json!({
        "success": true
    })))
}

/// Request Email OTP for disabling 2FA
pub async fn request_2fa_disable_code(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    state.auth_service.generate_email_otp(&claims.sub).await?;

    Ok(Json(json!({
        "message": "OTP sent to email"
    })))
}

use axum::extract::Path;

/// Reset 2FA for a specific user (Admin only)
pub async fn reset_user_2fa(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    let target_is_super_admin: bool = sqlx::query_scalar("SELECT is_super_admin FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_optional(&state.auth_service.pool)
        .await
        .map_err(crate::error::AppError::Database)?
        .ok_or(crate::error::AppError::NotFound("User not found".to_string()))?;

    if !claims.is_super_admin {
        let tenant_id = claims
            .tenant_id
            .clone()
            .ok_or(crate::error::AppError::Forbidden(
                "Tenant context required".to_string(),
            ))?;
        let has_team_update_permission = state
            .auth_service
            .has_permission(&claims.sub, &tenant_id, "team", "update")
            .await?;

        let target_in_same_tenant: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM tenant_members WHERE tenant_id = $1 AND user_id = $2)",
        )
        .bind(&tenant_id)
        .bind(&user_id)
        .fetch_one(&state.auth_service.pool)
        .await
        .map_err(crate::error::AppError::Database)?;

        if !access_rules::can_reset_user_2fa(
            claims.is_super_admin,
            has_team_update_permission,
            target_in_same_tenant,
            target_is_super_admin,
        ) {
            return Err(crate::error::AppError::Forbidden(
                "Not allowed to reset 2FA for target user".to_string(),
            ));
        }
    }

    state
        .auth_service
        .reset_2fa(&user_id, Some(&claims.sub), None)
        .await?;

    Ok(Json(json!({ "success": true })))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Set2FAPreferenceDto {
    method: String,
}

/// Set 2FA Preference
pub async fn set_2fa_preference(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<Set2FAPreferenceDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    state
        .auth_service
        .set_2fa_preference(&claims.sub, &payload.method)
        .await?;

    Ok(Json(json!({
        "success": true
    })))
}

/// Request Email 2FA Setup
pub async fn request_email_2fa_setup(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    state
        .auth_service
        .request_email_2fa_setup(&claims.sub)
        .await?;

    Ok(Json(json!({
        "message": "OTP sent to email"
    })))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct VerifyEmail2FASetupDto {
    pub code: String,
}

/// Verify Email 2FA Setup
pub async fn verify_email_2fa_setup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<VerifyEmail2FASetupDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;

    state
        .auth_service
        .verify_email_2fa_setup(&claims.sub, &payload.code)
        .await?;

    Ok(Json(json!({
        "success": true
    })))
}

use crate::models::TrustedDevice;

/// List Trusted Devices
pub async fn list_trusted_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TrustedDevice>>, crate::error::AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    let devices = state.auth_service.list_trusted_devices(&claims.sub).await?;

    Ok(Json(devices))
}

/// Revoke Trusted Device
pub async fn revoke_trusted_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(device_id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    state
        .auth_service
        .revoke_trusted_device(&claims.sub, &device_id)
        .await?;

    Ok(Json(json!({ "success": true })))
}
