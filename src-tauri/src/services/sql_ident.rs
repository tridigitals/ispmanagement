//! SQL identifier safety — pembatas untuk `format!()` yang menyisipkan
//! nama tabel/kolom ke dalam query.
//!
//! Aturan: parameter *nilai* selalu bind ($1/?1). Identifier (nama tabel &
//! kolom) tidak bisa di-bind, jadi harus lolos [`is_safe_sql_ident`] sebelum
//! masuk string SQL. Identifier yang sah di schema ini hanya `[A-Za-z_][A-Za-z0-9_]*`
//! (tanpetik, tanpa spasi, tanpa karakter quoting) — siapa pun yang menyisipkan
//! `"` , `;`, `--`, atau spasi otomatis ditolak.
//!
//! Dipakai oleh: backup restore (nama tabel & kolom key berasal dari ZIP yang
//! bisa diedit user → attacker-controllable), network_asset_service (tabel &
//! kolom relasi), system_service (fallback daftar tabel).

use crate::error::AppError;

/// `true` jika `s` adalah identifier SQL yang aman sesuai konvensi schema.
pub fn is_safe_sql_ident(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes.len() > 63 {
        return false;
    }
    let first = bytes[0];
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
}

/// Versi assert — kembalikan `Ok(s)` bila aman, `Err(Validation)` bila tidak.
pub fn assert_sql_ident(s: &str) -> Result<(), AppError> {
    if is_safe_sql_ident(s) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "Rejected unsafe SQL identifier: {s:?}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_schema_identifiers() {
        for ok in [
            "users",
            "support_tickets",
            "mikrotik_incidents",
            "customer_locations",
            "installation_work_orders",
            "network_assets",
            "audit_logs",
            "tenant_id",
            "_private",
            "col2",
        ] {
            assert!(is_safe_sql_ident(ok), "{ok} should be accepted");
        }
    }

    #[test]
    fn rejects_injection_and_quoting_vectors() {
        for bad in [
            "",
            "1users",
            "users; DROP TABLE users",
            "users--",
            "users/*x*/",
            "\"users\"",
            "'users'",
            "us ers",
            "users\u{0}",
            "user\u{0}s",
            "t\u{e9}ble",
            "x;PRAGMA",
            "a;DROP TABLE b--",
        ] {
            assert!(!is_safe_sql_ident(bad), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn rejects_overlong() {
        let long = "a".repeat(64);
        assert!(!is_safe_sql_ident(&long));
        let max = "a".repeat(63);
        assert!(is_safe_sql_ident(&max));
    }
}
