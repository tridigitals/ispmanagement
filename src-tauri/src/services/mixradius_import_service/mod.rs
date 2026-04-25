use crate::db::DbPool;
use crate::models::{
    MixradiusImportBatch, MixradiusImportBatchStatus, MixradiusImportConflictState,
    MixradiusImportCustomerConflictResolution, MixradiusImportExecuteRequest,
    MixradiusImportExecutionResult, MixradiusImportExecutionSummary,
    MixradiusImportLocationStrategy, MixradiusImportParseStatus, MixradiusImportPreview,
    MixradiusImportPreviewRequest, MixradiusImportPreviewRow, PaginatedResponse,
};
use crate::services::mixradius_import_executor::MixradiusImportExecutor;
use crate::services::mixradius_sql_parser::{
    parse_mixradius_backup, MixradiusParsedBackup, MixradiusSourceRow,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, QueryBuilder};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;

const MIXRADIUS_STAGE_INSERT_BATCH_SIZE: usize = 250;

#[derive(Clone)]
pub struct MixradiusImportService {
    pool: DbPool,
}

impl MixradiusImportService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn stage_backup<P: AsRef<Path>>(
        &self,
        tenant_id: &str,
        created_by: Option<&str>,
        backup_path: P,
    ) -> Result<MixradiusImportBatch> {
        let backup_path = backup_path.as_ref();
        let backup_bytes = fs::read(backup_path)
            .with_context(|| format!("failed to read backup file `{}`", backup_path.display()))?;
        let source_sha256 = format!("{:x}", Sha256::digest(&backup_bytes));
        let source_size_bytes = backup_bytes.len() as i64;
        let source_filename = backup_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("mixradius-backup.sql.gz")
            .to_string();

        let batch_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO public.mixradius_import_batches (
                id,
                tenant_id,
                source_filename,
                source_sha256,
                source_size_bytes,
                parse_status,
                execution_status,
                execution_mode,
                progress_json,
                summary_json,
                error_json,
                created_by,
                created_at,
                updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5,
                'running',
                'pending',
                'preview_only',
                $6, $7, $8, $9, $10, $11
            )
            "#,
        )
        .bind(&batch_id)
        .bind(tenant_id)
        .bind(&source_filename)
        .bind(&source_sha256)
        .bind(source_size_bytes)
        .bind(json!({"stage": "parsing"}))
        .bind(json!({}))
        .bind(json!([]))
        .bind(created_by)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("failed to register MixRadius import batch")?;

        let parsed = match parse_mixradius_backup(backup_path) {
            Ok(parsed) => parsed,
            Err(error) => {
                let error_text = error.to_string();
                sqlx::query(
                    r#"
                    UPDATE public.mixradius_import_batches
                    SET parse_status = 'failed',
                        error_json = $2,
                        progress_json = $3,
                        updated_at = $4
                    WHERE id = $1
                    "#,
                )
                .bind(&batch_id)
                .bind(json!([{ "message": error_text }]))
                .bind(json!({"stage": "failed"}))
                .bind(Utc::now())
                .execute(&self.pool)
                .await
                .context("failed to record MixRadius import failure")?;
                return Err(error);
            }
        };

        if let Err(error) = self.stage_parsed_backup(&batch_id, tenant_id, parsed).await {
            let error_text = error.to_string();
            sqlx::query(
                r#"
                UPDATE public.mixradius_import_batches
                SET parse_status = 'failed',
                    error_json = $2,
                    progress_json = $3,
                    updated_at = $4
                WHERE id = $1
                "#,
            )
            .bind(&batch_id)
            .bind(json!([{ "message": error_text }]))
            .bind(json!({"stage": "failed"}))
            .bind(Utc::now())
            .execute(&self.pool)
            .await
            .context("failed to record MixRadius import failure")?;
            return Err(error);
        }

        let batch = sqlx::query_as::<_, MixradiusImportBatch>(
            r#"
            SELECT
                id,
                tenant_id,
                source_filename,
                source_sha256,
                source_size_bytes,
                parse_status,
                execution_status,
                execution_mode,
                started_at,
                completed_at,
                progress_json,
                summary_json,
                error_json,
                created_by,
                created_at,
                updated_at
            FROM public.mixradius_import_batches
            WHERE id = $1
            "#,
        )
        .bind(&batch_id)
        .fetch_one(&self.pool)
        .await
        .context("failed to load staged MixRadius import batch")?;

        Ok(batch)
    }

    pub async fn list_batches(
        &self,
        tenant_id: &str,
        page: u32,
        per_page: u32,
        status: Option<&str>,
    ) -> Result<PaginatedResponse<MixradiusImportBatch>> {
        let page = page.max(1);
        let per_page = per_page.max(1);
        let offset = ((page - 1) * per_page) as i64;
        let status = status
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM public.mixradius_import_batches
            WHERE tenant_id = $1
              AND ($2::text IS NULL OR execution_status = $2 OR parse_status = $2)
            "#,
        )
        .bind(tenant_id)
        .bind(status.as_deref())
        .fetch_one(&self.pool)
        .await
        .context("failed to count MixRadius import batches")?;

        let data = sqlx::query_as::<_, MixradiusImportBatch>(
            r#"
            SELECT
                id,
                tenant_id,
                source_filename,
                source_sha256,
                source_size_bytes,
                parse_status,
                execution_status,
                execution_mode,
                started_at,
                completed_at,
                progress_json,
                summary_json,
                error_json,
                created_by,
                created_at,
                updated_at
            FROM public.mixradius_import_batches
            WHERE tenant_id = $1
              AND ($2::text IS NULL OR execution_status = $2 OR parse_status = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(status.as_deref())
        .bind(per_page as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .context("failed to list MixRadius import batches")?;

        Ok(PaginatedResponse {
            data,
            total,
            page,
            per_page,
        })
    }

    pub async fn get_batch(&self, tenant_id: &str, batch_id: &str) -> Result<MixradiusImportBatch> {
        sqlx::query_as::<_, MixradiusImportBatch>(
            r#"
            SELECT
                id,
                tenant_id,
                source_filename,
                source_sha256,
                source_size_bytes,
                parse_status,
                execution_status,
                execution_mode,
                started_at,
                completed_at,
                progress_json,
                summary_json,
                error_json,
                created_by,
                created_at,
                updated_at
            FROM public.mixradius_import_batches
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .fetch_one(&self.pool)
        .await
        .context("failed to load MixRadius import batch")
    }

    pub async fn build_preview(
        &self,
        tenant_id: &str,
        request: &MixradiusImportPreviewRequest,
    ) -> Result<MixradiusImportPreview> {
        let batch = self.get_batch(tenant_id, &request.batch_id).await?;
        if batch.parse_status != MixradiusImportParseStatus::Ready {
            return Err(anyhow!("MixRadius batch is not ready for preview"));
        }

        self.persist_progress_payload(
            tenant_id,
            &request.batch_id,
            "previewRequest",
            serde_json::to_value(request).context("failed to serialize preview request")?,
        )
        .await?;

        let rows = self
            .preview_rows_for_batch(
                tenant_id,
                &request.batch_id,
                &request.mapping_overrides,
                request.customer_conflict_resolution,
                request.location_strategy,
            )
            .await?;
        Ok(MixradiusImportPreview {
            batch_id: request.batch_id.clone(),
            total_rows: rows.len() as i64,
            rows,
            generated_at: Utc::now(),
        })
    }

    pub async fn execute_preview(
        &self,
        tenant_id: &str,
        request: &MixradiusImportExecuteRequest,
    ) -> Result<MixradiusImportExecutionResult> {
        self.persist_progress_payload(
            tenant_id,
            &request.batch_id,
            "executeRequest",
            serde_json::to_value(request).context("failed to serialize execute request")?,
        )
        .await?;
        let preview_request = MixradiusImportPreviewRequest {
            batch_id: request.batch_id.clone(),
            mapping_overrides: request.mapping_overrides.clone(),
            customer_conflict_resolution: request.customer_conflict_resolution,
            location_strategy: request.location_strategy,
            pppoe_provisioning_target: request.pppoe_provisioning_target,
        };
        let preview = self.build_preview(tenant_id, &preview_request).await?;
        let legacy_transaction_count = self
            .legacy_transaction_count(tenant_id, &request.batch_id)
            .await?;
        let production_invoice_count = self.production_invoice_count(tenant_id).await.unwrap_or(0);

        let blocked_rows = preview
            .rows
            .iter()
            .filter(|row| row.conflict_state == MixradiusImportConflictState::Blocked)
            .count() as i64;
        let conflict_rows = preview
            .rows
            .iter()
            .filter(|row| row.conflict_state == MixradiusImportConflictState::Conflict)
            .count() as i64;
        let skipped_rows = preview
            .rows
            .iter()
            .filter(|row| row.conflict_state == MixradiusImportConflictState::Skipped)
            .count() as i64;

        let mut warnings = Vec::new();
        let mut imported_rows = 0;
        let mut updated_rows = 0;
        let mut execution_conflict_rows = 0;

        if request.execution_mode != crate::models::MixradiusImportExecutionMode::PreviewOnly {
            sqlx::query(
                r#"
                UPDATE public.mixradius_import_batches
                SET execution_status = 'running',
                    execution_mode = $3,
                    started_at = COALESCE(started_at, $4),
                    progress_json = jsonb_set(progress_json, '{stage}', '"executing_packages"', true),
                    updated_at = $4
                WHERE tenant_id = $1 AND id = $2
                "#,
            )
            .bind(tenant_id)
            .bind(&request.batch_id)
            .bind(request.execution_mode)
            .bind(Utc::now())
            .execute(&self.pool)
            .await
            .context("failed to mark MixRadius batch as running")?;

            let executor = MixradiusImportExecutor::new(self.pool.clone());
            let mut phase_reports = serde_json::Map::new();
            let mut execution_errors: Vec<Value> = Vec::new();
            let mut progress_stage: &str;

            let package_summary = match executor
                .execute_package_imports_with_mode(
                    tenant_id,
                    &request.batch_id,
                    &request.mapping_overrides,
                    request.execution_mode,
                )
                .await
            {
                Ok(summary) => summary,
                Err(error) => {
                    let error_message = format!("{error:#}");
                    execution_errors
                        .push(json!({"phase": "packages", "message": error_message.clone()}));
                    phase_reports.insert(
                        "packages".to_string(),
                        json!({"status": "failed", "message": error_message}),
                    );
                    progress_stage = "packages_failed";
                    let summary_json = build_execution_report_json(
                        request,
                        preview.total_rows,
                        imported_rows,
                        updated_rows,
                        skipped_rows,
                        blocked_rows,
                        conflict_rows + execution_conflict_rows,
                        &warnings,
                        &phase_reports,
                        &execution_errors,
                        legacy_transaction_count,
                        production_invoice_count,
                    );
                    self.finalize_execution_report(
                        tenant_id,
                        &request.batch_id,
                        request.execution_mode,
                        "failed",
                        progress_stage,
                        summary_json,
                        json!(execution_errors),
                        &phase_reports,
                    )
                    .await?;
                    let batch = self.get_batch(tenant_id, &request.batch_id).await?;
                    return Ok(MixradiusImportExecutionResult {
                        batch,
                        summary: MixradiusImportExecutionSummary {
                            batch_id: request.batch_id.clone(),
                            mode: request.execution_mode,
                            total_rows: preview.total_rows,
                            imported_rows,
                            updated_rows,
                            skipped_rows,
                            blocked_rows,
                            conflict_rows: conflict_rows + execution_conflict_rows,
                            warnings: warnings.clone(),
                        },
                        preview: Some(preview),
                        warnings,
                    });
                }
            };
            imported_rows = package_summary.imported_rows;
            updated_rows = package_summary.updated_rows;
            execution_conflict_rows = package_summary.conflict_rows;
            warnings.extend(package_summary.warnings);
            phase_reports.insert(
                "packages".to_string(),
                json!({
                    "status": "completed",
                    "importedRows": package_summary.imported_rows,
                    "updatedRows": package_summary.updated_rows,
                    "skippedRows": package_summary.skipped_rows,
                    "conflictRows": package_summary.conflict_rows
                }),
            );
            progress_stage = "packages_imported_partial";

            let customer_summary = match executor
                .execute_customer_imports(tenant_id, &request.batch_id)
                .await
            {
                Ok(summary) => summary,
                Err(error) => {
                    let error_message = format!("{error:#}");
                    execution_errors
                        .push(json!({"phase": "customers", "message": error_message.clone()}));
                    phase_reports.insert(
                        "customers".to_string(),
                        json!({"status": "failed", "message": error_message}),
                    );
                    progress_stage = "customers_failed_partial";
                    let summary_json = build_execution_report_json(
                        request,
                        preview.total_rows,
                        imported_rows,
                        updated_rows,
                        skipped_rows,
                        blocked_rows,
                        conflict_rows + execution_conflict_rows,
                        &warnings,
                        &phase_reports,
                        &execution_errors,
                        legacy_transaction_count,
                        production_invoice_count,
                    );
                    self.finalize_execution_report(
                        tenant_id,
                        &request.batch_id,
                        request.execution_mode,
                        "partial_success",
                        progress_stage,
                        summary_json,
                        json!(execution_errors),
                        &phase_reports,
                    )
                    .await?;
                    let batch = self.get_batch(tenant_id, &request.batch_id).await?;
                    return Ok(MixradiusImportExecutionResult {
                        batch,
                        summary: MixradiusImportExecutionSummary {
                            batch_id: request.batch_id.clone(),
                            mode: request.execution_mode,
                            total_rows: preview.total_rows,
                            imported_rows,
                            updated_rows,
                            skipped_rows,
                            blocked_rows,
                            conflict_rows: conflict_rows + execution_conflict_rows,
                            warnings: warnings.clone(),
                        },
                        preview: Some(preview),
                        warnings,
                    });
                }
            };
            if customer_summary.total_rows > 0 {
                imported_rows +=
                    customer_summary.imported_rows + customer_summary.location_imported_rows;
                updated_rows +=
                    customer_summary.updated_rows + customer_summary.location_updated_rows;
                execution_conflict_rows += customer_summary.conflict_rows;
                warnings.extend(customer_summary.warnings);
                progress_stage = "customers_imported_partial";
            }
            phase_reports.insert(
                "customers".to_string(),
                json!({
                    "status": "completed",
                    "totalRows": customer_summary.total_rows,
                    "importedRows": customer_summary.imported_rows,
                    "updatedRows": customer_summary.updated_rows,
                    "locationImportedRows": customer_summary.location_imported_rows,
                    "locationUpdatedRows": customer_summary.location_updated_rows,
                    "skippedRows": customer_summary.skipped_rows,
                    "conflictRows": customer_summary.conflict_rows
                }),
            );

            let subscription_summary = match executor
                .execute_subscription_imports(tenant_id, &request.batch_id)
                .await
            {
                Ok(summary) => summary,
                Err(error) => {
                    let error_message = format!("{error:#}");
                    execution_errors
                        .push(json!({"phase": "subscriptions", "message": error_message.clone()}));
                    phase_reports.insert(
                        "subscriptions".to_string(),
                        json!({"status": "failed", "message": error_message}),
                    );
                    progress_stage = "subscriptions_failed_partial";
                    let summary_json = build_execution_report_json(
                        request,
                        preview.total_rows,
                        imported_rows,
                        updated_rows,
                        skipped_rows,
                        blocked_rows,
                        conflict_rows + execution_conflict_rows,
                        &warnings,
                        &phase_reports,
                        &execution_errors,
                        legacy_transaction_count,
                        production_invoice_count,
                    );
                    self.finalize_execution_report(
                        tenant_id,
                        &request.batch_id,
                        request.execution_mode,
                        "partial_success",
                        progress_stage,
                        summary_json,
                        json!(execution_errors),
                        &phase_reports,
                    )
                    .await?;
                    let batch = self.get_batch(tenant_id, &request.batch_id).await?;
                    return Ok(MixradiusImportExecutionResult {
                        batch,
                        summary: MixradiusImportExecutionSummary {
                            batch_id: request.batch_id.clone(),
                            mode: request.execution_mode,
                            total_rows: preview.total_rows,
                            imported_rows,
                            updated_rows,
                            skipped_rows,
                            blocked_rows,
                            conflict_rows: conflict_rows + execution_conflict_rows,
                            warnings: warnings.clone(),
                        },
                        preview: Some(preview),
                        warnings,
                    });
                }
            };
            if subscription_summary.total_rows > 0 {
                imported_rows += subscription_summary.imported_rows;
                updated_rows += subscription_summary.updated_rows;
                execution_conflict_rows += subscription_summary.conflict_rows;
                warnings.extend(subscription_summary.warnings);
                progress_stage = "subscriptions_imported_partial";
            }
            phase_reports.insert(
                "subscriptions".to_string(),
                json!({
                    "status": "completed",
                    "totalRows": subscription_summary.total_rows,
                    "importedRows": subscription_summary.imported_rows,
                    "updatedRows": subscription_summary.updated_rows,
                    "skippedRows": subscription_summary.skipped_rows,
                    "conflictRows": subscription_summary.conflict_rows
                }),
            );

            let pppoe_summary = match executor
                .execute_pppoe_imports_with_target(
                    tenant_id,
                    &request.batch_id,
                    &request.mapping_overrides,
                    request.pppoe_provisioning_target.unwrap_or_default(),
                )
                .await
            {
                Ok(summary) => summary,
                Err(error) => {
                    let error_message = format!("{error:#}");
                    execution_errors
                        .push(json!({"phase": "pppoe", "message": error_message.clone()}));
                    phase_reports.insert(
                        "pppoe".to_string(),
                        json!({"status": "failed", "message": error_message}),
                    );
                    progress_stage = "pppoe_failed_partial";
                    warnings.push(format!(
                        "Import PPPoE MixRadius gagal setelah fase sebelumnya sukses: {error}"
                    ));
                    let summary_json = build_execution_report_json(
                        request,
                        preview.total_rows,
                        imported_rows,
                        updated_rows,
                        skipped_rows,
                        blocked_rows,
                        conflict_rows + execution_conflict_rows,
                        &warnings,
                        &phase_reports,
                        &execution_errors,
                        legacy_transaction_count,
                        production_invoice_count,
                    );
                    self.finalize_execution_report(
                        tenant_id,
                        &request.batch_id,
                        request.execution_mode,
                        "partial_success",
                        progress_stage,
                        summary_json,
                        json!(execution_errors),
                        &phase_reports,
                    )
                    .await?;
                    let batch = self.get_batch(tenant_id, &request.batch_id).await?;
                    return Ok(MixradiusImportExecutionResult {
                        batch,
                        summary: MixradiusImportExecutionSummary {
                            batch_id: request.batch_id.clone(),
                            mode: request.execution_mode,
                            total_rows: preview.total_rows,
                            imported_rows,
                            updated_rows,
                            skipped_rows,
                            blocked_rows,
                            conflict_rows: conflict_rows + execution_conflict_rows,
                            warnings: warnings.clone(),
                        },
                        preview: Some(preview),
                        warnings,
                    });
                }
            };
            if pppoe_summary.total_rows > 0
                && (pppoe_summary.imported_rows > 0
                    || pppoe_summary.updated_rows > 0
                    || pppoe_summary.conflict_rows > 0
                    || pppoe_summary.skipped_rows > 0
                    || !pppoe_summary.warnings.is_empty())
            {
                imported_rows += pppoe_summary.imported_rows;
                updated_rows += pppoe_summary.updated_rows;
                execution_conflict_rows += pppoe_summary.conflict_rows;
                warnings.extend(pppoe_summary.warnings);
                progress_stage = "pppoe_imported_partial";
            }
            phase_reports.insert(
                "pppoe".to_string(),
                json!({
                    "status": "completed",
                    "totalRows": pppoe_summary.total_rows,
                    "importedRows": pppoe_summary.imported_rows,
                    "updatedRows": pppoe_summary.updated_rows,
                    "skippedRows": pppoe_summary.skipped_rows,
                    "conflictRows": pppoe_summary.conflict_rows
                }),
            );

            warnings.push(
                "Import MixRadius sudah mengeksekusi sinkronisasi package, customer, lokasi, subscription, dan PPPoE. Billing invoice produksi masih tahap berikutnya."
                    .to_string(),
            );

            let summary_json = build_execution_report_json(
                request,
                preview.total_rows,
                imported_rows,
                updated_rows,
                skipped_rows,
                blocked_rows,
                conflict_rows + execution_conflict_rows,
                &warnings,
                &phase_reports,
                &execution_errors,
                legacy_transaction_count,
                production_invoice_count,
            );

            self.finalize_execution_report(
                tenant_id,
                &request.batch_id,
                request.execution_mode,
                "completed",
                progress_stage,
                summary_json,
                json!(execution_errors),
                &phase_reports,
            )
            .await?;
        }

        let batch = self.get_batch(tenant_id, &request.batch_id).await?;

        Ok(MixradiusImportExecutionResult {
            batch,
            summary: MixradiusImportExecutionSummary {
                batch_id: request.batch_id.clone(),
                mode: request.execution_mode,
                total_rows: preview.total_rows,
                imported_rows,
                updated_rows,
                skipped_rows,
                blocked_rows,
                conflict_rows: conflict_rows + execution_conflict_rows,
                warnings: warnings.clone(),
            },
            preview: Some(preview),
            warnings,
        })
    }

    pub async fn cancel_batch(
        &self,
        tenant_id: &str,
        batch_id: &str,
    ) -> Result<MixradiusImportBatch> {
        sqlx::query(
            r#"
            UPDATE public.mixradius_import_batches
            SET execution_status = 'cancelled',
                progress_json = jsonb_set(progress_json, '{stage}', '"cancelled"', true),
                updated_at = $3
            WHERE tenant_id = $1
              AND id = $2
              AND execution_status IN ('pending', 'running')
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .context("failed to cancel MixRadius import batch")?;

        self.get_batch(tenant_id, batch_id).await
    }

    pub async fn delete_batch(&self, tenant_id: &str, batch_id: &str) -> Result<()> {
        let batch = self.get_batch(tenant_id, batch_id).await?;

        if batch.parse_status == MixradiusImportParseStatus::Running
            || batch.execution_status == MixradiusImportBatchStatus::Running
        {
            return Err(anyhow!(
                "Batch yang sedang diproses tidak bisa dihapus. Batalkan atau tunggu sampai selesai."
            ));
        }

        let deleted = sqlx::query(
            r#"
            DELETE FROM public.mixradius_import_batches
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .execute(&self.pool)
        .await
        .context("failed to delete MixRadius import batch")?;

        if deleted.rows_affected() == 0 {
            return Err(anyhow!("MixRadius import batch tidak ditemukan"));
        }

        Ok(())
    }

    async fn legacy_transaction_count(&self, tenant_id: &str, batch_id: &str) -> Result<i64> {
        let staged_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM public.mixradius_staging_transactions
            WHERE tenant_id = $1 AND import_batch_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .fetch_one(&self.pool)
        .await
        .context("failed to count staged MixRadius transactions")?;

        if staged_count > 0 {
            return Ok(staged_count);
        }

        sqlx::query_scalar(
            r#"
            SELECT COALESCE(
                (summary_json->>'transactions')::bigint,
                (summary_json->>'legacyTransactionCount')::bigint,
                0
            )
            FROM public.mixradius_import_batches
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .fetch_one(&self.pool)
        .await
        .context("failed to load MixRadius legacy transaction count from batch summary")
    }

    async fn production_invoice_count(&self, tenant_id: &str) -> Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM public.invoices WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await
            .context("failed to count production invoices")
    }

    async fn preview_rows_for_batch(
        &self,
        tenant_id: &str,
        batch_id: &str,
        mapping_overrides: &[crate::models::MixradiusImportMappingOverride],
        customer_conflict_resolution: Option<MixradiusImportCustomerConflictResolution>,
        location_strategy: Option<MixradiusImportLocationStrategy>,
    ) -> Result<Vec<MixradiusImportPreviewRow>> {
        let nas_rows = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT source_ref, nas_name
            FROM public.mixradius_staging_nas
            WHERE tenant_id = $1 AND import_batch_id = $2
            ORDER BY created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to load MixRadius NAS preview rows")?;

        let plan_rows = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT source_ref, plan_name
            FROM public.mixradius_staging_plans
            WHERE tenant_id = $1 AND import_batch_id = $2
              AND COALESCE(source_json->'values'->>8, 'PPP') = 'PPP'
            ORDER BY created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to load MixRadius plan preview rows")?;

        let customer_rows = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT member_id, COALESCE(fullname, username, member_id)
            FROM public.mixradius_staging_customers
            WHERE tenant_id = $1 AND import_batch_id = $2
              AND COALESCE(source_json->'values'->>3, 'PPP') = 'PPP'
            ORDER BY created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to load MixRadius customer preview rows")?;

        let mut rows = Vec::new();
        for (index, (source_ref, name)) in nas_rows.into_iter().enumerate() {
            let override_target = find_mapping_override(mapping_overrides, "nas", &source_ref);
            rows.push(MixradiusImportPreviewRow {
                row_number: (index + 1) as i64,
                source_kind: "nas".into(),
                source_ref: source_ref.clone(),
                target_kind: Some("router".into()),
                target_id: override_target.map(|value| value.to_string()),
                display_name: Some(name),
                conflict_state: if override_target.is_some() {
                    MixradiusImportConflictState::AutoMatched
                } else {
                    MixradiusImportConflictState::Blocked
                },
                notes: Some(if override_target.is_some() {
                    "Router target dipilih dari mapping override admin.".into()
                } else {
                    "Router target belum dipilih di preview MixRadius.".into()
                }),
            });
        }

        let base = rows.len() as i64;
        for (index, (source_ref, name)) in plan_rows.into_iter().enumerate() {
            let override_target = find_mapping_override(mapping_overrides, "plan", &source_ref);
            rows.push(MixradiusImportPreviewRow {
                row_number: base + index as i64 + 1,
                source_kind: "plan".into(),
                source_ref: source_ref.clone(),
                target_kind: Some("package".into()),
                target_id: override_target.map(|value| value.to_string()),
                display_name: Some(name),
                conflict_state: if override_target.is_some() {
                    MixradiusImportConflictState::AutoMatched
                } else {
                    MixradiusImportConflictState::NeedsReview
                },
                notes: Some(if override_target.is_some() {
                    "Package target dipilih dari mapping override admin.".into()
                } else {
                    "Package akan di-resolve oleh mapper/executor MixRadius.".into()
                }),
            });
        }

        let base = rows.len() as i64;
        for (index, (source_ref, name)) in customer_rows.into_iter().enumerate() {
            let conflict_state = match customer_conflict_resolution {
                Some(MixradiusImportCustomerConflictResolution::Skip) => {
                    MixradiusImportConflictState::Skipped
                }
                _ => MixradiusImportConflictState::NeedsReview,
            };
            let resolution_note = match customer_conflict_resolution {
                Some(MixradiusImportCustomerConflictResolution::Merge) => {
                    "Customer akan di-merge mengikuti keputusan admin."
                }
                Some(MixradiusImportCustomerConflictResolution::CreateNew) => {
                    "Customer akan dibuat baru mengikuti keputusan admin."
                }
                Some(MixradiusImportCustomerConflictResolution::Skip) => {
                    "Customer dilewati mengikuti keputusan admin."
                }
                None => "Customer akan di-review terhadap data lokal sebelum execute.",
            };
            let location_note = match location_strategy {
                Some(MixradiusImportLocationStrategy::Preserve) => " Strategi lokasi: preserve.",
                Some(MixradiusImportLocationStrategy::Merge) => " Strategi lokasi: merge.",
                Some(MixradiusImportLocationStrategy::Replace) => " Strategi lokasi: replace.",
                None => "",
            };
            rows.push(MixradiusImportPreviewRow {
                row_number: base + index as i64 + 1,
                source_kind: "customer".into(),
                source_ref,
                target_kind: Some("customer".into()),
                target_id: None,
                display_name: Some(name),
                conflict_state,
                notes: Some(format!("{resolution_note}{location_note}")),
            });
        }

        Ok(rows)
    }

    async fn persist_progress_payload(
        &self,
        tenant_id: &str,
        batch_id: &str,
        key: &str,
        payload: Value,
    ) -> Result<()> {
        let mut batch = self.get_batch(tenant_id, batch_id).await?;
        let progress = batch
            .progress_json
            .as_object_mut()
            .ok_or_else(|| anyhow!("MixRadius batch progress_json must be an object"))?;
        progress.insert(key.to_string(), payload);

        sqlx::query(
            r#"
            UPDATE public.mixradius_import_batches
            SET progress_json = $3,
                updated_at = $4
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .bind(batch.progress_json)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .context("failed to persist MixRadius progress payload")?;

        Ok(())
    }

    async fn finalize_execution_report(
        &self,
        tenant_id: &str,
        batch_id: &str,
        execution_mode: crate::models::MixradiusImportExecutionMode,
        execution_status: &str,
        progress_stage: &str,
        summary_json: Value,
        error_json: Value,
        phase_reports: &serde_json::Map<String, Value>,
    ) -> Result<()> {
        let progress_patch = json!({
            "stage": progress_stage,
            "phaseReports": phase_reports,
        });

        sqlx::query(
            r#"
            UPDATE public.mixradius_import_batches
            SET execution_status = $3,
                execution_mode = $4,
                completed_at = $5,
                summary_json = $6,
                error_json = $7,
                progress_json = progress_json || $8,
                updated_at = $5
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(batch_id)
        .bind(execution_status)
        .bind(execution_mode)
        .bind(Utc::now())
        .bind(summary_json)
        .bind(error_json)
        .bind(progress_patch)
        .execute(&self.pool)
        .await
        .context("failed to persist MixRadius execution report")?;

        Ok(())
    }

    async fn stage_parsed_backup(
        &self,
        batch_id: &str,
        tenant_id: &str,
        parsed: MixradiusParsedBackup,
    ) -> Result<Value> {
        let now = Utc::now();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start MixRadius staging transaction")?;

        let customer_id_to_member_id: HashMap<String, String> = parsed
            .customer_rows
            .iter()
            .map(|row| (value(row, 0), value(row, 1)))
            .collect();

        for rows in parsed.nas_rows.chunks(MIXRADIUS_STAGE_INSERT_BATCH_SIZE) {
            let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
                r#"
                INSERT INTO public.mixradius_staging_nas (
                    id, tenant_id, import_batch_id, source_ref,
                    nas_name, nas_ip_or_cidr, shortname, source_json, created_at, updated_at
                )
                "#,
            );
            qb.push_values(rows, |mut b, row| {
                b.push_bind(Uuid::new_v4().to_string())
                    .push_bind(tenant_id)
                    .push_bind(batch_id)
                    .push_bind(source_ref(row, 0))
                    .push_bind(value(row, 2))
                    .push_bind(value(row, 1))
                    .push_bind(optional_value(row, 2))
                    .push_bind(source_json(row))
                    .push_bind(now)
                    .push_bind(now);
            });
            qb.build()
                .execute(&mut *tx)
                .await
                .context("failed to insert MixRadius NAS staging rows")?;
        }

        for rows in parsed.plan_rows.chunks(MIXRADIUS_STAGE_INSERT_BATCH_SIZE) {
            let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
                r#"
                INSERT INTO public.mixradius_staging_plans (
                    id, tenant_id, import_batch_id, source_ref,
                    plan_name, bandwidth_name, price, validity, shared_users, source_json, created_at, updated_at
                )
                "#,
            );
            qb.push_values(rows, |mut b, row| {
                b.push_bind(Uuid::new_v4().to_string())
                    .push_bind(tenant_id)
                    .push_bind(batch_id)
                    .push_bind(source_ref(row, 0))
                    .push_bind(value(row, 1))
                    .push_bind(optional_value(row, 2))
                    .push_bind(parse_decimal(row, 4))
                    .push_bind(optional_validity(row, 15, 16))
                    .push_bind(parse_i32(row, 18))
                    .push_bind(source_json(row))
                    .push_bind(now)
                    .push_bind(now);
            });
            qb.build()
                .execute(&mut *tx)
                .await
                .context("failed to insert MixRadius plan staging rows")?;
        }

        for rows in parsed
            .customer_rows
            .chunks(MIXRADIUS_STAGE_INSERT_BATCH_SIZE)
        {
            let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
                r#"
                INSERT INTO public.mixradius_staging_customers (
                    id, tenant_id, import_batch_id, source_ref, member_id, username, password, fullname,
                    email, phonenumber, identity_number, address, plan_name, price, total, renewed_on,
                    expired_on, trx_invoice, trx_status, payment_type, auth_status, bind_mac, mac_address,
                    source_json, created_at, updated_at
                )
                "#,
            );
            qb.push_values(rows, |mut b, row| {
                b.push_bind(Uuid::new_v4().to_string())
                    .push_bind(tenant_id)
                    .push_bind(batch_id)
                    .push_bind(source_ref(row, 0))
                    .push_bind(value(row, 1))
                    .push_bind(optional_value(row, 8))
                    .push_bind(optional_value(row, 9))
                    .push_bind(optional_value(row, 10))
                    .push_bind(optional_value(row, 11))
                    .push_bind(optional_value(row, 13))
                    .push_bind(optional_value(row, 12))
                    .push_bind(optional_value(row, 14))
                    .push_bind(optional_value(row, 16))
                    .push_bind(parse_decimal(row, 17))
                    .push_bind(parse_decimal(row, 22))
                    .push_bind(parse_datetime(row, 23))
                    .push_bind(parse_datetime(row, 24))
                    .push_bind(optional_value(row, 33))
                    .push_bind(optional_value(row, 35))
                    .push_bind(optional_value(row, 34))
                    .push_bind(optional_value(row, 37))
                    .push_bind(optional_value(row, 38))
                    .push_bind(optional_value(row, 39))
                    .push_bind(source_json(row))
                    .push_bind(now)
                    .push_bind(now);
            });
            qb.build()
                .execute(&mut *tx)
                .await
                .context("failed to insert MixRadius customer staging rows")?;
        }

        let mut prepared_customer_locations: Vec<(
            String,
            String,
            Option<f64>,
            Option<f64>,
            Value,
        )> = Vec::with_capacity(parsed.customer_location_rows.len());
        for row in &parsed.customer_location_rows {
            let source_customer_id = value(row, 1);
            let member_id = customer_id_to_member_id
                .get(&source_customer_id)
                .cloned()
                .ok_or_else(|| {
                    anyhow!(
                        "MixRadius customer map row {} references missing customer_id {}",
                        source_ref(row, 0),
                        source_customer_id
                    )
                })?;
            prepared_customer_locations.push((
                source_ref(row, 0),
                member_id,
                parse_decimal(row, 2),
                parse_decimal(row, 3),
                source_json(row),
            ));
        }
        for rows in prepared_customer_locations.chunks(MIXRADIUS_STAGE_INSERT_BATCH_SIZE) {
            let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
                r#"
                INSERT INTO public.mixradius_staging_customer_locations (
                    id, tenant_id, import_batch_id, source_ref, member_id, latitude, longitude, source_json, created_at, updated_at
                )
                "#,
            );
            qb.push_values(rows, |mut b, row| {
                b.push_bind(Uuid::new_v4().to_string())
                    .push_bind(tenant_id)
                    .push_bind(batch_id)
                    .push_bind(&row.0)
                    .push_bind(&row.1)
                    .push_bind(row.2)
                    .push_bind(row.3)
                    .push_bind(&row.4)
                    .push_bind(now)
                    .push_bind(now);
            });
            qb.build()
                .execute(&mut *tx)
                .await
                .context("failed to insert MixRadius location staging rows")?;
        }

        for rows in parsed
            .transaction_rows
            .chunks(MIXRADIUS_STAGE_INSERT_BATCH_SIZE)
        {
            let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
                r#"
                INSERT INTO public.mixradius_staging_transactions (
                    id, tenant_id, import_batch_id, source_ref, invoice_no, member_id, username,
                    transaction_status, payment_type, amount, paid_at, source_json, created_at, updated_at
                )
                "#,
            );
            qb.push_values(rows, |mut b, row| {
                b.push_bind(Uuid::new_v4().to_string())
                    .push_bind(tenant_id)
                    .push_bind(batch_id)
                    .push_bind(source_ref(row, 0))
                    .push_bind(optional_value(row, 1))
                    .push_bind(optional_value(row, 3))
                    .push_bind(optional_value(row, 4))
                    .push_bind(optional_value(row, 18))
                    .push_bind(optional_value(row, 17))
                    .push_bind(parse_decimal(row, 15))
                    .push_bind(parse_datetime(row, 20))
                    .push_bind(source_json(row))
                    .push_bind(now)
                    .push_bind(now);
            });
            qb.build()
                .execute(&mut *tx)
                .await
                .context("failed to insert MixRadius transaction staging rows")?;
        }

        for rows in parsed.usage_rows.chunks(MIXRADIUS_STAGE_INSERT_BATCH_SIZE) {
            let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
                r#"
                INSERT INTO public.mixradius_staging_usage (
                    id, tenant_id, import_batch_id, source_ref, member_id, username, usage_date,
                    session_count, download_bytes, upload_bytes, source_json, created_at, updated_at
                )
                "#,
            );
            qb.push_values(rows, |mut b, row| {
                b.push_bind(Uuid::new_v4().to_string())
                    .push_bind(tenant_id)
                    .push_bind(batch_id)
                    .push_bind(source_ref(row, 0))
                    .push_bind(optional_value(row, 2))
                    .push_bind(optional_value(row, 1))
                    .push_bind(parse_date_from_datetime(row, 3))
                    .push_bind(Some(1_i32))
                    .push_bind(None::<i64>)
                    .push_bind(None::<i64>)
                    .push_bind(source_json(row))
                    .push_bind(now)
                    .push_bind(now);
            });
            qb.build()
                .execute(&mut *tx)
                .await
                .context("failed to insert MixRadius usage staging rows")?;
        }

        let summary = json!({
            "customersTotal": parsed.summary.customers_total_count,
            "customersPpp": parsed.summary.customers_ppp_count,
            "plansPpp": parsed.summary.plans_ppp_count,
            "nas": parsed.summary.nas_count,
            "transactions": parsed.summary.transactions_count,
            "radacct": parsed.summary.radacct_count,
            "customerLocations": parsed.customer_location_rows.len(),
            "usageRows": parsed.usage_rows.len()
        });

        sqlx::query(
            r#"
            UPDATE public.mixradius_import_batches
            SET parse_status = 'ready',
                summary_json = $2,
                progress_json = $3,
                updated_at = $4
            WHERE id = $1
            "#,
        )
        .bind(batch_id)
        .bind(summary.clone())
        .bind(json!({"stage": "staged", "completed": true}))
        .bind(Utc::now())
        .execute(&mut *tx)
        .await
        .context("failed to finalize MixRadius import batch")?;

        tx.commit()
            .await
            .context("failed to commit MixRadius staging rows")?;

        Ok(summary)
    }
}

fn source_ref(row: &MixradiusSourceRow, idx: usize) -> String {
    row.values
        .get(idx)
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

fn value(row: &MixradiusSourceRow, idx: usize) -> String {
    row.values.get(idx).cloned().unwrap_or_default()
}

fn optional_value(row: &MixradiusSourceRow, idx: usize) -> Option<String> {
    row.values
        .get(idx)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_decimal(row: &MixradiusSourceRow, idx: usize) -> Option<f64> {
    optional_value(row, idx).and_then(|value| value.parse::<f64>().ok())
}

fn parse_i32(row: &MixradiusSourceRow, idx: usize) -> Option<i32> {
    optional_value(row, idx).and_then(|value| value.parse::<i32>().ok())
}

fn parse_datetime(row: &MixradiusSourceRow, idx: usize) -> Option<DateTime<Utc>> {
    optional_value(row, idx).and_then(|value| {
        NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
    })
}

fn parse_date_from_datetime(row: &MixradiusSourceRow, idx: usize) -> Option<chrono::NaiveDate> {
    parse_datetime(row, idx).map(|dt| dt.date_naive())
}

fn optional_validity(
    row: &MixradiusSourceRow,
    value_idx: usize,
    unit_idx: usize,
) -> Option<String> {
    let value = optional_value(row, value_idx)?;
    let unit = optional_value(row, unit_idx)?;
    Some(format!("{value} {unit}"))
}

fn source_json(row: &MixradiusSourceRow) -> Value {
    json!({ "values": row.values })
}

fn build_execution_report_json(
    request: &MixradiusImportExecuteRequest,
    total_rows: i64,
    imported_rows: i64,
    updated_rows: i64,
    skipped_rows: i64,
    blocked_rows: i64,
    conflict_rows: i64,
    warnings: &[String],
    phase_reports: &serde_json::Map<String, Value>,
    errors: &[Value],
    legacy_transaction_count: i64,
    production_invoice_count: i64,
) -> Value {
    json!({
        "batchId": request.batch_id,
        "mode": request.execution_mode,
        "totalRows": total_rows,
        "importedRows": imported_rows,
        "updatedRows": updated_rows,
        "skippedRows": skipped_rows,
        "blockedRows": blocked_rows,
        "conflictRows": conflict_rows,
        "warnings": warnings,
        "phaseReports": phase_reports,
        "errors": errors,
        "legacyTransactionCount": legacy_transaction_count,
        "productionInvoiceCount": production_invoice_count,
    })
}

fn find_mapping_override<'a>(
    overrides: &'a [crate::models::MixradiusImportMappingOverride],
    source_kind: &str,
    source_value: &str,
) -> Option<&'a str> {
    overrides
        .iter()
        .find(|item| item.source_kind == source_kind && item.source_value == source_value)
        .map(|item| item.target_value.as_str())
}

#[cfg(all(test, feature = "postgres"))]
mod tests;
