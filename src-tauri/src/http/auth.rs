use super::AppState;
use crate::models::{LoginDto, RegisterDto, UserResponse};
use crate::security::access_rules;
use crate::services::{AuthResponse, AuthSettings};
use axum::{
    extract::ConnectInfo,
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose, Engine as _};
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
            crate::error::AppError::Database(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", err),
            ),
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
            crate::error::AppError::Forbidden(msg) => {
                // Log detailed permission info server-side, don't expose to client
                tracing::warn!("Permission denied: {}", msg);
                (StatusCode::FORBIDDEN, "Permission denied".to_string())
            },
            crate::error::AppError::Cache(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            crate::error::AppError::RateLimited(msg) => (StatusCode::TOO_MANY_REQUESTS, msg),
            crate::error::AppError::ServiceUnavailable(msg) => {
                (StatusCode::SERVICE_UNAVAILABLE, msg)
            }
            crate::error::AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            crate::error::AppError::Authentication(msg) => (StatusCode::UNAUTHORIZED, msg),
            crate::error::AppError::Configuration(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            crate::error::AppError::AccountPendingApproval => (
                StatusCode::FORBIDDEN,
                "Account pending approval".to_string(),
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
    headers: HeaderMap,
) -> Result<Json<AuthSettings>, crate::error::AppError> {
    // Require valid auth — don't expose security config to unauthenticated users
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;
    state.auth_service.validate_token(auth_header).await?;

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
) -> Result<axum::response::Response, crate::error::AppError> {
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

    // Build response with httpOnly cookie if token is present
    let mut http_response = axum::response::Json(serde_json::to_value(&response).unwrap_or_default()).into_response();
    if let Some(ref token) = response.token {
        let cookie = format!(
            "{}={}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={}",
            crate::http::AUTH_COOKIE,
            token,
            3600 * 100 // 100 hours matching JWT expiry
        );
        http_response.headers_mut().insert(
            axum::http::header::SET_COOKIE,
            axum::http::HeaderValue::from_str(&cookie).unwrap(),
        );
    }
    Ok(http_response)
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<axum::response::Response, crate::error::AppError> {
    // Try to get token for server-side invalidation
    if let Ok(token) = crate::http::extract_token(&headers) {
        let ip = extract_ip(&headers, addr);
        let _ = state.auth_service.logout(&token, Some(ip)).await;
    }

    // Clear the httpOnly cookie
    let mut response = axum::response::Json(serde_json::json!({"message": "Logged out"})).into_response();
    let clear_cookie = format!(
        "{}=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0",
        crate::http::AUTH_COOKIE
    );
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&clear_cookie).unwrap(),
    );
    Ok(response)
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
    headers: HeaderMap,
    payload: Result<Json<ValidateTokenDto>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let token_from_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    let token_from_body = match payload {
        Ok(Json(body)) => Some(body.token),
        Err(_) => None,
    };

    let token = token_from_header
        .or(token_from_body)
        .ok_or(crate::error::AppError::Unauthorized)?;

    state.auth_service.validate_token(&token).await?;
    Ok(Json(json!({"valid": true})))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
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

// ==================== 2FA Setup Endpoints (temp_token variants for forced enrollment) ====================

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct TempTokenDto {
    temp_token: String,
}

/// Enable 2FA from temp token (forced enrollment flow): Generate Secret & QR Code
pub async fn enable_2fa_temp(
    State(state): State<AppState>,
    Json(payload): Json<TempTokenDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = state
        .auth_service
        .validate_2fa_token(&payload.temp_token)
        .await?;
    let (secret, qr) = state.auth_service.enable_2fa(&claims.sub).await?;

    Ok(Json(json!({
        "secret": secret,
        "qr": qr
    })))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Verify2FASetupTempDto {
    temp_token: String,
    secret: String,
    code: String,
}

/// Verify 2FA Setup from temp token (forced enrollment): Activate 2FA + complete login
pub async fn verify_2fa_setup_temp(
    State(state): State<AppState>,
    Json(payload): Json<Verify2FASetupTempDto>,
) -> Result<Json<AuthResponse>, crate::error::AppError> {
    let claims = state
        .auth_service
        .validate_2fa_token(&payload.temp_token)
        .await?;
    // Activate 2FA for the user
    state
        .auth_service
        .verify_2fa_setup(&claims.sub, &payload.secret, &payload.code)
        .await?;
    // Complete login: generate proper JWT session
    let user = state.auth_service.get_user_by_id(&claims.sub).await?;
    let response = state.auth_service.complete_login(user).await?;

    Ok(Json(response))
}

/// Request Email 2FA Setup from temp token (forced enrollment): Send OTP to email
pub async fn request_email_2fa_setup_temp(
    State(state): State<AppState>,
    Json(payload): Json<TempTokenDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = state
        .auth_service
        .validate_2fa_token(&payload.temp_token)
        .await?;
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
pub struct VerifyEmail2FASetupTempDto {
    temp_token: String,
    code: String,
}

/// Verify Email 2FA Setup from temp token (forced enrollment): Activate email 2FA + complete login
pub async fn verify_email_2fa_setup_temp(
    State(state): State<AppState>,
    Json(payload): Json<VerifyEmail2FASetupTempDto>,
) -> Result<Json<AuthResponse>, crate::error::AppError> {
    let claims = state
        .auth_service
        .validate_2fa_token(&payload.temp_token)
        .await?;
    state
        .auth_service
        .verify_email_2fa_setup(&claims.sub, &payload.code)
        .await?;
    // Complete login after successful 2FA setup
    let user = state.auth_service.get_user_by_id(&claims.sub).await?;
    let response = state.auth_service.complete_login(user).await?;

    Ok(Json(response))
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
pub struct Disable2FADto {
    #[serde(default)]
    code: Option<String>,
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
    let code = payload.code.as_deref().unwrap_or("");

    // If no code provided, generate OTP first (user must confirm with code)
    if code.is_empty() {
        state.auth_service.generate_email_otp(&claims.sub).await?;
        return Ok(Json(json!({
            "requires_verification": true,
            "message": "OTP sent to email. Please send again with code."
        })));
    }

    state
        .auth_service
        .disable_2fa(&claims.sub, code)
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
    let target_is_super_admin: bool =
        sqlx::query_scalar("SELECT is_super_admin FROM users WHERE id = $1")
            .bind(&user_id)
            .fetch_optional(&state.auth_service.pool)
            .await
            .map_err(crate::error::AppError::Database)?
            .ok_or(crate::error::AppError::NotFound(
                "User not found".to_string(),
            ))?;

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

/// Change password (authenticated user)
pub async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ChangePasswordDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;
    state
        .auth_service
        .change_password(&claims.sub, &payload.current_password, &payload.new_password)
        .await?;

    Ok(Json(json!({ "success": true })))
}

/// Update current user profile (authenticated user)
pub async fn update_me(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateMeDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(auth_header).await?;

    let mut updates = Vec::new();
    let mut idx = 1u32;
    if payload.name.is_some() {
        updates.push(format!("name = ${}", idx));
        idx += 1;
    }
    if payload.phone.is_some() {
        updates.push(format!("phone = ${}", idx));
        idx += 1;
    }
    if payload.email.is_some() {
        updates.push(format!("email = ${}", idx));
        idx += 1;
    }

    if updates.is_empty() {
        return Err(crate::error::AppError::Validation(
            "No fields to update".to_string(),
        ));
    }

    updates.push(format!("updated_at = ${}", idx));

    let query_str = format!(
        "UPDATE users SET {} WHERE id = ${}",
        updates.join(", "),
        idx + 1
    );

    let mut query = sqlx::query(&query_str);
    if let Some(ref name) = payload.name {
        query = query.bind(name);
    }
    if let Some(ref phone) = payload.phone {
        query = query.bind(phone);
    }
    if let Some(ref email) = payload.email {
        query = query.bind(email);
    }

    #[cfg(feature = "postgres")]
    {
        query = query.bind(chrono::Utc::now());
    }
    #[cfg(not(feature = "postgres"))]
    {
        query = query.bind(chrono::Utc::now().to_rfc3339());
    }

    query = query.bind(&claims.sub);
    query.execute(&state.auth_service.pool).await?;

    Ok(Json(json!({ "success": true })))
}

#[derive(serde::Deserialize)]
pub struct ChangePasswordDto {
    current_password: String,
    new_password: String,
}

#[derive(serde::Deserialize)]
pub struct UpdateMeDto {
    name: Option<String>,
    phone: Option<String>,
    email: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct UploadAvatarRequest {
    /// Base64-encoded image content (JPEG, PNG, or WebP)
    content: String,
}

/// Upload avatar for the current user.
/// Accepts base64-encoded image (JPEG/PNG/WebP), max 5MB.
pub async fn upload_avatar(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UploadAvatarRequest>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| crate::error::AppError::Unauthorized)?;
    let claims = state.auth_service.validate_token(auth_header).await?;

    // Decode base64
    let bytes = general_purpose::STANDARD
        .decode(&payload.content)
        .map_err(|e| crate::error::AppError::Validation(format!("Invalid base64: {}", e)))?;

    // Validate size (max 5MB)
    if bytes.len() > 5 * 1024 * 1024 {
        return Err(crate::error::AppError::Validation(
            "Ukuran gambar maksimal 5MB".to_string(),
        ));
    }

    // Detect format from magic bytes
    let ext = if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        "jpg"
    } else if bytes.len() >= 8
        && bytes[0] == 0x89
        && bytes[1] == 0x50
        && bytes[2] == 0x4E
        && bytes[3] == 0x47
    {
        "png"
    } else if bytes.len() >= 12
        && bytes[0] == 0x52
        && bytes[1] == 0x49
        && bytes[2] == 0x46
        && bytes[3] == 0x46
        && bytes[8] == 0x57
        && bytes[9] == 0x45
        && bytes[10] == 0x42
        && bytes[11] == 0x50
    {
        "webp"
    } else {
        return Err(crate::error::AppError::Validation(
            "Format gambar tidak didukung. Gunakan JPEG, PNG, atau WebP.".to_string(),
        ));
    };

    // Build upload path: uploads/{tenant_id}/avatars/{user_id}.{ext}
    let tenant_id = claims.tenant_id.as_deref().unwrap_or("system");
    let avatars_dir = state
        .app_data_dir
        .join("uploads")
        .join(tenant_id)
        .join("avatars");

    if !avatars_dir.exists() {
        tokio::fs::create_dir_all(&avatars_dir)
            .await
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    }

    let filename = format!("{}.{}", claims.sub, ext);
    let file_path = avatars_dir.join(&filename);
    tokio::fs::write(&file_path, &bytes)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    // Build the avatar URL (relative path for serving)
    let avatar_url = format!("/api/auth/avatar/{}/{}", tenant_id, filename);

    // Update users.avatar_url
    #[cfg(feature = "postgres")]
    sqlx::query("UPDATE users SET avatar_url = $1, updated_at = $2 WHERE id = $3")
        .bind(&avatar_url)
        .bind(chrono::Utc::now())
        .bind(&claims.sub)
        .execute(&state.auth_service.pool)
        .await?;

    #[cfg(not(feature = "postgres"))]
    sqlx::query("UPDATE users SET avatar_url = ?, updated_at = ? WHERE id = ?")
        .bind(&avatar_url)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&claims.sub)
        .execute(&state.auth_service.pool)
        .await?;

    Ok(Json(json!({ "success": true, "avatar_url": avatar_url })))
}

/// Serve avatar image file.
pub async fn serve_avatar(
    State(state): State<AppState>,
    axum::extract::Path((tenant_id, filename)): axum::extract::Path<(String, String)>,
) -> Result<Response, crate::error::AppError> {
    // Sanitize path components to prevent directory traversal
    if tenant_id.contains("..") || filename.contains("..") || filename.contains('/') {
        return Err(crate::error::AppError::Validation("Invalid path".to_string()));
    }

    let file_path = state
        .app_data_dir
        .join("uploads")
        .join(&tenant_id)
        .join("avatars")
        .join(&filename);

    if !file_path.exists() {
        return Err(crate::error::AppError::NotFound("Avatar not found".to_string()));
    }

    let bytes = tokio::fs::read(&file_path)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    let content_type = if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
        "image/jpeg"
    } else if filename.ends_with(".png") {
        "image/png"
    } else if filename.ends_with(".webp") {
        "image/webp"
    } else {
        "application/octet-stream"
    };

    Ok((
        [(header::CONTENT_TYPE, content_type)],
        bytes,
    )
        .into_response())
}

