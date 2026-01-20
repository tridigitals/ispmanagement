use crate::services::{StorageService, AuthService};
use tauri::State;

#[tauri::command]
pub async fn upload_file(
    token: String,
    state: State<'_, StorageService>,
    auth_service: State<'_, AuthService>,
    file_name: String,
    content_type: String,
    data: Vec<u8>,
) -> Result<crate::models::FileRecord, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;

    // Determine tenant_id
    // If Super Admin AND no tenant_id in token (global admin), they might want to upload as "system" or specific tenant.
    // For simplicity in this command:
    // 1. If user has tenant_id, force use that.
    // 2. If user is global (no tenant_id) but super_admin, allows upload (system file).
    
    // NOTE: This simplifies "Tenant Upload". If Super Admin wants to upload *to a specific tenant*, 
    // we'd need a separate arg `target_tenant_id` or a separate admin command.
    // For FileManager "Tenant Mode", the user is logged in as that tenant context, so this works perfectly.
    
    // For "Admin Mode" FileManager (Global), if they use this, it uploads as NULL tenant (System File).
    
    let _tenant_id = claims.tenant_id.as_deref().unwrap_or("system"); // Or handle NULL in service if DB allows
    
    // Actually, our DB schema enforces tenant_id NOT NULL for file_records?
    // Let's check schema.
    // Schema: tenant_id TEXT NOT NULL. So we cannot pass NULL easily unless we have a "system" tenant or change schema.
    // Wait, in `seed_defaults`, we insert settings with NULL tenant_id.
    // But `file_records` schema said: `tenant_id TEXT NOT NULL`.
    // So global files are tricky unless we have a "system" tenant or loosen schema.
    // Let's assume for now this is for TENANT upload mostly.
    
    if claims.tenant_id.is_none() && !claims.is_super_admin {
         return Err("Unauthorized: No tenant context".to_string());
    }
    
    // If global admin, we might need a workaround or just fail if they try to upload without target.
    // For now, let's assume we use the user's tenant.
    // If global admin, we fail for now OR we need to pass tenant_id.
    
    // REVISION: Let's allow passing `target_tenant_id` OPTIONALLY.
    // But wait, `upload_file` signature changes break frontend if not careful.
    // Let's stick to: Token defines context.
    
    // Logic:
    // 1. If claims.tenant_id exists -> use it.
    // 2. If claims.tenant_id is None (Global Admin) -> Allow? But what tenant_id to use?
    //    We can't use NULL because of DB constraint.
    //    We'll return error for Global Admin for now unless we relax DB.
    
    let tid = claims.tenant_id.ok_or_else(|| "Global upload not supported yet (requires tenant context)".to_string())?;

    state.upload(&tid, &file_name, &content_type, &data, Some(&claims.sub)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_files_admin(
    token: String,
    state: State<'_, StorageService>,
    auth_service: State<'_, AuthService>,
    page: u32,
    per_page: u32,
    search: Option<String>,
) -> Result<crate::models::PaginatedResponse<crate::models::FileRecord>, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    let (data, total) = state.list_all_files(page, per_page, search).await.map_err(|e| e.to_string())?;
    
    Ok(crate::models::PaginatedResponse {
        data,
        total,
        page,
        per_page,
    })
}

#[tauri::command]
pub async fn delete_file_admin(
    token: String,
    state: State<'_, StorageService>,
    auth_service: State<'_, AuthService>,
    file_id: String,
) -> Result<bool, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;

    if !claims.is_super_admin {
        return Err("Unauthorized".to_string());
    }

    state.delete_file(&file_id).await.map(|_| true).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_files_tenant(
    token: String,
    state: State<'_, StorageService>,
    auth_service: State<'_, AuthService>,
    page: u32,
    per_page: u32,
    search: Option<String>,
) -> Result<crate::models::PaginatedResponse<crate::models::FileRecord>, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;

    // Must have a tenant_id in token (logged in as tenant user/admin)
    let tenant_id = claims.tenant_id.ok_or("Unauthorized: No tenant context".to_string())?;

    let (data, total) = state.list_tenant_files(&tenant_id, page, per_page, search).await.map_err(|e| e.to_string())?;
    
    Ok(crate::models::PaginatedResponse {
        data,
        total,
        page,
        per_page,
    })
}

#[tauri::command]
pub async fn delete_file_tenant(
    token: String,
    state: State<'_, StorageService>,
    auth_service: State<'_, AuthService>,
    file_id: String,
) -> Result<bool, String> {
    let claims = auth_service.validate_token(&token).await.map_err(|e| e.to_string())?;

    // Must have a tenant_id in token
    let tenant_id = claims.tenant_id.ok_or("Unauthorized: No tenant context".to_string())?;

    // Ideally check if user has 'storage:delete' permission here too.
    // For now assuming any authenticated tenant user can delete (or rely on UI hiding).
    // Better: check role permissions via RoleService, but we keep it simple for MVP.
    
    state.delete_tenant_file(&file_id, &tenant_id).await.map(|_| true).map_err(|e| e.to_string())
}
