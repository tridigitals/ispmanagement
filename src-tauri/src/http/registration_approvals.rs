use super::AppState;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;

/// Reuse the superadmin auth pattern
async fn require_super_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::services::auth_service::Claims, crate::error::AppError> {
    let token = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(crate::error::AppError::Unauthorized)?;

    let claims = state.auth_service.validate_token(token).await?;

    if !claims.is_super_admin {
        return Err(crate::error::AppError::Unauthorized);
    }

    Ok(claims)
}

/// GET /api/superadmin/registration-approvals
pub async fn list_pending(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let _claims = require_super_admin(&state, &headers).await?;

    let pending_users = state.auth_service.list_pending_users().await?;

    let items: Vec<serde_json::Value> = pending_users
        .into_iter()
        .map(|u| {
            serde_json::json!({
                "id": u.id,
                "email": u.email,
                "name": u.name,
                "pending_review_message": u.pending_review_message,
                "created_at": u.created_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "users": items,
        "total": items.len(),
    })))
}

#[derive(Deserialize)]
pub struct ApproveDto {
    pub tenant_id: String,
    pub role_id: String,
}

/// POST /api/superadmin/registration-approvals/{user_id}/approve
pub async fn approve(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(payload): Json<ApproveDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = require_super_admin(&state, &headers).await?;

    state
        .auth_service
        .approve_pending_user(&claims.sub, &user_id, &payload.tenant_id, &payload.role_id)
        .await?;

    Ok(Json(serde_json::json!({
        "message": "User approved successfully"
    })))
}

#[derive(Deserialize)]
pub struct RejectDto {
    pub reason: String,
}

/// POST /api/superadmin/registration-approvals/{user_id}/reject
pub async fn reject(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(payload): Json<RejectDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let claims = require_super_admin(&state, &headers).await?;

    state
        .auth_service
        .reject_pending_user(&claims.sub, &user_id, &payload.reason)
        .await?;

    Ok(Json(serde_json::json!({
        "message": "User rejected"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approve_dto_deserializes_with_tenant_and_role() {
        let json = r#"{"tenant_id": "t1", "role_id": "r1"}"#;
        let dto: ApproveDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.tenant_id, "t1");
        assert_eq!(dto.role_id, "r1");
    }

    #[test]
    fn reject_dto_deserializes_with_reason() {
        let json = r#"{"reason": "spam account"}"#;
        let dto: RejectDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.reason, "spam account");
    }
}
