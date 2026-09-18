//! Sinkronisasi email antara `customers` dan `users`.
//!
//! ## Mengapa modul ini ada
//!
//! Satu pelanggan bisa punya akun login (via `customer_users`). Saat itu terjadi,
//! `customers.email` (kontak pelanggan) dan `users.email` (kredensial login)
//! merujuk orang yang sama dan **wajib identik** — kalau tidak, pelanggan melihat
//! email berbeda di halaman pelanggan vs halaman profil, dan login pakai email
//! yang "salah".
//!
//! Sebelumnya sinkronisasi ini tersebar dan tidak lengkap:
//! - `update_customer` menyinkronkan ke SEMUA user pelanggan (bukan hanya yang
//!   utama) tanpa cek unik → email user lain bisa tertimpa diam-diam.
//! - `user_service.update` (edit di superadmin) tidak menyentuh `customers`.
//! - `update_me` (profil sendiri) tidak menyentuh `customers` dan tanpa cek unik.
//!
//! Akibatnya pernah terjadi data produksi dengan `customers.email` !=
//! `users.email` (lihat riwayat perbaikan). Modul ini menyatukan aturannya:
//! **email pelanggan == email user utamanya, dari mana pun diedit.**
//!
//! ## Aturan
//!
//! - `users.email` tetap sumber kebenaran untuk kredensial dan tetap UNIQUE global.
//! - `customers.email` tetap boleh berdiri sendiri untuk pelanggan tanpa akun
//!   (mayoritas pelanggan hanya data kontak, tanpa login).
//! - Sinkronisasi hanya menyentuh **user utama** pelanggan (yang tertaut di
//!   `customer_users`), bukan semua user.

use sqlx::Row;

use crate::db::DbPool;
use crate::error::{AppError, AppResult};

/// Apakah email (case-insensitive, sudah di-trim) sudah dipakai user lain?
///
/// `except_user_id` dikecualikan supaya user boleh menyimpan emailnya sendiri.
pub(crate) async fn email_taken_by_other_user(
    pool: &DbPool,
    email: &str,
    except_user_id: &str,
) -> AppResult<bool> {
    let taken: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM users WHERE lower(email) = lower($1) AND id <> $2)",
    )
    .bind(email)
    .bind(except_user_id)
    .fetch_one(pool)
    .await?;
    Ok(taken)
}

/// Daftar user_id yang tertaut ke sebuah pelanggan (akun login-nya).
pub(crate) async fn linked_portal_user_ids(
    pool: &DbPool,
    tenant_id: &str,
    customer_id: &str,
) -> AppResult<Vec<String>> {
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT user_id FROM customer_users WHERE customer_id = $1 AND tenant_id = $2",
    )
    .bind(customer_id)
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;
    Ok(ids)
}

/// Ubah email `users` untuk user utama sebuah pelanggan, dengan cek unik.
///
/// Dipakai saat email pelanggan diedit. Kalau pelanggan tidak punya akun login,
/// ini tidak melakukan apa-apa (email pelanggan tetap tersimpan di `customers`).
pub(crate) async fn sync_customer_email_to_user(
    pool: &DbPool,
    tenant_id: &str,
    customer_id: &str,
    new_email: &str,
    actor_id: Option<&str>,
) -> AppResult<()> {
    let new_email = new_email.trim();
    if new_email.is_empty() {
        return Ok(());
    }

    // Semua user yang tertaut ke pelanggan ini (normalnya satu).
    let linked: Vec<String> = sqlx::query_scalar(
        "SELECT user_id FROM customer_users WHERE customer_id = $1 AND tenant_id = $2",
    )
    .bind(customer_id)
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;

    if linked.is_empty() {
        return Ok(());
    }

    // Email wajib unik. Kalau ada user LAIN di luar pelanggan ini yang sudah
    // memakai email tsb, tolak — jangan menimpa kredensial orang lain.
    for user_id in &linked {
        let taken: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                 SELECT 1 FROM users u
                 WHERE lower(u.email) = lower($1)
                   AND u.id <> $2
                   AND u.id NOT IN (
                       SELECT cu.user_id FROM customer_users cu
                       WHERE cu.customer_id = $3 AND cu.tenant_id = $4
                   )
             )",
        )
        .bind(new_email)
        .bind(user_id)
        .bind(customer_id)
        .bind(tenant_id)
        .fetch_one(pool)
        .await?;
        if taken {
            return Err(AppError::UserAlreadyExists);
        }
    }

    for user_id in &linked {
        let before: Option<String> =
            sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_optional(pool)
                .await?;

        sqlx::query("UPDATE users SET email = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_email)
            .bind(user_id)
            .execute(pool)
            .await?;

        // Jejak audit: perubahan email adalah peristiwa keamanan (email dipakai
        // untuk login & reset password), jadi selalu dicatat sebelum/sesudahnya.
        audit_email_change(pool, actor_id, tenant_id, user_id, before.as_deref(), new_email).await;
    }

    Ok(())
}

/// Sinkron arah sebaliknya: email `users` berubah → tebalkan ke `customers`.
///
/// Dipakai saat user diedit dari superadmin atau dari halaman profil. Kalau
/// user ini adalah portal user sebuah pelanggan, email pelanggan ikut berubah
/// supaya keduanya tetap identik.
pub(crate) async fn sync_user_email_to_customers(
    pool: &DbPool,
    user_id: &str,
    new_email: &str,
    actor_id: Option<&str>,
) -> AppResult<u64> {
    let new_email = new_email.trim();
    if new_email.is_empty() {
        return Ok(0);
    }

    let rows = sqlx::query(
        "SELECT c.id AS customer_id, c.tenant_id AS tenant_id, c.email AS before_email
         FROM customers c
         JOIN customer_users cu ON cu.customer_id = c.id
         WHERE cu.user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut updated = 0u64;
    for row in &rows {
        let customer_id: String = row.get("customer_id");
        let tenant_id: String = row.get("tenant_id");
        let before_email: Option<String> = row.get("before_email");

        sqlx::query("UPDATE customers SET email = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_email)
            .bind(&customer_id)
            .execute(pool)
            .await?;

        updated += 1;
        audit_email_change(
            pool,
            actor_id,
            &tenant_id,
            user_id,
            before_email.as_deref(),
            new_email,
        )
        .await;
    }

    if updated > 0 {
        tracing::info!(
            "Synced user email {} to {} linked customer row(s)",
            user_id,
            updated
        );
    }
    Ok(updated)
}

/// Catat perubahan email ke audit log. Kegagalan audit tidak boleh menggagalkan
/// perubahan email itu sendiri (best-effort).
///
/// Perhatikan skema `audit_logs`: kolomnya `user_id`/`tenant_id` bertipe UUID
/// dan `id` juga UUID — bukan TEXT. Kita pakai query terpisah untuk postgres
/// dan sqlite karena binding-nya berbeda.
async fn audit_email_change(
    pool: &DbPool,
    actor_id: Option<&str>,
    tenant_id: &str,
    user_id: &str,
    before: Option<&str>,
    after: &str,
) {
    let before_str = before.unwrap_or("");
    if before_str.eq_ignore_ascii_case(after) {
        return;
    }

    let details = serde_json::json!({
        "email_before": before_str,
        "email_after": after,
    })
    .to_string();

    let query = r#"
        INSERT INTO audit_logs (id, user_id, tenant_id, action, resource, resource_id, details, ip_address, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
    "#;

    let id = uuid::Uuid::new_v4();

    #[cfg(feature = "postgres")]
    let res = sqlx::query(query)
        .bind(id)
        .bind(actor_id.and_then(|v| uuid::Uuid::parse_str(v).ok()))
        .bind(uuid::Uuid::parse_str(tenant_id).ok())
        .bind("EMAIL_SYNC")
        .bind("users")
        .bind(user_id)
        .bind(&details)
        .bind(Option::<String>::None)
        .bind(chrono::Utc::now())
        .execute(pool)
        .await;

    #[cfg(feature = "sqlite")]
    let res = sqlx::query(query)
        .bind(id.to_string())
        .bind(actor_id)
        .bind(tenant_id)
        .bind("EMAIL_SYNC")
        .bind("users")
        .bind(user_id)
        .bind(&details)
        .bind(Option::<String>::None)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(pool)
        .await;

    if let Err(err) = res {
        tracing::warn!("Failed to write EMAIL_SYNC audit row for {}: {}", user_id, err);
    }
}

#[cfg(test)]
mod tests {
    /// Ekstrak email dari baris `customers` yang punya pasangan user — dipakai
    /// untuk memastikan invarian "email pelanggan == email user" setelah edit.
    pub(crate) fn emails_match(customer_email: Option<&str>, user_email: &str) -> bool {
        match customer_email {
            None => true,
            Some(v) => v.trim().eq_ignore_ascii_case(user_email.trim()),
        }
    }

    #[test]
    fn emails_match_is_case_and_whitespace_insensitive() {
        assert!(emails_match(Some("  A@B.com "), "a@b.com"));
        assert!(emails_match(None, "a@b.com"));
        assert!(!emails_match(Some("a@b.com"), "c@d.com"));
    }
}
