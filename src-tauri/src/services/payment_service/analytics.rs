//! Billing analytics — MRR, ARR, collection rate, aging report, churn.

use chrono::{Datelike, Utc};
use serde::Serialize;
use sqlx::Row;

use crate::error::AppResult;
use super::PaymentService;

/// Top-level analytics response returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct BillingAnalytics {
    /// Monthly Recurring Revenue — sum of all active subscription monthly prices.
    pub mrr: f64,
    /// Annual Recurring Revenue — MRR × 12.
    pub arr: f64,
    /// Total revenue collected this calendar month (paid invoices).
    pub total_revenue: f64,
    /// Percentage of invoices paid on or before due date (last 90 days).
    pub collection_rate: f64,
    /// Average days from issue → payment (last 90 days of paid invoices).
    pub avg_days_to_pay: f64,
    /// Outstanding invoices grouped by age bracket.
    pub aging: AgingReport,
    /// % of subscriptions cancelled this month vs active at start of month.
    pub churn_rate: f64,
    /// Number of currently active subscriptions.
    pub active_subscriptions: i64,
    /// Total unique customers with at least one subscription (any status).
    pub total_customers: i64,
    /// Revenue per month for the last 6 months (oldest → newest).
    pub revenue_trend: Vec<RevenueTrendPoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgingReport {
    /// 0–30 days past due.
    pub current: f64,
    /// 31–60 days past due.
    pub days_31_60: f64,
    /// 61–90 days past due.
    pub days_61_90: f64,
    /// >90 days past due.
    pub over_90: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevenueTrendPoint {
    /// "YYYY-MM"
    pub month: String,
    pub revenue: f64,
}

/// Compute billing analytics for the given tenant.
pub async fn compute_billing_analytics_for_service(
    service: &PaymentService,
    tenant_id: &str,
) -> AppResult<BillingAnalytics> {
    let now = Utc::now();
    let month_start = now.with_day(1).unwrap().date_naive().and_hms_opt(0, 0, 0).unwrap();
    let pool = &service.pool;

    // ── MRR ────────────────────────────────────────────────────────────
    let mrr: f64 = sqlx::query_scalar(
        r#"SELECT COALESCE(SUM(
               CASE
                 WHEN billing_cycle = 'yearly'  THEN price / 12.0
                 WHEN billing_cycle = 'quarterly' THEN price / 3.0
                 ELSE price
               END
             ), 0.0)::FLOAT8
           FROM customer_subscriptions
           WHERE tenant_id = $1 AND status = 'active'"#,
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;

    let arr = mrr * 12.0;

    // ── Total revenue this month (paid invoices) ──────────────────────
    let total_revenue: f64 = sqlx::query_scalar(
        r#"SELECT COALESCE(SUM(amount), 0.0)::FLOAT8
           FROM invoices
           WHERE tenant_id = $1
             AND status = 'paid'
             AND paid_at >= $2"#,
    )
    .bind(tenant_id)
    .bind(month_start)
    .fetch_one(pool)
    .await?;

    // ── Collection rate (last 90 days) ────────────────────────────────
    let cutoff_90 = (now - chrono::Duration::days(90)).naive_utc();

    let (total_invoices, on_time_invoices): (i64, i64) = sqlx::query_as(
        r#"SELECT
               COUNT(*) AS total,
               COUNT(*) FILTER (WHERE paid_at IS NOT NULL AND paid_at <= due_date) AS on_time
           FROM invoices
           WHERE tenant_id = $1
             AND created_at >= $2
             AND status IN ('paid', 'overdue', 'pending')"#,
    )
    .bind(tenant_id)
    .bind(cutoff_90)
    .fetch_one(pool)
    .await?;

    let collection_rate = if total_invoices > 0 {
        (on_time_invoices as f64 / total_invoices as f64) * 100.0
    } else {
        0.0
    };

    // ── Average days to pay ───────────────────────────────────────────
    let avg_days: f64 = sqlx::query_scalar(
        r#"SELECT COALESCE(AVG(EXTRACT(EPOCH FROM (paid_at - created_at)) / 86400.0), 0.0)::FLOAT8
           FROM invoices
           WHERE tenant_id = $1
             AND status = 'paid'
             AND paid_at IS NOT NULL
             AND created_at >= $2"#,
    )
    .bind(tenant_id)
    .bind(cutoff_90)
    .fetch_one(pool)
    .await?;

    // ── Aging report (overdue invoices) ───────────────────────────────
    let aging_rows = sqlx::query(
        r#"SELECT
               (EXTRACT(EPOCH FROM (NOW() - due_date)) / 86400.0)::FLOAT8 AS days_overdue,
               amount::FLOAT8 AS amount
           FROM invoices
           WHERE tenant_id = $1
             AND status IN ('pending', 'overdue')
             AND due_date < NOW()"#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;

    let (mut current, mut d31_60, mut d61_90, mut over_90) = (0.0, 0.0, 0.0, 0.0);
    for row in &aging_rows {
        let days: f64 = row.get("days_overdue");
        let amt: f64 = row.get("amount");
        if days <= 30.0 {
            current += amt;
        } else if days <= 60.0 {
            d31_60 += amt;
        } else if days <= 90.0 {
            d61_90 += amt;
        } else {
            over_90 += amt;
        }
    }

    // ── Active subscriptions & total customers ────────────────────────
    let active_subscriptions: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM customer_subscriptions
           WHERE tenant_id = $1 AND status = 'active'"#,
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;

    let total_customers: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(DISTINCT customer_id) FROM customer_subscriptions
           WHERE tenant_id = $1"#,
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;

    // ── Churn rate (this month) ───────────────────────────────────────
    let cancelled_this_month: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM customer_subscriptions
           WHERE tenant_id = $1
             AND status = 'cancelled'
             AND updated_at >= $2"#,
    )
    .bind(tenant_id)
    .bind(month_start)
    .fetch_one(pool)
    .await?;

    let active_at_month_start = active_subscriptions + cancelled_this_month;
    let churn_rate = if active_at_month_start > 0 {
        (cancelled_this_month as f64 / active_at_month_start as f64) * 100.0
    } else {
        0.0
    };

    // ── Revenue trend (last 6 months) ─────────────────────────────────
    let trend_rows = sqlx::query(
        r#"SELECT
               TO_CHAR(DATE_TRUNC('month', paid_at), 'YYYY-MM') AS month,
               COALESCE(SUM(amount), 0.0)::FLOAT8 AS revenue
           FROM invoices
           WHERE tenant_id = $1
             AND status = 'paid'
             AND paid_at >= NOW() - INTERVAL '6 months'
           GROUP BY DATE_TRUNC('month', paid_at)
           ORDER BY month ASC"#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;

    let revenue_trend: Vec<RevenueTrendPoint> = trend_rows
        .iter()
        .map(|r| RevenueTrendPoint {
            month: r.get("month"),
            revenue: r.get("revenue"),
        })
        .collect();

    Ok(BillingAnalytics {
        mrr,
        arr,
        total_revenue,
        collection_rate: (collection_rate * 100.0).round() / 100.0,
        avg_days_to_pay: (avg_days * 10.0).round() / 10.0,
        aging: AgingReport {
            current,
            days_31_60: d31_60,
            days_61_90: d61_90,
            over_90,
        },
        churn_rate: (churn_rate * 100.0).round() / 100.0,
        active_subscriptions,
        total_customers,
        revenue_trend,
    })
}
