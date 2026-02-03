use crate::error::AppResult;
use crate::services::backup::{BackupRecord, BackupService};
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBackupsArgs {
    token: String,
    tenant_only: Option<bool>,
}

#[tauri::command]
pub async fn list_backups(
    service: State<'_, BackupService>,
    auth_service: State<'_, crate::services::AuthService>,
    args: ListBackupsArgs,
) -> AppResult<Vec<BackupRecord>> {
    let claims = auth_service.validate_token(&args.token).await?;
    let backups = service.list_backups().await?;
    if claims.is_super_admin && args.tenant_only != Some(true) {
        return Ok(backups);
    }

    let tenant_id = claims.tenant_id.as_ref().ok_or(crate::error::AppError::Forbidden(
        "Tenant context missing".to_string(),
    ))?;
    let perms = auth_service.get_user_permissions(&claims.sub, tenant_id).await?;
    let can_read = perms
        .iter()
        .any(|p| p == "*" || p == "backups:read" || p == "backups:*");
    if !can_read {
        return Err(crate::error::AppError::Forbidden(
            "Missing permission backups:read".to_string(),
        ));
    }

    Ok(backups
        .into_iter()
        .filter(|b| b.backup_type == "tenant" && b.tenant_id.as_deref() == Some(tenant_id))
        .collect())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBackupArgs {
    token: String,
    backup_type: String, // "global" or "tenant"
    target_id: Option<String>,
}

#[tauri::command]
pub async fn create_backup(
    service: State<'_, BackupService>,
    auth_service: State<'_, crate::services::AuthService>,
    args: CreateBackupArgs,
) -> AppResult<String> {
    let claims = auth_service.validate_token(&args.token).await?;

    match args.backup_type.as_str() {
        "global" => {
            if !claims.is_super_admin {
                return Err(crate::error::AppError::Forbidden(
                    "Only Super Admin can create global backups".to_string(),
                ));
            }
            service.create_global_backup().await
        }
        "tenant" => {
            let tid = match args.target_id {
                Some(tid) => tid,
                None => {
                    // Prefer tenant context if present (works for tenant admins and also for superadmins
                    // operating within a tenant-scoped session/token).
                    if let Some(tid) = claims.tenant_id.clone() {
                        tid
                    } else {
                        return Err(crate::error::AppError::Validation(
                            "Target ID required for tenant backup".to_string(),
                        ));
                    }
                }
            };

            // Superadmin can backup any tenant; tenant admin/owner can only backup their own tenant.
            if !claims.is_super_admin {
                let claim_tid =
                    claims
                        .tenant_id
                        .as_ref()
                        .ok_or(crate::error::AppError::Forbidden(
                            "No tenant context found".to_string(),
                        ))?;
                if claim_tid != &tid {
                    return Err(crate::error::AppError::Forbidden(
                        "Cannot backup other tenants".to_string(),
                    ));
                }

                let perms = auth_service.get_user_permissions(&claims.sub, claim_tid).await?;
                let can_create = perms.iter().any(|p| p == "*" || p == "backups:create" || p == "backups:*");
                if !can_create {
                    return Err(crate::error::AppError::Forbidden(
                        "Missing permission backups:create".to_string(),
                    ));
                }
            }

            service.create_tenant_backup(&tid).await
        }
        _ => Err(crate::error::AppError::Validation(
            "Invalid backup type".to_string(),
        )),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteBackupArgs {
    token: String,
    filename: String,
}

#[tauri::command]
pub async fn delete_backup(
    service: State<'_, BackupService>,
    auth_service: State<'_, crate::services::AuthService>,
    args: DeleteBackupArgs,
) -> AppResult<()> {
    let claims = auth_service.validate_token(&args.token).await?;
    if !claims.is_super_admin {
        let tenant_id = claims.tenant_id.as_ref().ok_or(crate::error::AppError::Forbidden(
            "Tenant context missing".to_string(),
        ))?;
        let perms = auth_service.get_user_permissions(&claims.sub, tenant_id).await?;
        let can_delete = perms
            .iter()
            .any(|p| p == "*" || p == "backups:delete" || p == "backups:*");
        if !can_delete {
            return Err(crate::error::AppError::Forbidden(
                "Missing permission backups:delete".to_string(),
            ));
        }
        let expected_prefix = format!("tenant_{}_", tenant_id);
        if !args.filename.starts_with(&expected_prefix) {
            return Err(crate::error::AppError::Forbidden(
                "Cannot delete other tenant's backups".to_string(),
            ));
        }
    }
    service.delete_backup(args.filename).await
}

#[tauri::command]
pub async fn save_backup_to_disk(
    service: State<'_, BackupService>,
    filename: String,
) -> AppResult<()> {
    let _source_path = service.get_backup_path(&filename)?;
    Ok(())
}

#[tauri::command]
pub async fn restore_backup_from_file(
    service: State<'_, BackupService>,
    auth_service: State<'_, crate::services::AuthService>,
    token: String,
    path: String,
) -> AppResult<()> {
    // 1) Validate token
    let claims = auth_service.validate_token(&token).await?;

    // 2) Restore
    let zip_path = std::path::PathBuf::from(path);
    if !zip_path.exists() {
        return Err(crate::error::AppError::NotFound("File not found".to_string()));
    }

    if claims.is_super_admin {
        // If restoring a tenant backup, scope restore to that tenant
        let tenant_id = zip_path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|name| {
                if name.starts_with("tenant_") {
                    let parts: Vec<&str> = name.split('_').collect();
                    if parts.len() >= 3 { Some(parts[1].to_string()) } else { None }
                } else {
                    None
                }
            });

        service
            .restore_from_zip(zip_path, tenant_id.as_deref())
            .await
    } else {
        let tenant_id = claims.tenant_id.as_ref().ok_or(crate::error::AppError::Forbidden(
            "Tenant context missing".to_string(),
        ))?;
        service.restore_from_zip(zip_path, Some(tenant_id)).await
    }
}

#[tauri::command]
pub async fn restore_local_backup_command(
    service: State<'_, BackupService>,
    auth_service: State<'_, crate::services::AuthService>,
    token: String,
    filename: String,
) -> AppResult<()> {
    let claims = auth_service.validate_token(&token).await?;

    if claims.is_super_admin {
        // If a superadmin restores a tenant backup, scope it to that tenant
        let tenant_id = if filename.starts_with("tenant_") {
            let parts: Vec<&str> = filename.split('_').collect();
            if parts.len() >= 3 {
                Some(parts[1].to_string())
            } else {
                None
            }
        } else {
            None
        };

        service
            .restore_local_backup(filename, tenant_id.as_deref())
            .await
    } else {
        let tenant_id = claims.tenant_id.as_ref().ok_or(crate::error::AppError::Forbidden(
            "Tenant context missing".to_string(),
        ))?;
        service.restore_local_backup(filename, Some(tenant_id)).await
    }
}
