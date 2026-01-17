//! SaaS Boilerplate - Main Library Entry Point
//! 
//! This is the core library for the Tauri application.
//! It wires together all modules: database, services, and commands.

mod commands;
mod db;
mod error;
mod models;
mod services;
mod http;

use commands::*;
use commands::audit::list_audit_logs;
use db::connection::{init_db, seed_defaults};
use services::{AuthService, EmailService, SettingsService, UserService, TeamService, AuditService, RoleService, SystemService};
use tauri::Manager;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging
fn init_logging() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("saas_tauri=debug".parse().unwrap()))
        .init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(unknown_lints)]
#[allow(clippy::all)] 
pub fn run() {
    init_logging();
    info!("Starting Application");

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init());

    // Only enable single-instance in production to allow dev and prod to run simultaneously
    #[cfg(not(debug_assertions))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.get_webview_window("main")
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
                let audit_service = AuditService::new(pool.clone());
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

                let settings_service = SettingsService::new(pool.clone(), audit_service.clone());
                let email_service = EmailService::new(settings_service.clone());
                let auth_service = AuthService::new(pool.clone(), jwt_secret, email_service.clone(), audit_service.clone());
                let user_service = UserService::new(pool.clone(), audit_service.clone());
                let team_service = TeamService::new(pool.clone(), auth_service.clone(), audit_service.clone());
                let system_service = SystemService::new(pool.clone());
                
                // Create WebSocket hub for real-time sync (shared between HTTP and Tauri)
                let ws_hub = std::sync::Arc::new(http::WsHub::new());

                // Manage state - Crucial: This must happen before setup returns
                app_handle.manage(auth_service.clone());
                app_handle.manage(user_service.clone());
                app_handle.manage(settings_service.clone());
                app_handle.manage(email_service.clone());
                app_handle.manage(team_service.clone());
                app_handle.manage(audit_service.clone());
                app_handle.manage(role_service.clone());
                app_handle.manage(system_service.clone());
                app_handle.manage(ws_hub.clone());
                info!("Services added to Tauri state.");

                // Start HTTP Server (This can run in background)
                let app_dir = app_data_dir.clone();
                tauri::async_runtime::spawn(async move {
                    http::start_server(auth_service, user_service, settings_service, email_service, team_service, role_service, audit_service, system_service, ws_hub, app_dir, 3000).await;
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
                                    // General
                                    get_app_version,
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
                        