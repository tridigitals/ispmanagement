use crate::error::{AppError, AppResult};
use crate::http::AppState;
use crate::models::{
    MixradiusImportBatch, MixradiusImportExecuteRequest, MixradiusImportExecutionResult,
    MixradiusImportPreview, MixradiusImportPreviewRequest, PaginatedResponse,
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/imports", get(list_batches).post(upload_backup))
        .route("/imports/{batch_id}", get(get_batch))
        .route("/imports/{batch_id}/preview", post(preview_batch))
        .route("/imports/{batch_id}/execute", post(execute_batch))
        .route("/imports/{batch_id}/cancel", post(cancel_batch))
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

    if dto.file_name.trim().is_empty() {
        return Err(AppError::Validation("file_name is required".into()));
    }
    if dto.file_size_bytes <= 0 {
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
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(Json(batch))
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
