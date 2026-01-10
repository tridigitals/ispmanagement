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
use db::connection::{init_db, seed_defaults};
use services::{AuthService, EmailService, SettingsService, UserService, TeamService};
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
pub fn run() {
    // Load .env file - try multiple locations
    // Priority: parent dir (root) > current dir (src-tauri) > src-tauri subfolder
    let env_paths = [
        std::path::PathBuf::from("../.env"),        // Parent directory (root, for cargo run from src-tauri)
        std::path::PathBuf::from(".env"),           // Current directory
        std::path::PathBuf::from("src-tauri/.env"), // From root when running binary
    ];
    
    let mut loaded = false;
    for path in &env_paths {
        eprintln!("Checking .env at: {:?} (exists: {})", path, path.exists());
        if path.exists() {
            if let Ok(_) = dotenvy::from_path(path) {
                eprintln!("✓ Loaded .env from: {:?}", path);
                loaded = true;
                break;
            }
        }
    }
    
    if !loaded {
        eprintln!("⚠ Could not load .env file from any location");
    }

    init_logging();
    info!("Starting SaaS Boilerplate Application");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            // Get app data directory
            let app_data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");

            // Create app data directory if it doesn't exist
            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");

            info!("App data directory: {:?}", app_data_dir);

            // Initialize database and services in async context
            tauri::async_runtime::spawn(async move {
                let init_result: Result<(), String> = (async {
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

                    // Seed RBAC permissions and roles
                    services::seed_permissions(&pool)
                        .await
                        .map_err(|e| format!("Failed to seed permissions: {}", e))?;
                    services::seed_roles(&pool)
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

                    // Create services
                    let settings_service = SettingsService::new(pool.clone());
                    let email_service = EmailService::new(settings_service.clone());
                    let auth_service = AuthService::new(pool.clone(), jwt_secret, email_service.clone());
                    let user_service = UserService::new(pool.clone());
                    let team_service = TeamService::new(pool.clone(), auth_service.clone());

                    // Manage state
                    app_handle.manage(auth_service.clone());
                    app_handle.manage(user_service.clone());
                    app_handle.manage(settings_service.clone());
                    app_handle.manage(email_service.clone());
                    app_handle.manage(team_service.clone());
                    info!("Services added to Tauri state.");

                    // Start HTTP Server
                    let app_dir = app_data_dir.clone();
                    tauri::async_runtime::spawn(async move {
                        http::start_server(auth_service, user_service, settings_service, email_service, app_dir, 3000).await;
                    });

                    info!("Services initialized successfully");
                    Ok(())
                }).await;

                if let Err(e) = init_result {
                    tracing::error!("!!! FAILED TO INITIALIZE BACKEND: {} !!!", e);
                    // Optionally, you could emit an event to the frontend to show a fatal error screen
                    // app_handle.emit_all("backend-init-error", &e).unwrap();
                }
            });

            Ok(())
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
                                ])
                                .run(tauri::generate_context!())
                                .expect("error while running tauri application");
                        }
                        