use crate::error::{AppError, AppResult};
use crate::http::AppState;
use crate::models::{
    MixradiusImportBatch, MixradiusImportExecuteRequest, MixradiusImportExecutionResult,
    MixradiusImportPreview, MixradiusImportPreviewRequest, PaginatedResponse,
};
use axum::{
    extract::{Multipart, Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use tracing::{error, info, warn};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/imports", get(list_batches).post(upload_backup))
        .route("/imports/upload", post(upload_backup_file))
        .route("/imports/{batch_id}", get(get_batch))
        .route("/imports/{batch_id}/preview", post(preview_batch))
        .route("/imports/{batch_id}/execute", post(execute_batch))
        .route("/imports/{batch_id}/cancel", post(cancel_batch))
}

fn safe_upload_filename(raw: &str) -> String {
    let trimmed = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("mixradius-backup.sql.gz")
        .trim();
    let sanitized: String = trimmed
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "mixradius-backup.sql.gz".to_string()
    } else {
        sanitized
    }
}

fn bearer_token(headers: &HeaderMap) -> AppResult<String> {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or(AppError::Unauthorized)
}

async fn tenant_and_claims(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<(String, crate::services::auth_service::Claims)> {
    let token = bearer_token(headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims.tenant_id.clone().ok_or(AppError::Unauthorized)?;
    Ok((tenant_id, claims))
}

async fn require_mixradius_permission(
    state: &AppState,
    claims: &crate::services::auth_service::Claims,
    tenant_id: &str,
    action: &str,
) -> AppResult<()> {
    state
        .auth_service
        .check_permission(&claims.sub, tenant_id, "pppoe", action)
        .await
}

#[derive(Debug, Deserialize)]
struct ListBatchesQuery {
    page: Option<u32>,
    per_page: Option<u32>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UploadMixradiusImportRequest {
    file_name: String,
    file_size_bytes: i64,
    content_type: Option<String>,
    source_checksum: Option<String>,
    local_path: Option<String>,
}

async fn list_batches(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListBatchesQuery>,
) -> AppResult<Json<PaginatedResponse<MixradiusImportBatch>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_mixradius_permission(&state, &claims, &tenant_id, "manage").await?;

    let batches = state
        .mixradius_import_service
        .list_batches(
            &tenant_id,
            query.page.unwrap_or(1),
            query.per_page.unwrap_or(25),
            query.status.as_deref(),
        )
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(Json(batches))
}

async fn get_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(batch_id): Path<String>,
) -> AppResult<Json<MixradiusImportBatch>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_mixradius_permission(&state, &claims, &tenant_id, "manage").await?;

    let batch = state
        .mixradius_import_service
        .get_batch(&tenant_id, &batch_id)
        .await
        .map_err(|error| AppError::NotFound(error.to_string()))?;

    Ok(Json(batch))
}

async fn upload_backup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<UploadMixradiusImportRequest>,
) -> AppResult<Json<MixradiusImportBatch>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_mixradius_permission(&state, &claims, &tenant_id, "manage").await?;

    info!(
        tenant_id = %tenant_id,
        user_id = %claims.sub,
        file_name = %dto.file_name,
        file_size_bytes = dto.file_size_bytes,
        has_local_path = dto.local_path.as_ref().map(|v| !v.trim().is_empty()).unwrap_or(false),
        "MixRadius local-path upload request received"
    );

    if dto.file_name.trim().is_empty() {
        warn!(tenant_id = %tenant_id, user_id = %claims.sub, "MixRadius upload rejected: empty file_name");
        return Err(AppError::Validation("file_name is required".into()));
    }
    if dto.file_size_bytes <= 0 {
        warn!(
            tenant_id = %tenant_id,
            user_id = %claims.sub,
            file_size_bytes = dto.file_size_bytes,
            "MixRadius upload rejected: non-positive file_size_bytes"
        );
        return Err(AppError::Validation(
            "file_size_bytes must be greater than zero".into(),
        ));
    }

    let local_path = dto
        .local_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            warn!(
                tenant_id = %tenant_id,
                user_id = %claims.sub,
                "MixRadius upload rejected: missing local_path"
            );
            AppError::Validation(
                "local_path is required for MixRadius import upload in the current implementation"
                    .into(),
            )
        })?;

    let _source_metadata = (
        dto.content_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
        dto.source_checksum
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    );

    let batch = state
        .mixradius_import_service
        .stage_backup(&tenant_id, Some(&claims.sub), local_path)
        .await
        .map_err(|error| {
            error!(
                tenant_id = %tenant_id,
                user_id = %claims.sub,
                local_path = %local_path,
                error = %error,
                "MixRadius local-path upload failed during stage_backup"
            );
            AppError::Internal(error.to_string())
        })?;

    info!(
        tenant_id = %tenant_id,
        user_id = %claims.sub,
        batch_id = %batch.id,
        source_filename = %batch.source_filename,
        "MixRadius local-path upload staged successfully"
    );

    Ok(Json(batch))
}

async fn upload_backup_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> AppResult<Json<MixradiusImportBatch>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_mixradius_permission(&state, &claims, &tenant_id, "manage").await?;

    info!(
        tenant_id = %tenant_id,
        user_id = %claims.sub,
        "MixRadius browser upload request received"
    );

    let temp_dir = std::env::temp_dir();
    let mut temp_path = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
    {
        if field.name().unwrap_or_default() != "file" {
            continue;
        }

        let filename = safe_upload_filename(field.file_name().unwrap_or("mixradius-backup.sql.gz"));
        let path = temp_dir.join(format!("mixradius_import_{}_{}", Uuid::new_v4(), filename));
        let data = field.bytes().await.map_err(|error| {
            error!(
                tenant_id = %tenant_id,
                user_id = %claims.sub,
                file_name = %filename,
                error = %error,
                "MixRadius browser upload failed while reading multipart field"
            );
            AppError::Internal(error.to_string())
        })?;

        info!(
            tenant_id = %tenant_id,
            user_id = %claims.sub,
            file_name = %filename,
            temp_path = %path.display(),
            file_size_bytes = data.len(),
            "MixRadius browser upload file extracted from multipart"
        );

        if data.is_empty() {
            warn!(
                tenant_id = %tenant_id,
                user_id = %claims.sub,
                file_name = %filename,
                "MixRadius browser upload rejected: empty file payload"
            );
            return Err(AppError::Validation("No file uploaded".to_string()));
        }

        tokio::fs::write(&path, data).await.map_err(|error| {
            error!(
                tenant_id = %tenant_id,
                user_id = %claims.sub,
                temp_path = %path.display(),
                error = %error,
                "MixRadius browser upload failed while writing temp file"
            );
            AppError::Internal(error.to_string())
        })?;
        temp_path = Some(path);
        break;
    }

    let temp_path = temp_path.ok_or_else(|| {
        warn!(
            tenant_id = %tenant_id,
            user_id = %claims.sub,
            "MixRadius browser upload rejected: multipart did not contain file field"
        );
        AppError::Validation("No file uploaded".to_string())
    })?;

    let result = state
        .mixradius_import_service
        .stage_backup(&tenant_id, Some(&claims.sub), &temp_path)
        .await
        .map_err(|error| {
            error!(
                tenant_id = %tenant_id,
                user_id = %claims.sub,
                temp_path = %temp_path.display(),
                error = %error,
                "MixRadius browser upload failed during stage_backup"
            );
            AppError::Internal(error.to_string())
        });

    if let Err(error) = tokio::fs::remove_file(&temp_path).await {
        warn!(
            tenant_id = %tenant_id,
            user_id = %claims.sub,
            temp_path = %temp_path.display(),
            error = %error,
            "MixRadius browser upload temp file cleanup failed"
        );
    }

    if let Ok(batch) = &result {
        info!(
            tenant_id = %tenant_id,
            user_id = %claims.sub,
            batch_id = %batch.id,
            source_filename = %batch.source_filename,
            "MixRadius browser upload staged successfully"
        );
    }

    result.map(Json)
}

async fn preview_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(batch_id): Path<String>,
    Json(mut dto): Json<MixradiusImportPreviewRequest>,
) -> AppResult<Json<MixradiusImportPreview>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_mixradius_permission(&state, &claims, &tenant_id, "manage").await?;
    dto.batch_id = batch_id;

    let preview = state
        .mixradius_import_service
        .build_preview(&tenant_id, &dto)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(Json(preview))
}

async fn execute_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(batch_id): Path<String>,
    Json(mut dto): Json<MixradiusImportExecuteRequest>,
) -> AppResult<Json<MixradiusImportExecutionResult>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_mixradius_permission(&state, &claims, &tenant_id, "manage").await?;
    dto.batch_id = batch_id;

    let result = state
        .mixradius_import_service
        .execute_preview(&tenant_id, &dto)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(Json(result))
}

async fn cancel_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(batch_id): Path<String>,
) -> AppResult<Json<MixradiusImportBatch>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_mixradius_permission(&state, &claims, &tenant_id, "manage").await?;

    let batch = state
        .mixradius_import_service
        .cancel_batch(&tenant_id, &batch_id)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(Json(batch))
}
