//! Team management commands

use crate::models::TeamMemberWithUser;
use crate::services::{AuthService, TeamService};
use tauri::State;

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

    auth.check_permission(&claims.sub, &tenant_id, "roles", "update")
        .await
        .map_err(|e| e.to_string())?;

    let requester_level = team_service
        .get_user_role_level(&claims.sub, &tenant_id)
        .await?;
    let target_level = team_service.get_member_role_level(&member_id).await?;

    // Check 1: Cannot edit someone with higher or equal role
    if requester_level <= target_level {
        return Err(
            "Insufficient permissions: Cannot edit member with equal or higher role".to_string(),
        );
    }

    let new_role_level = team_service.get_role_level_by_id(&role_id).await?;

    // Check 2: Cannot promote to higher than own role
    if requester_level < new_role_level {
        return Err(
            "Insufficient permissions: Cannot assign role higher than your own".to_string(),
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

    // Check: Cannot remove someone with higher or equal role
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
