//! Normalisasi email — satu sumber kebenaran.
//!
//! ## Kenapa ini ada
//!
//! Sebelumnya email disimpan APA ADANYA di `users.email`, sementara `UNIQUE`
//! bawaan Postgres bersifat **case-sensitive** (`users_email_key`). Akibatnya:
//!
//! - `Dodo@gmail.com` dan `dodo@gmail.com` menjadi **dua baris berbeda**;
//! - login justru men-`to_lowercase()` input, lalu mencari `WHERE email = $1`,
//!   sehingga user yang tersimpan dengan huruf besar **tidak pernah ketemu** dan
//!   tidak bisa masuk sama sekali;
//! - undangan/reset password untuk alamat yang sama bisa mengenai baris yang
//!   salah, karena ada dua baris untuk satu alamat.
//!
//! Perbaikannya dua lapis, dan keduanya diperlukan:
//!
//! 1. **Aplikasi** — semua jalur tulis memakai [`normalize_email`], semua
//!    pencarian memakai [`EMAIL_MATCHES_SQL`] atau `lower(email) = lower($1)`
//!    supaya pencarian tidak bergantung pada bentuk huruf yang tersimpan.
//! 2. **Database** — `UNIQUE (lower(email))` supaya invarian ini tidak bisa
//!    dilanggar lagi, bahkan oleh kode masa depan yang lupa menormalkan.

/// Normalkan email untuk disimpan: trim spasi, lalu lowercase.
///
/// Dipakai di **semua** jalur tulis `users.email` supaya satu alamat selalu
/// tersimpan dalam satu bentuk. Jangan menyimpan email mentah dari input
/// pengguna.
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Ekspresi SQL untuk mencocokkan email milik user dengan email pencarian,
/// tanpa bergantung pada bentuk huruf yang tersimpan.
///
/// `lower()` diterapkan di **kedua sisi** — penting untuk database yang masih
/// menyimpan email huruf besar dari sebelum normalisasi diterapkan. Tanpa ini,
/// user lama ber-email `Dodo@gmail.com` tetap tidak bisa login.
pub const EMAIL_MATCHES_SQL: &str = "lower(email) = lower($1)";

/// Varian `EMAIL_MATCHES_SQL` untuk SQLite (`?` sebagai placeholder).
pub const EMAIL_MATCHES_SQLITE: &str = "lower(email) = lower(?)";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menormalkan_huruf_besar_dan_spasi() {
        assert_eq!(normalize_email("Dodo@gmail.com"), "dodo@gmail.com");
        assert_eq!(normalize_email("DODO@GMAIL.COM"), "dodo@gmail.com");
        assert_eq!(normalize_email("  Dodo@Gmail.com  "), "dodo@gmail.com");
        // Email yang sudah normal tidak berubah (idempoten).
        assert_eq!(normalize_email("dodo@gmail.com"), "dodo@gmail.com");
    }

    #[test]
    fn mempertahankan_bagian_yang_signifikan() {
        // Lowercase hanya menyentuh huruf; tanda plus/titik dipertahankan apa
        // adanya. Gmail memperlakukan titik sebagai hal yang sama, tapi itu
        // kebijakan penyedia email — bukan sesuatu yang boleh kita tebak.
        assert_eq!(normalize_email("Dodo+ISP@Gmail.com"), "dodo+isp@gmail.com");
        assert_eq!(normalize_email("Do.Do@Gmail.com"), "do.do@gmail.com");
    }
}
