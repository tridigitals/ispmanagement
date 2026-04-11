use crate::db::DbPool;
use crate::models::MixradiusImportBatch;
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

        if let Err(error) = self
            .stage_parsed_backup(&batch_id, tenant_id, parsed)
            .await
        {
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

fn optional_validity(row: &MixradiusSourceRow, value_idx: usize, unit_idx: usize) -> Option<String> {
    let value = optional_value(row, value_idx)?;
    let unit = optional_value(row, unit_idx)?;
    Some(format!("{value} {unit}"))
}

fn source_json(row: &MixradiusSourceRow) -> Value {
    json!({ "values": row.values })
}

#[cfg(all(test, feature = "postgres"))]
mod tests;
