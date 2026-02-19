//! Payment HTTP Handlers (Webhooks)

use crate::http::AppState;
use crate::models::{BankAccount, CreateBankAccountRequest, Invoice};
use crate::services::{BulkGenerateInvoicesResult, Claims};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/invoices", get(list_invoices))
        .route("/invoices/all", get(list_all_invoices))
        .route("/fx-rate", get(get_fx_rate))
        .route("/invoices/plan", post(create_invoice_for_plan))
        .route("/invoices/customer-package", get(list_customer_package_invoices))
        .route(
            "/invoices/customer-package/create",
            post(create_invoice_for_customer_subscription),
        )
        .route(
            "/invoices/customer-package/generate-due",
            post(generate_due_customer_package_invoices),
        )
        .route(
            "/invoices/{id}/customer-package/verify",
            post(verify_customer_package_payment),
        )
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
#[serde(deny_unknown_fields)]
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
async fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Claims, (StatusCode, Json<ErrorResponse>)> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Missing authorization header".to_string(),
                }),
            )
        })?;

    state
        .auth_service
        .validate_token(auth_header)
        .await
        .map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

fn require_superadmin(claims: &Claims) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if !claims.is_super_admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Superadmin access required".to_string(),
            }),
        ));
    }
    Ok(())
}

async fn require_payment_read_access(
    state: &AppState,
    claims: &Claims,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if claims.is_super_admin {
        return Ok(());
    }

    let tenant_id = claims.tenant_id.as_deref().ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Tenant context required".to_string(),
            }),
        )
    })?;

    state
        .auth_service
        .check_permission(&claims.sub, tenant_id, "billing", "read")
        .await
        .map_err(|e| {
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(())
}

async fn require_payment_manage_access(
    state: &AppState,
    claims: &Claims,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if claims.is_super_admin {
        return Ok(());
    }

    let tenant_id = claims.tenant_id.as_deref().ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Tenant context required".to_string(),
            }),
        )
    })?;

    state
        .auth_service
        .check_permission(&claims.sub, tenant_id, "billing", "manage")
        .await
        .map_err(|e| {
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(())
}

async fn get_fx_rate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<FxRateQuery>,
) -> Result<Json<FxRateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_payment_read_access(&state, &claims).await?;

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
    require_payment_read_access(&state, &claims).await?;
    let Some(tenant_id) = claims.tenant_id.as_deref() else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Tenant context required".to_string(),
            }),
        ));
    };

    state
        .payment_service
        .list_invoices(Some(tenant_id))
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

async fn list_all_invoices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Invoice>>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state
        .payment_service
        .list_invoices(None)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct CreateInvoiceForPlanBody {
    plan_id: String,
    billing_cycle: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateInvoiceForCustomerSubscriptionBody {
    subscription_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct VerifyCustomerPackagePaymentBody {
    status: String,
    rejection_reason: Option<String>,
}

async fn authorize_invoice_access(
    state: &AppState,
    claims: &Claims,
    invoice_id: &str,
) -> Result<Invoice, (StatusCode, Json<ErrorResponse>)> {
    let invoice = state.payment_service.get_invoice(invoice_id).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    if claims.is_super_admin {
        return Ok(invoice);
    }

    let Some(tenant_id) = claims.tenant_id.as_deref() else {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Tenant context required".to_string(),
            }),
        ));
    };

    if tenant_id != invoice.tenant_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Invoice access denied".to_string(),
            }),
        ));
    }

    Ok(invoice)
}

async fn create_invoice_for_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateInvoiceForPlanBody>,
) -> Result<Json<Invoice>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_payment_manage_access(&state, &claims).await?;
    let tenant_id = claims.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "No tenant context".to_string(),
            }),
        )
    })?;

    let plan = state
        .plan_service
        .get_plan(&body.plan_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    let billing_cycle = body.billing_cycle.trim().to_ascii_lowercase();
    if billing_cycle != "monthly" && billing_cycle != "yearly" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "billingCycle must be monthly or yearly".to_string(),
            }),
        ));
    }

    let amount = if billing_cycle == "yearly" {
        plan.price_yearly
    } else {
        plan.price_monthly
    };

    let desc = format!("{} Plan ({} billing)", plan.name, billing_cycle);
    let ext_id = format!("plan:{}:{}", body.plan_id, billing_cycle);

    state
        .payment_service
        .create_invoice(&tenant_id, amount, Some(desc), Some(ext_id))
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

async fn get_invoice(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Invoice>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_payment_read_access(&state, &claims).await?;
    let invoice = authorize_invoice_access(&state, &claims, &id).await?;
    Ok(Json(invoice))
}

async fn list_customer_package_invoices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Invoice>>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_payment_read_access(&state, &claims).await?;
    let Some(tenant_id) = claims.tenant_id.as_deref() else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Tenant context required".to_string(),
            }),
        ));
    };

    state
        .payment_service
        .list_customer_package_invoices(tenant_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

async fn create_invoice_for_customer_subscription(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateInvoiceForCustomerSubscriptionBody>,
) -> Result<Json<Invoice>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_payment_manage_access(&state, &claims).await?;
    let tenant_id = claims.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "No tenant context".to_string(),
            }),
        )
    })?;

    state
        .payment_service
        .create_invoice_for_customer_subscription(&tenant_id, &body.subscription_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

async fn generate_due_customer_package_invoices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<BulkGenerateInvoicesResult>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_payment_manage_access(&state, &claims).await?;
    let tenant_id = claims.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "No tenant context".to_string(),
            }),
        )
    })?;

    state
        .payment_service
        .generate_due_customer_package_invoices(&tenant_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

async fn verify_customer_package_payment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<VerifyCustomerPackagePaymentBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_payment_manage_access(&state, &claims).await?;

    let invoice = authorize_invoice_access(&state, &claims, &id).await?;
    let is_customer_package = invoice
        .external_id
        .as_deref()
        .map(|v| v.starts_with("pkgsub:"))
        .unwrap_or(false);
    if !is_customer_package {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Only customer package invoices can be verified here".to_string(),
            }),
        ));
    }

    state
        .payment_service
        .verify_payment(&id, &body.status, body.rejection_reason)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

async fn pay_invoice_midtrans(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<String>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_payment_manage_access(&state, &claims).await?;
    let _ = authorize_invoice_access(&state, &claims, &id).await?;

    state
        .payment_service
        .initiate_midtrans(&id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

async fn check_payment_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<String>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_payment_read_access(&state, &claims).await?;
    let _ = authorize_invoice_access(&state, &claims, &id).await?;

    state
        .payment_service
        .check_transaction_status(&id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

async fn list_bank_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<BankAccount>>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_payment_read_access(&state, &claims).await?;

    state
        .payment_service
        .list_bank_accounts()
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

async fn create_bank_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateBankAccountRequest>,
) -> Result<Json<BankAccount>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state
        .payment_service
        .create_bank_account(req)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

async fn delete_bank_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    state
        .payment_service
        .delete_bank_account(&id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

async fn midtrans_notification(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let payment_service = &state.payment_service;
    tracing::info!("Received Midtrans notification");

    // 1. Extract fields
    let order_id = payload["order_id"].as_str().unwrap_or("");
    let status_code = payload["status_code"].as_str().unwrap_or("");
    let gross_amount = payload["gross_amount"].as_str().unwrap_or("");
    let signature_key = payload["signature_key"].as_str().unwrap_or("");
    let transaction_status = payload["transaction_status"].as_str().unwrap_or("");

    if order_id.is_empty()
        || status_code.is_empty()
        || gross_amount.is_empty()
        || signature_key.is_empty()
    {
        return (StatusCode::BAD_REQUEST, "Invalid Payload");
    }

    // 2. Verify Midtrans signature before processing status changes.
    let signature_ok = match payment_service
        .verify_midtrans_signature(order_id, status_code, gross_amount, signature_key)
        .await
    {
        Ok(ok) => ok,
        Err(e) => {
            tracing::error!("Failed Midtrans signature verification: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Verification Error");
        }
    };

    if !signature_ok {
        tracing::warn!("Midtrans notification rejected due to invalid signature");
        return (StatusCode::UNAUTHORIZED, "Invalid Signature");
    }

    // 3. Determine Payment Status
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

    // 4. Update Invoice Status
    match payment_service
        .process_midtrans_notification(order_id, payment_status)
        .await
    {
        Ok(_) => (StatusCode::OK, "OK"),
        Err(e) => {
            eprintln!("Failed to process notification: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Processing Error")
        }
    }
}
