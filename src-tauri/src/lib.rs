//! SaaS Boilerplate - Main Library Entry Point (Rebuild Triggered)
//!
//! This is the core library for the Tauri application.
//! It wires together all modules: database, services, and commands.

pub mod commands;
pub mod db;
pub mod error;
pub mod http;
pub mod models;
pub mod services;

use commands::audit::list_audit_logs;
use commands::*;
use db::connection::{init_db, seed_defaults};
use services::backup::BackupScheduler;
use services::metrics_service::MetricsService;
use services::{
    AnnouncementScheduler, AuditService, AuthService, BackupService, EmailOutboxService, EmailService,
    NotificationService, PaymentService,
    PlanService, RoleService, SettingsService, SystemService, TeamService, UserService,
};
use tauri::Manager;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging
fn init_logging() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("saas_tauri=debug".parse().unwrap()),
        )
        .init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(unknown_lints)]
#[allow(clippy::all)]
pub fn run() {
    init_logging();
    info!("Starting Application");

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init());

    // Only enable single-instance in production to allow dev and prod to run simultaneously
    #[cfg(not(debug_assertions))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app
                .get_webview_window("main")
                .expect("no main window")
                .set_focus();
        }));
    }

    builder
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Get app data directory

            // Force Open DevTools (Debugging Production Issue)
            if let Some(window) = app.get_webview_window("main") {
                window.open_devtools();
            }

            let app_data_dir = match app_handle.path().app_data_dir() {
                Ok(path) => path,
                Err(e) => {
                    let err = format!("Failed to get app data directory: {}", e);
                    #[cfg(windows)] show_error_dialog(&err);
                    return Err(err.into());
                }
            };

            // Create app data directory if it doesn't exist
            if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
                let err = format!("Failed to create app data directory at {:?}: {}", app_data_dir, e);
                #[cfg(windows)] show_error_dialog(&err);
                return Err(err.into());
            }

            info!("App data directory: {:?}", app_data_dir);

            info!("App data directory: {:?}", app_data_dir);

            // =========================================================
            // CONFIGURATION LOADING STRATEGY
            // =========================================================

            #[cfg(debug_assertions)]
            {
                // DEVELOPMENT MODE: Always use the project's local .env file
                // We do NOT want to touch AppData in dev, to avoid confusion.
                // Search in CWD (src-tauri) and Parent (project root)
                let cwd = std::env::current_dir().unwrap_or_default();
                let params = vec![cwd.join(".env"), cwd.join("../.env")];

                let mut loaded = false;
                for path in params {
                    if path.exists() {
                        if let Ok(_) = dotenvy::from_path(&path) {
                            info!("(DEV) Loaded local .env from: {:?}", path);
                            loaded = true;
                            break;
                        }
                    }
                }

                if !loaded {
                    tracing::warn!("(DEV) Could not find local .env in {:?} or parent. Please ensure it exists.", cwd);
                }
            }

            #[cfg(not(debug_assertions))]
            {
                // PRODUCTION MODE: Use AppData (Secure)
                let env_path = app_data_dir.join(".env");

                if !env_path.exists() {
                    // Create default .env if not exists
                    let default_content = r#"# Application Configuration
# APP_NAME="Tri Digital Solution"
# DATABASE_URL=postgres://user:password@localhost:5432/dbname
"#;
                    if let Err(e) = std::fs::write(&env_path, default_content) {
                        tracing::error!("Failed to create default .env: {}", e);
                    } else {
                        info!("Created default .env at {:?}", env_path);
                    }
                }

                // Load from AppData
                if let Ok(_) = dotenvy::from_path(&env_path) {
                    info!("(PROD) Loaded .env from AppData: {:?}", env_path);
                }
            }

            // Initialize database and services
            // We use block_on here to ensure services are ready before the window starts
            // and potentially calls commands that require these managed states.
            let init_result: Result<(), String> = tauri::async_runtime::block_on(async {
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
                role_service.seed_permissions()
                    .await
                    .map_err(|e| format!("Failed to seed permissions: {}", e))?;
                role_service.seed_roles()
                    .await
                    .map_err(|e| format!("Failed to seed roles: {}", e))?;
                info!("RBAC permissions and roles seeded.");

                // Get JWT secret from settings
                let jwt_secret = sqlx::query_scalar::<_, String>(
                    "SELECT value FROM settings WHERE key = 'jwt_secret' AND tenant_id IS NULL"
                )
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());
                info!("JWT Secret loaded.");

                // Initialize App Data Dir for Storage
                let app_data_dir = app_handle.path().app_data_dir().unwrap_or(std::path::PathBuf::from("app_data"));

                let settings_service = SettingsService::new(pool.clone(), audit_service.clone());
                let email_service = EmailService::new(settings_service.clone());
                let auth_service = AuthService::new(pool.clone(), jwt_secret, email_service.clone(), audit_service.clone(), settings_service.clone());
                let user_service = UserService::new(pool.clone(), audit_service.clone());
                let team_service = TeamService::new(pool.clone(), auth_service.clone(), audit_service.clone(), plan_service.clone());
                let metrics_service = std::sync::Arc::new(MetricsService::new());
                let system_service = SystemService::new(pool.clone(), metrics_service.clone());
                let storage_service = crate::services::StorageService::new(pool.clone(), plan_service.clone(), app_data_dir.clone());
                let backup_service = BackupService::new(pool.clone(), app_data_dir.clone());

                // Start Backup Scheduler
                let scheduler = BackupScheduler::new(pool.clone(), backup_service.clone(), settings_service.clone());
                scheduler.start().await;

                // Create WebSocket hub for real-time sync (shared between HTTP and Tauri)
                let ws_hub = std::sync::Arc::new(http::WsHub::new());

                let email_outbox_service =
                    EmailOutboxService::new(pool.clone(), settings_service.clone(), email_service.clone());
                email_outbox_service.start_sender().await;

                let notification_service = NotificationService::new(
                    pool.clone(),
                    ws_hub.clone(),
                    email_outbox_service.clone(),
                );
                let payment_service = PaymentService::new(pool.clone(), notification_service.clone());

                // Start Announcement Scheduler (scheduled broadcasts -> notifications)
                let announcement_scheduler =
                    AnnouncementScheduler::new(pool.clone(), notification_service.clone(), audit_service.clone());
                announcement_scheduler.start().await;

                // Seed default features
                plan_service.seed_default_features()
                    .await
                    .map_err(|e| format!("Failed to seed default features: {}", e))?;
                info!("Default features seeded.");

                // Manage state - Crucial: This must happen before setup returns
                app_handle.manage(auth_service.clone());
                app_handle.manage(user_service.clone());
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
                app_handle.manage(email_outbox_service.clone());
                app_handle.manage(ws_hub.clone());
                app_handle.manage(metrics_service.clone());
                info!("Services added to Tauri state.");


                // Start HTTP Server (This can run in background)
                let app_dir = app_data_dir.clone();
                tauri::async_runtime::spawn(async move {
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
                        backup_service,
                        ws_hub,
                        app_dir,
                        3000,
                        pool.clone(),
                        metrics_service,
                    ).await;
                });


                info!("Services initialized successfully");
                Ok(())
            });

            if let Err(e) = init_result {
                tracing::warn!("!!! BACKEND INITIALIZATION SKIPPED: {} !!!", e);
                tracing::warn!("This is expected if you are running as a Remote Client without a local database.");
                tracing::warn!("The application will continue in Client-Only mode.");

                // We do NOT exit here anymore. We allow the app to launch.
                // If the frontend tries to use local backend features, they will fail (gracefully or with errors),
                // but if VITE_USE_REMOTE_API is true, everything will work fine.
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Force exit to ensure background tasks (like HTTP server) are killed
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Auth commands
            register,
            login,
            logout,
            change_password,
            get_current_user,
            validate_token,
            verify_email,
            forgot_password,
            reset_password,
            enable_2fa,
            verify_2fa_setup,
            disable_2fa,
            verify_login_2fa,
            request_email_otp,
            verify_email_otp,
            get_2fa_methods,
            request_email_2fa_setup,
            verify_email_2fa_setup,
            set_2fa_preference,
            request_2fa_disable_code,
            reset_user_2fa,
            list_trusted_devices,
            revoke_trusted_device,
            // User commands
            list_users,
            get_user,
            create_user,
            update_user,
            delete_user,
                        // Settings commands
                        get_all_settings,
                        get_auth_settings,
                        get_public_settings,
                        get_setting,
                        get_setting_value,
                        upsert_setting,
                        delete_setting,
                        upload_logo,
                        get_logo,
                        // Install commands
                        is_installed,
                        install_app,
                        // Email commands
                        send_test_email,
                        test_smtp_connection,
                                    // Super Admin commands
                                    list_tenants,
                                    delete_tenant,
                                    create_tenant,
                                    update_tenant,
                                    // Roles commands
                                    get_roles,
                                    get_permissions,
                                    get_role,
                                    create_new_role,
                                    update_existing_role,
                                    delete_existing_role,
                                    // Team commands
                                    list_team_members,
                                    add_team_member,
                                    update_team_member_role,
                                    remove_team_member,
                                    // Audit commands
                                    list_audit_logs,
                                    // System Health commands
                                    get_system_health,
                                    get_system_diagnostics,
                                    // Plan commands
                                    list_plans,
                                    get_plan,
                                    create_plan,
                                    update_plan,
                                    delete_plan,
                                    list_features,
                                    // create_feature, // System Managed
                                    // delete_feature, // System Managed
                                    set_plan_feature,
                                    assign_plan_to_tenant,
                                    get_tenant_subscription,
                                    get_tenant_subscription_details,
                                    check_feature_access,
                                    // Storage commands
                                    upload_file,
                                    list_files_admin,
                                    delete_file_admin,
                                    list_files_tenant,
                                    delete_file_tenant,
                                    // Payment commands
                                    list_bank_accounts,
                                    create_bank_account,
                                    delete_bank_account,
                                    create_invoice_for_plan,
                                    get_fx_rate,
                                    get_invoice,
                                    pay_invoice_midtrans,
                                    check_payment_status,
                                    list_invoices,
                                    list_all_invoices,
                                    submit_payment_proof,
                                    verify_payment,
                                    // Tenant Self-Management
                                    get_current_tenant,
                                    update_current_tenant,
                                    // General
                                    get_app_version,
                                    // Notifications
                                    list_notifications,
                                    get_unread_count,
                                    mark_as_read,
                                    mark_all_as_read,
                                    delete_notification,
                                    get_preferences,
                                    update_preference,
                                    subscribe_push,
                                    unsubscribe_push,
                                    send_test,
                                    // Email Outbox (Admin)
                                    list_email_outbox,
                                    get_email_outbox_stats,
                                    retry_email_outbox,
                                    delete_email_outbox,
                                    get_email_outbox,
                                    bulk_retry_email_outbox,
                                    bulk_delete_email_outbox,
                                    export_email_outbox_csv,
                                    // Backup commands
                                    list_backups,
                                    create_backup,
                                    delete_backup,
                                    save_backup_to_disk,
                                    restore_backup_from_file,
                                    restore_local_backup_command,
                                    // Support tickets
                                    list_support_tickets,
                                    get_support_ticket_stats,
                                    create_support_ticket,
                                    get_support_ticket,
                                    reply_support_ticket,
                                    update_support_ticket,
                                    // Announcements
                                    list_active_announcements,
                                    list_recent_announcements,
                                    get_announcement,
                                    dismiss_announcement,
                                    list_announcements_admin,
                                    create_announcement_admin,
                                    update_announcement_admin,
                                    delete_announcement_admin,
                                ])
                                .run(tauri::generate_context!())
                                .expect("error while running tauri application");
}

#[cfg(windows)]
fn show_error_dialog(message: &str) {
    let script = format!(
        "Add-Type -AssemblyName PresentationCore,PresentationFramework; [System.Windows.MessageBox]::Show('{}', 'Startup Error', 'OK', 'Error')",
        message.replace("'", "''")
    );
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .spawn();
}
