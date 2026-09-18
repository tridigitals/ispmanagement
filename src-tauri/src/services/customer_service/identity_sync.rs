//! Sinkronisasi data identitas (`email`, `phone`) antara `customers` dan `users`.
//!
//! ## Mengapa modul ini ada
//!
//! Satu pelanggan bisa punya akun login (via `customer_users`). Saat itu terjadi,
//! `customers.email`/`customers.phone` (kontak pelanggan) dan `users.email` /
//! `users.phone` merujuk orang yang sama dan **wajib identik** — kalau tidak,
//! pelanggan melihat data berbeda di halaman pelanggan vs halaman profil, dan
//! login pakai email yang "salah".
//!
//! Sebelumnya sinkronisasi ini tersebar dan tidak lengkap:
//! - `update_customer` menyinkronkan ke SEMUA user pelanggan (bukan hanya yang
//!   utama) tanpa cek unik → email user lain bisa tertimpa diam-diam.
//! - `user_service.update` (edit di superadmin) tidak menyentuh `customers`.
//! - `update_me` (profil sendiri) tidak menyentuh `customers` dan tanpa cek unik.
//! - `phone` tidak disinkronkan sama sekali di jalur mana pun.
//!
//! Akibatnya pernah terjadi data produksi dengan `customers.email` !=
//! `users.email` (lihat riwayat perbaikan). Modul ini menyatukan aturannya:
//! **data pelanggan == data akun login utamanya, dari mana pun diedit.**
//!
//! ## Aturan
//!
//! - `users.email` tetap sumber kebenaran untuk kredensial dan tetap UNIQUE global.
//! - `customers.email` tetap boleh berdiri sendiri untuk pelanggan tanpa akun
//!   (mayoritas pelanggan hanya data kontak, tanpa login).
//! - **Satu pelanggan = satu akun login.** `add_portal_user` menolak penautan
//!   kedua, jadi daftar user tertaut selalu 0 atau 1. Semua helper di sini tetap
//!   defensif kalau-kalau data lama punya lebih dari satu.
//! - `phone` ikut disinkronkan (bukan kredensial, jadi tidak wajib unik).

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
///
/// Karena aturan "1 pelanggan = 1 login", panjangnya normalnya 0 atau 1.
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

/// Berapa akun login yang tertaut ke pelanggan ini?
pub(crate) async fn count_linked_portal_users(
    pool: &DbPool,
    tenant_id: &str,
    customer_id: &str,
) -> AppResult<i64> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM customer_users WHERE customer_id = $1 AND tenant_id = $2",
    )
    .bind(customer_id)
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;
    Ok(n)
}

/// Pastikan email pelanggan tidak direbut dari user lain, SEBELUM menulis apa pun.
///
/// Dipanggil di awal `update_customer`: kalau email baru ternyata milik user lain
/// (di luar pelanggan ini), batalkan lebih dulu — jangan sampai `customers.email`
/// sudah terlanjur berubah sementara sinkronisasi ke `users` gagal.
pub(crate) async fn ensure_customer_email_available(
    pool: &DbPool,
    tenant_id: &str,
    customer_id: &str,
    new_email: &str,
) -> AppResult<()> {
    let candidate = new_email.trim();
    if candidate.is_empty() {
        return Ok(());
    }

    for user_id in linked_portal_user_ids(pool, tenant_id, customer_id).await? {
        if email_taken_by_other_user(pool, candidate, &user_id).await? {
            return Err(AppError::UserAlreadyExists);
        }
    }
    Ok(())
}

/// Selaraskan data akun ke pelanggan SEBELUM akun itu ditautkan.
///
/// Dipanggil dari `add_portal_user`. Tanpa langkah ini, menautkan akun yang
/// emailnya berbeda dari pelanggan langsung menghasilkan pasangan yang tidak
/// identik — melanggar aturan "email pelanggan == email user" tanpa melalui
/// validasi apa pun (bug yang ditemukan lewat uji E2E).
///
/// Aturan yang dipakai: `users.email` adalah kredensial login dan UNIQUE,
/// jadi ia yang menang. Kalau email akun sudah dipakai user lain, tolak —
/// jangan sampai pelanggan dipaksa memakai email milik orang lain.
pub(crate) async fn align_user_to_customer_before_link(
    pool: &DbPool,
    tenant_id: &str,
    customer_id: &str,
    user_id: &str,
) -> AppResult<()> {
    let user_row: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT email, phone FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    let Some((user_email, user_phone)) = user_row else {
        return Err(AppError::NotFound("User not found".to_string()));
    };

    let user_email = user_email.trim().to_string();
    if user_email.is_empty() {
        return Err(AppError::Validation(
            "User must have an email before being linked to a customer".to_string(),
        ));
    }

    // Email akun harus bebas dari user lain sebelum dipakai sebagai identitas
    // pelanggan. Helper ini mengecualikan user ini sendiri.
    if email_taken_by_other_user(pool, &user_email, user_id).await? {
        return Err(AppError::UserAlreadyExists);
    }

    let customer_before: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT email, phone FROM customers WHERE id = $1")
            .bind(customer_id)
            .fetch_optional(pool)
            .await?;
    let Some((cust_email, cust_phone)) = customer_before else {
        return Err(AppError::NotFound("Customer not found".to_string()));
    };

    // Samakan email pelanggan dengan email akun (email akun = kredensial login).
    if !cust_email
        .as_deref()
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case(&user_email)
    {
        sqlx::query("UPDATE customers SET email = $1, updated_at = NOW() WHERE id = $2")
            .bind(&user_email)
            .bind(customer_id)
            .execute(pool)
            .await?;

        audit_identity_change(
            pool,
            None,
            tenant_id,
            user_id,
            "email",
            cust_email.as_deref(),
            &user_email,
        )
        .await;
    }

    // Phone: nomor akun menang kalau ada; kalau akun kosong, pakai nomor
    // pelanggan supaya tidak ada data yang hilang.
    let target_phone = match user_phone.as_deref().map(str::trim) {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => cust_phone.as_deref().unwrap_or("").trim().to_string(),
    };

    if !target_phone.is_empty() && cust_phone.as_deref().unwrap_or("").trim() != target_phone {
        sqlx::query("UPDATE customers SET phone = $1, updated_at = NOW() WHERE id = $2")
            .bind(&target_phone)
            .bind(customer_id)
            .execute(pool)
            .await?;

        audit_identity_change(
            pool,
            None,
            tenant_id,
            user_id,
            "phone",
            cust_phone.as_deref(),
            &target_phone,
        )
        .await;
    }

    // Kalau akun belum punya nomor tapi pelanggan punya, isi akun dari pelanggan
    // supaya kedua sisi tetap identik.
    if user_phone.as_deref().unwrap_or("").trim().is_empty() && !target_phone.is_empty() {
        sqlx::query("UPDATE users SET phone = $1, updated_at = NOW() WHERE id = $2")
            .bind(&target_phone)
            .bind(user_id)
            .execute(pool)
            .await?;
    }

    Ok(())
}

/// Sinkronkan email + phone pelanggan ke akun login-nya.
///
/// Dipakai saat pelanggan diedit. Kalau pelanggan tidak punya akun login, ini
/// tidak melakukan apa-apa (datanya tetap tersimpan di `customers`).
pub(crate) async fn sync_customer_identity_to_user(
    pool: &DbPool,
    tenant_id: &str,
    customer_id: &str,
    new_email: Option<&str>,
    new_phone: Option<&str>,
    actor_id: Option<&str>,
) -> AppResult<()> {
    let email = new_email.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let phone = new_phone.map(|v| v.trim().to_string());

    if email.is_none() && phone.is_none() {
        return Ok(());
    }

    let linked = linked_portal_user_ids(pool, tenant_id, customer_id).await?;
    if linked.is_empty() {
        return Ok(());
    }

    // Email wajib unik. Kalau ada user LAIN di luar pelanggan ini yang sudah
    // memakai email tsb, tolak — jangan menimpa kredensial orang lain.
    if let Some(ref new_email) = email {
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
    }

    for user_id in &linked {
        let before: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT email, phone FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_optional(pool)
                .await?;

        if let Some(ref new_email) = email {
            sqlx::query("UPDATE users SET email = $1, updated_at = NOW() WHERE id = $2")
                .bind(new_email)
                .bind(user_id)
                .execute(pool)
                .await?;
        }

        if let Some(ref new_phone) = phone {
            sqlx::query("UPDATE users SET phone = $1, updated_at = NOW() WHERE id = $2")
                .bind(new_phone)
                .bind(user_id)
                .execute(pool)
                .await?;
        }

        // Jejak audit: perubahan email adalah peristiwa keamanan (email dipakai
        // untuk login & reset password), jadi selalu dicatat sebelum/sesudahnya.
        if let Some(ref new_email) = email {
            audit_identity_change(
                pool,
                actor_id,
                tenant_id,
                user_id,
                "email",
                before.as_ref().map(|(e, _)| e.as_str()),
                new_email,
            )
            .await;
        }
        let old_phone = before.as_ref().and_then(|(_, p)| p.clone());
        if let Some(ref new_phone) = phone {
            audit_identity_change(
                pool,
                actor_id,
                tenant_id,
                user_id,
                "phone",
                old_phone.as_deref(),
                new_phone,
            )
            .await;
        }
    }

    Ok(())
}

/// Sinkron arah sebaliknya: identitas `users` berubah → tebalkan ke `customers`.
///
/// Dipakai saat user diedit dari superadmin atau dari halaman profil. Kalau
/// user ini adalah akun login sebuah pelanggan, datanya ikut berubah supaya
/// keduanya tetap identik.
pub(crate) async fn sync_user_identity_to_customers(
    pool: &DbPool,
    user_id: &str,
    new_email: Option<&str>,
    new_phone: Option<&str>,
    actor_id: Option<&str>,
) -> AppResult<u64> {
    let email = new_email.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let phone = new_phone.map(|v| v.trim().to_string());

    if email.is_none() && phone.is_none() {
        return Ok(0);
    }

    let rows = sqlx::query(
        "SELECT c.id AS customer_id, c.tenant_id AS tenant_id,
                c.email AS before_email, c.phone AS before_phone
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
        let before_phone: Option<String> = row.get("before_phone");

        if let Some(ref new_email) = email {
            sqlx::query("UPDATE customers SET email = $1, updated_at = NOW() WHERE id = $2")
                .bind(new_email)
                .bind(&customer_id)
                .execute(pool)
                .await?;
            audit_identity_change(
                pool,
                actor_id,
                &tenant_id,
                user_id,
                "email",
                before_email.as_deref(),
                new_email,
            )
            .await;
        }

        if let Some(ref new_phone) = phone {
            sqlx::query("UPDATE customers SET phone = $1, updated_at = NOW() WHERE id = $2")
                .bind(new_phone)
                .bind(&customer_id)
                .execute(pool)
                .await?;
            audit_identity_change(
                pool,
                actor_id,
                &tenant_id,
                user_id,
                "phone",
                before_phone.as_deref(),
                new_phone,
            )
            .await;
        }

        updated += 1;
    }

    if updated > 0 {
        tracing::info!(
            "Synced user identity {} to {} linked customer row(s)",
            user_id,
            updated
        );
    }
    Ok(updated)
}

/// Catat perubahan identitas ke audit log. Kegagalan audit tidak boleh
/// menggagalkan perubahan itu sendiri (best-effort).
///
/// Perhatikan skema `audit_logs`: kolomnya `user_id`/`tenant_id` bertipe UUID
/// dan `id` juga UUID — bukan TEXT. Kita pakai query terpisah untuk postgres
/// dan sqlite karena binding-nya berbeda.
async fn audit_identity_change(
    pool: &DbPool,
    actor_id: Option<&str>,
    tenant_id: &str,
    user_id: &str,
    field: &str,
    before: Option<&str>,
    after: &str,
) {
    let before_str = before.unwrap_or("");
    if before_str == after {
        return;
    }

    let details = serde_json::json!({
        "field": field,
        "before": before_str,
        "after": after,
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
        .bind("IDENTITY_SYNC")
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
        .bind("IDENTITY_SYNC")
        .bind("users")
        .bind(user_id)
        .bind(&details)
        .bind(Option::<String>::None)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(pool)
        .await;

    if let Err(err) = res {
        tracing::warn!(
            "Failed to write IDENTITY_SYNC audit row for {}: {}",
            user_id,
            err
        );
    }
}

#[cfg(test)]
mod tests {
    /// Invarian: data pelanggan dan akun login-nya harus sama setelah sinkronisasi.
    pub(crate) fn emails_match(customer_email: Option<&str>, user_email: &str) -> bool {
        match customer_email {
            None => true,
            Some(v) => v.trim().eq_ignore_ascii_case(user_email.trim()),
        }
    }

    /// Phone dibandingkan apa adanya (setelah trim) — nomor telepon tidak
    /// case-sensitive tapi formatnya bisa beda (+62 vs 62) dan itu urusan
    /// normalisasi di layer lain, bukan invarian kesamaan di sini.
    pub(crate) fn phones_match(customer_phone: Option<&str>, user_phone: Option<&str>) -> bool {
        match (customer_phone, user_phone) {
            (None, None) => true,
            (None, Some(v)) | (Some(v), None) => v.trim().is_empty(),
            (Some(a), Some(b)) => a.trim() == b.trim(),
        }
    }

    #[test]
    fn emails_match_is_case_and_whitespace_insensitive() {
        assert!(emails_match(Some("  A@B.com "), "a@b.com"));
        assert!(emails_match(None, "a@b.com"));
        assert!(!emails_match(Some("a@b.com"), "c@d.com"));
    }

    #[test]
    fn phones_match_handles_empty_and_none() {
        assert!(phones_match(Some(" 08123 "), Some("08123")));
        assert!(phones_match(None, None));
        assert!(phones_match(None, Some("")));
        assert!(phones_match(Some(""), None));
        assert!(!phones_match(Some("0812"), Some("0813")));
    }
}
