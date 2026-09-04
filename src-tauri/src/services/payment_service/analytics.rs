//! Billing analytics — MRR, ARR, collection rate, aging report, churn.
//!
//! KENAPA FILE INI DITULIS ULANG (2026-09-04)
//!
//! Halaman `/admin/billing` memajang "MRR Rp 0" dan "0 langganan aktif" di
//! sebelah piutang Rp 57,6 juta. Semua angka itu benar secara SQL, tapi
//! kombinasinya membuat layar terlihat seperti gagal memuat data. Lima
//! masalah dibuktikan langsung di data produksi:
//!
//! 1. UANG MASUK DAN UANG KELUAR DICAMPUR. Tabel `invoices` menyimpan dua
//!    jenis tagihan: milik pelanggan ISP (`external_id LIKE 'pkgsub:%'`) dan
//!    tagihan tenant ke platform (`plan:%`). `total_revenue` dan
//!    `revenue_trend` menjumlahkan keduanya, jadi batang Mei 2026 setinggi
//!    Rp 1.280.000 padahal pemasukan dari pelanggan bulan itu NOL — seluruh
//!    nilainya adalah tagihan platform yang dibayar tenant sendiri. Sisa
//!    kode (`list_invoices`, `list_customer_package_invoices`) sudah lama
//!    memisahkan keduanya lewat prefix; hanya analytics yang tidak.
//!
//! 2. AGING TIDAK UTUH. Filter `status IN ('pending','overdue')` melewatkan
//!    `verification_pending` (invoice yang buktinya sudah diunggah dan
//!    menunggu verifikasi — tetap piutang), dan `due_date < NOW()` membuang
//!    seluruh invoice yang belum jatuh tempo. Uang yang belum jatuh tempo
//!    tidak punya tempat di layar sama sekali, jadi total di kartu aging
//!    tidak pernah sama dengan total piutang.
//!
//! 3. REVENUE TREND BOLONG. `GROUP BY DATE_TRUNC` hanya menghasilkan baris
//!    untuk bulan yang punya invoice lunas, jadi label "6 bulan terakhir"
//!    hanya menampilkan 4 batang. Justru bulan tanpa pemasukan (Juli–
//!    September 2026) yang paling perlu terlihat.
//!
//! 4. PERSENTASE TANPA BASIS SAMPEL. `collection_rate` 0% dihitung dari 2
//!    invoice, dan `avg_days_to_pay` 0 hari berasal dari himpunan kosong.
//!    Keduanya tidak bisa dibedakan dari "semua pelanggan gagal bayar"
//!    kecuali pembilang dan penyebutnya ikut dikirim.
//!
//! 5. MRR NOL TANPA PENJELASAN. Tidak ada satu pun `customer_subscriptions`
//!    berstatus `active`: 542 `suspended`, 5 `pending_installation`, 2
//!    `cancelled`. Itu konsekuensi `mixradius_import_mapper` yang memetakan
//!    UNPAID+kedaluwarsa ke `suspended`. Selama rincian status tidak
//!    dikirim, "MRR Rp 0" terbaca sebagai bug, bukan sebagai kondisi bisnis.
//!
//! Pembagian bucket dan pengisian bulan kosong sengaja dikerjakan di Rust
//! (`bucket_aging`, `month_keys_ending_now`, `fill_revenue_trend`) supaya
//! bisa diuji tanpa database dan tidak bergantung pada dialek SQL.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde::Serialize;
use sqlx::Row;

use super::core::CUSTOMER_PACKAGE_INVOICE_PREFIX;
use super::PaymentService;
use crate::error::AppResult;

/// Jumlah bulan yang selalu dikirim pada `revenue_trend`.
const TREND_MONTHS: usize = 6;

/// Rentang hari untuk metrik penagihan (collection rate, rata-rata pelunasan).
const COLLECTION_WINDOW_DAYS: i64 = 90;

/// Top-level analytics response returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct BillingAnalytics {
    /// Monthly Recurring Revenue — sum of all active subscription monthly prices.
    pub mrr: f64,
    /// Annual Recurring Revenue — MRR × 12.
    pub arr: f64,
    /// Pemasukan dari pelanggan pada bulan kalender ini. Tagihan platform
    /// (`plan:%`) tidak dihitung — itu biaya tenant, bukan pendapatan.
    pub total_revenue: f64,
    /// Percentage of invoices paid on or before due date.
    pub collection_rate: f64,
    /// Pembilang/penyebut di balik `collection_rate`, supaya 0% bisa
    /// dibedakan dari "tidak ada data".
    pub collection_sample: CollectionSample,
    /// Average days from issue → payment.
    pub avg_days_to_pay: f64,
    /// Banyak invoice lunas yang dipakai menghitung `avg_days_to_pay`.
    pub avg_days_sample: i64,
    /// Outstanding invoices grouped by age bracket.
    pub aging: AgingReport,
    /// Jumlah seluruh bucket aging. Dikirim dari server supaya klien tidak
    /// menjumlahkan sendiri lalu berbeda saat bucket baru ditambahkan.
    pub aging_total: f64,
    /// % of subscriptions cancelled this month vs active at start of month.
    pub churn_rate: f64,
    /// Number of currently active subscriptions.
    pub active_subscriptions: i64,
    /// Total unique customers with at least one subscription (any status).
    pub total_customers: i64,
    /// Rincian status langganan, urut terbanyak. Menjelaskan MRR nol.
    pub subscription_breakdown: Vec<SubscriptionStatusCount>,
    /// Revenue per month, selalu berisi `TREND_MONTHS` titik (oldest → newest).
    pub revenue_trend: Vec<RevenueTrendPoint>,
    /// Tagihan tenant ke platform yang belum lunas. Dipisah supaya tidak
    /// tercampur ke piutang pelanggan.
    pub platform_dues: PlatformDues,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgingReport {
    /// Belum jatuh tempo.
    pub not_due: f64,
    /// 0–30 days past due.
    pub current: f64,
    /// 31–60 days past due.
    pub days_31_60: f64,
    /// 61–90 days past due.
    pub days_61_90: f64,
    /// >90 days past due.
    pub over_90: f64,
}

impl AgingReport {
    pub fn total(&self) -> f64 {
        self.not_due + self.current + self.days_31_60 + self.days_61_90 + self.over_90
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CollectionSample {
    /// Invoice pelanggan yang masuk jendela penilaian.
    pub invoices_considered: i64,
    /// Dari jumlah itu, yang lunas tepat waktu.
    pub paid_on_time: i64,
    /// Dari jumlah itu, yang lunas (kapan pun).
    pub paid_total: i64,
    /// Panjang jendela dalam hari.
    pub window_days: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubscriptionStatusCount {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlatformDues {
    pub outstanding_amount: f64,
    pub outstanding_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevenueTrendPoint {
    /// "YYYY-MM"
    pub month: String,
    pub revenue: f64,
}

/// Status invoice yang masih dianggap piutang.
///
/// `verification_pending` ikut karena bukti bayarnya belum diverifikasi —
/// uangnya belum diakui. Sejalan dengan `list_invoices` di `mod.rs`.
const OUTSTANDING_STATUSES: [&str; 3] = ["pending", "verification_pending", "overdue"];

/// Bagi (hari lewat jatuh tempo, nilai) ke bucket aging.
///
/// `days_overdue` negatif berarti belum jatuh tempo. Versi lama membuang
/// baris itu di SQL sehingga uangnya hilang dari layar.
pub fn bucket_aging(rows: &[(f64, f64)]) -> AgingReport {
    let mut report = AgingReport {
        not_due: 0.0,
        current: 0.0,
        days_31_60: 0.0,
        days_61_90: 0.0,
        over_90: 0.0,
    };
    for (days, amount) in rows {
        if *days < 0.0 {
            report.not_due += amount;
        } else if *days <= 30.0 {
            report.current += amount;
        } else if *days <= 60.0 {
            report.days_31_60 += amount;
        } else if *days <= 90.0 {
            report.days_61_90 += amount;
        } else {
            report.over_90 += amount;
        }
    }
    report
}

/// Kunci "YYYY-MM" untuk `count` bulan terakhir, berakhir pada bulan `now`.
pub fn month_keys_ending_now(now: DateTime<Utc>, count: usize) -> Vec<String> {
    let mut keys = Vec::with_capacity(count);
    let (mut year, mut month) = (now.year(), now.month());
    for _ in 0..count {
        keys.push(format!("{year:04}-{month:02}"));
        if month == 1 {
            month = 12;
            year -= 1;
        } else {
            month -= 1;
        }
    }
    keys.reverse();
    keys
}

/// Susun tren dengan setiap bulan terisi; bulan tanpa pemasukan jadi 0.0.
pub fn fill_revenue_trend(
    months: &[String],
    paid: &HashMap<String, f64>,
) -> Vec<RevenueTrendPoint> {
    months
        .iter()
        .map(|month| RevenueTrendPoint {
            month: month.clone(),
            revenue: paid.get(month).copied().unwrap_or(0.0),
        })
        .collect()
}

/// Bulatkan ke `decimals` angka desimal.
fn round_to(value: f64, decimals: u32) -> f64 {
    let factor = 10f64.powi(decimals as i32);
    (value * factor).round() / factor
}

/// Hitung rasio dalam persen; himpunan kosong menghasilkan 0.0.
fn percentage(numerator: i64, denominator: i64) -> f64 {
    if denominator <= 0 {
        return 0.0;
    }
    round_to((numerator as f64 / denominator as f64) * 100.0, 2)
}

/// Awal bulan pertama pada jendela `count` bulan yang berakhir di bulan `now`.
fn month_window_start(now: DateTime<Utc>, count: usize) -> DateTime<Utc> {
    let months_back = count.saturating_sub(1) as i32;
    let mut year = now.year();
    let mut month = now.month() as i32 - months_back;
    while month < 1 {
        month += 12;
        year -= 1;
    }
    NaiveDate::from_ymd_opt(year, month as u32, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc())
        .unwrap_or(now)
}

/// Kueri per-backend. Dipisah karena placeholder dan cast tipe berbeda;
/// seluruh logika agregasi tetap satu jalur di Rust.
#[cfg(feature = "postgres")]
mod sql {
    pub const MRR: &str = "SELECT COALESCE(SUM(CASE WHEN billing_cycle = 'yearly' THEN price / 12.0 WHEN billing_cycle = 'quarterly' THEN price / 3.0 ELSE price END), 0)::FLOAT8 FROM customer_subscriptions WHERE tenant_id = $1 AND status IN ('active', 'grace_active')";
    pub const BREAKDOWN: &str = "SELECT status, COUNT(*)::BIGINT FROM customer_subscriptions WHERE tenant_id = $1 GROUP BY status ORDER BY 2 DESC, 1 ASC";
    pub const ACTIVE_SUBS: &str = "SELECT COUNT(*)::BIGINT FROM customer_subscriptions WHERE tenant_id = $1 AND status IN ('active', 'grace_active')";
    pub const TOTAL_CUSTOMERS: &str = "SELECT COUNT(DISTINCT customer_id)::BIGINT FROM customer_subscriptions WHERE tenant_id = $1";
    pub const CANCELLED_THIS_MONTH: &str = "SELECT COUNT(*)::BIGINT FROM customer_subscriptions WHERE tenant_id = $1 AND status = 'cancelled' AND updated_at >= $2";
    pub const REVENUE_THIS_MONTH: &str = "SELECT COALESCE(SUM(amount), 0)::FLOAT8 FROM invoices WHERE tenant_id = $1 AND status = 'paid' AND paid_at >= $2 AND external_id LIKE $3";
    pub const COLLECTION: &str = "SELECT COUNT(*)::BIGINT, COUNT(*) FILTER (WHERE status = 'paid' AND paid_at IS NOT NULL AND paid_at <= due_date)::BIGINT, COUNT(*) FILTER (WHERE status = 'paid')::BIGINT FROM invoices WHERE tenant_id = $1 AND created_at >= $2 AND external_id LIKE $3 AND status IN ('paid', 'pending', 'verification_pending', 'overdue')";
    pub const PAID_DURATIONS: &str = "SELECT created_at, paid_at FROM invoices WHERE tenant_id = $1 AND status = 'paid' AND paid_at IS NOT NULL AND created_at >= $2 AND external_id LIKE $3";
    pub const OUTSTANDING: &str = "SELECT due_date, amount::FLOAT8 FROM invoices WHERE tenant_id = $1 AND external_id LIKE $2 AND status IN ('pending', 'verification_pending', 'overdue')";
    pub const TREND_PAID: &str = "SELECT paid_at, amount::FLOAT8 FROM invoices WHERE tenant_id = $1 AND status = 'paid' AND paid_at IS NOT NULL AND paid_at >= $2 AND external_id LIKE $3";
    pub const PLATFORM_DUES: &str = "SELECT COUNT(*)::BIGINT, COALESCE(SUM(amount), 0)::FLOAT8 FROM invoices WHERE tenant_id = $1 AND (external_id IS NULL OR external_id NOT LIKE $2) AND status IN ('pending', 'verification_pending', 'overdue')";
}

#[cfg(feature = "sqlite")]
mod sql {
    pub const MRR: &str = "SELECT CAST(COALESCE(SUM(CASE WHEN billing_cycle = 'yearly' THEN price / 12.0 WHEN billing_cycle = 'quarterly' THEN price / 3.0 ELSE price END), 0) AS REAL) FROM customer_subscriptions WHERE tenant_id = ? AND status IN ('active', 'grace_active')";
    pub const BREAKDOWN: &str = "SELECT status, COUNT(*) FROM customer_subscriptions WHERE tenant_id = ? GROUP BY status ORDER BY 2 DESC, 1 ASC";
    pub const ACTIVE_SUBS: &str = "SELECT COUNT(*) FROM customer_subscriptions WHERE tenant_id = ? AND status IN ('active', 'grace_active')";
    pub const TOTAL_CUSTOMERS: &str =
        "SELECT COUNT(DISTINCT customer_id) FROM customer_subscriptions WHERE tenant_id = ?";
    pub const CANCELLED_THIS_MONTH: &str = "SELECT COUNT(*) FROM customer_subscriptions WHERE tenant_id = ? AND status = 'cancelled' AND updated_at >= ?";
    pub const REVENUE_THIS_MONTH: &str = "SELECT CAST(COALESCE(SUM(amount), 0) AS REAL) FROM invoices WHERE tenant_id = ? AND status = 'paid' AND paid_at >= ? AND external_id LIKE ?";
    pub const COLLECTION: &str = "SELECT COUNT(*), COALESCE(SUM(CASE WHEN status = 'paid' AND paid_at IS NOT NULL AND paid_at <= due_date THEN 1 ELSE 0 END), 0), COALESCE(SUM(CASE WHEN status = 'paid' THEN 1 ELSE 0 END), 0) FROM invoices WHERE tenant_id = ? AND created_at >= ? AND external_id LIKE ? AND status IN ('paid', 'pending', 'verification_pending', 'overdue')";
    pub const PAID_DURATIONS: &str = "SELECT created_at, paid_at FROM invoices WHERE tenant_id = ? AND status = 'paid' AND paid_at IS NOT NULL AND created_at >= ? AND external_id LIKE ?";
    pub const OUTSTANDING: &str = "SELECT due_date, CAST(amount AS REAL) FROM invoices WHERE tenant_id = ? AND external_id LIKE ? AND status IN ('pending', 'verification_pending', 'overdue')";
    pub const TREND_PAID: &str = "SELECT paid_at, CAST(amount AS REAL) FROM invoices WHERE tenant_id = ? AND status = 'paid' AND paid_at IS NOT NULL AND paid_at >= ? AND external_id LIKE ?";
    pub const PLATFORM_DUES: &str = "SELECT COUNT(*), CAST(COALESCE(SUM(amount), 0) AS REAL) FROM invoices WHERE tenant_id = ? AND (external_id IS NULL OR external_id NOT LIKE ?) AND status IN ('pending', 'verification_pending', 'overdue')";
}

/// Compute billing analytics for the given tenant.
pub async fn compute_billing_analytics_for_service(
    service: &PaymentService,
    tenant_id: &str,
) -> AppResult<BillingAnalytics> {
    let now = Utc::now();
    // Awal bulan kalender ini = jendela 1 bulan yang berakhir sekarang.
    let month_start = month_window_start(now, 1);
    let collection_cutoff = now - Duration::days(COLLECTION_WINDOW_DAYS);
    let trend_start = month_window_start(now, TREND_MONTHS);
    // Hanya invoice pelanggan. `plan:%` adalah tagihan tenant ke platform.
    let customer_prefix = format!("{CUSTOMER_PACKAGE_INVOICE_PREFIX}%");
    let pool = &service.pool;

    // ── MRR / ARR ─────────────────────────────────────────────────────
    let mrr: f64 = sqlx::query_scalar(sql::MRR)
        .bind(tenant_id)
        .fetch_one(pool)
        .await?;
    let arr = mrr * 12.0;

    // ── Rincian status langganan (menjelaskan MRR nol) ────────────────
    let breakdown_rows = sqlx::query(sql::BREAKDOWN)
        .bind(tenant_id)
        .fetch_all(pool)
        .await?;
    let subscription_breakdown: Vec<SubscriptionStatusCount> = breakdown_rows
        .iter()
        .map(|row| SubscriptionStatusCount {
            status: row.get::<String, _>(0),
            count: row.get::<i64, _>(1),
        })
        .collect();

    // ── Pemasukan bulan ini (khusus pelanggan) ────────────────────────
    let total_revenue: f64 = sqlx::query_scalar(sql::REVENUE_THIS_MONTH)
        .bind(tenant_id)
        .bind(month_start)
        .bind(&customer_prefix)
        .fetch_one(pool)
        .await?;

    // ── Collection rate + basis sampelnya ─────────────────────────────
    let (invoices_considered, paid_on_time, paid_total): (i64, i64, i64) =
        sqlx::query_as(sql::COLLECTION)
            .bind(tenant_id)
            .bind(collection_cutoff)
            .bind(&customer_prefix)
            .fetch_one(pool)
            .await?;

    // ── Rata-rata hari pelunasan (dihitung di Rust agar identik di dua backend)
    let duration_rows = sqlx::query(sql::PAID_DURATIONS)
        .bind(tenant_id)
        .bind(collection_cutoff)
        .bind(&customer_prefix)
        .fetch_all(pool)
        .await?;
    let mut total_days = 0.0f64;
    for row in &duration_rows {
        let created_at: DateTime<Utc> = row.get(0);
        let paid_at: DateTime<Utc> = row.get(1);
        total_days += (paid_at - created_at).num_seconds() as f64 / 86_400.0;
    }
    let avg_days_sample = duration_rows.len() as i64;
    let avg_days_to_pay = if avg_days_sample > 0 {
        round_to(total_days / avg_days_sample as f64, 1)
    } else {
        0.0
    };

    // ── Aging: SELURUH piutang, termasuk yang belum jatuh tempo ───────
    let outstanding_rows = sqlx::query(sql::OUTSTANDING)
        .bind(tenant_id)
        .bind(&customer_prefix)
        .fetch_all(pool)
        .await?;
    let aging_input: Vec<(f64, f64)> = outstanding_rows
        .iter()
        .map(|row| {
            let due_date: DateTime<Utc> = row.get(0);
            let amount: f64 = row.get(1);
            ((now - due_date).num_seconds() as f64 / 86_400.0, amount)
        })
        .collect();
    let aging = bucket_aging(&aging_input);
    let aging_total = round_to(aging.total(), 2);

    // ── Langganan aktif, pelanggan, churn ─────────────────────────────
    let active_subscriptions: i64 = sqlx::query_scalar(sql::ACTIVE_SUBS)
        .bind(tenant_id)
        .fetch_one(pool)
        .await?;
    let total_customers: i64 = sqlx::query_scalar(sql::TOTAL_CUSTOMERS)
        .bind(tenant_id)
        .fetch_one(pool)
        .await?;
    let cancelled_this_month: i64 = sqlx::query_scalar(sql::CANCELLED_THIS_MONTH)
        .bind(tenant_id)
        .bind(month_start)
        .fetch_one(pool)
        .await?;
    let churn_rate = percentage(
        cancelled_this_month,
        active_subscriptions + cancelled_this_month,
    );

    // ── Tren pemasukan: setiap bulan hadir, termasuk yang nol ─────────
    let trend_rows = sqlx::query(sql::TREND_PAID)
        .bind(tenant_id)
        .bind(trend_start)
        .bind(&customer_prefix)
        .fetch_all(pool)
        .await?;
    let mut paid_by_month: HashMap<String, f64> = HashMap::new();
    for row in &trend_rows {
        let paid_at: DateTime<Utc> = row.get(0);
        let amount: f64 = row.get(1);
        let key = format!("{:04}-{:02}", paid_at.year(), paid_at.month());
        *paid_by_month.entry(key).or_insert(0.0) += amount;
    }
    let months = month_keys_ending_now(now, TREND_MONTHS);
    let revenue_trend = fill_revenue_trend(&months, &paid_by_month);

    // ── Tagihan platform, dipisah dari piutang pelanggan ──────────────
    let (platform_count, platform_amount): (i64, f64) = sqlx::query_as(sql::PLATFORM_DUES)
        .bind(tenant_id)
        .bind(&customer_prefix)
        .fetch_one(pool)
        .await?;

    Ok(BillingAnalytics {
        mrr: round_to(mrr, 2),
        arr: round_to(arr, 2),
        total_revenue: round_to(total_revenue, 2),
        collection_rate: percentage(paid_on_time, invoices_considered),
        collection_sample: CollectionSample {
            invoices_considered,
            paid_on_time,
            paid_total,
            window_days: COLLECTION_WINDOW_DAYS,
        },
        avg_days_to_pay,
        avg_days_sample,
        aging,
        aging_total,
        churn_rate,
        active_subscriptions,
        total_customers,
        subscription_breakdown,
        revenue_trend,
        platform_dues: PlatformDues {
            outstanding_amount: round_to(platform_amount, 2),
            outstanding_count: platform_count,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(year: i32, month: u32, day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap()
    }

    /// Versi lama membuang invoice yang belum jatuh tempo di SQL
    /// (`due_date < NOW()`), jadi uangnya tidak muncul di bucket mana pun.
    #[test]
    fn aging_keeps_invoices_that_are_not_due_yet() {
        let report = bucket_aging(&[(-5.0, 100_000.0), (10.0, 50_000.0)]);
        assert_eq!(report.not_due, 100_000.0);
        assert_eq!(report.current, 50_000.0);
        assert_eq!(report.total(), 150_000.0);
    }

    #[test]
    fn aging_bucket_boundaries_are_inclusive_on_upper_edge() {
        let report = bucket_aging(&[
            (0.0, 1.0),
            (30.0, 2.0),
            (30.5, 4.0),
            (60.0, 8.0),
            (90.0, 16.0),
            (90.1, 32.0),
        ]);
        assert_eq!(report.current, 3.0, "0 dan 30 hari masuk bucket pertama");
        assert_eq!(report.days_31_60, 12.0, "30,5 dan 60 hari");
        assert_eq!(report.days_61_90, 16.0, "tepat 90 hari belum >90");
        assert_eq!(report.over_90, 32.0);
        assert_eq!(report.total(), 63.0);
    }

    #[test]
    fn aging_total_matches_sum_of_every_bucket() {
        let rows = vec![
            (-1.0, 7.0),
            (15.0, 11.0),
            (45.0, 13.0),
            (75.0, 17.0),
            (365.0, 19.0),
        ];
        let report = bucket_aging(&rows);
        let expected: f64 = rows.iter().map(|(_, amount)| amount).sum();
        assert_eq!(report.total(), expected);
    }

    /// Bulan tanpa invoice lunas dulu hilang dari hasil, sehingga grafik
    /// "6 bulan terakhir" hanya memunculkan 4 batang.
    #[test]
    fn revenue_trend_always_returns_every_month_in_window() {
        let months = month_keys_ending_now(at(2026, 9, 4), TREND_MONTHS);
        assert_eq!(
            months,
            vec!["2026-04", "2026-05", "2026-06", "2026-07", "2026-08", "2026-09"]
        );

        let mut paid = HashMap::new();
        paid.insert("2026-04".to_string(), 480_000.0);
        paid.insert("2026-06".to_string(), 125_000.0);

        let trend = fill_revenue_trend(&months, &paid);
        assert_eq!(trend.len(), TREND_MONTHS);
        assert_eq!(trend[0].revenue, 480_000.0);
        assert_eq!(trend[1].revenue, 0.0, "Mei tanpa pemasukan tetap hadir");
        assert_eq!(trend[2].revenue, 125_000.0);
        assert_eq!(trend[5].month, "2026-09");
        assert_eq!(trend[5].revenue, 0.0);
    }

    #[test]
    fn month_keys_cross_year_boundary() {
        let months = month_keys_ending_now(at(2026, 2, 15), 4);
        assert_eq!(months, vec!["2025-11", "2025-12", "2026-01", "2026-02"]);
    }

    /// `paid_at >= NOW() - INTERVAL '6 months'` memotong sebagian bulan
    /// tertua; jendela harus mulai dari awal bulan.
    #[test]
    fn trend_window_starts_at_first_day_of_oldest_month() {
        let start = month_window_start(at(2026, 9, 4), TREND_MONTHS);
        assert_eq!(start, Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap());

        let crossing = month_window_start(at(2026, 2, 20), TREND_MONTHS);
        assert_eq!(crossing, Utc.with_ymd_and_hms(2025, 9, 1, 0, 0, 0).unwrap());
    }

    #[test]
    fn percentage_of_empty_sample_is_zero_not_nan() {
        assert_eq!(percentage(0, 0), 0.0);
        assert_eq!(percentage(5, 0), 0.0);
        assert_eq!(percentage(1, 3), 33.33);
        assert_eq!(percentage(2, 2), 100.0);
    }

    /// Status piutang harus sejalan dengan `list_invoices` di mod.rs, yang
    /// sudah lama menghitung `verification_pending` sebagai belum lunas.
    #[test]
    fn outstanding_statuses_include_verification_pending() {
        assert!(OUTSTANDING_STATUSES.contains(&"verification_pending"));
        assert!(OUTSTANDING_STATUSES.contains(&"pending"));
        assert!(OUTSTANDING_STATUSES.contains(&"overdue"));
        assert!(!OUTSTANDING_STATUSES.contains(&"paid"));
        for status in OUTSTANDING_STATUSES {
            assert!(
                sql::OUTSTANDING.contains(status),
                "kueri aging kehilangan status {status}"
            );
        }
    }

    /// Setiap kueri yang menyentuh uang pelanggan wajib menyaring prefix,
    /// kalau tidak tagihan platform ikut terhitung sebagai pendapatan.
    #[test]
    fn customer_money_queries_filter_by_invoice_prefix() {
        for query in [
            sql::REVENUE_THIS_MONTH,
            sql::COLLECTION,
            sql::PAID_DURATIONS,
            sql::OUTSTANDING,
            sql::TREND_PAID,
        ] {
            assert!(
                query.contains("external_id LIKE"),
                "kueri pendapatan pelanggan tanpa filter prefix: {query}"
            );
        }
        assert!(
            sql::PLATFORM_DUES.contains("NOT LIKE"),
            "tagihan platform harus memakai negasi prefix"
        );
    }

    #[test]
    fn subscription_queries_treat_grace_active_as_active() {
        assert!(sql::MRR.contains("grace_active"));
        assert!(sql::ACTIVE_SUBS.contains("grace_active"));
    }
}
