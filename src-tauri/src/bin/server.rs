use saas_tauri_lib::{
    db::connection::{init_db, seed_defaults},
    http::{self, WsHub},
    services::backup::BackupScheduler,
    services::{
        metrics_service::MetricsService, AnnouncementScheduler, AuditService, AuthService,
        BackupService, EmailOutboxService, EmailService, NotificationService, PaymentService,
        PlanService, RoleService, SettingsService, StorageService, SystemService, TeamService,
        UserService, MikrotikService,
    },
};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    info!("Starting SaaS Standalone Server...");

    // 2. Load .env
    dotenvy::dotenv().ok();

    // 3. Database Setup
    // For the server, we don't use AppData. We use the current directory or a configured path.
    // However, init_db expects a "base path" to create the SQLite DB file IF using SQLite.
    // But for a VPS, you typically use Postgres.
    // If Postgres is used, the path argument to init_db might be ignored for the DB connection itself,
    // but it might be used for other file storage?
    // Let's check init_db implementation later if needed. For now, we pass current dir.
    let app_data_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    info!("Initializing database connection...");
    let pool = init_db(app_data_dir.clone()).await?;
    info!("Database initialized.");

    // 4. Seed Defaults
    seed_defaults(&pool).await?;

    // 5. Initialize Services (Copied logic from lib.rs)
    let plan_service = PlanService::new(pool.clone());
    let audit_service = AuditService::new(pool.clone(), Some(plan_service.clone()));
    let role_service = RoleService::new(pool.clone(), audit_service.clone());

    // Seed RBAC
    role_service.seed_permissions().await?;
    role_service.seed_roles().await?;

    // JWT Secret
    let jwt_secret = sqlx::query_scalar::<_, String>(
        "SELECT value FROM settings WHERE key = 'jwt_secret' AND tenant_id IS NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

    let settings_service = SettingsService::new(pool.clone(), audit_service.clone());
    let email_service = EmailService::new(settings_service.clone());
    let auth_service = AuthService::new(
        pool.clone(),
        jwt_secret,
        email_service.clone(),
        audit_service.clone(),
        settings_service.clone(),
    );
    let user_service = UserService::new(pool.clone(), audit_service.clone());
    let team_service = TeamService::new(
        pool.clone(),
        auth_service.clone(),
        audit_service.clone(),
        plan_service.clone(),
    );
    let metrics_service = Arc::new(MetricsService::new());
    let system_service = SystemService::new(pool.clone(), metrics_service.clone());
    // Use a specific "storage" folder for uploads on the server
    let storage_dir = app_data_dir.join("storage");
    if !storage_dir.exists() {
        std::fs::create_dir_all(&storage_dir)?;
    }
    let storage_service = StorageService::new(pool.clone(), plan_service.clone(), storage_dir);

    let ws_hub = Arc::new(WsHub::new());
    let email_outbox_service = EmailOutboxService::new(
        pool.clone(),
        settings_service.clone(),
        email_service.clone(),
    );
    email_outbox_service.start_sender().await;
    let notification_service =
        NotificationService::new(pool.clone(), ws_hub.clone(), email_outbox_service.clone());
    let payment_service = PaymentService::new(pool.clone(), notification_service.clone());
    let backup_service = BackupService::new(pool.clone(), app_data_dir.clone());

    // MikroTik monitoring (tenant-scoped)
    let mikrotik_service = MikrotikService::new(pool.clone(), notification_service.clone());
    Arc::new(mikrotik_service.clone()).start_poller();

    // Scheduled broadcasts -> notifications
    let announcement_scheduler = AnnouncementScheduler::new(
        pool.clone(),
        notification_service.clone(),
        audit_service.clone(),
    );
    announcement_scheduler.start().await;

    let scheduler = BackupScheduler::new(
        pool.clone(),
        backup_service.clone(),
        settings_service.clone(),
    );
    scheduler.start().await;

    plan_service.seed_default_features().await?;

    // 6. Start HTTP Server
    // Default to port 3000 if PORT env not set
    http::start_server(
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
        backup_service,
        ws_hub,
        app_data_dir,
        3000,
        pool,
        metrics_service,
    )
    .await;

    Ok(())
}
