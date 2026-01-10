//! Team management commands

use tauri::State;
use crate::services::{AuthService, TeamService};
use crate::models::TeamMemberWithUser;

/// List all members of the current team
#[tauri::command]
pub async fn list_team_members(
    token: String,
    auth: State<'_, AuthService>,
    team_service: State<'_, TeamService>,
) -> Result<Vec<TeamMemberWithUser>, String> {
    let claims = auth.validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    
    let tenant_id = claims.tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;
        
    team_service.list_members(&tenant_id)
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
    let claims = auth.validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    
    let tenant_id = claims.tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;
        
    // TODO: proper permission check
    
    team_service.add_member(&tenant_id, &email, &name, &role_id, password)
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
    auth.validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
        
    // TODO: proper permission check
    
    team_service.update_member(&member_id, &role_id)
        .await
}

/// Remove a team member
#[tauri::command]
pub async fn remove_team_member(
    token: String,
    member_id: String,
    auth: State<'_, AuthService>,
    team_service: State<'_, TeamService>,
) -> Result<(), String> {
    auth.validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
        
    // TODO: proper permission check
    
    team_service.remove_member(&member_id)
        .await
}
