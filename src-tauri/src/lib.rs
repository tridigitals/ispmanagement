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
use services::{AuthService, EmailService, SettingsService, UserService};
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
            tauri::async_runtime::block_on(async move {
                // Initialize database
                let pool = init_db(app_data_dir)
                    .await
                    .expect("Failed to initialize database");

                // Seed default settings
                seed_defaults(&pool)
                    .await
                    .expect("Failed to seed default settings");

                // Get JWT secret from settings
                let jwt_secret = sqlx::query_scalar::<_, String>(
                    "SELECT value FROM settings WHERE key = 'jwt_secret'"
                )
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

                // Create services
                let settings_service = SettingsService::new(pool.clone());
                let email_service = EmailService::new(settings_service.clone());
                let auth_service = AuthService::new(pool.clone(), jwt_secret, email_service.clone());
                let user_service = UserService::new(pool.clone());

                // Manage state
                app_handle.manage(auth_service.clone());
                app_handle.manage(user_service.clone());
                app_handle.manage(settings_service.clone());
                app_handle.manage(email_service.clone());

                // Start HTTP Server
                let auth_svc = auth_service.clone();
                let user_svc = user_service.clone();
                let settings_svc = settings_service.clone();
                let email_svc = email_service.clone();

                tauri::async_runtime::spawn(async move {
                     http::start_server(auth_svc, user_svc, settings_svc, email_svc, 3000).await;
                });

                info!("Services initialized successfully");
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}