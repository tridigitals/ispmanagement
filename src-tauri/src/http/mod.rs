use crate::services::{
    AuditService, AuthService, CustomerService, EmailService, IspPackageService, MikrotikService,
    NotificationService, PaymentService, PlanService, PppoeService, RoleService, SettingsService,
    StorageService, SystemService, TeamService, UserService,
};
use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post, put},
    Router,
};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::RwLock as TokioRwLock;
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    timeout::TimeoutLayer,
};
use tracing::info;

use std::path::PathBuf;
use std::{collections::HashMap, time::Instant};

pub mod announcements;
pub mod audit;
pub mod auth;
pub mod backup;
pub mod customers;
pub mod email_outbox;
pub mod install;
pub mod isp_packages;
pub mod middleware;
pub mod mikrotik;
pub mod notifications;
pub mod payment;
pub mod plans;
pub mod pppoe;
pub mod public;
pub mod roles;
pub mod settings;
pub mod storage;
pub mod superadmin;
pub mod support;
pub mod system;
pub mod team;
pub mod tenant;
pub mod users;
pub mod websocket;
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
    pub mikrotik_service: Arc<MikrotikService>,
    pub customer_service: Arc<CustomerService>,
    pub pppoe_service: Arc<PppoeService>,
    pub isp_package_service: Arc<IspPackageService>,
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
    isp_package_service: IspPackageService,
    backup_service: crate::services::BackupService,
    ws_hub: Arc<WsHub>,
    app_data_dir: PathBuf,
    default_port: u16,
    pool: crate::db::DbPool,
    metrics_service: Arc<crate::services::metrics_service::MetricsService>,
) {
    // Initialize rate limiter
    let rate_limiter = Arc::new(crate::services::rate_limiter::RateLimiter::default());

    // Spawn background task to cleanup expired rate limit entries every minute
    let cleanup_limiter = rate_limiter.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            cleanup_limiter.cleanup();
        }
    });

    let security_config = Arc::new(TokioRwLock::new(SecurityRuntimeConfig {
        api_rate_limit_per_minute: 100,
        enable_ip_blocking: false,
        ip_block_threshold: 5,
        ip_block_duration_minutes: 15,
        refreshed_at: Instant::now(),
    }));
    let ip_blocklist: Arc<TokioRwLock<IpBlockMap>> = Arc::new(TokioRwLock::new(HashMap::new()));
    let ip_abuse: Arc<TokioRwLock<IpAbuseMap>> = Arc::new(TokioRwLock::new(HashMap::new()));

    // Refresh security config from DB every 30 seconds (best-effort, cached).
    {
        let cfg = security_config.clone();
        let settings = settings_service.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;

                let api_rate = settings
                    .get_value(None, "api_rate_limit_per_minute")
                    .await
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse::<u32>().ok())
                    .filter(|v| *v >= 10 && *v <= 10_000)
                    .unwrap_or(100);

                let enable_ip_blocking = settings
                    .get_value(None, "enable_ip_blocking")
                    .await
                    .ok()
                    .flatten()
                    .map(|s| s == "true")
                    .unwrap_or(false);

                let ip_block_threshold = settings
                    .get_value(None, "ip_block_threshold")
                    .await
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse::<u32>().ok())
                    .filter(|v| *v >= 2 && *v <= 100)
                    .unwrap_or(5);

                let ip_block_duration_minutes = settings
                    .get_value(None, "ip_block_duration_minutes")
                    .await
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse::<i64>().ok())
                    .filter(|v| *v >= 1 && *v <= 24 * 60)
                    .unwrap_or(15);

                let mut lock = cfg.write().await;
                lock.api_rate_limit_per_minute = api_rate;
                lock.enable_ip_blocking = enable_ip_blocking;
                lock.ip_block_threshold = ip_block_threshold;
                lock.ip_block_duration_minutes = ip_block_duration_minutes;
                lock.refreshed_at = Instant::now();
            }
        });
    }

    // Cleanup IP blocklist periodically
    {
        let bl = ip_blocklist.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let now = chrono::Utc::now();
                bl.write().await.retain(|_, until| *until > now);
            }
        });
    }

    // Initialize and spawn AlertService for error alerting via email
    let alert_service =
        crate::services::AlertService::new(email_service.clone(), settings_service.clone());
    let alert_metrics = metrics_service.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let metrics = alert_metrics.get_metrics();
            alert_service.check_and_alert(&metrics).await;
        }
    });

    let state = AppState {
        auth_service: Arc::new(auth_service),
        user_service: Arc::new(user_service),
        settings_service: Arc::new(settings_service),
        email_service: Arc::new(email_service),
        team_service: Arc::new(team_service),
        audit_service: Arc::new(audit_service),
        role_service: Arc::new(role_service),
        system_service: Arc::new(system_service),
        plan_service: Arc::new(plan_service.clone()),
        storage_service: Arc::new(storage_service),
        payment_service: Arc::new(payment_service.clone()),
        notification_service: Arc::new(notification_service),
        mikrotik_service: Arc::new(mikrotik_service),
        customer_service: Arc::new(customer_service),
        pppoe_service: Arc::new(pppoe_service),
        isp_package_service: Arc::new(isp_package_service),
        backup_service: Arc::new(backup_service),
        ws_hub,
        app_data_dir,
        rate_limiter,
        metrics_service,
        security_config,
        ip_blocklist,
        ip_abuse,
    };

    // --- Dynamic CORS Implementation ---

    // 1. Create a shared cache for allowed origins
    use std::collections::HashSet;
    use std::sync::RwLock;

    // Initial static origins from env
    let env_origins_str = env::var("CORS_ALLOWED_ORIGINS").unwrap_or_else(|_| {
        "http://localhost:5173,http://localhost:3000,http://localhost:1420,tauri://localhost,http://tauri.localhost,https://tauri.localhost,https://saas.tridigitals.com,https://billing.tridigitals.com".to_string()
    });

    let mut initial_set = HashSet::new();
    for s in env_origins_str.split(',') {
        let clean = s.trim().trim_end_matches('/');
        initial_set.insert(clean.to_string());
    }

    let static_origins = initial_set.clone();
    let cors_cache = Arc::new(RwLock::new(initial_set));

    // 2. Spawn a background task to refresh the cache from DB every 30 seconds
    let cache_for_task = cors_cache.clone();
    let pool_for_task = pool.clone();
    let static_origins_for_task = static_origins.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        let mut warned_missing_schema = false;
        loop {
            interval.tick().await;

            // Re-fetch custom domains
            let rows: Result<Vec<(String,)>, _> = sqlx::query_as("SELECT custom_domain FROM tenants WHERE custom_domain IS NOT NULL AND custom_domain != ''")
                .fetch_all(&pool_for_task)
                .await;

            match rows {
                Ok(domains) => {
                    warned_missing_schema = false;
                    let mut new_custom_domains = static_origins_for_task.clone();
                    for (d,) in domains {
                        let url_str = if d.starts_with("http") {
                            d
                        } else {
                            format!("https://{}", d)
                        };
                        let clean_url = url_str.trim().trim_end_matches('/').to_string();
                        new_custom_domains.insert(clean_url);
                    }

                    // Re-add env origins from runtime value.
                    // If env is missing, keep using the static fallback origins.
                    let env_origins_refresh = env::var("CORS_ALLOWED_ORIGINS")
                        .unwrap_or_else(|_| env_origins_str.clone());
                    for s in env_origins_refresh.split(',') {
                        if !s.trim().is_empty() {
                            let clean = s.trim().trim_end_matches('/');
                            new_custom_domains.insert(clean.to_string());
                        }
                    }

                    // Update the lock
                    if let Ok(mut lock) = cache_for_task.write() {
                        *lock = new_custom_domains;
                        // tracing::info!("CORS Cache Updated. Count: {}", lock.len());
                    }
                }
                Err(e) => {
                    let is_undefined_table = e
                        .as_database_error()
                        .and_then(|db| db.code().map(|c| c == "42P01"))
                        .unwrap_or(false);

                    if is_undefined_table {
                        if !warned_missing_schema {
                            warned_missing_schema = true;
                            tracing::warn!(
                                "CORS domain refresh skipped: database schema not migrated yet (missing tenants table)."
                            );
                        }
                    } else {
                        tracing::error!("Failed to refresh CORS domains: {}", e);
                    }
                }
            }
        }
    });

    // 3. Define the dynamic CORS layer
    let cache_for_layer = cors_cache.clone();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin: &_, _req: &_| {
            if let Ok(origin_str) = origin.to_str() {
                if let Ok(lock) = cache_for_layer.read() {
                    if lock.contains(origin_str) {
                        return true;
                    }
                }
            }
            false
        }))
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router

    let app = Router::new()
        .route("/", get(root_handler))
        // Install Routes
        .route("/api/install/check", get(install::check_installed))
        .route("/api/install", post(install::install_app))
        // Auth Routes
        .route("/api/auth/settings", get(auth::get_auth_settings))
        .route("/api/auth/me", get(auth::get_current_user))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/verify-email", post(auth::verify_email))
        .route("/api/auth/forgot-password", post(auth::forgot_password))
        .route("/api/auth/reset-password", post(auth::reset_password))
        .route("/api/auth/validate", post(auth::validate_token))
        .route("/api/auth/2fa/verify", post(auth::verify_login_2fa))
        .route("/api/auth/2fa/email/request", post(auth::request_email_otp))
        .route("/api/auth/2fa/email/verify", post(auth::verify_email_otp))
        .route("/api/auth/2fa/methods", get(auth::get_2fa_methods))
        // 2FA Setup Routes
        .route("/api/auth/2fa/enable", post(auth::enable_2fa))
        .route("/api/auth/2fa/verify-setup", post(auth::verify_2fa_setup))
        .route("/api/auth/2fa/disable", post(auth::disable_2fa))
        .route(
            "/api/auth/2fa/disable-request",
            post(auth::request_2fa_disable_code),
        )
        .route("/api/auth/2fa/reset/{user_id}", post(auth::reset_user_2fa))
        .route("/api/auth/2fa/preference", post(auth::set_2fa_preference))
        .route(
            "/api/auth/2fa/email/enable-request",
            post(auth::request_email_2fa_setup),
        )
        .route(
            "/api/auth/2fa/email/enable-verify",
            post(auth::verify_email_2fa_setup),
        )
        // Trusted Devices Routes
        .route("/api/auth/trusted-devices", get(auth::list_trusted_devices))
        .route(
            "/api/auth/trusted-devices/{device_id}",
            delete(auth::revoke_trusted_device),
        )
        // User Routes
        .route(
            "/api/users",
            get(users::list_users).post(users::create_user),
        )
        .route(
            "/api/users/me/addresses",
            get(users::list_my_addresses).post(users::create_my_address),
        )
        .route(
            "/api/users/me/addresses/{address_id}",
            put(users::update_my_address).delete(users::delete_my_address),
        )
        .route(
            "/api/users/{id}/addresses",
            get(users::list_user_addresses_admin),
        )
        .route(
            "/api/users/{id}",
            get(users::get_user)
                .put(users::update_user)
                .delete(users::delete_user),
        )
        // Super Admin Routes
        .route(
            "/api/superadmin/tenants",
            get(superadmin::list_tenants).post(superadmin::create_tenant),
        )
        .route(
            "/api/superadmin/tenants/{id}",
            delete(superadmin::delete_tenant).put(superadmin::update_tenant),
        )
        .route("/api/superadmin/audit-logs", get(audit::list_audit_logs))
        .route("/api/admin/audit-logs", get(audit::list_tenant_audit_logs))
        .route("/api/superadmin/system", get(system::get_system_health))
        .route(
            "/api/superadmin/diagnostics",
            get(system::get_system_diagnostics),
        )
        // Support Tickets (tenant scoped; authorization derives tenant from token)
        .route(
            "/api/support/tickets",
            get(support::list_support_tickets).post(support::create_support_ticket),
        )
        .route(
            "/api/support/tickets/stats",
            get(support::get_support_ticket_stats),
        )
        .route(
            "/api/support/tickets/{id}",
            get(support::get_support_ticket).put(support::update_support_ticket),
        )
        .route(
            "/api/support/tickets/{id}/messages",
            post(support::reply_support_ticket),
        )
        // Plans Routes
        .nest("/api/plans", plans::plan_routes())
        // Payment Routes
        .nest("/api/payment", payment::router())
        // Notification Routes
        .nest("/api/notifications", notifications::router())
        // Email Outbox (admin monitor)
        .nest("/api/email-outbox", email_outbox::router())
        // MikroTik routers (tenant admin)
        .nest("/api/admin/mikrotik", mikrotik::router())
        // Announcements (banner + admin broadcast)
        .nest("/api/announcements", announcements::router())
        // Customers + portal (tenant scoped)
        .nest("/api/customers", customers::router())
        // Installation work orders (tenant scoped)
        .nest("/api/admin/work-orders", work_orders::router())
        // PPPoE accounts (tenant scoped)
        .nest("/api/admin/pppoe", pppoe::router())
        // ISP packages + router mapping (tenant scoped)
        .nest("/api/admin/isp-packages", isp_packages::router())
        // Settings Routes
        .route(
            "/api/settings",
            get(settings::get_all_settings).post(settings::upsert_setting),
        )
        .route("/api/settings/public", get(settings::get_public_settings))
        .route(
            "/api/settings/email-verification-readiness",
            get(settings::get_email_verification_readiness),
        )
        .route(
            "/api/settings/logo",
            get(settings::get_logo).post(settings::upload_logo),
        )
        .route("/api/settings/test-email", post(settings::send_test_email))
        .route(
            "/api/settings/test-smtp",
            post(settings::test_smtp_connection),
        )
        .route(
            "/api/settings/{key}",
            get(settings::get_setting).delete(settings::delete_setting),
        )
        .route(
            "/api/settings/{key}/value",
            get(settings::get_setting_value),
        )
        // Team Routes
        .route(
            "/api/team",
            get(team::list_team_members).post(team::add_team_member),
        )
        .route(
            "/api/team/{id}",
            put(team::update_team_member).delete(team::remove_team_member),
        )
        // Tenant Routes
        .route(
            "/api/tenant/me",
            get(tenant::get_current_tenant).put(tenant::update_current_tenant),
        )
        // Roles Routes
        .route(
            "/api/roles",
            get(roles::get_roles).post(roles::create_new_role),
        )
        .route(
            "/api/roles/{id}",
            get(roles::get_role)
                .put(roles::update_existing_role)
                .delete(roles::delete_existing_role),
        )
        .route("/api/permissions", get(roles::get_permissions))
        // WebSocket Route
        .route("/api/ws", get(websocket::ws_handler))
        // Backup Routes
        .nest("/api/backups", backup::router())
        // Storage Routes
        .route("/api/storage/files", get(storage::list_files))
        .route("/api/storage/files/{id}", delete(storage::delete_file))
        .route("/api/storage/files/{id}/content", get(storage::serve_file))
        .route(
            "/api/storage/files/{id}/download",
            get(storage::download_file),
        )
        .route("/api/storage/upload", post(storage::upload_file_http))
        .route("/api/storage/upload/init", post(storage::init_upload))
        .route("/api/storage/upload/chunk", post(storage::upload_chunk))
        .route(
            "/api/storage/upload/complete",
            post(storage::complete_upload),
        )
        // Public Routes
        .route(
            "/api/public/tenant-lookup",
            get(public::lookup_tenant_by_domain),
        )
        .route(
            "/api/public/customer-registration-status",
            get(public::customer_registration_status_by_domain),
        )
        .route(
            "/api/public/customer-invite/validate",
            get(public::validate_customer_registration_invite_by_domain),
        )
        .route(
            "/api/public/customer-register",
            post(public::register_customer_by_domain),
        )
        .route(
            "/api/public/tenants/{slug}",
            get(public::get_tenant_by_slug),
        )
        .route("/api/public/tenant/{slug}", get(public::get_tenant_by_slug))
        .route(
            "/api/public/domains/{domain}",
            get(public::get_tenant_by_domain),
        )
        .route(
            "/api/public/domain/{domain}",
            get(public::get_tenant_by_domain),
        )
        .route("/api/public/unsubscribe/{token}", get(public::unsubscribe))
        // Version Route
        .route("/api/version", get(get_app_version))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 1024)) // 1GB Upload Limit
        .layer({
            #[allow(deprecated)]
            TimeoutLayer::new(Duration::from_secs(3600))
        }) // 1 Hour Timeout for large uploads
        .layer(axum::middleware::from_fn(middleware::metrics_middleware))
        .layer(axum::Extension(state.metrics_service.clone()))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::security_enforcer_middleware,
        ))
        .layer(axum::middleware::from_fn(
            middleware::security_headers_middleware,
        ))
        .layer(cors)
        .with_state(state);

    // Determine port
    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(default_port);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("HTTP API listening on {}", addr);

    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(
                "Failed to bind to {}: {}. Is another instance running?",
                addr,
                e
            );
            return;
        }
    };

    if let Err(e) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    {
        tracing::error!("HTTP API server error: {}", e);
    }
}

async fn root_handler() -> &'static str {
    "SaaS API is running. Use the frontend to interact."
}

async fn get_app_version() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION")
    }))
}
