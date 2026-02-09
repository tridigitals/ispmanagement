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

    // Tenant backup/restore is intentionally disabled. Backups are managed by Super Admin only.
    if !claims.is_super_admin {
        return Err(crate::error::AppError::Forbidden(
            "Backups are managed by Super Admin".to_string(),
        ));
    }

    let backups = state.backup_service.list_backups().await?;
    if claims.is_super_admin && query.tenant_only != Some(true) {
        return Ok(Json(backups));
    }

    let filtered: Vec<BackupRecord> = backups
        .into_iter()
        .filter(|b| b.backup_type == "tenant")
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
            // Audit (best-effort)
            let path_for_log = path.clone();
            let details = serde_json::json!({ "type": "global", "path": path_for_log }).to_string();
            state
                .audit_service
                .log(
                    Some(&claims.sub),
                    None,
                    "create",
                    "backups",
                    None,
                    Some(details.as_str()),
                    None,
                )
                .await;
            Ok(Json(path))
        }
        "tenant" => {
            if !claims.is_super_admin {
                return Err(crate::error::AppError::Forbidden(
                    "Backups are managed by Super Admin".to_string(),
                ));
            }
            let target_id = payload.target_id.ok_or(crate::error::AppError::Validation(
                "Target ID required".to_string(),
            ))?;

            let path = state
                .backup_service
                .create_tenant_backup(&target_id)
                .await?;
            // Audit (best-effort)
            let path_for_log = path.clone();
            let details =
                serde_json::json!({ "type": "tenant", "tenant_id": target_id, "path": path_for_log })
                    .to_string();
            state
                .audit_service
                .log(
                    Some(&claims.sub),
                    None,
                    "create",
                    "backups",
                    None,
                    Some(details.as_str()),
                    None,
                )
                .await;
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
        return Err(crate::error::AppError::Forbidden(
            "Backups are managed by Super Admin".to_string(),
        ));
    }

    let filename_for_log = filename.clone();
    state.backup_service.delete_backup(filename).await?;
    // Audit (best-effort)
    let details = serde_json::json!({ "filename": filename_for_log }).to_string();
    state
        .audit_service
        .log(
            Some(&claims.sub),
            None,
            "delete",
            "backups",
            None,
            Some(details.as_str()),
            None,
        )
        .await;
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
        return Err(crate::error::AppError::Forbidden(
            "Backups are managed by Super Admin".to_string(),
        ));
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

    if !claims.is_super_admin {
        return Err(crate::error::AppError::Forbidden(
            "Backups are managed by Super Admin".to_string(),
        ));
    }

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

    let res = state
        .backup_service
        .restore_from_zip(temp_path.clone(), None)
        .await;

    // Cleanup
    let _ = tokio::fs::remove_file(temp_path).await;

    if res.is_ok() {
        // Audit (best-effort)
        let details = serde_json::json!({ "source": "upload" }).to_string();
        state
            .audit_service
            .log(
                Some(&claims.sub),
                None,
                "restore",
                "backups",
                None,
                Some(details.as_str()),
                None,
            )
            .await;
    }

    res.map(|_| Json(()))
}

async fn restore_local_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(filename): Path<String>,
) -> AppResult<Json<()>> {
    let token = extract_token(&headers)?;
    let claims = state.auth_service.validate_token(&token).await?;

    if !claims.is_super_admin {
        return Err(crate::error::AppError::Forbidden(
            "Backups are managed by Super Admin".to_string(),
        ));
    }

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

    let res = state
        .backup_service
        .restore_local_backup(filename.clone(), tenant_id)
        .await;

    if res.is_ok() {
        // Audit (best-effort)
        let details = serde_json::json!({ "source": "local", "filename": filename }).to_string();
        state
            .audit_service
            .log(
                Some(&claims.sub),
                None,
                "restore",
                "backups",
                None,
                Some(details.as_str()),
                None,
            )
            .await;
    }

    res.map(|_| Json(()))
}
