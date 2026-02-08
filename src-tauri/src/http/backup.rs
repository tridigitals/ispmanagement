use crate::error::AppResult;
use crate::http::AppState;
use crate::services::backup::BackupRecord;
use axum::{
    extract::Query,
    extract::{Path, State},
    http::HeaderMap,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_backups))
        .route("/", post(create_backup))
        .route("/restore", post(restore_backup))
        .route("/{filename}/restore", post(restore_local_backup))
        .route("/{filename}", delete(delete_backup))
        .route("/{filename}/download", get(download_backup))
}

fn extract_token(headers: &HeaderMap) -> Result<String, crate::error::AppError> {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or(crate::error::AppError::Unauthorized)
}

fn has_permission(perms: &[String], resource: &str, action: &str) -> bool {
    let perm = format!("{}:{}", resource, action);
    let wildcard = format!("{}:*", resource);
    perms
        .iter()
        .any(|p| p == "*" || p == &perm || p == &wildcard)
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct ListBackupsQuery {
    tenant_only: Option<bool>,
}

async fn list_backups(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListBackupsQuery>,
) -> AppResult<Json<Vec<BackupRecord>>> {
    let token = extract_token(&headers)?;

    // Check permission
    let claims = state.auth_service.validate_token(&token).await?;

    let backups = state.backup_service.list_backups().await?;
    if claims.is_super_admin && query.tenant_only != Some(true) {
        return Ok(Json(backups));
    }

    let tenant_id = claims
        .tenant_id
        .as_ref()
        .ok_or(crate::error::AppError::Forbidden(
            "Tenant context missing".to_string(),
        ))?;
    let perms = state
        .auth_service
        .get_user_permissions(&claims.sub, tenant_id)
        .await?;
    if !has_permission(&perms, "backups", "read") {
        return Err(crate::error::AppError::Forbidden(
            "Missing permission backups:read".to_string(),
        ));
    }

    let filtered: Vec<BackupRecord> = backups
        .into_iter()
        .filter(|b| b.backup_type == "tenant" && b.tenant_id.as_deref() == Some(tenant_id))
        .collect();

    Ok(Json(filtered))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBackupRequest {
    backup_type: String, // "global" or "tenant"
    target_id: Option<String>,
}

async fn create_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateBackupRequest>,
) -> AppResult<Json<String>> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    match payload.backup_type.as_str() {
        "global" => {
            if !claims.is_super_admin {
                return Err(crate::error::AppError::Forbidden(
                    "Only Super Admin can create global backups".to_string(),
                ));
            }
            let path = state.backup_service.create_global_backup().await?;
            Ok(Json(path))
        }
        "tenant" => {
            let target_id = match payload.target_id {
                Some(tid) => tid,
                None => {
                    if claims.is_super_admin {
                        return Err(crate::error::AppError::Validation(
                            "Target ID required".to_string(),
                        ));
                    }
                    claims
                        .tenant_id
                        .clone()
                        .ok_or(crate::error::AppError::Forbidden(
                            "No tenant context found".to_string(),
                        ))?
                }
            };

            // Permission Check: Super Admin OR (Tenant Admin AND target_id == tenant_id)
            if !claims.is_super_admin {
                // RBAC check (tenant)
                let tenant_id =
                    claims
                        .tenant_id
                        .as_ref()
                        .ok_or(crate::error::AppError::Forbidden(
                            "No tenant context found".to_string(),
                        ))?;
                let perms = state
                    .auth_service
                    .get_user_permissions(&claims.sub, tenant_id)
                    .await?;
                if !has_permission(&perms, "backups", "create") {
                    return Err(crate::error::AppError::Forbidden(
                        "Missing permission backups:create".to_string(),
                    ));
                }

                // Check if claims.tenant_id matches target_id
                if let Some(tid) = &claims.tenant_id {
                    if tid != &target_id {
                        return Err(crate::error::AppError::Forbidden(
                            "Cannot backup other tenants".to_string(),
                        ));
                    }
                } else {
                    return Err(crate::error::AppError::Forbidden(
                        "No tenant context found".to_string(),
                    ));
                }
            }

            let path = state
                .backup_service
                .create_tenant_backup(&target_id)
                .await?;
            Ok(Json(path))
        }
        _ => Err(crate::error::AppError::Validation(
            "Invalid backup type".to_string(),
        )),
    }
}

async fn delete_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> AppResult<Json<()>> {
    let token = extract_token(&headers)?;

    let claims = state.auth_service.validate_token(&token).await?;
    if !claims.is_super_admin {
        let tenant_id = claims
            .tenant_id
            .as_ref()
            .ok_or(crate::error::AppError::Forbidden(
                "Tenant context missing".to_string(),
            ))?;
        let perms = state
            .auth_service
            .get_user_permissions(&claims.sub, tenant_id)
            .await?;
        if !has_permission(&perms, "backups", "delete") {
            return Err(crate::error::AppError::Forbidden(
                "Missing permission backups:delete".to_string(),
            ));
        }
        let expected_prefix = format!("tenant_{}_", tenant_id);
        if !filename.starts_with(&expected_prefix) {
            return Err(crate::error::AppError::Forbidden(
                "Cannot delete other tenant's backups".to_string(),
            ));
        }
    }

    state.backup_service.delete_backup(filename).await?;
    Ok(Json(()))
}

async fn download_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> AppResult<impl axum::response::IntoResponse> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    let file_path = state.backup_service.get_backup_path(&filename)?;

    // Permission Check:
    // Super Admin can download anything
    // Tenant Admin can only download their own tenant ZIP
    if !claims.is_super_admin {
        // RBAC check (tenant)
        let tenant_id = claims
            .tenant_id
            .as_ref()
            .ok_or(crate::error::AppError::Forbidden(
                "Tenant context missing".to_string(),
            ))?;
        let perms = state
            .auth_service
            .get_user_permissions(&claims.sub, tenant_id)
            .await?;
        if !has_permission(&perms, "backups", "download") {
            return Err(crate::error::AppError::Forbidden(
                "Missing permission backups:download".to_string(),
            ));
        }

        if !filename.starts_with("tenant_") {
            return Err(crate::error::AppError::Forbidden(
                "Cannot access global backups".to_string(),
            ));
        }

        let expected_prefix = format!("tenant_{}_", tenant_id);

        if !filename.starts_with(&expected_prefix) {
            return Err(crate::error::AppError::Forbidden(
                "Cannot download other tenant's backups".to_string(),
            ));
        }
    }

    // Read file and stream
    let file = tokio::fs::File::open(&file_path).await.map_err(|e| {
        crate::error::AppError::Internal(format!("Failed to open backup file: {}", e))
    })?;

    let stream = tokio_util::io::ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    let content_type = if filename.ends_with(".zip") {
        "application/zip"
    } else if filename.ends_with(".sql") {
        "application/sql"
    } else {
        "application/octet-stream"
    };

    let disposition = format!("attachment; filename=\"{}\"", filename);

    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static(content_type),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                axum::http::HeaderValue::from_str(&disposition).map_err(|_| {
                    crate::error::AppError::Internal("Invalid header value".to_string())
                })?,
            ),
        ],
        body,
    ))
}

async fn restore_backup(
    headers: HeaderMap,
    State(state): State<AppState>,
    multipart: axum::extract::Multipart,
) -> AppResult<Json<()>> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    // Use a local scope to ensure multipart is processed
    let mut multipart = multipart;

    // Create a temporary file for the uploaded zip
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("restore_{}.zip", uuid::Uuid::new_v4()));

    let mut file_saved = false;

    // Save uploaded file
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
    {
        let name = field.name().unwrap_or_default();
        if name == "file" {
            let data = field
                .bytes()
                .await
                .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
            tokio::fs::write(&temp_path, data)
                .await
                .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
            file_saved = true;
        }
    }

    if !file_saved {
        return Err(crate::error::AppError::Validation(
            "No file uploaded".to_string(),
        ));
    }

    // Determine if global or tenant based on claims
    let res = if claims.is_super_admin {
        state
            .backup_service
            .restore_from_zip(temp_path.clone(), None)
            .await
    } else {
        let tenant_id = claims
            .tenant_id
            .as_ref()
            .ok_or(crate::error::AppError::Forbidden(
                "Tenant context missing".to_string(),
            ))?;
        let perms = state
            .auth_service
            .get_user_permissions(&claims.sub, tenant_id)
            .await?;
        if !has_permission(&perms, "backups", "restore") {
            return Err(crate::error::AppError::Forbidden(
                "Missing permission backups:restore".to_string(),
            ));
        }
        state
            .backup_service
            .restore_from_zip(temp_path.clone(), Some(tenant_id))
            .await
    };

    // Cleanup
    let _ = tokio::fs::remove_file(temp_path).await;

    res.map(|_| Json(()))
}

async fn restore_local_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> AppResult<Json<()>> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    let res = if claims.is_super_admin {
        // If a superadmin restores a tenant backup, scope it to that tenant
        let tenant_id = if filename.starts_with("tenant_") {
            let parts: Vec<&str> = filename.split('_').collect();
            if parts.len() >= 3 {
                Some(parts[1])
            } else {
                None
            }
        } else {
            None
        };

        state
            .backup_service
            .restore_local_backup(filename.clone(), tenant_id)
            .await
    } else {
        // Tenant admin check
        if !filename.starts_with("tenant_") {
            return Err(crate::error::AppError::Forbidden(
                "Unauthorized backup access".to_string(),
            ));
        }
        let tenant_id = claims
            .tenant_id
            .as_ref()
            .ok_or(crate::error::AppError::Forbidden(
                "Tenant context missing".to_string(),
            ))?;
        let perms = state
            .auth_service
            .get_user_permissions(&claims.sub, tenant_id)
            .await?;
        if !has_permission(&perms, "backups", "restore") {
            return Err(crate::error::AppError::Forbidden(
                "Missing permission backups:restore".to_string(),
            ));
        }
        let expected_prefix = format!("tenant_{}_", tenant_id);
        if !filename.starts_with(&expected_prefix) {
            return Err(crate::error::AppError::Forbidden(
                "Cannot restore other tenant's backups".to_string(),
            ));
        }
        state
            .backup_service
            .restore_local_backup(filename, Some(tenant_id))
            .await
    };

    res.map(|_| Json(()))
}
