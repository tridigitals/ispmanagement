use axum::{
    extract::{Path, Query, State, ConnectInfo},
    http::HeaderMap,
    Json,
};
use std::net::SocketAddr;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::models::{CreateUserDto, UpdateUserDto, UserResponse, PaginatedResponse};
use super::AppState;
use crate::http::auth::extract_ip;

#[derive(Deserialize)]
pub struct ListUsersQuery {
    page: Option<u32>,
    #[serde(rename = "perPage")]
    per_page: Option<u32>,
}

fn extract_token(headers: &HeaderMap) -> Result<String, crate::error::AppError> {
    headers.get("Authorization")
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
    state.auth_service.check_admin(&token).await?;

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
    state.auth_service.check_admin(&token).await?;
    
    let user = state.user_service.get_by_id(&id).await?;
    Ok(Json(user))
}

#[derive(Deserialize)]
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
    let claims = state.auth_service.check_admin(&token).await?;
    let ip = extract_ip(&headers, addr);

    use validator::Validate;
    let dto = CreateUserDto {
        email: payload.email,
        password: payload.password,
        name: payload.name,
    };

    if let Err(e) = dto.validate() {
        return Err(crate::error::AppError::Validation(format!("Validation error: {}", e)));
    }

    let user = state.user_service.create(dto, Some(&claims.sub), Some(&ip)).await?;
    Ok(Json(user))
}

#[derive(Deserialize)]
pub struct UpdateUserDto2 {
    email: Option<String>,
    name: Option<String>,
    role: Option<String>,
    #[serde(rename = "isActive")]
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

    // Check if admin OR if updating self
    if claims.role != "admin" && claims.sub != id {
        return Err(crate::error::AppError::Unauthorized);
    }

    // Prevent non-admins from changing their own role or active status
    if claims.role != "admin" && (payload.role.is_some() || payload.is_active.is_some()) {
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
        return Err(crate::error::AppError::Validation(format!("Validation error: {}", e)));
    }

    let user = state.user_service.update(&id, dto, Some(&claims.sub), Some(&ip)).await?;
    Ok(Json(user))
}

pub async fn delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.check_admin(&token).await?;
    let ip = extract_ip(&headers, addr);
    
    state.user_service.delete(&id, Some(&claims.sub), Some(&ip)).await?;
    Ok(Json(json!({"message": "User deleted"})))
}
