//! Payment HTTP Handlers (Webhooks)

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json, routing::post, Router,
};
use serde_json::Value;
use crate::http::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/midtrans/notification", post(midtrans_notification))
}

async fn midtrans_notification(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let payment_service = &state.payment_service;
    println!("Received Midtrans Notification: {:?}", payload);

    // 1. Extract fields
    let order_id = payload["order_id"].as_str().unwrap_or("");
    let _status_code = payload["status_code"].as_str().unwrap_or("");
    let _gross_amount = payload["gross_amount"].as_str().unwrap_or("");
    let _signature_key = payload["signature_key"].as_str().unwrap_or("");
    let transaction_status = payload["transaction_status"].as_str().unwrap_or("");

    if order_id.is_empty() {
        return (StatusCode::BAD_REQUEST, "Invalid Payload");
    }

    // 2. Determine Payment Status
    let mut payment_status = match transaction_status {
        "capture" => "paid",
        "settlement" => "paid",
        "pending" => "pending",
        "deny" | "expire" | "cancel" => "failed",
        _ => "pending",
    };

    if transaction_status == "capture" {
        if let Some(fraud) = payload["fraud_status"].as_str() {
            if fraud == "challenge" {
                payment_status = "pending";
            }
        }
    }

    // 3. Update Invoice Status
    match payment_service.process_midtrans_notification(order_id, payment_status).await {
        Ok(_) => (StatusCode::OK, "OK"),
        Err(e) => {
            eprintln!("Failed to process notification: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Processing Error")
        }
    }
}
