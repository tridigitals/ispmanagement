//! Enforcement limit resource per-plan.
//!
//! Sebelum ini hanya `max_users`, `max_storage_gb`, dan beberapa fitur boolean
//! yang benar-benar ditegakkan. Untuk sebuah SaaS ISP, satuan yang menentukan
//! skala justru **jumlah pelanggan, router, dan OLT** — dan ketiganya tidak
//! dibatasi sama sekali, sehingga tenant berpelanggan 10 dan 10.000 membayar
//! harga yang sama.
//!
//! `PlanService` sudah punya `get_feature_limit`, tapi service pemilik tabel
//! (`CustomerService`, `MikrotikService`, `OltService`) tidak memegang
//! `PlanService`. Menyuntikkan PlanService ke mereka akan menyentuh belasan
//! titik konstruksi (bootstrap, server, test). Helper ini menyelesaikan hal
//! yang sama hanya dengan `DbPool`, jadi blast radius-nya kecil.
//!
//! Batas diambil dari katalog yang sama dengan seed (`plan_catalog::FEATURES`),
//! dengan urutan: override subscription tenant → nilai plan → default fitur.

use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::services::plan_catalog::FEATURES;
use crate::services::sql_ident::{assert_sql_ident, is_safe_sql_ident};

/// Nama tabel yang boleh dihitung. Whitelist ini penting karena nama tabel
/// tidak bisa di-bind sebagai parameter SQL, jadi ia disisipkan ke query.
/// Hanya fungsi di file ini yang boleh memanggil dengan nilai literal.
const ALLOWED_TABLES: &[&str] = &[
    "customers",
    "mikrotik_routers",
    "olts",
    "pppoe_accounts",
    "tenant_members",
];

fn is_allowed_table(table: &str) -> bool {
    is_safe_sql_ident(table) && ALLOWED_TABLES.contains(&table)
}

/// Parser nilai limit — **harus sama persis** dengan `PlanService::parse_limit`.
///
/// Dijaga oleh test `parse_matches_plan_service_semantics` supaya dua pembaca
/// tidak pernah menyimpang lagi (bug aslinya: `''` dan `"0.5"` gagal
/// `parse::<i64>()` lalu diam-diam dianggap "tanpa batas").
pub fn parse_limit(value: &str) -> Option<i64> {
    let v = value.trim();
    if v.eq_ignore_ascii_case("unlimited") || v == "-1" {
        return None;
    }
    let parsed = v.parse::<f64>().ok().filter(|n| n.is_finite());
    Some(parsed.map(|n| n.ceil().max(0.0) as i64).unwrap_or(0))
}

/// Nilai efektif sebuah fitur untuk tenant, tanpa perlu instance `PlanService`.
///
/// Urutan resolusi meniru `PlanService::check_feature_access_with_conn`:
/// 1. `feature_overrides` pada subscription tenant (JSON), bila ada
/// 2. nilai `plan_features` milik plan tenant
/// 3. `default_value` fitur — dan bila fitur tidak dikenal, `"0"` (paling
///    ketat), bukan "tanpa batas"
pub async fn effective_feature_value(pool: &DbPool, tenant_id: &str, code: &str) -> AppResult<String> {
    // 1. Override subscription (kolom JSON berbentuk map code -> value)
    // `feature_overrides` bertipe JSONB, jadi harus di-cast ke TEXT — bind ke
    // `Option<String>` tanpa cast akan gagal decode (`mismatched types`).
    // Pola cast ini sama dengan `plan_service.rs`.
    #[cfg(feature = "postgres")]
    let overrides: Option<String> = sqlx::query_scalar(
        "SELECT feature_overrides::TEXT FROM tenant_subscriptions WHERE tenant_id = $1 LIMIT 1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;
    // Di SQLite kolomnya sudah TEXT, jadi tidak perlu cast.
    #[cfg(feature = "sqlite")]
    let overrides: Option<String> = sqlx::query_scalar(
        "SELECT feature_overrides FROM tenant_subscriptions WHERE tenant_id = ? LIMIT 1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;

    if let Some(ref raw) = overrides {
        if let Ok(map) = serde_json::from_str::<serde_json::Value>(raw) {
            if let Some(v) = map.get(code) {
                return Ok(v
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| v.to_string()));
            }
        }
    }

    // 2. Nilai plan
    #[cfg(feature = "postgres")]
    let plan_value: Option<String> = sqlx::query_scalar(
        r#"
        SELECT pf.value
        FROM tenant_subscriptions s
        JOIN plan_features pf ON pf.plan_id = s.plan_id
        JOIN features f ON f.id = pf.feature_id
        WHERE s.tenant_id = $1 AND f.code = $2
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(code)
    .fetch_optional(pool)
    .await?;
    #[cfg(feature = "sqlite")]
    let plan_value: Option<String> = sqlx::query_scalar(
        r#"
        SELECT pf.value
        FROM tenant_subscriptions s
        JOIN plan_features pf ON pf.plan_id = s.plan_id
        JOIN features f ON f.id = pf.feature_id
        WHERE s.tenant_id = ?1 AND f.code = ?2
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(code)
    .fetch_optional(pool)
    .await?;

    if let Some(v) = plan_value {
        if !v.trim().is_empty() {
            return Ok(v);
        }
    }

    // 3. Default fitur dari katalog, atau "0" bila fitur tidak dikenal.
    Ok(FEATURES
        .iter()
        .find(|f| f.code == code)
        .map(|f| f.default_value.to_string())
        .unwrap_or_else(|| "0".to_string()))
}

/// Batas efektif untuk tenant: `None` = tanpa batas.
pub async fn effective_limit(pool: &DbPool, tenant_id: &str, code: &str) -> AppResult<Option<i64>> {
    let value = effective_feature_value(pool, tenant_id, code).await?;
    Ok(parse_limit(&value))
}

/// Jumlah baris saat ini untuk tenant.
pub async fn current_resource_count(pool: &DbPool, table: &str, tenant_id: &str) -> AppResult<i64> {
    if !is_allowed_table(table) {
        return Err(AppError::Validation(format!(
            "Rejected table for plan-limit count: {table:?}"
        )));
    }
    if let Err(e) = assert_sql_ident(table) {
        return Err(e);
    }

    #[cfg(feature = "postgres")]
    let query = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = $1");
    #[cfg(feature = "sqlite")]
    let query = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?1");

    let count: i64 = sqlx::query_scalar(&query)
        .bind(tenant_id)
        .fetch_one(pool)
        .await?;
    Ok(count)
}

/// Tegakkan batas sebelum menambah resource.
///
/// `label` dipakai untuk pesan error yang bisa dibaca user (mis. "pelanggan").
/// Bila tenant sudah **di atas** batas (terjadi setelah downgrade plan), pesan
/// membedakan "sudah penuh" dari "sudah melebihi" supaya admin tahu harus
/// upgrade, bukan sekadar menghapus data.
pub async fn enforce(
    pool: &DbPool,
    tenant_id: &str,
    table: &str,
    code: &str,
    label: &str,
) -> AppResult<()> {
    let Some(max) = effective_limit(pool, tenant_id, code).await? else {
        return Ok(()); // tanpa batas
    };
    let count = current_resource_count(pool, table, tenant_id).await?;

    if count >= max {
        return Err(AppError::Validation(format!(
            "Batas plan tercapai: maksimal {max} {label}. \
             Saat ini {count}. Upgrade plan untuk menambah {label} lagi."
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_allowed_table, parse_limit};

    /// Parser ini harus identik dengan `PlanService::parse_limit`. Kalau salah
    /// satu diubah, test ini menahan supaya dua pembaca tidak menyimpang lagi.
    #[test]
    fn parse_matches_plan_service_semantics() {
        assert_eq!(parse_limit("unlimited"), None);
        assert_eq!(parse_limit("UNLIMITED"), None);
        assert_eq!(parse_limit(" -1 "), None);
        assert_eq!(parse_limit("0"), Some(0));
        assert_eq!(parse_limit("25"), Some(25));
        assert_eq!(parse_limit("0.5"), Some(1));
        assert_eq!(parse_limit(""), Some(0));
        assert_eq!(parse_limit("   "), Some(0));
        assert_eq!(parse_limit("abc"), Some(0));
        assert_eq!(parse_limit("-9"), Some(0));
    }

    /// Regresi: `feature_overrides` bertipe JSONB. Bind langsung ke
    /// `Option<String>` gagal decode (`mismatched types ... TEXT vs JSONB`) dan
    /// seluruh endpoint balas 500. Wajib di-cast ke TEXT — ini penyebab nyata
    /// 500 pada `/api/plans/subscriptions/details`.
    #[test]
    fn override_jsonb_selalu_di_cast_ke_text() {
        let source = include_str!("resource_limit.rs");
        let prod = source.split("#[cfg(test)]").next().expect("bagian produksi");
        // Bentuk Postgres ($1) WAJIB pakai cast ::TEXT.
        assert!(
            prod.contains("feature_overrides::TEXT"),
            "query feature_overrides wajib pakai cast ::TEXT"
        );
        assert!(
            !prod.contains("SELECT feature_overrides FROM tenant_subscriptions WHERE tenant_id = $1"),
            "bind JSONB langsung ke String akan gagal decode (mismatched types TEXT vs JSONB)"
        );
    }

    /// Nama tabel disisipkan ke SQL, jadi whitelist wajib ketat.
    #[test]
    fn hanya_tabel_whitelist_dan_identifier_aman() {
        for ok in ["customers", "mikrotik_routers", "olts", "pppoe_accounts", "tenant_members"] {
            assert!(is_allowed_table(ok), "{ok} seharusnya diizinkan");
        }
        for bad in [
            "users; DROP TABLE plans",
            "users--",
            "tenant_members;",
            "\"users\"",
            "",
            "pg_shadow",
            "users",
            "plan_features",
        ] {
            assert!(!is_allowed_table(bad), "{bad:?} seharusnya DITOLAK");
        }
    }
}
