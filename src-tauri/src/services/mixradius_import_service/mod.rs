use crate::db::DbPool;
use crate::models::{
    MixradiusImportBatch, MixradiusImportConflictState,
    MixradiusImportCustomerConflictResolution, MixradiusImportExecuteRequest,
    MixradiusImportExecutionResult, MixradiusImportExecutionSummary, MixradiusImportLocationStrategy,
    MixradiusImportParseStatus, MixradiusImportPreview, MixradiusImportPreviewRequest,
    MixradiusImportPreviewRow, PaginatedResponse,
};
use crate::services::mixradius_sql_parser::{
    parse_mixradius_backup, MixradiusParsedBackup, MixradiusSourceRow,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;

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
        let batch = self.get_batch(tenant_id, &request.batch_id).await?;
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
        };
        let preview = self.build_preview(tenant_id, &preview_request).await?;

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
        if request.execution_mode != crate::models::MixradiusImportExecutionMode::PreviewOnly {
            warnings.push(
                "Execution pipeline MixRadius belum diaktifkan penuh; hasil ini masih preview-only."
                    .to_string(),
            );
        }

        Ok(MixradiusImportExecutionResult {
            batch,
            summary: MixradiusImportExecutionSummary {
                batch_id: request.batch_id.clone(),
                mode: request.execution_mode,
                total_rows: preview.total_rows,
                imported_rows: 0,
                updated_rows: 0,
                skipped_rows,
                blocked_rows,
                conflict_rows,
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
            ORDER BY created_at ASC
            LIMIT 100
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
                Some(MixradiusImportLocationStrategy::Preserve) => {
                    " Strategi lokasi: preserve."
                }
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

        for row in &parsed.nas_rows {
            sqlx::query(
                r#"
                INSERT INTO public.mixradius_staging_nas (
                    id, tenant_id, import_batch_id, source_ref,
                    nas_name, nas_ip_or_cidr, shortname, source_json, created_at, updated_at
                )
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(batch_id)
            .bind(source_ref(row, 0))
            .bind(value(row, 2))
            .bind(value(row, 1))
            .bind(optional_value(row, 2))
            .bind(source_json(row))
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .context("failed to insert MixRadius NAS staging row")?;
        }

        for row in &parsed.plan_rows {
            sqlx::query(
                r#"
                INSERT INTO public.mixradius_staging_plans (
                    id, tenant_id, import_batch_id, source_ref,
                    plan_name, bandwidth_name, price, validity, shared_users, source_json, created_at, updated_at
                )
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(batch_id)
            .bind(source_ref(row, 0))
            .bind(value(row, 1))
            .bind(optional_value(row, 2))
            .bind(parse_decimal(row, 4))
            .bind(optional_validity(row, 15, 16))
            .bind(parse_i32(row, 18))
            .bind(source_json(row))
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .context("failed to insert MixRadius plan staging row")?;
        }

        for row in &parsed.customer_rows {
            sqlx::query(
                r#"
                INSERT INTO public.mixradius_staging_customers (
                    id, tenant_id, import_batch_id, source_ref, member_id, username, password, fullname,
                    email, phonenumber, identity_number, address, plan_name, price, total, renewed_on,
                    expired_on, trx_invoice, trx_status, payment_type, auth_status, bind_mac, mac_address,
                    source_json, created_at, updated_at
                )
                VALUES (
                    $1,$2,$3,$4,$5,$6,$7,$8,
                    $9,$10,$11,$12,$13,$14,$15,$16,
                    $17,$18,$19,$20,$21,$22,$23,
                    $24,$25,$26
                )
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(batch_id)
            .bind(source_ref(row, 0))
            .bind(value(row, 1))
            .bind(optional_value(row, 8))
            .bind(optional_value(row, 9))
            .bind(optional_value(row, 10))
            .bind(optional_value(row, 11))
            .bind(optional_value(row, 13))
            .bind(optional_value(row, 12))
            .bind(optional_value(row, 14))
            .bind(optional_value(row, 16))
            .bind(parse_decimal(row, 17))
            .bind(parse_decimal(row, 22))
            .bind(parse_datetime(row, 23))
            .bind(parse_datetime(row, 24))
            .bind(optional_value(row, 33))
            .bind(optional_value(row, 35))
            .bind(optional_value(row, 34))
            .bind(optional_value(row, 37))
            .bind(optional_value(row, 38))
            .bind(optional_value(row, 39))
            .bind(source_json(row))
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .context("failed to insert MixRadius customer staging row")?;
        }

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

            sqlx::query(
                r#"
                INSERT INTO public.mixradius_staging_customer_locations (
                    id, tenant_id, import_batch_id, source_ref, member_id, latitude, longitude, source_json, created_at, updated_at
                )
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(batch_id)
            .bind(source_ref(row, 0))
            .bind(member_id)
            .bind(parse_decimal(row, 2))
            .bind(parse_decimal(row, 3))
            .bind(source_json(row))
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .context("failed to insert MixRadius location staging row")?;
        }

        for row in &parsed.transaction_rows {
            sqlx::query(
                r#"
                INSERT INTO public.mixradius_staging_transactions (
                    id, tenant_id, import_batch_id, source_ref, invoice_no, member_id, username,
                    transaction_status, payment_type, amount, paid_at, source_json, created_at, updated_at
                )
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(batch_id)
            .bind(source_ref(row, 0))
            .bind(optional_value(row, 1))
            .bind(optional_value(row, 3))
            .bind(optional_value(row, 4))
            .bind(optional_value(row, 18))
            .bind(optional_value(row, 17))
            .bind(parse_decimal(row, 15))
            .bind(parse_datetime(row, 20))
            .bind(source_json(row))
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .context("failed to insert MixRadius transaction staging row")?;
        }

        for row in &parsed.usage_rows {
            sqlx::query(
                r#"
                INSERT INTO public.mixradius_staging_usage (
                    id, tenant_id, import_batch_id, source_ref, member_id, username, usage_date,
                    session_count, download_bytes, upload_bytes, source_json, created_at, updated_at
                )
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(tenant_id)
            .bind(batch_id)
            .bind(source_ref(row, 0))
            .bind(optional_value(row, 2))
            .bind(optional_value(row, 1))
            .bind(parse_date_from_datetime(row, 3))
            .bind(Some(1_i32))
            .bind(None::<i64>)
            .bind(None::<i64>)
            .bind(source_json(row))
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .context("failed to insert MixRadius usage staging row")?;
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
