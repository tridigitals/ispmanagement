//! Fallback handlers for unmatched routes — return a structured 404 JSON
//! response so the FE never has to parse an HTML 404 page (e.g. when
//! nginx proxies a missing route through SvelteKit dev or static).
//!
//! We use `_ => { /* not used */ }` handler pattern so that handler can
//! be plugged into `routing::any()` which requires an async signature.

use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub async fn unknown_route_handler(_req: Request) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "Route not found",
            "detail": "The requested endpoint does not exist or has been removed.",
        })),
    )
        .into_response()
}
