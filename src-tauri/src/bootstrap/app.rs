#[cfg(feature = "desktop")]
use crate::db::connection::{init_db, seed_defaults};
#[cfg(feature = "desktop")]
use crate::services::backup::BackupScheduler;
#[cfg(feature = "desktop")]
use crate::services::metrics_service::MetricsService;
#[cfg(feature = "desktop")]
use crate::services::{
    AnnouncementScheduler, AuditService, AuthService, BackupService, CustomerService,
    DhcpStaticServiceManager, EmailOutboxService, EmailService, IspPackageService,
    ManagedRadiusService, MikrotikService, NetworkMappingService, NotificationService,
    PaymentService, PlanService, PppoeService, RadiusRuntimeConfig, RadiusService, RoleService,
    SettingsService, SystemService, TeamService, UserService,
};
#[cfg(feature = "desktop")]
use tauri::Manager;
#[cfg(feature = "desktop")]
use tracing::info;

#[cfg(feature = "desktop")]
pub async fn initialize_backend<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    app_data_dir: std::path::PathBuf,
) -> Result<(), String> {
    // Initialize database
    info!("Attempting to initialize database...");
    let pool = init_db(app_data_dir.clone())
        .await
        .map_err(|e| format!("Failed to initialize database: {}", e))?;
    info!("Database initialized.");

    // Seed default settings
    seed_defaults(&pool)
        .await
        .map_err(|e| format!("Failed to seed default settings: {}", e))?;
    info!("Default settings seeded.");

    // Create services - AuditService must be first
    let plan_service = PlanService::new(pool.clone());
    let audit_service = AuditService::new(pool.clone(), Some(plan_service.clone()));
    // RoleService needs AuditService
    let role_service = RoleService::new(pool.clone(), audit_service.clone());

    // Seed RBAC permissions and roles using RoleService instance
    role_service
        .seed_permissions()
        .await
        .map_err(|e| format!("Failed to seed permissions: {}", e))?;
    role_service
        .seed_roles()
        .await
        .map_err(|e| format!("Failed to seed roles: {}", e))?;
    info!("RBAC permissions and roles seeded.");

    // Get JWT secret from settings
    let jwt_secret = sqlx::query_scalar::<_, String>(
        "SELECT value FROM settings WHERE key = 'jwt_secret' AND tenant_id IS NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());
    info!("JWT Secret loaded.");

    // Initialize App Data Dir for Storage
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or(std::path::PathBuf::from("app_data"));

    let settings_service = SettingsService::new(pool.clone(), audit_service.clone());
    let email_service = EmailService::new(settings_service.clone());
    let managed_radius_service = ManagedRadiusService::new(pool.clone());
    let radius_service = RadiusService::new(pool.clone(), RadiusRuntimeConfig::from_env());
    let auth_service = AuthService::new(
        pool.clone(),
        jwt_secret,
        email_service.clone(),
        audit_service.clone(),
        settings_service.clone(),
    );
    let user_service = UserService::new(pool.clone(), audit_service.clone());
    let pppoe_service = PppoeService::new(
        pool.clone(),
        auth_service.clone(),
        audit_service.clone(),
        settings_service.clone(),
    );
    let dhcp_static_service =
        DhcpStaticServiceManager::new(pool.clone(), auth_service.clone(), audit_service.clone());
    let isp_package_service =
        IspPackageService::new(pool.clone(), auth_service.clone(), audit_service.clone());
    let network_mapping_service = NetworkMappingService::new(pool.clone(), auth_service.clone());
    let team_service = TeamService::new(
        pool.clone(),
        auth_service.clone(),
        audit_service.clone(),
        plan_service.clone(),
    );
    let metrics_service = std::sync::Arc::new(MetricsService::new());
    let system_service = SystemService::new(pool.clone(), metrics_service.clone());
    let storage_service = crate::services::StorageService::new(
        pool.clone(),
        plan_service.clone(),
        app_data_dir.clone(),
    );
    let backup_service = BackupService::new(pool.clone(), app_data_dir.clone());

    // Start Backup Scheduler
    let scheduler = BackupScheduler::new(
        pool.clone(),
        backup_service.clone(),
        settings_service.clone(),
    );
    scheduler.start().await;

    // Create WebSocket hub for real-time sync (shared between HTTP and Tauri)
    let ws_hub = std::sync::Arc::new(crate::http::WsHub::new());

    let email_outbox_service = EmailOutboxService::new(
        pool.clone(),
        settings_service.clone(),
        email_service.clone(),
    );
    email_outbox_service.start_sender().await;

    let whatsapp_gateway_service =
        crate::services::WhatsappGatewayService::new(pool.clone(), settings_service.clone());
    let message_template_service = crate::services::MessageTemplateService::new(pool.clone());
    let notification_service = NotificationService::new_with_whatsapp(
        pool.clone(),
        ws_hub.clone(),
        email_outbox_service.clone(),
        whatsapp_gateway_service.clone(),
    );
    let customer_service = CustomerService::new(
        pool.clone(),
        auth_service.clone(),
        audit_service.clone(),
        notification_service.clone(),
        pppoe_service.clone(),
        user_service.clone(),
    );
    customer_service.start_installation_sla_scheduler();
    let payment_service = PaymentService::new(
        pool.clone(),
        notification_service.clone(),
        pppoe_service.clone(),
    );
    payment_service.start_customer_invoice_scheduler();

    // MikroTik monitoring (tenant-scoped)
    let mikrotik_service = MikrotikService::new(
        pool.clone(),
        notification_service.clone(),
        audit_service.clone(),
        settings_service.clone(),
    );
    std::sync::Arc::new(mikrotik_service.clone()).start_poller();

    // Start Announcement Scheduler (scheduled broadcasts -> notifications)
    let announcement_scheduler = AnnouncementScheduler::new(
        pool.clone(),
        notification_service.clone(),
        audit_service.clone(),
    );
    announcement_scheduler.start().await;
    radius_service.start().await?;

    // Seed default features
    plan_service
        .seed_default_features()
        .await
        .map_err(|e| format!("Failed to seed default features: {}", e))?;
    info!("Default features seeded.");

    // Manage state - Crucial: This must happen before setup returns
    app_handle.manage(auth_service.clone());
    app_handle.manage(user_service.clone());
    app_handle.manage(customer_service.clone());
    app_handle.manage(pppoe_service.clone());
    app_handle.manage(dhcp_static_service.clone());
    app_handle.manage(isp_package_service.clone());
    app_handle.manage(network_mapping_service.clone());
    app_handle.manage(settings_service.clone());
    app_handle.manage(email_service.clone());
    app_handle.manage(team_service.clone());
    app_handle.manage(audit_service.clone());
    app_handle.manage(role_service.clone());
    app_handle.manage(system_service.clone());
    app_handle.manage(plan_service.clone());
    app_handle.manage(storage_service.clone());
    app_handle.manage(backup_service.clone());
    app_handle.manage(payment_service.clone());
    app_handle.manage(notification_service.clone());
    app_handle.manage(whatsapp_gateway_service.clone());
    app_handle.manage(message_template_service.clone());
    app_handle.manage(email_outbox_service.clone());
    app_handle.manage(mikrotik_service.clone());
    app_handle.manage(managed_radius_service.clone());
    app_handle.manage(radius_service.clone());
    app_handle.manage(ws_hub.clone());
    app_handle.manage(metrics_service.clone());
    info!("Services added to Tauri state.");

    // Start HTTP Server (This can run in background)
    let app_dir = app_data_dir.clone();
    crate::bootstrap::http::spawn_http_server(
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
        network_mapping_service,
        backup_service,
        radius_service,
        ws_hub,
        app_dir,
        3000,
        pool.clone(),
        metrics_service,
    );

    info!("Services initialized successfully");
    Ok(())
}

#[cfg(test)]
fn initialize_backend_source() -> &'static str {
    include_str!("app.rs")
}

#[cfg(test)]
mod tests {
    fn assert_relative_order(source: &str, first: &str, second: &str) {
        let first_index = source
            .find(first)
            .unwrap_or_else(|| panic!("expected snippet not found: {first}"));
        let second_index = source
            .find(second)
            .unwrap_or_else(|| panic!("expected snippet not found: {second}"));

        assert!(
            first_index < second_index,
            "expected `{first}` to appear before `{second}` in initialize_backend()"
        );
    }

    #[test]
    fn initialize_backend_initializes_database_before_seeding_and_rbac() {
        let bootstrap = super::initialize_backend_source();

        assert_relative_order(bootstrap, "let pool = init_db(", "seed_defaults(&pool)");
        assert_relative_order(
            bootstrap,
            "seed_defaults(&pool)",
            "role_service\n        .seed_permissions()",
        );
        assert_relative_order(
            bootstrap,
            "role_service\n        .seed_permissions()",
            "role_service\n        .seed_roles()",
        );
    }

    #[test]
    fn initialize_backend_starts_http_only_after_scheduler_and_state_management_contracts() {
        let bootstrap = super::initialize_backend_source();

        assert_relative_order(
            bootstrap,
            "scheduler.start().await;",
            "crate::bootstrap::http::spawn_http_server(",
        );
        assert_relative_order(
            bootstrap,
            "email_outbox_service.start_sender().await;",
            "crate::bootstrap::http::spawn_http_server(",
        );
        assert_relative_order(
            bootstrap,
            "announcement_scheduler.start().await;",
            "radius_service.start().await?;",
        );
        assert_relative_order(
            bootstrap,
            "radius_service.start().await?;",
            "crate::bootstrap::http::spawn_http_server(",
        );
        assert_relative_order(
            bootstrap,
            "app_handle.manage(auth_service.clone());",
            "app_handle.manage(radius_service.clone());",
        );
        assert_relative_order(
            bootstrap,
            "app_handle.manage(radius_service.clone());",
            "app_handle.manage(metrics_service.clone());",
        );
        assert_relative_order(
            bootstrap,
            "app_handle.manage(metrics_service.clone());",
            "crate::bootstrap::http::spawn_http_server(",
        );
    }
}
