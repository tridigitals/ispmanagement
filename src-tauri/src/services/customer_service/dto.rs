use chrono::{DateTime, Utc};

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct OverdueInstallationReminderRow {
    pub(super) work_order_id: String,
    pub(super) status: String,
    pub(super) scheduled_at: Option<DateTime<Utc>>,
    pub(super) created_at: DateTime<Utc>,
    pub(super) customer_name: Option<String>,
    pub(super) location_label: Option<String>,
    pub(super) package_name: Option<String>,
}

#[derive(sqlx::FromRow)]
pub(super) struct InviteSummaryRow {
    pub(super) total: i64,
    pub(super) active: i64,
    pub(super) revoked: i64,
    pub(super) expired: i64,
    pub(super) used_up: i64,
    pub(super) total_uses: i64,
    pub(super) total_capacity: i64,
    pub(super) created_last_30d: i64,
    pub(super) used_last_30d: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct LifecycleStageRow {
    pub(super) stage: String,
    pub(super) count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct AgingBucketRow {
    pub(super) bucket: String,
    pub(super) count: i64,
}
