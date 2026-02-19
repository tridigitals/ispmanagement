use super::AppState;
use crate::http::auth::extract_ip;
use crate::models::{
    CreateUserAddressDto, CreateUserDto, PaginatedResponse, UpdateUserAddressDto, UpdateUserDto,
    UserAddress, UserResponse,
};
use crate::security::access_rules;
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListUsersQuery {
    page: Option<u32>,
    #[serde(rename = "perPage", alias = "per_page")]
    per_page: Option<u32>,
}

fn extract_token(headers: &HeaderMap) -> Result<String, crate::error::AppError> {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or(crate::error::AppError::Unauthorized)
}

pub async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<PaginatedResponse<UserResponse>>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    if !access_rules::can_access_global_user_management(claims.is_super_admin) {
        return Err(crate::error::AppError::Unauthorized);
    }

    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(10);

    let (users, total) = state.user_service.list(page, per_page).await?;

    Ok(Json(PaginatedResponse {
        data: users,
        total,
        page,
        per_page,
    }))
}

pub async fn get_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<UserResponse>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    if !access_rules::can_access_global_user_management(claims.is_super_admin) {
        return Err(crate::error::AppError::Unauthorized);
    }

    let user = state.user_service.get_by_id(&id).await?;
    Ok(Json(user))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateUserDto2 {
    email: String,
    password: String,
    name: String,
}

pub async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<CreateUserDto2>,
) -> Result<Json<UserResponse>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    if !access_rules::can_access_global_user_management(claims.is_super_admin) {
        return Err(crate::error::AppError::Unauthorized);
    }
    let ip = extract_ip(&headers, addr);

    use validator::Validate;
    let dto = CreateUserDto {
        email: payload.email,
        password: payload.password,
        name: payload.name,
    };

    if let Err(e) = dto.validate() {
        return Err(crate::error::AppError::Validation(format!(
            "Validation error: {}",
            e
        )));
    }

    let user = state
        .user_service
        .create(dto, Some(&claims.sub), Some(&ip))
        .await?;
    Ok(Json(user))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateUserDto2 {
    email: Option<String>,
    name: Option<String>,
    role: Option<String>,
    #[serde(rename = "isActive", alias = "is_active")]
    is_active: Option<bool>,
}

pub async fn update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateUserDto2>,
) -> Result<Json<UserResponse>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);

    let attempts_privileged_change = payload.role.is_some() || payload.is_active.is_some();
    if !access_rules::can_update_user(
        claims.is_super_admin,
        &claims.sub,
        &id,
        attempts_privileged_change,
    ) {
        return Err(crate::error::AppError::Unauthorized);
    }

    use validator::Validate;
    let dto = UpdateUserDto {
        email: payload.email,
        name: payload.name,
        role: payload.role,
        is_active: payload.is_active,
        is_super_admin: None,
    };

    if let Err(e) = dto.validate() {
        return Err(crate::error::AppError::Validation(format!(
            "Validation error: {}",
            e
        )));
    }

    let user = state
        .user_service
        .update(&id, dto, Some(&claims.sub), Some(&ip))
        .await?;
    Ok(Json(user))
}

pub async fn delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    if !access_rules::can_access_global_user_management(claims.is_super_admin) {
        return Err(crate::error::AppError::Unauthorized);
    }
    let ip = extract_ip(&headers, addr);

    state
        .user_service
        .delete(&id, Some(&claims.sub), Some(&ip))
        .await?;
    Ok(Json(json!({"message": "User deleted"})))
}

// --- User Addresses (Self) ---

pub async fn list_my_addresses(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserAddress>>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    let addresses = state.user_service.list_addresses(&claims.sub).await?;
    Ok(Json(addresses))
}

pub async fn create_my_address(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<UserAddress>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);
    let dto_value = payload
        .get("dto")
        .cloned()
        .unwrap_or_else(|| payload.clone());
    let payload: CreateUserAddressDto =
        serde_json::from_value(dto_value).map_err(|e| crate::error::AppError::Validation(e.to_string()))?;

    let address = state
        .user_service
        .create_address(&claims.sub, payload, Some(&claims.sub), Some(&ip))
        .await?;
    Ok(Json(address))
}

pub async fn update_my_address(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(address_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<UserAddress>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);
    let dto_value = payload
        .get("dto")
        .cloned()
        .unwrap_or_else(|| payload.clone());
    let payload: UpdateUserAddressDto =
        serde_json::from_value(dto_value).map_err(|e| crate::error::AppError::Validation(e.to_string()))?;

    let address = state
        .user_service
        .update_address(
            &claims.sub,
            &address_id,
            payload,
            Some(&claims.sub),
            Some(&ip),
        )
        .await?;
    Ok(Json(address))
}

pub async fn delete_my_address(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(address_id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);

    state
        .user_service
        .delete_address(&claims.sub, &address_id, Some(&claims.sub), Some(&ip))
        .await?;
    Ok(Json(json!({ "message": "Address deleted" })))
}

// --- User Addresses (Admin) ---

pub async fn list_user_addresses_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<Json<Vec<UserAddress>>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    if !claims.is_super_admin {
        return Err(crate::error::AppError::Unauthorized);
    }

    let addresses = state.user_service.list_addresses(&user_id).await?;
    Ok(Json(addresses))
}
