//! Payment HTTP Handlers (Webhooks)

use crate::http::{middleware::CorrelationId, AppState};
use crate::models::{
    BankAccount, BillingCollectionLogView, CreateBankAccountRequest, Invoice,
    InvoiceReminderLogView, PaginatedResponse,
};
use crate::services::{BillingCollectionRunResult, BulkGenerateInvoicesResult, Claims};
use axum::{
    extract::{Extension, Form, Path, Query, State},
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
        .route(
            "/invoices/customer-package",
            get(list_customer_package_invoices),
        )
        .route(
            "/invoices/customer-package/create",
            post(create_invoice_for_customer_subscription),
        )
        .route("/billing/change-package", post(change_subscription_package))
        .route(
            "/invoices/installation/create",
            post(create_invoice_for_installation_work_order),
        )
        .route(
            "/invoices/customer-package/generate-due",
            post(generate_due_customer_package_invoices),
        )
        .route("/invoices/bulk-send", post(bulk_send_invoices))
        .route("/billing/analytics", get(get_billing_analytics))
        .route(
            "/billing-collection/logs",
            get(list_billing_collection_logs),
        )
        .route(
            "/billing-collection/reminders",
            get(list_invoice_reminder_logs),
        )
        .route(
            "/billing-collection/run-now",
            post(run_billing_collection_now),
        )
        .route(
            "/invoices/{id}/customer-package/verify",
            post(verify_customer_package_payment),
        )
        .route("/invoices/{id}/verify", post(verify_invoice_payment))
        .route("/invoices/{id}/proof", post(submit_payment_proof))
        .route("/invoices/{id}", get(get_invoice))
        .route("/invoices/{id}/midtrans", post(pay_invoice_midtrans))
        .route("/invoices/{id}/duitku", post(pay_invoice_duitku))
        .route("/invoices/{id}/status", get(check_payment_status))
        .route("/duitku/payment-methods", get(list_duitku_payment_methods))
        .route("/banks", get(list_bank_accounts).post(create_bank_account))
        .route("/banks/{id}", delete(delete_bank_account))
        .route("/midtrans/notification", post(midtrans_notification))
        .route("/duitku/callback", post(duitku_callback))
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

enum PaymentReadScope {
    Billing,
    CustomerPortal { customer_id: String },
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

#[derive(Deserialize)]
struct ListCustomerPackageInvoicesQuery {
    sort_by: Option<String>,
    sort_dir: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
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

fn is_customer_role(role: &str) -> bool {
    role.trim().eq_ignore_ascii_case("customer")
}

async fn resolve_payment_read_scope(
    state: &AppState,
    claims: &Claims,
) -> Result<PaymentReadScope, (StatusCode, Json<ErrorResponse>)> {
    if claims.is_super_admin {
        return Ok(PaymentReadScope::Billing);
    }

    let tenant_id = claims.tenant_id.as_deref().ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Tenant context required".to_string(),
            }),
        )
    })?;

    if state
        .auth_service
        .check_permission(&claims.sub, tenant_id, "billing", "read")
        .await
        .is_ok()
    {
        return Ok(PaymentReadScope::Billing);
    }

    if state
        .auth_service
        .check_permission(&claims.sub, tenant_id, "customers", "read_own")
        .await
        .is_ok()
    {
        let customer_id = state
            .customer_service
            .get_portal_customer_id(&claims.sub, tenant_id)
            .await
            .map_err(|e| {
                (
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                )
            })?;
        return Ok(PaymentReadScope::CustomerPortal { customer_id });
    }

    // Backward-compatible fallback:
    // Some tenants still use role-only customer setup without explicit customers.read_own permission.
    if is_customer_role(&claims.role) {
        let customer_id = state
            .customer_service
            .get_portal_customer_id(&claims.sub, tenant_id)
            .await
            .map_err(|e| {
                (
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                )
            })?;
        return Ok(PaymentReadScope::CustomerPortal { customer_id });
    }

    Err((
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "Permission denied: billing read or customer portal access required".to_string(),
        }),
    ))
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

async fn require_work_order_manage_access(
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
        .check_permission(&claims.sub, tenant_id, "work_orders", "manage")
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
    let scope = resolve_payment_read_scope(&state, &claims).await?;
    if !matches!(scope, PaymentReadScope::Billing) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Billing read access required".to_string(),
            }),
        ));
    }

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
    let scope = resolve_payment_read_scope(&state, &claims).await?;
    let Some(tenant_id) = claims.tenant_id.as_deref() else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Tenant context required".to_string(),
            }),
        ));
    };

    let result = match scope {
        PaymentReadScope::Billing => state.payment_service.list_invoices(Some(tenant_id)).await,
        PaymentReadScope::CustomerPortal { customer_id } => {
            state
                .payment_service
                .list_customer_portal_invoices(tenant_id, &customer_id)
                .await
        }
    };

    result.map(Json).map_err(|e| {
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
struct CreateInvoiceForInstallationWorkOrderBody {
    work_order_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct VerifyCustomerPackagePaymentBody {
    status: String,
    rejection_reason: Option<String>,
    #[serde(default, alias = "invoice_id", alias = "id")]
    invoice_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct SubmitPaymentProofBody {
    file_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct PayInvoiceDuitkuBody {
    payment_method: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct DuitkuPaymentMethodsQuery {
    amount: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct BillingCollectionLogsQuery {
    action: Option<String>,
    result: Option<String>,
    from: Option<String>,
    to: Option<String>,
    search: Option<String>,
    limit: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct InvoiceReminderLogsQuery {
    reminder_code: Option<String>,
    status: Option<String>,
    from: Option<String>,
    to: Option<String>,
    search: Option<String>,
    limit: Option<u32>,
}

fn parse_utc_datetime_query(
    raw: Option<String>,
    field: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, (StatusCode, Json<ErrorResponse>)> {
    let Some(value) = raw.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };

    let parsed = chrono::DateTime::parse_from_rfc3339(&value).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("{field} must be ISO-8601 datetime (RFC3339)"),
            }),
        )
    })?;
    Ok(Some(parsed.with_timezone(&chrono::Utc)))
}

async fn authorize_invoice_access(
    state: &AppState,
    claims: &Claims,
    scope: &PaymentReadScope,
    invoice_id: &str,
) -> Result<Invoice, (StatusCode, Json<ErrorResponse>)> {
    let invoice = state
        .payment_service
        .get_invoice(invoice_id)
        .await
        .map_err(|e| {
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

    if let PaymentReadScope::CustomerPortal { customer_id } = scope {
        let owned = state
            .payment_service
            .customer_owns_package_invoice(tenant_id, customer_id, invoice_id)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: e.to_string(),
                    }),
                )
            })?;

        if !owned {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: "Invoice access denied".to_string(),
                }),
            ));
        }
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
    let scope = resolve_payment_read_scope(&state, &claims).await?;
    let invoice = authorize_invoice_access(&state, &claims, &scope, &id).await?;
    Ok(Json(invoice))
}

async fn list_customer_package_invoices(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListCustomerPackageInvoicesQuery>,
) -> Result<Json<PaginatedResponse<Invoice>>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    let scope = resolve_payment_read_scope(&state, &claims).await?;
    if !matches!(scope, PaymentReadScope::Billing) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Billing read access required".to_string(),
            }),
        ));
    }
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
        .list_customer_package_invoices(tenant_id, q.sort_by, q.sort_dir, q.page.unwrap_or(1), q.per_page.unwrap_or(25))
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

async fn change_subscription_package(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<crate::services::payment_service::dto::ChangePackageRequest>,
) -> Result<
    Json<crate::services::payment_service::dto::ChangePackageResult>,
    (StatusCode, Json<ErrorResponse>),
> {
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
        .change_subscription_package(&tenant_id, body)
        .await
        .map(Json)
        .map_err(|e| {
            let status = match &e {
                crate::error::AppError::NotFound(_) => StatusCode::NOT_FOUND,
                crate::error::AppError::Validation(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (
                status,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

async fn create_invoice_for_installation_work_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateInvoiceForInstallationWorkOrderBody>,
) -> Result<Json<Invoice>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_work_order_manage_access(&state, &claims).await?;
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
        .create_invoice_for_installation_work_order(&tenant_id, &body.work_order_id)
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

/// `POST /api/payment/invoices/bulk-send` — fan-out send for multiple invoices.
///
/// Phase 3 of bulk-send-invoice. Auth: same `billing:manage` permission used
/// by the rest of the manual billing routes. Tenant comes from the caller's
/// JWT; per-invoice tenant ownership is enforced inside the service.
///
/// Body shape: accepts BOTH the flat `BulkSendInvoiceRequest` AND a wrapped
/// `{ "request": BulkSendInvoiceRequest }` shape so the Tauri-IPC frontend
/// (which uses named arg `request: <dto>`) can hit the same HTTP fallback
/// without rewiring its call sites.
async fn bulk_send_invoices(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<
    Json<crate::services::payment_service::dto::BulkSendInvoiceResult>,
    (StatusCode, Json<ErrorResponse>),
> {
    let claims = authenticate(&state, &headers).await?;
    require_payment_manage_access(&state, &claims).await?;
    let tenant_id = claims.tenant_id.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "No tenant context".to_string(),
            }),
        )
    })?;

    // Accept wrapped `{request: {...}}` (Tauri IPC fallback) OR flat body.
    let candidate = body.get("request").cloned().unwrap_or(body);
    let req: crate::services::payment_service::dto::BulkSendInvoiceRequest =
        serde_json::from_value(candidate).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Invalid bulk-send request body: {}", e),
                }),
            )
        })?;

    state
        .payment_service
        .bulk_send_invoices(&claims.sub, &tenant_id, req)
        .await
        .map(Json)
        .map_err(|e| match e {
            crate::error::AppError::Validation(_) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ),
        })
}

/// `GET /api/payment/billing/analytics` — aggregated billing metrics.
///
/// Returns MRR, ARR, collection rate, aging report, churn, and revenue trend.
/// Auth: `billing:read` scope. Tenant comes from JWT.
async fn get_billing_analytics(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<
    Json<crate::services::payment_service::analytics::BillingAnalytics>,
    (StatusCode, Json<ErrorResponse>),
> {
    let claims = authenticate(&state, &headers).await?;
    let scope = resolve_payment_read_scope(&state, &claims).await?;
    if !matches!(scope, PaymentReadScope::Billing) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Billing read access required".to_string(),
            }),
        ));
    }
    let tenant_id = claims.tenant_id.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Tenant context required".to_string(),
            }),
        )
    })?;

    crate::services::payment_service::analytics::compute_billing_analytics_for_service(
        &state.payment_service,
        &tenant_id,
    )
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

async fn list_billing_collection_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<BillingCollectionLogsQuery>,
) -> Result<Json<Vec<BillingCollectionLogView>>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    let scope = resolve_payment_read_scope(&state, &claims).await?;
    if !matches!(scope, PaymentReadScope::Billing) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Billing read access required".to_string(),
            }),
        ));
    }
    let Some(tenant_id) = claims.tenant_id.as_deref() else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Tenant context required".to_string(),
            }),
        ));
    };

    let from = parse_utc_datetime_query(q.from, "from")?;
    let to = parse_utc_datetime_query(q.to, "to")?;
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);

    state
        .payment_service
        .list_billing_collection_logs(
            tenant_id,
            q.action.as_deref(),
            q.result.as_deref(),
            from,
            to,
            q.search.as_deref(),
            limit,
        )
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

async fn list_invoice_reminder_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<InvoiceReminderLogsQuery>,
) -> Result<Json<Vec<InvoiceReminderLogView>>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    let scope = resolve_payment_read_scope(&state, &claims).await?;
    if !matches!(scope, PaymentReadScope::Billing) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Billing read access required".to_string(),
            }),
        ));
    }
    let Some(tenant_id) = claims.tenant_id.as_deref() else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Tenant context required".to_string(),
            }),
        ));
    };

    let from = parse_utc_datetime_query(q.from, "from")?;
    let to = parse_utc_datetime_query(q.to, "to")?;
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);

    state
        .payment_service
        .list_invoice_reminder_logs(
            tenant_id,
            q.reminder_code.as_deref(),
            q.status.as_deref(),
            from,
            to,
            q.search.as_deref(),
            limit,
        )
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

async fn run_billing_collection_now(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<BillingCollectionRunResult>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_payment_manage_access(&state, &claims).await?;
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
        .run_billing_collection_now(tenant_id)
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

    let invoice =
        authorize_invoice_access(&state, &claims, &PaymentReadScope::Billing, &id).await?;
    if let Some(invoice_id) = body.invoice_id.as_deref() {
        if invoice_id != id {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invoice ID mismatch".to_string(),
                }),
            ));
        }
    }
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

async fn verify_invoice_payment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<VerifyCustomerPackagePaymentBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    require_superadmin(&claims)?;

    if let Some(invoice_id) = body.invoice_id.as_deref() {
        if invoice_id != id {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invoice ID mismatch".to_string(),
                }),
            ));
        }
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

async fn submit_payment_proof(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<SubmitPaymentProofBody>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    let scope = resolve_payment_read_scope(&state, &claims).await?;
    let _ = authorize_invoice_access(&state, &claims, &scope, &id).await?;

    if body.file_path.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "filePath is required".to_string(),
            }),
        ));
    }

    state
        .payment_service
        .submit_payment_proof(&id, body.file_path.trim())
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
    let scope = resolve_payment_read_scope(&state, &claims).await?;
    let _ = authorize_invoice_access(&state, &claims, &scope, &id).await?;

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

async fn pay_invoice_duitku(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<PayInvoiceDuitkuBody>,
) -> Result<Json<String>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    let scope = resolve_payment_read_scope(&state, &claims).await?;
    let _ = authorize_invoice_access(&state, &claims, &scope, &id).await?;

    state
        .payment_service
        .initiate_duitku(&id, body.payment_method.as_deref())
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

async fn list_duitku_payment_methods(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<DuitkuPaymentMethodsQuery>,
) -> Result<
    Json<Vec<crate::services::payment_service::DuitkuPaymentMethod>>,
    (StatusCode, Json<ErrorResponse>),
> {
    let claims = authenticate(&state, &headers).await?;
    require_payment_manage_access(&state, &claims).await?;
    let tenant_id = if claims.is_super_admin {
        None
    } else {
        claims.tenant_id.as_deref()
    };

    state
        .payment_service
        .list_duitku_payment_methods(tenant_id, q.amount)
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
    let scope = resolve_payment_read_scope(&state, &claims).await?;
    let _ = authorize_invoice_access(&state, &claims, &scope, &id).await?;

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
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<BankAccount>>, (StatusCode, Json<ErrorResponse>)> {
    let claims = authenticate(&state, &headers).await?;
    let _ = resolve_payment_read_scope(&state, &claims).await?;

    // For customer portal: accept tenant_id from query param (e.g., from invoice's merchant_id)
    // For admin: use their own tenant_id from claims
    let tenant_id = params
        .get("tenant_id")
        .map(|s| s.as_str())
        .or(claims.tenant_id.as_deref());

    state
        .payment_service
        .list_bank_accounts(tenant_id)
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

    // Superadmin creates bank account — if they have a tenant_id, attach it; otherwise global
    let tenant_id = claims.tenant_id.clone();

    state
        .payment_service
        .create_bank_account(req, tenant_id)
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
    Extension(correlation_id): Extension<CorrelationId>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let payment_service = &state.payment_service;
    tracing::info!(
        request_id = correlation_id.as_str(),
        "Received Midtrans notification"
    );

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
        .process_midtrans_notification(
            order_id,
            payment_status,
            Some(correlation_id.as_str()),
            Some(signature_key),
        )
        .await
    {
        Ok(_) => (StatusCode::OK, "OK"),
        Err(e) => {
            tracing::error!(
                request_id = correlation_id.as_str(),
                order_id,
                error = %e,
                "Failed to process notification"
            );
            (StatusCode::INTERNAL_SERVER_ERROR, "Processing Error")
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DuitkuCallbackPayload {
    merchant_code: String,
    amount: String,
    merchant_order_id: String,
    result_code: String,
    signature: String,
    #[serde(default)]
    reference: Option<String>,
}

async fn duitku_callback(
    State(state): State<AppState>,
    Extension(correlation_id): Extension<CorrelationId>,
    Form(payload): Form<DuitkuCallbackPayload>,
) -> impl IntoResponse {
    let payment_service = &state.payment_service;
    tracing::info!(
        request_id = correlation_id.as_str(),
        merchant_order_id = payload.merchant_order_id.as_str(),
        "Received Duitku callback"
    );

    if payload.merchant_code.trim().is_empty()
        || payload.amount.trim().is_empty()
        || payload.merchant_order_id.trim().is_empty()
        || payload.result_code.trim().is_empty()
        || payload.signature.trim().is_empty()
    {
        return (StatusCode::BAD_REQUEST, "Invalid Payload");
    }

    let signature_ok = match payment_service
        .verify_duitku_callback_signature(
            &payload.merchant_code,
            &payload.amount,
            &payload.merchant_order_id,
            &payload.signature,
        )
        .await
    {
        Ok(ok) => ok,
        Err(e) => {
            tracing::error!("Failed Duitku signature verification: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Verification Error");
        }
    };

    if !signature_ok {
        tracing::warn!("Duitku callback rejected due to invalid signature");
        return (StatusCode::UNAUTHORIZED, "Invalid Signature");
    }

    let payment_status =
        crate::services::payment_service::duitku_callback_result_code_to_invoice_status(
            payload.result_code.trim(),
        );

    match payment_service
        .process_midtrans_notification(
            &payload.merchant_order_id,
            payment_status,
            Some(correlation_id.as_str()),
            payload.reference.as_deref().or(Some("duitku-callback")),
        )
        .await
    {
        Ok(_) => (StatusCode::OK, "SUCCESS"),
        Err(e) => {
            tracing::error!(
                request_id = correlation_id.as_str(),
                merchant_order_id = payload.merchant_order_id.as_str(),
                error = %e,
                "Failed to process Duitku callback"
            );
            (StatusCode::INTERNAL_SERVER_ERROR, "Processing Error")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_customer_role, parse_utc_datetime_query, ListCustomerPackageInvoicesQuery,
        VerifyCustomerPackagePaymentBody,
    };
    use axum::extract::Query;
    use axum::http::{StatusCode, Uri};

    #[test]
    fn customer_role_detection_is_trimmed_and_case_insensitive() {
        assert!(is_customer_role("customer"));
        assert!(is_customer_role(" Customer "));
        assert!(is_customer_role("CUSTOMER"));

        assert!(!is_customer_role("admin"));
        assert!(!is_customer_role(""));
    }

    #[test]
    fn verify_customer_package_body_supports_current_invoice_id_aliases() {
        let camel: VerifyCustomerPackagePaymentBody =
            serde_json::from_str(r#"{"status":"paid","invoiceId":"inv-1"}"#)
                .expect("invoiceId payload should deserialize");
        assert_eq!(camel.invoice_id.as_deref(), Some("inv-1"));

        let snake: VerifyCustomerPackagePaymentBody =
            serde_json::from_str(r#"{"status":"paid","invoice_id":"inv-2"}"#)
                .expect("invoice_id payload should deserialize");
        assert_eq!(snake.invoice_id.as_deref(), Some("inv-2"));

        let short_alias: VerifyCustomerPackagePaymentBody =
            serde_json::from_str(r#"{"status":"paid","id":"inv-3"}"#)
                .expect("id payload should deserialize");
        assert_eq!(short_alias.invoice_id.as_deref(), Some("inv-3"));
    }

    #[test]
    fn parse_utc_datetime_query_handles_empty_invalid_and_valid_values() {
        let none_res = parse_utc_datetime_query(None, "from");
        assert!(none_res.is_ok());
        match none_res {
            Ok(v) => assert_eq!(v, None),
            Err(_) => panic!("None should parse"),
        }

        let blank_res = parse_utc_datetime_query(Some("   ".to_string()), "to");
        assert!(blank_res.is_ok());
        match blank_res {
            Ok(v) => assert_eq!(v, None),
            Err(_) => panic!("blank should parse"),
        }

        let err = parse_utc_datetime_query(Some("not-a-date".to_string()), "from")
            .expect_err("invalid datetime should fail");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert_eq!(err.1 .0.error, "from must be ISO-8601 datetime (RFC3339)");

        let parsed_res =
            parse_utc_datetime_query(Some("2026-03-27T11:22:33+07:00".to_string()), "from");
        assert!(parsed_res.is_ok());
        let parsed = match parsed_res {
            Ok(Some(dt)) => dt,
            Ok(None) => panic!("valid datetime should be Some"),
            Err(_) => panic!("valid datetime should parse"),
        };
        assert_eq!(parsed.to_rfc3339(), "2026-03-27T04:22:33+00:00");
    }

    #[test]
    fn list_customer_package_query_parsing_characterizes_current_http_shape() {
        let uri: Uri = "/?sort_by=due_date&sort_dir=desc"
            .parse()
            .expect("valid uri");
        let Query(params) =
            Query::<ListCustomerPackageInvoicesQuery>::try_from_uri(&uri).expect("params parse");

        assert_eq!(params.sort_by.as_deref(), Some("due_date"));
        assert_eq!(params.sort_dir.as_deref(), Some("desc"));

        let camel_uri: Uri = "/?sortBy=due_date&sortDir=asc".parse().expect("valid uri");
        let camel_parse = Query::<ListCustomerPackageInvoicesQuery>::try_from_uri(&camel_uri);
        assert!(camel_parse.is_err());
        let err_text = match camel_parse {
            Ok(_) => panic!("camelCase params should be rejected by deny_unknown_fields"),
            Err(err) => err.to_string(),
        };
        assert!(err_text.contains("unknown field `sortBy`"));
    }
}
