#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct AssignmentSubscriptionRef {
    pub(super) customer_id: String,
    pub(super) location_id: String,
    pub(super) router_id: Option<String>,
    pub(super) latitude: Option<f64>,
    pub(super) longitude: Option<f64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct AssignmentCandidateNodeRow {
    pub(super) node_id: String,
    pub(super) name: String,
    pub(super) node_type: String,
    pub(super) status: String,
    pub(super) capacity_json: serde_json::Value,
    pub(super) health_json: serde_json::Value,
    pub(super) distance_m: Option<f64>,
    pub(super) avg_link_utilization_pct: Option<f64>,
    pub(super) down_links: i64,
    pub(super) link_count: i64,
}
