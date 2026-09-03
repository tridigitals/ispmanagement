//! Normalisasi parameter paginasi.
//!
//! LATAR BELAKANG (dua bug nyata, bukan pembersihan kosmetik):
//!
//! 1. BATAS TERLALU KETAT DI SATU TEMPAT. `payment_service` memakai
//!    `per_page.clamp(1, 100)`, sedangkan frontend butuh agregat atas seluruh
//!    tagihan tenant (484 baris di tenant produksi). Akibatnya dashboard yang
//!    meminta `per_page: 1000` tetap menerima 100 baris lalu menjumlahkannya
//!    sebagai total tenant — angka di layar salah tanpa peringatan apa pun.
//!
//! 2. TIDAK ADA BATAS SAMA SEKALI DI TEMPAT LAIN. `pppoe_service::list_accounts`
//!    dan `customer_service::list_customers` memakai `per_page` mentah dari
//!    permintaan. Dua konsekuensinya serius:
//!      - `(page - 1) * per_page` pada `u32` bisa overflow. Di build debug itu
//!        panic; di release nilainya membungkus (wrap) sehingga OFFSET jadi
//!        angka acak dan halaman berisi baris yang salah.
//!      - `LIMIT 4000000000` memaksa Postgres dan proses aplikasi mengalokasi
//!        seluruh tabel. Satu permintaan tanpa autentikasi ulang cukup untuk
//!        menghabiskan memori server.
//!
//! Modul ini menyatukan keduanya: satu batas atas yang cukup besar untuk
//! agregat jujur (`MAX_PER_PAGE`), dan aritmetika offset yang tidak bisa
//! overflow karena dihitung di `i64` setelah kedua nilai dibatasi.

/// Batas atas baris per permintaan.
///
/// Dipilih 1.000 dengan alasan terukur, bukan angka bulat sembarang: tabel
/// terbesar yang ditarik penuh oleh UI saat ini adalah `pppoe_accounts`
/// (1.010 baris di tenant produksi) dan `invoices` (484 baris). Dengan 1.000,
/// UI menyelesaikan agregat dalam 1–2 permintaan alih-alih 11, sementara
/// backend tetap punya pagar terhadap permintaan yang mengada-ada.
pub const MAX_PER_PAGE: u32 = 1_000;

/// Nilai `per_page` bila pemanggil tidak menyebutkannya.
///
/// Tetap 25 supaya perilaku endpoint yang sudah dipakai tidak berubah diam-diam
/// hanya karena batas atasnya dinaikkan.
pub const DEFAULT_PER_PAGE: u32 = 25;

/// Parameter paginasi yang sudah aman dipakai untuk membangun query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    /// Nomor halaman, minimal 1.
    pub page: u32,
    /// Baris per halaman, dalam rentang `1..=MAX_PER_PAGE`.
    pub per_page: u32,
    /// `(page - 1) * per_page` yang dihitung di `i64`, siap di-bind ke OFFSET.
    pub offset: i64,
}

impl Page {
    /// `per_page` sebagai `i64` untuk di-bind ke LIMIT.
    pub fn limit(&self) -> i64 {
        self.per_page as i64
    }
}

/// Batasi `page` dan `per_page`, lalu hitung offset tanpa risiko overflow.
///
/// `page` 0 dinaikkan ke 1 (bukan ditolak) supaya pemanggil lama yang mengirim
/// 0 tidak berubah dari "halaman pertama" menjadi galat.
pub fn normalize(page: u32, per_page: u32) -> Page {
    let page = page.max(1);
    let per_page = if per_page == 0 {
        DEFAULT_PER_PAGE
    } else {
        per_page.min(MAX_PER_PAGE)
    };

    // Naikkan ke i64 SEBELUM dikalikan. `page` bisa sebesar u32::MAX dan
    // per_page bisa 1.000; perkaliannya melewati batas u32 tapi tidak pernah
    // mendekati batas i64.
    let offset = (page as i64 - 1) * per_page as i64;

    Page {
        page,
        per_page,
        offset,
    }
}

/// Varian untuk handler HTTP/Tauri yang menerima `Option<u32>`.
pub fn normalize_opt(page: Option<u32>, per_page: Option<u32>) -> Page {
    normalize(page.unwrap_or(1), per_page.unwrap_or(DEFAULT_PER_PAGE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halaman_nol_dianggap_halaman_pertama() {
        let p = normalize(0, 25);
        assert_eq!(p.page, 1);
        assert_eq!(p.offset, 0);
    }

    #[test]
    fn per_page_nol_memakai_default_bukan_nol_baris() {
        let p = normalize(1, 0);
        assert_eq!(p.per_page, DEFAULT_PER_PAGE);
    }

    #[test]
    fn per_page_dibatasi_di_batas_atas() {
        let p = normalize(1, 100_000);
        assert_eq!(p.per_page, MAX_PER_PAGE);
    }

    #[test]
    fn permintaan_seribu_baris_diterima_utuh() {
        // Inilah inti perbaikannya: sebelumnya clamp(1, 100) memotong ini ke 100
        // sehingga agregat frontend hanya menjumlahkan 100 baris pertama.
        let p = normalize(1, 1_000);
        assert_eq!(p.per_page, 1_000);
        assert_eq!(p.limit(), 1_000);
    }

    #[test]
    fn offset_tidak_overflow_pada_page_ekstrem() {
        // (u32::MAX - 1) * 1000 melewati batas u32; dulu ini panic di debug dan
        // membungkus di release.
        let p = normalize(u32::MAX, 1_000);
        assert_eq!(p.per_page, 1_000);
        assert_eq!(p.offset, (u32::MAX as i64 - 1) * 1_000);
        assert!(p.offset > u32::MAX as i64);
    }

    #[test]
    fn offset_mengikuti_halaman() {
        assert_eq!(normalize(1, 25).offset, 0);
        assert_eq!(normalize(2, 25).offset, 25);
        assert_eq!(normalize(4, 100).offset, 300);
    }

    #[test]
    fn opsi_kosong_memakai_default_yang_sama() {
        assert_eq!(normalize_opt(None, None), normalize(1, DEFAULT_PER_PAGE));
    }
}
