#[derive(Debug, Clone)]
pub struct ListQuery {
    pub q: Option<String>,
    pub page: u32,
    pub per_page: u32,
    pub status: Option<String>,
    pub kind: Option<String>,
    pub bbox: Option<(f64, f64, f64, f64)>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct NodeStatusRow {
    pub(super) id: String,
    pub(super) status: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct PathLinkRow {
    pub(super) id: String,
    pub(super) from_node_id: String,
    pub(super) to_node_id: String,
    pub(super) name: String,
    pub(super) link_type: String,
    pub(super) status: String,
    pub(super) distance_m: f64,
    pub(super) utilization_pct: Option<f64>,
    pub(super) loss_db: Option<f64>,
    pub(super) latency_ms: Option<f64>,
}

#[derive(Debug, Clone)]
pub(super) struct PathEdge {
    pub(super) link_id: String,
    pub(super) from_node_id: String,
    pub(super) to_node_id: String,
    pub(super) name: String,
    pub(super) link_type: String,
    pub(super) status: String,
    pub(super) distance_m: f64,
    pub(super) cost: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct CandidateNodeRow {
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

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct UuidTextRow {
    pub(super) id: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct SyncRouterRow {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) enabled: bool,
    pub(super) latitude: f64,
    pub(super) longitude: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct SyncCustomerLocationRow {
    pub(super) location_id: String,
    pub(super) customer_id: String,
    pub(super) customer_name: String,
    pub(super) customer_is_active: bool,
    pub(super) label: String,
    pub(super) subscription_id: String,
    pub(super) subscription_status: String,
    pub(super) package_name: Option<String>,
    pub(super) package_service_type: Option<String>,
    pub(super) pppoe_username: Option<String>,
    pub(super) pppoe_disabled: Option<bool>,
    pub(super) pppoe_session_active: Option<bool>,
    pub(super) pppoe_account_source: Option<String>,
    pub(super) pppoe_router_profile_name: Option<String>,
    pub(super) router_id: Option<String>,
    pub(super) latitude: f64,
    pub(super) longitude: f64,
}

#[derive(Debug, Clone)]
pub(super) struct SnappedPolylinePoint {
    pub(super) lng: f64,
    pub(super) lat: f64,
    pub(super) segment_index: usize,
    pub(super) t: f64,
    pub(super) distance_sq: f64,
}
