use crate::services::{
    AuditService, AuthService, CustomerService, DhcpStaticServiceManager, EmailService,
    IspPackageService, ManagedRadiusService, MessageTemplateService, MikrotikService,
    MixradiusImportService, NetworkAssetService, NetworkMappingService, NotificationService,
    OltService, PaymentService, PlanService, PppoeService, RadiusService, RoleService, SettingsService,
    StorageService, SystemService, TeamService, UserService, WhatsappGatewayService,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::{collections::HashMap, time::Instant};
use tokio::sync::RwLock as TokioRwLock;

pub mod announcements;
pub mod announcements_support_common;
pub mod audit;
pub mod auth;
pub mod backup;
pub mod customer_communication;
pub mod customers;
pub mod dhcp_static;
pub mod domain_resolver;
pub mod email_outbox;
pub mod install;
pub mod isp_packages;
pub mod message_templates;
pub mod middleware;
pub mod mikrotik;
pub mod mixradius_import;
pub mod network_assets;
pub mod network_mapping;
pub mod notifications;
pub mod olt;
pub mod payment;
pub mod plans;
pub mod pppoe;
pub mod public;
pub mod registration_approvals;
pub mod roles;
pub mod settings;
pub mod storage;
pub mod superadmin;
pub mod support;
pub mod system;
pub mod technician_location;
pub mod team;
pub mod tenant;
pub mod users;
pub mod websocket;
pub mod whatsapp;
pub mod work_orders;

pub use websocket::{WsEvent, WsHub};

type IpBlockMap = HashMap<String, chrono::DateTime<chrono::Utc>>;
type IpAbuseMap = HashMap<String, (u32, chrono::DateTime<chrono::Utc>)>;

#[derive(Clone, Debug)]
pub struct SecurityRuntimeConfig {
    pub api_rate_limit_per_minute: u32,
    pub enable_ip_blocking: bool,
    pub ip_block_threshold: u32,
    pub ip_block_duration_minutes: i64,
    pub refreshed_at: Instant,
}

// App State to share services with Axum handlers
#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub auth_service: Arc<AuthService>,
    pub user_service: Arc<UserService>,
    pub settings_service: Arc<SettingsService>,
    pub email_service: Arc<EmailService>,
    pub team_service: Arc<TeamService>,
    pub audit_service: Arc<AuditService>,
    pub role_service: Arc<RoleService>,
    pub system_service: Arc<SystemService>,
    pub plan_service: Arc<PlanService>,
    pub storage_service: Arc<StorageService>,
    pub payment_service: Arc<PaymentService>,
    pub notification_service: Arc<NotificationService>,
    pub whatsapp_gateway_service: Arc<WhatsappGatewayService>,
    pub message_template_service: Arc<MessageTemplateService>,
    pub mikrotik_service: Arc<MikrotikService>,
    pub managed_radius_service: Arc<ManagedRadiusService>,
    pub radius_service: Arc<RadiusService>,
    pub mixradius_import_service: Arc<MixradiusImportService>,
    pub customer_service: Arc<CustomerService>,
    pub pppoe_service: Arc<PppoeService>,
    pub dhcp_static_service: Arc<DhcpStaticServiceManager>,
    pub isp_package_service: Arc<IspPackageService>,
    pub network_asset_service: Arc<NetworkAssetService>,
    pub network_mapping_service: Arc<NetworkMappingService>,
    pub olt_service: Arc<OltService>,
    pub backup_service: Arc<crate::services::BackupService>,
    pub ws_hub: Arc<WsHub>,
    pub app_data_dir: PathBuf,
    pub rate_limiter: Arc<crate::services::rate_limiter::RateLimiter>,
    pub metrics_service: Arc<crate::services::metrics_service::MetricsService>,
    pub security_config: Arc<TokioRwLock<SecurityRuntimeConfig>>,
    pub ip_blocklist: Arc<TokioRwLock<IpBlockMap>>,
    pub ip_abuse: Arc<TokioRwLock<IpAbuseMap>>,
}

#[allow(clippy::too_many_arguments)]
pub async fn start_server(
    auth_service: AuthService,
    user_service: UserService,
    settings_service: SettingsService,
    email_service: EmailService,
    team_service: TeamService,
    role_service: RoleService,
    audit_service: AuditService,
    system_service: SystemService,
    plan_service: PlanService,
    storage_service: StorageService,
    payment_service: PaymentService,
    notification_service: NotificationService,
    mikrotik_service: MikrotikService,
    customer_service: CustomerService,
    pppoe_service: PppoeService,
    dhcp_static_service: DhcpStaticServiceManager,
    isp_package_service: IspPackageService,
    network_asset_service: Arc<NetworkAssetService>,
    network_mapping_service: NetworkMappingService,
    backup_service: crate::services::BackupService,
    radius_service: RadiusService,
    ws_hub: Arc<WsHub>,
    app_data_dir: PathBuf,
    default_port: u16,
    pool: crate::db::DbPool,
    metrics_service: Arc<crate::services::metrics_service::MetricsService>,
) {
    crate::bootstrap::http::start_server_impl(
        auth_service,
        user_service,
        settings_service,
        email_service,
        team_service,
        role_service,
        audit_service,
        system_service,
        plan_service,
        storage_service,
        payment_service,
        notification_service,
        mikrotik_service,
        customer_service,
        pppoe_service,
        dhcp_static_service,
        isp_package_service,
        network_asset_service,
        network_mapping_service,
        backup_service,
        radius_service,
        ws_hub,
        app_data_dir,
        default_port,
        pool,
        metrics_service,
    )
    .await;
}

pub(crate) async fn root_handler() -> &'static str {
    "SaaS API is running. Use the frontend to interact."
}

pub(crate) async fn get_app_version() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[cfg(test)]
fn start_server_source() -> &'static str {
    include_str!("mod.rs")
}

#[cfg(test)]
mod tests {
    #[test]
    fn start_server_delegates_http_bootstrap_to_bootstrap_module() {
        let source = super::start_server_source();
        assert!(
            source.contains("crate::bootstrap::http::start_server_impl("),
            "start_server() must delegate bootstrap internals to bootstrap::http::start_server_impl"
        );
    }
}
