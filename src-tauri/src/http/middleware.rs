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
use uuid::Uuid;

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

#[derive(Clone, Debug)]
pub struct CorrelationId(String);

impl CorrelationId {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub async fn correlation_id_middleware(mut request: Request<Body>, next: Next) -> Response {
    // ── Cookie → Authorization header promotion ──
    // If no Authorization header present but auth_token cookie exists,
    // inject the cookie value as a Bearer Authorization header so all
    // downstream handlers work transparently with httpOnly cookies.
    if !request.headers().contains_key("Authorization") {
        if let Some(cookie_header) = request.headers().get("cookie").and_then(|h| h.to_str().ok()) {
            for pair in cookie_header.split(';') {
                let pair = pair.trim();
                if let Some((name, value)) = pair.split_once('=') {
                    if name.trim() == crate::http::AUTH_COOKIE && !value.trim().is_empty() {
                        if let Ok(val) = axum::http::HeaderValue::from_str(&format!("Bearer {}", value.trim())) {
                            request.headers_mut().insert("Authorization", val);
                        }
                        break;
                    }
                }
            }
        }
    }

    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    request
        .extensions_mut()
        .insert(CorrelationId::new(request_id.clone()));

    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}

async fn rate_limit_key_for_request(
    state: &AppState,
    client_ip: &str,
    headers: &HeaderMap,
    path: &str,
) -> Option<String> {
    // For pre-auth abuse-prone endpoints, we intentionally key by IP.
    if should_rate_limit_by_ip(path) {
        return Some(format!("ip:{client_ip}"));
    }

    let auth_header = crate::http::extract_token(headers).ok();

    if let Some(tok) = auth_header {
        if let Ok(claims) = state.auth_service.validate_token(&tok).await {
            return Some(format!("user:{}", claims.sub));
        }
    }

    Some(format!("ip:{client_ip}"))
}

fn should_rate_limit_by_ip(path: &str) -> bool {
    matches!(
        path,
        "/api/auth/login"
            | "/api/auth/register"
            | "/api/auth/forgot-password"
            | "/api/auth/reset-password"
            | "/api/auth/2fa/verify"
            | "/api/auth/2fa/email/verify"
            | "/api/auth/2fa/email/request"
            | "/api/auth/2fa/email/enable-request"
            | "/api/public/customer-register"
    )
}

fn is_session_path(path: &str) -> bool {
    matches!(
        path,
        "/api/auth/validate" | "/api/auth/me" | "/api/tenant/me"
    )
}

fn is_notification_path(path: &str) -> bool {
    path.starts_with("/api/notifications")
}

fn policy_for_path(path: &str, default_limit: u32) -> (u32, u64) {
    // Wallboard live traffic is intentionally high-frequency.
    // Keep it isolated with a dedicated key scope and higher budget.
    if is_wallboard_live_path(path) {
        return (1800, 60);
    }

    // Chunked uploads are bursty during normal use, so keep them higher than
    // the general API baseline to avoid false-positive 429s.
    if path == "/api/storage/upload/chunk" {
        return (600, 60);
    }
    if path == "/api/storage/upload/init" || path == "/api/storage/upload/complete" {
        return (120, 60);
    }

    // Expensive operations should stay protected even with a more forgiving
    // default API budget for regular CRUD traffic.
    if path.starts_with("/api/backups") {
        return (30, 60);
    }
    if path.starts_with("/api/admin/pppoe/mixradius/") {
        return (30, 60);
    }
    if path == "/api/superadmin/diagnostics" {
        return (60, 60);
    }

    // Frequent background sync from authenticated UI should not contend with
    // general CRUD traffic or auth abuse protection.
    if is_session_path(path) {
        return (600, 60);
    }
    if path == "/api/notifications/unread-count" {
        return (default_limit.max(300), 60);
    }

    // Keep these strict and predictable: they are abuse magnets.
    // Window is seconds.
    if path == "/api/auth/login" {
        return (20, 60);
    }
    if path == "/api/auth/register" {
        return (10, 60);
    }
    if path == "/api/public/customer-register" {
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

    (default_limit.max(300), 60)
}

fn is_wallboard_live_path(path: &str) -> bool {
    path.starts_with("/api/admin/mikrotik/routers/") && path.ends_with("/interfaces/live")
}

fn rate_limit_scope(path: &str) -> &'static str {
    if is_wallboard_live_path(path) {
        "wallboard_live"
    } else if is_session_path(path) {
        "session"
    } else if is_notification_path(path) {
        "notifications"
    } else {
        "api"
    }
}

fn should_bypass_rate_limit(path: &str) -> bool {
    if should_rate_limit_by_ip(path) {
        return false;
    }

    path == "/"
        || path == "/api/version"
        || path == "/api/ws"
        || (path.starts_with("/api/public/") && path != "/api/public/customer-register")
        || path == "/api/install/check"
        || path.starts_with("/api/")
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

/// Resolve whether a session is restricted to the customer portal.
///
/// The JWT `role` claim carries the user's GLOBAL role, which can differ from
/// the role they hold inside the token's tenant (for example Admin in tenant A
/// but Customer in tenant B). Callers guarding admin surfaces must prefer the
/// tenant-scoped role, falling back to the claim when the session has no
/// tenant binding.
pub fn is_customer_portal_role(global_role: &str, tenant_role: Option<&str>) -> bool {
    // A blank tenant role means "not resolved" — fall back to the global claim
    // rather than treating it as an empty (internal) role.
    let effective = match tenant_role {
        Some(role) if !role.trim().is_empty() => role,
        _ => global_role,
    };

    let role = effective.trim().to_lowercase();
    role == "customer" || role == "pelanggan"
}

pub async fn security_enforcer_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();

    // ── Customer portal guard: deny /api/admin/* ──
    // See `is_customer_portal_role` — the tenant role wins over the global claim
    // so a privileged global role cannot unlock admin endpoints in a tenant
    // where the user is only a portal customer.
    if path.starts_with("/api/admin/") {
        let auth_header = crate::http::extract_token(request.headers()).ok();
        if let Some(tok) = auth_header {
            if let Ok(claims) = state.auth_service.validate_token(&tok).await {
                let mut tenant_role: Option<String> = None;

                if let Some(ref tid) = claims.tenant_id {
                    tenant_role = state
                        .auth_service
                        .get_tenant_role_name(&claims.sub, tid)
                        .await
                        .ok()
                        .flatten();
                }

                if is_customer_portal_role(&claims.role, tenant_role.as_deref()) {
                    let body = Json(json!({
                        "error": "Forbidden",
                        "message": "Customer portal users cannot access admin endpoints"
                    }));
                    return (StatusCode::FORBIDDEN, body).into_response();
                }
            }
        }
    }

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

#[cfg(test)]
mod tests {
    use super::{
        correlation_id_middleware, is_customer_portal_role, policy_for_path, rate_limit_scope,
        should_bypass_rate_limit, should_rate_limit_by_ip, CorrelationId,
    };
    use axum::{
        body::Body,
        extract::Extension,
        http::{Request, StatusCode},
        middleware::from_fn,
        response::IntoResponse,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn echo_request_id(
        Extension(correlation_id): Extension<CorrelationId>,
    ) -> impl IntoResponse {
        correlation_id.as_str().to_string()
    }

    #[tokio::test]
    async fn middleware_preserves_incoming_request_id() {
        let app = Router::new()
            .route("/", get(echo_request_id))
            .layer(from_fn(correlation_id_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-request-id", "req-123")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok()),
            Some("req-123")
        );
    }

    #[tokio::test]
    async fn middleware_generates_request_id_when_missing() {
        let app = Router::new()
            .route("/", get(echo_request_id))
            .layer(from_fn(correlation_id_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        let generated = response
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(!generated.is_empty());
    }

    #[test]
    fn policy_keeps_auth_endpoints_strict() {
        assert_eq!(policy_for_path("/api/auth/login", 300), (20, 60));
        assert_eq!(policy_for_path("/api/auth/register", 300), (10, 60));
        assert_eq!(policy_for_path("/api/auth/forgot-password", 300), (10, 60));
    }

    #[test]
    fn policy_applies_special_limits_for_expensive_paths() {
        assert_eq!(
            policy_for_path("/api/admin/mikrotik/routers/abc/interfaces/live", 300),
            (1800, 60)
        );
        assert_eq!(policy_for_path("/api/storage/upload/chunk", 300), (600, 60));
        assert_eq!(policy_for_path("/api/storage/upload/init", 300), (120, 60));
        assert_eq!(policy_for_path("/api/backups/restore", 300), (30, 60));
        assert_eq!(
            policy_for_path("/api/admin/pppoe/mixradius/imports/batch-1/execute", 300),
            (30, 60)
        );
    }

    #[test]
    fn policy_uses_more_forgiving_baseline_for_general_api_traffic() {
        assert_eq!(policy_for_path("/api/users", 300), (300, 60));
        assert_eq!(policy_for_path("/api/settings", 300), (300, 60));
    }

    #[test]
    fn policy_is_more_forgiving_for_expected_background_session_paths() {
        assert_eq!(policy_for_path("/api/auth/validate", 300), (600, 60));
        assert_eq!(policy_for_path("/api/auth/me", 300), (600, 60));
        assert_eq!(policy_for_path("/api/tenant/me", 300), (600, 60));
    }

    #[test]
    fn policy_gives_ui_reads_their_own_budget() {
        assert_eq!(
            policy_for_path("/api/notifications/unread-count", 300),
            (300, 60)
        );
        assert_eq!(
            policy_for_path("/api/payment/invoices/abc-123", 300),
            (300, 60)
        );
        assert_eq!(policy_for_path("/api/payment/invoices", 300), (300, 60));
        assert_eq!(
            policy_for_path("/api/payment/invoices/plan", 300),
            (300, 60)
        );
    }

    #[test]
    fn rate_limit_scope_only_splits_special_cases_that_still_use_rate_limit() {
        assert_eq!(rate_limit_scope("/api/auth/validate"), "session");
        assert_eq!(rate_limit_scope("/api/auth/me"), "session");
        assert_eq!(rate_limit_scope("/api/tenant/me"), "session");
        assert_eq!(
            rate_limit_scope("/api/notifications/unread-count"),
            "notifications"
        );
        assert_eq!(rate_limit_scope("/api/payment/invoices/abc-123"), "api");
        assert_eq!(
            rate_limit_scope("/api/payment/invoices/abc-123/status"),
            "api"
        );
        assert_eq!(rate_limit_scope("/api/payment/invoices/plan"), "api");
        assert_eq!(rate_limit_scope("/api/users"), "api");
    }

    #[test]
    fn only_pre_auth_abuse_prone_paths_are_keyed_by_ip() {
        assert!(should_rate_limit_by_ip("/api/auth/login"));
        assert!(should_rate_limit_by_ip("/api/auth/register"));
        assert!(should_rate_limit_by_ip("/api/auth/forgot-password"));
        assert!(should_rate_limit_by_ip("/api/auth/reset-password"));
        assert!(!should_rate_limit_by_ip("/api/auth/validate"));
        assert!(!should_rate_limit_by_ip("/api/auth/me"));
        assert!(!should_rate_limit_by_ip("/api/tenant/me"));
    }

    #[test]
    fn bypass_list_skips_non_auth_application_routes() {
        assert!(should_bypass_rate_limit("/"));
        assert!(should_bypass_rate_limit("/api/version"));
        assert!(should_bypass_rate_limit("/api/public/tenant-lookup"));
        assert!(should_bypass_rate_limit("/api/payment/invoices"));
        assert!(should_bypass_rate_limit(
            "/api/payment/invoices/customer-package"
        ));
        assert!(should_bypass_rate_limit("/api/users"));
        assert!(!should_bypass_rate_limit("/api/public/customer-register"));
        assert!(!should_bypass_rate_limit("/api/auth/login"));
    }

    #[test]
    fn portal_role_guard_prefers_tenant_role_over_global_claim() {
        // Global Admin, but only a portal customer inside this tenant → denied.
        assert!(is_customer_portal_role("Admin", Some("Customer")));
        assert!(is_customer_portal_role("admin", Some("customer")));
        assert!(is_customer_portal_role("Admin", Some("Pelanggan")));

        // Tenant role is internal → global claim must not re-trigger the guard.
        assert!(!is_customer_portal_role("Customer", Some("Admin")));
        assert!(!is_customer_portal_role("customer", Some("Technician")));
    }

    #[test]
    fn portal_role_guard_falls_back_to_global_claim_without_tenant() {
        assert!(is_customer_portal_role("Customer", None));
        assert!(is_customer_portal_role("pelanggan", None));
        assert!(!is_customer_portal_role("Owner", None));
        assert!(!is_customer_portal_role("Admin", None));
        assert!(!is_customer_portal_role("Technician", None));
    }

    #[test]
    fn portal_role_guard_normalizes_case_and_surrounding_whitespace() {
        assert!(is_customer_portal_role("  Customer  ", None));
        assert!(is_customer_portal_role("Admin", Some("  PELANGGAN ")));
        // Blank tenant role must fall back to the global claim, not to "no role".
        assert!(!is_customer_portal_role("Owner", Some("  ")));
    }
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
