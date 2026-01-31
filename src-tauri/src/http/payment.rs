//! Payment HTTP Handlers (Webhooks)

use axum::{
    extract::{Query, State, Path},
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    Json, 
    routing::{post, get, delete}, 
    Router,
};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use crate::http::AppState;
use crate::models::{Invoice, BankAccount, CreateBankAccountRequest};
use crate::services::Claims;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/invoices", get(list_invoices))
        .route("/invoices/all", get(list_all_invoices))
        .route("/fx-rate", get(get_fx_rate))
        .route("/invoices/plan", post(create_invoice_for_plan))
        .route("/invoices/{id}", get(get_invoice))
        .route("/invoices/{id}/midtrans", post(pay_invoice_midtrans))
        .route("/invoices/{id}/status", get(check_payment_status))
        .route("/banks", get(list_bank_accounts).post(create_bank_account))
        .route("/banks/{id}", delete(delete_bank_account))
        .route("/midtrans/notification", post(midtrans_notification))
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
struct FxRateQuery {
    base_currency: String,
    quote_currency: String,
}

#[derive(Serialize)]
struct FxRateResponse {
    base_currency: String,
    quote_currency: String,
    rate: f64,
    source: String,
    fetched_at: chrono::DateTime<chrono::Utc>,
}

// Helper to extract and validate token from headers
async fn authenticate(state: &AppState, headers: &HeaderMap) -> Result<Claims, (StatusCode, Json<ErrorResponse>)> {
    let auth_header = headers.get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            error: "Missing authorization header".to_string()
        })))?;

    state.auth_service.validate_token(auth_header).await
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            error: e.to_string()
        })))
}

fn require_superadmin(claims: &Claims) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if !claims.is_super_admin {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse {
            error: "Superadmin access required".to_string()
        })));
    }
    Ok(())
}

async fn get_fx_rate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<FxRateQuery>,
) -> Result<Json<FxRateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;

    let base = q.base_currency.trim().to_uppercase();
    let quote = q.quote_currency.trim().to_uppercase();

    let (rate, fetched_at, source) = state
        .payment_service
        .get_fx_rate(&base, &quote, claims.tenant_id.as_deref())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(FxRateResponse {
        base_currency: base,
        quote_currency: quote,
        rate,
        source,
        fetched_at,
    }))
}

async fn list_invoices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Invoice>>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    let tenant_id = claims.tenant_id.as_deref();

    state.payment_service.list_invoices(tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn list_all_invoices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Invoice>>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state.payment_service.list_invoices(None).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateInvoiceForPlanBody {
    plan_id: String,
    billing_cycle: String,
}

async fn create_invoice_for_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateInvoiceForPlanBody>,
) -> Result<Json<Invoice>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    let tenant_id = claims.tenant_id.ok_or_else(|| (StatusCode::BAD_REQUEST, Json(ErrorResponse {
        error: "No tenant context".to_string()
    })))?;

    let plan = state.plan_service.get_plan(&body.plan_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))?;
    
    let amount = if body.billing_cycle == "yearly" {
        plan.price_yearly
    } else {
        plan.price_monthly
    };

    let desc = format!("{} Plan ({} billing)", plan.name, body.billing_cycle);
    let ext_id = format!("{}:{}", body.plan_id, body.billing_cycle);

    state.payment_service.create_invoice(&tenant_id, amount, Some(desc), Some(ext_id)).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn get_invoice(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Invoice>, (StatusCode, Json<ErrorResponse>)> {
    let _claims = authenticate(&state, &headers).await?;
    
    // In a real app, we should verify that the invoice belongs to the tenant
    state.payment_service.get_invoice(&id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn pay_invoice_midtrans(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<String>, (StatusCode, Json<ErrorResponse>)> {
    let _claims = authenticate(&state, &headers).await?;

    state.payment_service.initiate_midtrans(&id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn check_payment_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<String>, (StatusCode, Json<ErrorResponse>)> {
    let _claims = authenticate(&state, &headers).await?;

    state.payment_service.check_transaction_status(&id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn list_bank_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<BankAccount>>, (StatusCode, Json<ErrorResponse>)> {
    let _claims = authenticate(&state, &headers).await?;

    state.payment_service.list_bank_accounts().await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn create_bank_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateBankAccountRequest>,
) -> Result<Json<BankAccount>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state.payment_service.create_bank_account(req).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
}

async fn delete_bank_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state.payment_service.delete_bank_account(&id).await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
            error: e.to_string()
        })))
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
