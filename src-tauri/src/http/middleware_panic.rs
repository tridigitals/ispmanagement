//! Panic recovery middleware.
//!
//! Wraps every request so that any panic inside a handler becomes a
//! `500 Internal Server Error` JSON envelope instead of taking down
//! the axum worker. The panic message is captured server-side via
//! `tracing::error!` but never sent to the client.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use futures::FutureExt;
use serde_json::json;
use tracing::error;

pub async fn panic_recovery_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    let next_fut = next.run(request);

    // Wrap in AssertUnwindSafe to allow the future to be a future::Future
    // even though the panic-recovery crosses an unwind boundary (this is
    // exactly the use-case the helper was designed for).
    let result = std::panic::AssertUnwindSafe(next_fut).catch_unwind().await;

    match result {
        Ok(response) => response,
        Err(panic_payload) => {
            let panic_msg = panic_msg_to_string(&panic_payload);
            error!(
                target = %method,
                path = %path,
                "request panicked: {}",
                panic_msg
            );

            // Swallow the panic — return a sanitized 500 to the client.
            let body = Json(json!({
                "error": "Internal server error",
                "detail": "An unexpected error occurred while processing this request. Please try again in a few seconds.",
            }));
            (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
        }
    }
}

fn panic_msg_to_string(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "<opaque panic payload>".to_string()
    }
}
