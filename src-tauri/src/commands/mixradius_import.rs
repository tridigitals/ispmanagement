use crate::models::{
    MixradiusImportBatch, MixradiusImportExecuteRequest, MixradiusImportExecutionResult,
    MixradiusImportMappingOverride, MixradiusImportPreview, MixradiusImportPreviewRequest,
    PaginatedResponse,
};
use crate::services::{AuthService, MixradiusImportService};
use tauri::State;

async fn tenant_and_claims(
    auth: &AuthService,
    token: &str,
) -> Result<(crate::services::auth_service::Claims, String), String> {
    let claims = auth
        .validate_token(token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "No tenant ID in token".to_string())?;
    Ok((claims, tenant_id))
}

async fn require_mixradius_permission(
    auth: &AuthService,
    claims: &crate::services::auth_service::Claims,
    tenant_id: &str,
    action: &str,
) -> Result<(), String> {
    auth.check_permission(&claims.sub, tenant_id, "pppoe", action)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upload_mixradius_import(
    token: String,
    file_name: String,
    file_size_bytes: i64,
    content_type: Option<String>,
    source_checksum: Option<String>,
    local_path: Option<String>,
    auth: State<'_, AuthService>,
    mixradius: State<'_, MixradiusImportService>,
) -> Result<MixradiusImportBatch, String> {
    let (claims, tenant_id) = tenant_and_claims(&auth, &token).await?;
    require_mixradius_permission(&auth, &claims, &tenant_id, "manage").await?;

    if file_name.trim().is_empty() {
        return Err("file_name is required".to_string());
    }
    if file_size_bytes <= 0 {
        return Err("file_size_bytes must be greater than zero".to_string());
    }
    let _ = (content_type, source_checksum);
    let local_path = local_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "local_path is required for MixRadius import upload in the current implementation"
                .to_string()
        })?;

    mixradius
        .stage_backup(&tenant_id, Some(&claims.sub), local_path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mixradius_import_batches(
    token: String,
    page: Option<u32>,
    per_page: Option<u32>,
    status: Option<String>,
    auth: State<'_, AuthService>,
    mixradius: State<'_, MixradiusImportService>,
) -> Result<PaginatedResponse<MixradiusImportBatch>, String> {
    let (claims, tenant_id) = tenant_and_claims(&auth, &token).await?;
    require_mixradius_permission(&auth, &claims, &tenant_id, "manage").await?;
    mixradius
        .list_batches(
            &tenant_id,
            page.unwrap_or(1),
            per_page.unwrap_or(25),
            status.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mixradius_import_batch(
    token: String,
    batch_id: String,
    auth: State<'_, AuthService>,
    mixradius: State<'_, MixradiusImportService>,
) -> Result<MixradiusImportBatch, String> {
    let (claims, tenant_id) = tenant_and_claims(&auth, &token).await?;
    require_mixradius_permission(&auth, &claims, &tenant_id, "manage").await?;
    mixradius
        .get_batch(&tenant_id, &batch_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn preview_mixradius_import(
    token: String,
    batch_id: String,
    mapping_overrides: Option<Vec<MixradiusImportMappingOverride>>,
    customer_conflict_resolution: Option<crate::models::MixradiusImportCustomerConflictResolution>,
    location_strategy: Option<crate::models::MixradiusImportLocationStrategy>,
    pppoe_provisioning_target: Option<crate::models::MixradiusImportPppoeProvisioningTarget>,
    auth: State<'_, AuthService>,
    mixradius: State<'_, MixradiusImportService>,
) -> Result<MixradiusImportPreview, String> {
    let (claims, tenant_id) = tenant_and_claims(&auth, &token).await?;
    require_mixradius_permission(&auth, &claims, &tenant_id, "manage").await?;
    mixradius
        .build_preview(
            &tenant_id,
            &MixradiusImportPreviewRequest {
                batch_id,
                mapping_overrides: mapping_overrides.unwrap_or_default(),
                customer_conflict_resolution,
                location_strategy,
                pppoe_provisioning_target,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn execute_mixradius_import(
    token: String,
    batch_id: String,
    execution_mode: crate::models::MixradiusImportExecutionMode,
    mapping_overrides: Option<Vec<MixradiusImportMappingOverride>>,
    customer_conflict_resolution: Option<crate::models::MixradiusImportCustomerConflictResolution>,
    location_strategy: Option<crate::models::MixradiusImportLocationStrategy>,
    pppoe_provisioning_target: Option<crate::models::MixradiusImportPppoeProvisioningTarget>,
    auth: State<'_, AuthService>,
    mixradius: State<'_, MixradiusImportService>,
) -> Result<MixradiusImportExecutionResult, String> {
    let (claims, tenant_id) = tenant_and_claims(&auth, &token).await?;
    require_mixradius_permission(&auth, &claims, &tenant_id, "manage").await?;
    mixradius
        .execute_preview(
            &tenant_id,
            &MixradiusImportExecuteRequest {
                batch_id,
                execution_mode,
                mapping_overrides: mapping_overrides.unwrap_or_default(),
                customer_conflict_resolution,
                location_strategy,
                pppoe_provisioning_target,
            },
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_mixradius_import(
    token: String,
    batch_id: String,
    auth: State<'_, AuthService>,
    mixradius: State<'_, MixradiusImportService>,
) -> Result<MixradiusImportBatch, String> {
    let (claims, tenant_id) = tenant_and_claims(&auth, &token).await?;
    require_mixradius_permission(&auth, &claims, &tenant_id, "manage").await?;
    mixradius
        .cancel_batch(&tenant_id, &batch_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_mixradius_import_batch(
    token: String,
    batch_id: String,
    auth: State<'_, AuthService>,
    mixradius: State<'_, MixradiusImportService>,
) -> Result<(), String> {
    let (claims, tenant_id) = tenant_and_claims(&auth, &token).await?;
    require_mixradius_permission(&auth, &claims, &tenant_id, "manage").await?;
    mixradius
        .delete_batch(&tenant_id, &batch_id)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn mixradius_import_commands_are_defined() {
        let source = include_str!("mixradius_import.rs");

        assert!(source.contains("pub async fn upload_mixradius_import"));
        assert!(source.contains("pub async fn list_mixradius_import_batches"));
        assert!(source.contains("pub async fn get_mixradius_import_batch"));
        assert!(source.contains("pub async fn preview_mixradius_import"));
        assert!(source.contains("pub async fn execute_mixradius_import"));
        assert!(source.contains("pub async fn cancel_mixradius_import"));
        assert!(source.contains("pub async fn delete_mixradius_import_batch"));
    }
}
