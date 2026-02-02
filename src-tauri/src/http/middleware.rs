//! HTTP Middleware - Rate Limiting, Security, and Metrics
//!
//! Provides middleware layers for rate limiting, security headers, and request metrics.

use axum::{
    body::Body,
    extract::ConnectInfo,
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
