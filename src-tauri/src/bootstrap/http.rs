use crate::models::tenant::CUSTOM_DOMAIN_STATUS_ACTIVE;
use crate::services::{
    AuditService, AuthService, CustomerService, DhcpStaticServiceManager, EmailService,
    IspPackageService, ManagedRadiusService, MessageTemplateService, MikrotikService,
    MixradiusImportService, NetworkAssetService, NetworkMappingService, NotificationService,
    PaymentService, PlanService, PppoeService, RadiusService, RoleService, SettingsService,
    StorageService, SystemService, TeamService, UserService,
};
use axum::{
    extract::DefaultBodyLimit,
    http::header::{HeaderName, ACCEPT, AUTHORIZATION, CONTENT_TYPE, ORIGIN, USER_AGENT},
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

use crate::http::{
    announcements, audit, auth, backup, customer_communication, customers, dhcp_static,
    email_outbox, install, isp_packages, message_templates, middleware, mikrotik, mixradius_import,
    network_assets, network_mapping, notifications, payment, plans, pppoe, public, roles, settings, storage,
    superadmin, support, system, team, tenant, users, websocket, whatsapp, work_orders, AppState,
    SecurityRuntimeConfig, WsHub,
};

type IpBlockMap = HashMap<String, chrono::DateTime<chrono::Utc>>;
type IpAbuseMap = HashMap<String, (u32, chrono::DateTime<chrono::Utc>)>;

fn build_runtime_cors_origins(
    static_origins: &std::collections::HashSet<String>,
    env_origins: &str,
    custom_domains: &[(String, Option<String>)],
) -> std::collections::HashSet<String> {
    let mut origins = static_origins.clone();

    for (domain, status) in custom_domains {
        if status.as_deref() != Some(CUSTOM_DOMAIN_STATUS_ACTIVE) {
            continue;
        }

        let clean = domain.trim().trim_end_matches('/');
        if clean.is_empty() {
            continue;
        }

        let url = if clean.starts_with("http://") || clean.starts_with("https://") {
            clean.to_string()
        } else {
            format!("https://{clean}")
        };
        origins.insert(url);
    }

    for origin in env_origins.split(',') {
        let clean = origin.trim().trim_end_matches('/');
        if !clean.is_empty() {
            origins.insert(clean.to_string());
        }
    }

    origins
}

#[allow(clippy::too_many_arguments)]
pub async fn start_server_impl(
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
    network_asset_service: NetworkAssetService,
    network_mapping_service: NetworkMappingService,
    backup_service: crate::services::BackupService,
    radius_service: RadiusService,
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
        api_rate_limit_per_minute: 300,
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
                    .unwrap_or(300);

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
        whatsapp_gateway_service: Arc::new(crate::services::WhatsappGatewayService::new(
            pool.clone(),
            settings_service.clone(),
        )),
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
        message_template_service: Arc::new(MessageTemplateService::new(pool.clone())),
        managed_radius_service: Arc::new(ManagedRadiusService::new(pool.clone())),
        radius_service: Arc::new(radius_service),
        mixradius_import_service: Arc::new(MixradiusImportService::new(pool.clone())),
        customer_service: Arc::new(customer_service),
        pppoe_service: Arc::new(pppoe_service),
        dhcp_static_service: Arc::new(dhcp_static_service),
        isp_package_service: Arc::new(isp_package_service),
        network_asset_service: Arc::new(network_asset_service),
        network_mapping_service: Arc::new(network_mapping_service),
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
            let rows: Result<Vec<(String, Option<String>)>, _> = sqlx::query_as("SELECT custom_domain, custom_domain_status FROM tenants WHERE custom_domain IS NOT NULL AND custom_domain != '' AND is_active = true")
                .fetch_all(&pool_for_task)
                .await;

            match rows {
                Ok(domains) => {
                    warned_missing_schema = false;
                    let env_origins_refresh = env::var("CORS_ALLOWED_ORIGINS")
                        .unwrap_or_else(|_| env_origins_str.clone());
                    let new_custom_domains = build_runtime_cors_origins(
                        &static_origins_for_task,
                        &env_origins_refresh,
                        &domains,
                    );

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
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            ACCEPT,
            ORIGIN,
            USER_AGENT,
            HeaderName::from_static("x-requested-with"),
            HeaderName::from_static("x-csrf-token"),
            HeaderName::from_static("x-request-id"),
        ])
        .expose_headers([
            HeaderName::from_static("content-disposition"),
            HeaderName::from_static("x-request-id"),
        ]);

    // Build router

    let app = Router::new()
        .route("/", get(crate::http::root_handler))
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
        .route(
            "/api/superadmin/tenants/{id}/domain-status",
            post(superadmin::update_tenant_domain_status),
        )
        .route(
            "/api/superadmin/radius/runtime",
            get(superadmin::get_managed_radius_runtime_status),
        )
        .route(
            "/api/superadmin/radius/servers",
            get(superadmin::list_managed_radius_servers)
                .post(superadmin::create_managed_radius_server),
        )
        .route(
            "/api/superadmin/radius/servers/{id}",
            put(superadmin::update_managed_radius_server),
        )
        .route(
            "/api/superadmin/radius/servers/{id}/active",
            post(superadmin::set_managed_radius_server_active),
        )
        .route(
            "/api/superadmin/radius/servers/{id}/default",
            post(superadmin::set_managed_radius_server_default),
        )
        .route(
            "/api/superadmin/radius/assignments",
            get(superadmin::list_managed_radius_assignments)
                .post(superadmin::create_managed_radius_assignment),
        )
        .route(
            "/api/superadmin/radius/assignments/{id}",
            put(superadmin::update_managed_radius_assignment),
        )
        .route(
            "/api/superadmin/radius/assignments/{id}/active",
            post(superadmin::set_managed_radius_assignment_active),
        )
        .route(
            "/api/superadmin/radius/mappings",
            get(superadmin::list_managed_radius_mappings)
                .post(superadmin::create_managed_radius_mapping),
        )
        .route(
            "/api/superadmin/radius/mappings/{id}",
            put(superadmin::update_managed_radius_mapping),
        )
        .route(
            "/api/superadmin/radius/mappings/{id}/active",
            post(superadmin::set_managed_radius_mapping_active),
        )
        .route(
            "/api/superadmin/radius/mappings/{id}/secret/rotate",
            post(superadmin::rotate_managed_radius_mapping_secret),
        )
        .route(
            "/api/superadmin/radius/mappings/{id}/secret/reveal",
            post(superadmin::reveal_managed_radius_mapping_secret),
        )
        .route(
            "/api/superadmin/radius/users",
            get(superadmin::list_managed_radius_users),
        )
        .route(
            "/api/superadmin/radius/sessions",
            get(superadmin::list_managed_radius_sessions),
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
        // WhatsApp gateway settings/test delivery
        .route("/api/whatsapp/events", get(whatsapp::list_events))
        .route("/api/whatsapp/readiness", get(whatsapp::readiness))
        .route("/api/whatsapp/customer-send", post(whatsapp::customer_send))
        .route("/api/whatsapp/test-send", post(whatsapp::test_send))
        .route(
            "/api/customer-communication/email-send",
            post(customer_communication::send_customer_email),
        )
        // Communication templates
        .nest("/api/message-templates", message_templates::router())
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
        // DHCP static services (tenant scoped)
        .nest("/api/admin/dhcp-static", dhcp_static::router())
        // MixRadius import wizard (tenant scoped)
        .nest("/api/admin/pppoe/mixradius", mixradius_import::router())
        // ISP packages + router mapping (tenant scoped)
        .nest("/api/admin/isp-packages", isp_packages::router())
        // FTTH network assets (tenant scoped)
        .nest("/api/admin/network-assets", network_assets::router())
        // Network topology mapping (tenant scoped)
        .nest("/api/admin/network-mapping", network_mapping::router())
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
        .route("/api/version", get(crate::http::get_app_version))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 1024)) // 1GB Upload Limit
        .layer({
            #[allow(deprecated)]
            TimeoutLayer::new(Duration::from_secs(3600))
        }) // 1 Hour Timeout for large uploads
        .layer(axum::middleware::from_fn(middleware::metrics_middleware))
        .layer(axum::middleware::from_fn(
            middleware::correlation_id_middleware,
        ))
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

#[cfg(feature = "desktop")]
#[allow(clippy::too_many_arguments)]
pub fn spawn_http_server(
    auth_service: crate::services::AuthService,
    user_service: crate::services::UserService,
    settings_service: crate::services::SettingsService,
    email_service: crate::services::EmailService,
    team_service: crate::services::TeamService,
    role_service: crate::services::RoleService,
    audit_service: crate::services::AuditService,
    system_service: crate::services::SystemService,
    plan_service: crate::services::PlanService,
    storage_service: crate::services::StorageService,
    payment_service: crate::services::PaymentService,
    notification_service: crate::services::NotificationService,
    mikrotik_service: crate::services::MikrotikService,
    customer_service: crate::services::CustomerService,
    pppoe_service: crate::services::PppoeService,
    dhcp_static_service: crate::services::DhcpStaticServiceManager,
    isp_package_service: crate::services::IspPackageService,
    network_asset_service: crate::services::NetworkAssetService,
    network_mapping_service: crate::services::NetworkMappingService,
    backup_service: crate::services::BackupService,
    radius_service: crate::services::RadiusService,
    ws_hub: std::sync::Arc<crate::http::WsHub>,
    app_data_dir: std::path::PathBuf,
    default_port: u16,
    pool: crate::db::DbPool,
    metrics_service: std::sync::Arc<crate::services::metrics_service::MetricsService>,
) {
    tauri::async_runtime::spawn(async move {
        crate::http::start_server(
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
    });
}

#[cfg(test)]
fn bootstrap_http_source() -> &'static str {
    include_str!("http.rs")
}

#[cfg(test)]
fn router_build_chain_source() -> &'static str {
    let source = bootstrap_http_source();
    let start = source
        .find("let app = Router::new()")
        .expect("start_server_impl() must construct the router");
    let tail = &source[start..];
    let end = tail
        .find("// Determine port")
        .expect("start_server_impl() must determine port after router build");

    &tail[..end]
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::models::tenant::{CUSTOM_DOMAIN_STATUS_ACTIVE, CUSTOM_DOMAIN_STATUS_PENDING};

    use super::build_runtime_cors_origins;

    fn assert_relative_order(source: &str, first: &str, second: &str) {
        let first_index = source
            .find(first)
            .unwrap_or_else(|| panic!("expected snippet not found: {first}"));
        let second_index = source
            .find(second)
            .unwrap_or_else(|| panic!("expected snippet not found: {second}"));

        assert!(
            first_index < second_index,
            "expected `{first}` to appear before `{second}` in bootstrap/http router chain"
        );
    }

    #[test]
    fn start_server_impl_builds_routes_before_layer_stack_and_state() {
        let router_chain = super::router_build_chain_source();

        assert_relative_order(
            router_chain,
            ".route(\"/api/version\", get(crate::http::get_app_version))",
            ".layer(DefaultBodyLimit::max(1024 * 1024 * 1024))",
        );
        assert_relative_order(
            router_chain,
            ".layer(DefaultBodyLimit::max(1024 * 1024 * 1024))",
            "TimeoutLayer::new(Duration::from_secs(3600))",
        );
        assert_relative_order(
            router_chain,
            "TimeoutLayer::new(Duration::from_secs(3600))",
            "middleware::metrics_middleware",
        );
        assert_relative_order(
            router_chain,
            "middleware::metrics_middleware",
            "middleware::correlation_id_middleware",
        );
        assert_relative_order(
            router_chain,
            "middleware::correlation_id_middleware",
            "middleware::security_enforcer_middleware",
        );
        assert_relative_order(
            router_chain,
            "middleware::security_enforcer_middleware",
            "middleware::security_headers_middleware",
        );
        assert_relative_order(
            router_chain,
            "middleware::security_headers_middleware",
            ".layer(cors)",
        );
        assert_relative_order(router_chain, ".layer(cors)", ".with_state(state);");
    }

    #[test]
    fn start_server_impl_keeps_thin_adapter_mount_points_in_main_router() {
        let router_chain = super::router_build_chain_source();

        for mount in [
            ".nest(\"/api/plans\", plans::plan_routes())",
            ".nest(\"/api/payment\", payment::router())",
            ".nest(\"/api/notifications\", notifications::router())",
            ".nest(\"/api/admin/network-assets\", network_assets::router())",
            ".nest(\"/api/admin/network-mapping\", network_mapping::router())",
            ".nest(\"/api/backups\", backup::router())",
        ] {
            assert!(
                router_chain.contains(mount),
                "expected router chain to include mount: {mount}"
            );
        }
    }

    #[test]
    fn spawn_http_server_wraps_http_start_server_in_background_runtime_task() {
        let source = super::bootstrap_http_source();

        assert_relative_order(
            source,
            "tauri::async_runtime::spawn(async move {",
            "crate::http::start_server(",
        );

        let start_server_index = source
            .find("crate::http::start_server(")
            .expect("expected start_server invocation in spawn_http_server");
        let tail = &source[start_server_index..];
        assert!(
            tail.contains(".await;"),
            "expected start_server invocation in spawn_http_server to be awaited"
        );
    }

    #[test]
    fn cors_domain_filters_out_non_active_statuses() {
        let static_origins = HashSet::from(["https://billing.acme.net".to_string()]);
        let origins = build_runtime_cors_origins(
            &static_origins,
            "",
            &[
                (
                    "portal.customer.net".to_string(),
                    Some(CUSTOM_DOMAIN_STATUS_ACTIVE.to_string()),
                ),
                (
                    "pending.customer.net".to_string(),
                    Some(CUSTOM_DOMAIN_STATUS_PENDING.to_string()),
                ),
                (
                    "failed.customer.net".to_string(),
                    Some("failed".to_string()),
                ),
            ],
        );

        assert!(origins.contains("https://billing.acme.net"));
        assert!(origins.contains("https://portal.customer.net"));
        assert!(!origins.contains("https://pending.customer.net"));
        assert!(!origins.contains("https://failed.customer.net"));
    }

    #[test]
    fn cors_domain_preserves_runtime_env_origins() {
        let static_origins = HashSet::from(["https://billing.acme.net".to_string()]);
        let origins = build_runtime_cors_origins(
            &static_origins,
            "https://ops.acme.net,http://localhost:5173/",
            &[],
        );

        assert!(origins.contains("https://billing.acme.net"));
        assert!(origins.contains("https://ops.acme.net"));
        assert!(origins.contains("http://localhost:5173"));
    }
}
