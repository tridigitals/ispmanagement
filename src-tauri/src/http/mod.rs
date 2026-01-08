use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use crate::services::{AuthService, UserService, SettingsService, EmailService};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

pub mod auth;

// App State to share services with Axum handlers
#[derive(Clone)]
pub struct AppState {
    pub auth_service: Arc<AuthService>,
    pub user_service: Arc<UserService>,
    pub settings_service: Arc<SettingsService>,
    pub email_service: Arc<EmailService>,
}

pub async fn start_server(
    auth_service: AuthService,
    user_service: UserService,
    settings_service: SettingsService,
    email_service: EmailService,
    port: u16,
) {
    let state = AppState {
        auth_service: Arc::new(auth_service),
        user_service: Arc::new(user_service),
        settings_service: Arc::new(settings_service),
        email_service: Arc::new(email_service),
    };

    // CORS configuration (allow all for dev simplicity, restrict in prod)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        .route("/", get(root_handler))
        // Auth Routes
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/verify-email", post(auth::verify_email))
        .route("/api/auth/forgot-password", post(auth::forgot_password))
        .route("/api/auth/reset-password", post(auth::reset_password))
        // Add more routes here (e.g., users, settings)
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("HTTP API listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root_handler() -> &'static str {
    "SaaS Boilerplate API is running. Use the frontend to interact."
}
