use axum::{
    routing::{get, post, delete, put},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use crate::services::{AuthService, UserService, SettingsService, EmailService};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};
use std::env;

use std::path::PathBuf;

pub mod auth;
pub mod install;
pub mod settings;
pub mod users;
pub mod superadmin;

// App State to share services with Axum handlers
#[derive(Clone)]
pub struct AppState {
    pub auth_service: Arc<AuthService>,
    pub user_service: Arc<UserService>,
    pub settings_service: Arc<SettingsService>,
    pub email_service: Arc<EmailService>,
    pub app_data_dir: PathBuf,
}

pub async fn start_server(
    auth_service: AuthService,
    user_service: UserService,
    settings_service: SettingsService,
    email_service: EmailService,
    app_data_dir: PathBuf,
    default_port: u16,
) {
    // .env is already loaded in lib.rs from root folder

    let state = AppState {
        auth_service: Arc::new(auth_service),
        user_service: Arc::new(user_service),
        settings_service: Arc::new(settings_service),
        email_service: Arc::new(email_service),
        app_data_dir,
    };

    // Dynamic CORS Configuration
    let origins_str = env::var("CORS_ALLOWED_ORIGINS").unwrap_or_else(|_| {
        "http://localhost:5173,http://localhost:3000,tauri://localhost,https://tauri.localhost".to_string()
    });

    let origins: Vec<axum::http::HeaderValue> = origins_str
        .split(',')
        .map(|s| s.trim())
        .filter_map(|s| {
            s.parse::<axum::http::HeaderValue>()
                .map_err(|e| warn!("Invalid CORS origin '{}': {}", s, e))
                .ok()
        })
        .collect();

    info!("Allowed CORS Origins: {:?}", origins);

    let cors = CorsLayer::new()
        .allow_origin(origins)
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

                // User Routes

                .route("/api/users", get(users::list_users).post(users::create_user))

                .route("/api/users/{id}", get(users::get_user).put(users::update_user).delete(users::delete_user))

                // Super Admin Routes

                .route("/api/superadmin/tenants", get(superadmin::list_tenants).post(superadmin::create_tenant))

                .route("/api/superadmin/tenants/{id}", delete(superadmin::delete_tenant))

                // Settings Routes

                .route("/api/settings", get(settings::get_all_settings).post(settings::upsert_setting))

                .route("/api/settings/public", get(settings::get_public_settings))

                .route("/api/settings/logo", get(settings::get_logo).post(settings::upload_logo))

                .route("/api/settings/test-email", post(settings::send_test_email))

                .route("/api/settings/{key}", get(settings::get_setting).delete(settings::delete_setting))

                .route("/api/settings/{key}/value", get(settings::get_setting_value))

                .layer(cors)

                .with_state(state);

        

    

    // Determine port
    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(default_port);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("HTTP API listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root_handler() -> &'static str {
    "SaaS Boilerplate API is running. Use the frontend to interact."
}