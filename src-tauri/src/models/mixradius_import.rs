use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, Type};
use validator::{Validate, ValidationError};

fn validate_non_blank(value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::new("required"));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum MixradiusImportBatchStatus {
    #[default]
    Pending,
    Running,
    PartialSuccess,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum MixradiusImportParseStatus {
    #[default]
    Pending,
    Running,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum MixradiusImportConflictState {
    AutoMatched,
    NeedsReview,
    Conflict,
    Blocked,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum MixradiusImportExecutionMode {
    #[default]
    PreviewOnly,
    SafeImport,
    ForceSync,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum MixradiusImportCustomerConflictResolution {
    Merge,
    CreateNew,
    Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum MixradiusImportLocationStrategy {
    Preserve,
    Merge,
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum MixradiusImportPppoeProvisioningTarget {
    #[default]
    Router,
    ManagedRadius,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MixradiusImportBatch {
    pub id: String,
    pub tenant_id: String,
    pub source_filename: String,
    pub source_sha256: String,
    pub source_size_bytes: i64,
    pub parse_status: MixradiusImportParseStatus,
    pub execution_status: MixradiusImportBatchStatus,
    pub execution_mode: MixradiusImportExecutionMode,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress_json: Value,
    pub summary_json: Value,
    pub error_json: Value,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixradiusImportPreview {
    pub batch_id: String,
    pub total_rows: i64,
    pub rows: Vec<MixradiusImportPreviewRow>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixradiusImportPreviewRow {
    pub row_number: i64,
    pub source_kind: String,
    pub source_ref: String,
    pub target_kind: Option<String>,
    pub target_id: Option<String>,
    pub display_name: Option<String>,
    pub conflict_state: MixradiusImportConflictState,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixradiusImportExecutionSummary {
    pub batch_id: String,
    pub mode: MixradiusImportExecutionMode,
    pub total_rows: i64,
    pub imported_rows: i64,
    pub updated_rows: i64,
    pub skipped_rows: i64,
    pub blocked_rows: i64,
    pub conflict_rows: i64,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixradiusImportExecutionResult {
    pub batch: MixradiusImportBatch,
    pub summary: MixradiusImportExecutionSummary,
    #[serde(default)]
    pub preview: Option<MixradiusImportPreview>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct MixradiusImportUploadRequest {
    #[validate(length(min = 1, message = "file_name is required"))]
    #[validate(custom(function = "validate_non_blank"))]
    pub file_name: String,
    #[validate(range(min = 1, message = "file_size_bytes must be greater than zero"))]
    pub file_size_bytes: i64,
    pub content_type: Option<String>,
    pub source_checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct MixradiusImportPreviewRequest {
    #[serde(default)]
    #[validate(length(min = 1, message = "batch_id is required"))]
    #[validate(custom(function = "validate_non_blank"))]
    pub batch_id: String,
    #[serde(default)]
    #[validate(nested)]
    pub mapping_overrides: Vec<MixradiusImportMappingOverride>,
    pub customer_conflict_resolution: Option<MixradiusImportCustomerConflictResolution>,
    pub location_strategy: Option<MixradiusImportLocationStrategy>,
    #[serde(alias = "pppoe_provisioning_target")]
    pub pppoe_provisioning_target: Option<MixradiusImportPppoeProvisioningTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct MixradiusImportExecuteRequest {
    #[serde(default)]
    #[validate(length(min = 1, message = "batch_id is required"))]
    #[validate(custom(function = "validate_non_blank"))]
    pub batch_id: String,
    pub execution_mode: MixradiusImportExecutionMode,
    #[serde(default)]
    #[validate(nested)]
    pub mapping_overrides: Vec<MixradiusImportMappingOverride>,
    pub customer_conflict_resolution: Option<MixradiusImportCustomerConflictResolution>,
    pub location_strategy: Option<MixradiusImportLocationStrategy>,
    #[serde(alias = "pppoe_provisioning_target")]
    pub pppoe_provisioning_target: Option<MixradiusImportPppoeProvisioningTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct MixradiusImportMappingOverride {
    #[serde(alias = "source_kind")]
    #[validate(length(min = 1, message = "source_kind is required"))]
    #[validate(custom(function = "validate_non_blank"))]
    pub source_kind: String,
    #[serde(alias = "source_value")]
    #[validate(length(min = 1, message = "source_value is required"))]
    #[validate(custom(function = "validate_non_blank"))]
    pub source_value: String,
    #[serde(alias = "target_kind")]
    #[validate(length(min = 1, message = "target_kind is required"))]
    #[validate(custom(function = "validate_non_blank"))]
    pub target_kind: String,
    #[serde(alias = "target_value")]
    #[validate(length(min = 1, message = "target_value is required"))]
    #[validate(custom(function = "validate_non_blank"))]
    pub target_value: String,
}
