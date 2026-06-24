//! Team management commands

use crate::models::TeamMemberWithUser;
use crate::services::{AuthService, TeamService};
use tauri::State;

pub(crate) fn enforce_member_role_change_permissions(
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

/// List all members of the current team
#[tauri::command]
pub async fn list_team_members(
    token: String,
    auth: State<'_, AuthService>,
    team_service: State<'_, TeamService>,
) -> Result<Vec<TeamMemberWithUser>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "team", "read")
        .await
        .map_err(|e| e.to_string())?;

    team_service
        .list_members(&tenant_id)
        .await
        .map_err(|e| e.to_string())
}

/// Add a new team member
#[tauri::command]
pub async fn add_team_member(
    token: String,
    email: String,
    name: String,
    role_id: String,
    password: Option<String>,
    auth: State<'_, AuthService>,
    team_service: State<'_, TeamService>,
) -> Result<TeamMemberWithUser, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "team", "create")
        .await
        .map_err(|e| e.to_string())?;

    // Check Role Level to prevent privilege escalation
    let requester_level = team_service
        .get_user_role_level(&claims.sub, &tenant_id)
        .await?;
    let new_role_level = team_service.get_role_level_by_id(&role_id).await?;

    if requester_level < new_role_level {
        return Err(
            "Insufficient permissions: Cannot assign role higher than your own".to_string(),
        );
    }

    // Block assigning Customer role via team management
    let role_name = team_service
        .get_role_name_by_id(&role_id)
        .await
        .map_err(|e| e.to_string())?;

    if role_name.as_deref() == Some("Customer") {
        return Err(
            "Cannot assign Customer role via team management. Create customer accounts from the Customers module instead.".to_string(),
        );
    }

    team_service
        .add_member(
            &tenant_id,
            &email,
            &name,
            &role_id,
            password,
            Some(&claims.sub),
            Some("127.0.0.1"),
        )
        .await
}

/// Update a team member's role
#[tauri::command]
pub async fn update_team_member_role(
    token: String,
    member_id: String,
    role_id: String,
    auth: State<'_, AuthService>,
    team_service: State<'_, TeamService>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "team", "update")
        .await
        .map_err(|e| e.to_string())?;

    let requester_level = team_service
        .get_user_role_level(&claims.sub, &tenant_id)
        .await?;
    let target_level = team_service.get_member_role_level(&member_id).await?;
    let new_role_level = team_service.get_role_level_by_id(&role_id).await?;
    enforce_member_role_change_permissions(requester_level, target_level, new_role_level)?;

    // Block assigning Customer role via team management
    let role_name = team_service
        .get_role_name_by_id(&role_id)
        .await
        .map_err(|e| e.to_string())?;

    if role_name.as_deref() == Some("Customer") {
        return Err(
            "Cannot assign Customer role via team management. Create customer accounts from the Customers module instead.".to_string(),
        );
    }

    team_service
        .update_member(&tenant_id, &member_id, &role_id, Some(&claims.sub), None)
        .await
        .map_err(|e| e.to_string())
}

/// Remove a team member
#[tauri::command]
pub async fn remove_team_member(
    token: String,
    member_id: String,
    auth: State<'_, AuthService>,
    team_service: State<'_, TeamService>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "team", "delete")
        .await
        .map_err(|e| e.to_string())?;

    let requester_level = team_service
        .get_user_role_level(&claims.sub, &tenant_id)
        .await?;
    let target_level = team_service.get_member_role_level(&member_id).await?;

    if requester_level <= target_level {
        return Err(
            "Insufficient permissions: Cannot remove member with equal or higher role".to_string(),
        );
    }

    team_service
        .remove_member(&tenant_id, &member_id, Some(&claims.sub), None)
        .await
        .map_err(|e| e.to_string())
}

/// List soft-deleted members of the current team
#[tauri::command]
pub async fn list_deleted_team_members(
    token: String,
    auth: State<'_, AuthService>,
    team_service: State<'_, TeamService>,
) -> Result<Vec<TeamMemberWithUser>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "team", "read")
        .await
        .map_err(|e| e.to_string())?;

    team_service
        .list_deleted_members(&tenant_id)
        .await
        .map_err(|e| e.to_string())
}

/// Restore a soft-deleted team member
#[tauri::command]
pub async fn restore_team_member(
    token: String,
    member_id: String,
    auth: State<'_, AuthService>,
    team_service: State<'_, TeamService>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "team", "update")
        .await
        .map_err(|e| e.to_string())?;

    team_service
        .restore_member(&tenant_id, &member_id, Some(&claims.sub), None)
        .await
        .map_err(|e| e.to_string())
}

/// Permanently delete a soft-deleted team member
#[tauri::command]
pub async fn hard_delete_team_member(
    token: String,
    member_id: String,
    auth: State<'_, AuthService>,
    team_service: State<'_, TeamService>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    auth.check_permission(&claims.sub, &tenant_id, "team", "delete")
        .await
        .map_err(|e| e.to_string())?;

    team_service
        .hard_delete_member(&tenant_id, &member_id, Some(&claims.sub), None)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::enforce_member_role_change_permissions;

    #[test]
    fn role_change_allows_only_lower_targets_and_assignable_roles() {
        assert!(
            enforce_member_role_change_permissions(100, 50, 80).is_ok(),
            "higher-level operator should be able to update lower-level member into lower-level role"
        );
    }

    #[test]
    fn role_change_blocks_equal_or_higher_target_member() {
        let err = enforce_member_role_change_permissions(50, 50, 10)
            .expect_err("equal-level member must not be editable");

        assert!(err.contains("equal or higher role"));
    }

    #[test]
    fn role_change_blocks_assigning_role_above_requester_level() {
        let err = enforce_member_role_change_permissions(50, 10, 80)
            .expect_err("requester must not assign a role above their own level");

        assert!(err.contains("higher than your own"));
    }
}
