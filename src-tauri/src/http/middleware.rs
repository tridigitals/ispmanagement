//! HTTP Middleware - Rate Limiting, Security, and Metrics
//!
//! Provides middleware layers for rate limiting, security headers, and request metrics.

use axum::{
    body::Body,
    extract::ConnectInfo,
    extract::State,
    http::{header, HeaderMap, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use crate::services::metrics_service::MetricsService;
use crate::services::rate_limiter::RateLimiter;
use crate::{http::AppState, services::rate_limiter::RateLimitInfo};
use chrono::Utc;

/// Rate limiter configuration for different endpoint types
#[derive(Clone)]
pub struct RateLimitConfig {
    /// Rate limiter instance
    pub limiter: Arc<RateLimiter>,
    /// Maximum requests allowed in window
    pub limit: u32,
    /// Window size in seconds
    pub window_secs: u64,
}

impl RateLimitConfig {
    pub fn new(limiter: Arc<RateLimiter>, limit: u32, window_secs: u64) -> Self {
        Self {
            limiter,
            limit,
            window_secs,
        }
    }
}

async fn rate_limit_key_for_request(
    state: &AppState,
    client_ip: &str,
    headers: &HeaderMap,
    path: &str,
) -> Option<String> {
    // For pre-auth endpoints, we intentionally key by IP.
    if path.starts_with("/api/auth/") {
        return Some(format!("ip:{client_ip}"));
    }

    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    if let Some(tok) = auth_header {
        if let Ok(claims) = state.auth_service.validate_token(tok).await {
            return Some(format!("user:{}", claims.sub));
        }
    }

    Some(format!("ip:{client_ip}"))
}

fn policy_for_path(path: &str, default_limit: u32) -> (u32, u64) {
    // Wallboard live traffic is intentionally high-frequency.
    // Keep it isolated with a dedicated key scope and higher budget.
    if is_wallboard_live_path(path) {
        return (1800, 60);
    }

    // Keep these strict and predictable: they are abuse magnets.
    // Window is seconds.
    if path == "/api/auth/login" {
        return (20, 60);
    }
    if path == "/api/auth/register" {
        return (10, 60);
    }
    if path == "/api/auth/forgot-password" {
        return (10, 60);
    }
    if path == "/api/auth/reset-password" {
        return (10, 60);
    }
    if path == "/api/auth/2fa/verify" || path == "/api/auth/2fa/email/verify" {
        return (20, 60);
    }
    if path == "/api/auth/2fa/email/request" || path == "/api/auth/2fa/email/enable-request" {
        return (30, 60);
    }

    (default_limit.max(10), 60)
}

fn is_wallboard_live_path(path: &str) -> bool {
    path.starts_with("/api/admin/mikrotik/routers/") && path.ends_with("/interfaces/live")
}

fn rate_limit_scope(path: &str) -> &'static str {
    if is_wallboard_live_path(path) {
        "wallboard_live"
    } else {
        "api"
    }
}

fn should_bypass_rate_limit(path: &str) -> bool {
    path == "/"
        || path == "/api/version"
        || path == "/api/ws"
        || path.starts_with("/api/public/")
        || path == "/api/install/check"
}

fn into_rate_limited_response(info: RateLimitInfo) -> Response {
    let body = Json(json!({
        "error": "Rate limit exceeded",
        "limit": info.limit,
        "retry_after": info.reset_in_secs
    }));

    let mut response = (StatusCode::TOO_MANY_REQUESTS, body).into_response();
    let headers = response.headers_mut();
    headers.insert(
        "X-RateLimit-Limit",
        HeaderValue::from_str(&info.limit.to_string()).unwrap(),
    );
    headers.insert("X-RateLimit-Remaining", HeaderValue::from_static("0"));
    headers.insert(
        "X-RateLimit-Reset",
        HeaderValue::from_str(&info.reset_in_secs.to_string()).unwrap(),
    );
    headers.insert(
        "Retry-After",
        HeaderValue::from_str(&info.reset_in_secs.to_string()).unwrap(),
    );
    response
}

pub async fn security_enforcer_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    if should_bypass_rate_limit(&path) {
        return next.run(request).await;
    }

    let client_ip = extract_client_ip(request.headers(), Some(addr));

    // Blocked IP check (best-effort).
    let enable_ip_blocking = { state.security_config.read().await.enable_ip_blocking };
    if enable_ip_blocking {
        if let Some(until) = state.ip_blocklist.read().await.get(&client_ip).copied() {
            if until > Utc::now() {
                let body = Json(json!({
                    "error": "IP temporarily blocked",
                    "blocked_until": until.to_rfc3339(),
                }));
                return (StatusCode::FORBIDDEN, body).into_response();
            }
        }
    }

    // Policy selection.
    let cfg = state.security_config.read().await.clone();
    let (limit, window) = policy_for_path(&path, cfg.api_rate_limit_per_minute);

    // Key selection: IP for auth endpoints, user-id for authenticated routes when possible.
    let key = rate_limit_key_for_request(&state, &client_ip, request.headers(), &path)
        .await
        .unwrap_or_else(|| format!("ip:{client_ip}"));
    let scoped_key = format!("{}:{}", rate_limit_scope(&path), key);

    match state.rate_limiter.check(&scoped_key, limit, window) {
        Ok(info) => {
            let mut response = next.run(request).await;
            let headers = response.headers_mut();
            headers.insert(
                "X-RateLimit-Limit",
                HeaderValue::from_str(&info.limit.to_string()).unwrap(),
            );
            headers.insert(
                "X-RateLimit-Remaining",
                HeaderValue::from_str(&info.remaining.to_string()).unwrap(),
            );
            headers.insert(
                "X-RateLimit-Reset",
                HeaderValue::from_str(&info.reset_in_secs.to_string()).unwrap(),
            );
            response
        }
        Err(info) => {
            // Metrics
            state.metrics_service.record_rate_limited();

            // Optional IP blocking escalation
            if cfg.enable_ip_blocking {
                let now = Utc::now();
                let mut abuse = state.ip_abuse.write().await;
                let entry = abuse.entry(client_ip.clone()).or_insert((0, now));
                // Reset the window if old
                if now - entry.1 > chrono::Duration::minutes(10) {
                    *entry = (0, now);
                }
                entry.0 = entry.0.saturating_add(1);
                if entry.0 >= cfg.ip_block_threshold {
                    let until = now + chrono::Duration::minutes(cfg.ip_block_duration_minutes);
                    state
                        .ip_blocklist
                        .write()
                        .await
                        .insert(client_ip.clone(), until);
                }
            }

            into_rate_limited_response(info)
        }
    }
}

/// Extract client IP from request headers or socket address
pub fn extract_client_ip(headers: &HeaderMap, addr: Option<SocketAddr>) -> String {
    // Check X-Forwarded-For header first (for proxies/load balancers)
    if let Some(forwarded) = headers.get("X-Forwarded-For") {
        if let Ok(s) = forwarded.to_str() {
            // Take the first IP (original client)
            if let Some(ip) = s.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }

    // Check X-Real-IP header (used by nginx)
    if let Some(real_ip) = headers.get("X-Real-IP") {
        if let Ok(s) = real_ip.to_str() {
            return s.trim().to_string();
        }
    }

    // Fall back to socket address
    addr.map(|a| a.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Rate limiting middleware
///
/// Returns HTTP 429 Too Many Requests if rate limit is exceeded
pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    config: RateLimitConfig,
    request: Request<Body>,
    next: Next,
) -> Response {
    let client_ip = extract_client_ip(&headers, Some(addr));

    match config
        .limiter
        .check(&client_ip, config.limit, config.window_secs)
    {
        Ok(info) => {
            // Request allowed - add rate limit headers and continue
            let mut response = next.run(request).await;

            let headers = response.headers_mut();
            headers.insert(
                "X-RateLimit-Limit",
                HeaderValue::from_str(&info.limit.to_string()).unwrap(),
            );
            headers.insert(
                "X-RateLimit-Remaining",
                HeaderValue::from_str(&info.remaining.to_string()).unwrap(),
            );
            headers.insert(
                "X-RateLimit-Reset",
                HeaderValue::from_str(&info.reset_in_secs.to_string()).unwrap(),
            );

            response
        }
        Err(info) => {
            // Record rate limit event
            if let Some(metrics) = request.extensions().get::<Arc<MetricsService>>() {
                metrics.record_rate_limited();
            }

            // Rate limit exceeded
            let body = Json(json!({
                "error": "Rate limit exceeded",
                "limit": info.limit,
                "retry_after": info.reset_in_secs
            }));

            let mut response = (StatusCode::TOO_MANY_REQUESTS, body).into_response();

            let headers = response.headers_mut();
            headers.insert(
                "X-RateLimit-Limit",
                HeaderValue::from_str(&info.limit.to_string()).unwrap(),
            );
            headers.insert("X-RateLimit-Remaining", HeaderValue::from_static("0"));
            headers.insert(
                "X-RateLimit-Reset",
                HeaderValue::from_str(&info.reset_in_secs.to_string()).unwrap(),
            );
            headers.insert(
                "Retry-After",
                HeaderValue::from_str(&info.reset_in_secs.to_string()).unwrap(),
            );

            response
        }
    }
}

/// Security headers middleware
///
/// Adds common security headers to all responses
pub async fn security_headers_middleware(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Prevent MIME type sniffing
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );

    // Prevent clickjacking
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));

    // Enable XSS filter (legacy, but still useful)
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("1; mode=block"),
    );

    // Referrer policy
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // HSTS (only meaningful over HTTPS). Safe to add; browsers ignore it on HTTP.
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=15552000; includeSubDomains"),
    );

    response
}

/// Request metrics middleware
///
/// Tracks request count, response times, and error rates
pub async fn metrics_middleware(request: Request<Body>, next: Next) -> Response {
    // Try to get metrics service from extensions
    let metrics = request.extensions().get::<Arc<MetricsService>>().cloned();

    let start = Instant::now();

    // Execute the request
    let response = next.run(request).await;

    // Calculate duration and record if metrics service is available
    if let Some(metrics) = metrics {
        let duration = start.elapsed();
        let is_error = response.status().is_client_error() || response.status().is_server_error();
        metrics.record_request(duration, is_error);
    }

    response
}
