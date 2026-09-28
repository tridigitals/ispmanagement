//! Team management HTTP handlers

use super::{websocket::WsEvent, AppState};
use crate::error::AppError;
use crate::http::auth::extract_ip;
use crate::models::TeamMemberWithUser;
use axum::{
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use std::net::SocketAddr;

fn enforce_member_role_change_permissions(
    requester_level: i32,
    target_level: i32,
    new_role_level: i32,
) -> Result<(), String> {
    if requester_level <= target_level {
        return Err(
            "Insufficient permissions: Cannot edit member with equal or higher role".to_string(),
        );
    }

    if requester_level < new_role_level {
        return Err(
            "Insufficient permissions: Cannot assign role higher than your own".to_string(),
        );
    }

    Ok(())
}

fn map_team_service_error(msg: String) -> crate::error::AppError {
    if msg.to_lowercase().contains("not found") {
        crate::error::AppError::NotFound(msg)
    } else {
        crate::error::AppError::Internal(msg)
    }
}

// Helper to extract token from headers
fn extract_token(headers: &HeaderMap) -> Result<String, crate::error::AppError> {
    crate::http::extract_token(headers)
}

/// List all team members
pub async fn list_team_members(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamMemberWithUser>>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| crate::error::AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "read")
        .await?;

    let members = state.team_service.list_members(&tenant_id).await?;
    Ok(Json(members))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddMemberDto {
    email: String,
    name: String,
    #[serde(rename = "roleId", alias = "role_id")]
    role_id: String,
    password: Option<String>,
    /// Status akun sejak dibuat. Kosong = aktif (perilaku lama, supaya klien
    /// lama tidak berubah perilaku).
    #[serde(rename = "isActive", alias = "is_active", default)]
    is_active: Option<bool>,
    /// Tandai email sudah diverifikasi. Kosong = sudah (admin menambahkan akun
    /// ini sendiri, dan tidak ada email verifikasi yang dikirim).
    #[serde(rename = "emailVerified", alias = "email_verified", default)]
    email_verified: Option<bool>,
}

/// Add a new team member
pub async fn add_team_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<AddMemberDto>,
) -> Result<Json<TeamMemberWithUser>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| crate::error::AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "create")
        .await?;

    // Check Role Level
    let requester_level = state
        .team_service
        .get_user_role_level(&claims.sub, &tenant_id)
        .await
        .map_err(crate::error::AppError::Internal)?;
    let new_role_level = state
        .team_service
        .get_role_level_by_id(&payload.role_id)
        .await
        .map_err(crate::error::AppError::Internal)?;

    if requester_level < new_role_level {
        return Err(crate::error::AppError::Forbidden(
            "Insufficient permissions: Cannot assign role higher than your own".to_string(),
        ));
    }

    // Block assigning Customer role via team management.
    // Customer accounts must be created from the Customers module.
    let role_name = state
        .team_service
        .get_role_name_by_id(&payload.role_id)
        .await
        .map_err(crate::error::AppError::Internal)?;

    if role_name.as_deref() == Some("Customer") {
        return Err(crate::error::AppError::Validation(
            "Cannot assign Customer role via team management. Create customer accounts from the Customers module instead.".to_string(),
        ));
    }

    let member = state
        .team_service
        .add_member(
            &tenant_id,
            &payload.email,
            &payload.name,
            &payload.role_id,
            payload.password,
            payload.is_active,
            payload.email_verified,
            Some(&claims.sub),
            Some(&ip),
        )
        .await
        .map_err(crate::error::AppError::Internal)?;

    // Broadcast member added event
    state.ws_hub.broadcast(WsEvent::MemberUpdated {
        user_id: member.user_id.clone(),
    });

    Ok(Json(member))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateMemberDto {
    /// Opsional: memperbarui status akun saja tidak boleh menuntut ganti role.
    #[serde(rename = "roleId", alias = "role_id", default)]
    role_id: Option<String>,
    /// Aktifkan / nonaktifkan akun (kolom `users.is_active`).
    #[serde(rename = "isActive", alias = "is_active", default)]
    is_active: Option<bool>,
    /// `true` = tandai email terverifikasi, `false` = cabut verifikasinya.
    #[serde(rename = "emailVerified", alias = "email_verified", default)]
    email_verified: Option<bool>,
}

/// Update a team member's role
pub async fn update_team_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateMemberDto>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| crate::error::AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "update")
        .await?;

    let requester_level = state
        .team_service
        .get_user_role_level(&claims.sub, &tenant_id)
        .await
        .map_err(crate::error::AppError::Internal)?;
    let target_level = state
        .team_service
        .get_member_role_level(&id)
        .await
        .map_err(crate::error::AppError::Internal)?;
    // Guard level & larangan role Customer hanya relevan bila role memang
    // diganti. Mengubah status akun saja tidak boleh ikut menuntut role.
    if let Some(rid) = payload.role_id.as_deref() {
        let new_role_level = state
            .team_service
            .get_role_level_by_id(rid)
            .await
            .map_err(crate::error::AppError::Internal)?;

        enforce_member_role_change_permissions(requester_level, target_level, new_role_level)
            .map_err(crate::error::AppError::Forbidden)?;

        // Block assigning Customer role via team management.
        let role_name = state
            .team_service
            .get_role_name_by_id(rid)
            .await
            .map_err(crate::error::AppError::Internal)?;

        if role_name.as_deref() == Some("Customer") {
            return Err(crate::error::AppError::Validation(
                "Cannot assign Customer role via team management. Create customer accounts from the Customers module instead.".to_string(),
            ));
        }
    }

    state
        .team_service
        .update_member(
            &tenant_id,
            &id,
            payload.role_id.as_deref(),
            payload.is_active,
            payload.email_verified,
            Some(&claims.sub),
            Some(&ip),
        )
        .await
        .map_err(map_team_service_error)?;

    // Broadcast member updated event - permissions may have changed
    state.ws_hub.broadcast(WsEvent::PermissionsChanged);

    Ok(Json(serde_json::json!({"success": true})))
}

/// Remove a team member
pub async fn remove_team_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let ip = extract_ip(&headers, addr);

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| crate::error::AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "delete")
        .await?;

    let requester_level = state
        .team_service
        .get_user_role_level(&claims.sub, &tenant_id)
        .await
        .map_err(crate::error::AppError::Internal)?;
    let target_level = state
        .team_service
        .get_member_role_level(&id)
        .await
        .map_err(crate::error::AppError::Internal)?;

    if requester_level <= target_level {
        return Err(crate::error::AppError::Forbidden(
            "Insufficient permissions: Cannot remove member with equal or higher role".to_string(),
        ));
    }

    state
        .team_service
        .remove_member(&tenant_id, &id, Some(&claims.sub), Some(&ip))
        .await
        .map_err(map_team_service_error)?;

    // Broadcast member removed event
    state.ws_hub.broadcast(WsEvent::PermissionsChanged);

    Ok(Json(serde_json::json!({"success": true})))
}

/// List soft-deleted team members
pub async fn list_deleted_members(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TeamMemberWithUser>>, AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "read")
        .await?;

    let members = state
        .team_service
        .list_deleted_members(&tenant_id)
        .await
        .map_err(|e| map_team_service_error(e.to_string()))?;

    Ok(Json(members))
}

/// Restore a soft-deleted team member
pub async fn restore_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<serde_json::Value>, AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "update")
        .await?;

    let ip = extract_ip(&headers, addr);
    state
        .team_service
        .restore_member(&tenant_id, &id, Some(&claims.sub), Some(&ip))
        .await
        .map_err(map_team_service_error)?;

    state.ws_hub.broadcast(WsEvent::PermissionsChanged);

    Ok(Json(serde_json::json!({"success": true})))
}

/// Permanently delete a soft-deleted team member
pub async fn hard_delete_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Json<serde_json::Value>, AppError> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims
        .tenant_id
        .ok_or(AppError::Validation("No tenant ID in token".to_string()))?;

    state
        .auth_service
        .check_permission(&claims.sub, &tenant_id, "team", "delete")
        .await?;

    let ip = extract_ip(&headers, addr);
    state
        .team_service
        .hard_delete_member(&tenant_id, &id, Some(&claims.sub), Some(&ip))
        .await
        .map_err(map_team_service_error)?;

    state.ws_hub.broadcast(WsEvent::PermissionsChanged);

    Ok(Json(serde_json::json!({"success": true})))
}

#[cfg(test)]
mod tests {
    use super::enforce_member_role_change_permissions;

    /// Guard ini hanya menolak saat requester **setingkat atau di bawah** target
    /// (`requester_level <= target_level`). Artinya bila level target salah
    /// terbaca 0, siapa pun bisa mengubah/menurunkan Owner.
    ///
    /// Itu memang pernah terjadi: `get_member_role_level` memakai INNER JOIN
    /// `roles`, sedangkan Owner tenant lama punya `tenant_members.role_id =
    /// NULL` — join tidak menghasilkan baris, fungsi mengembalikan 0.
    /// Perbaikannya ada di `team_service` (fallback lewat kolom teks
    /// `tm.role`). Tes ini mengunci kontrak guard-nya supaya regresi level
    /// tidak kembali lolos diam-diam.
    #[test]
    fn owner_berlevel_rendah_tidak_boleh_mengubah_owner() {
        // Kabar buruk: bila level terbaca 0, guard MELOLOSKAN (celah lama).
        assert!(enforce_member_role_change_permissions(0, 100, 10).is_err(),
            "requester 0 tidak boleh mengubah target level 100");
    }

    #[test]
    fn pemanggil_setingkat_atau_di_bawah_target_ditolak() {
        assert!(enforce_member_role_change_permissions(10, 10, 5).is_err());
        assert!(enforce_member_role_change_permissions(10, 50, 5).is_err());
    }

    #[test]
    fn pemanggil_di_atas_target_dan_tidak_menaikkan_level_diizinkan() {
        assert!(enforce_member_role_change_permissions(100, 10, 20).is_ok());
        // Tidak boleh memberi role di atas level sendiri.
        assert!(enforce_member_role_change_permissions(50, 10, 80).is_err());
    }
}
