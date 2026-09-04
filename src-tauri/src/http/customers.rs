use crate::error::{AppError, AppResult};
use crate::http::auth::extract_ip;
use crate::http::{middleware::CorrelationId, AppState};
use crate::models::{
    AddCustomerPortalUserRequest, BackofficeInstallationOrderResponse,
    CreateBackofficeInstallationOrderRequest, CreateCustomerLocationRequest,
    CreateCustomerPortalUserRequest, CreateCustomerRegistrationInviteRequest,
    CreateCustomerRequest, CreateCustomerSubscriptionRequest, CreateCustomerWithPortalRequest,
    CreateMyCustomerLocationRequest, Customer, CustomerLifecycleObservability, CustomerListItem,
    CustomerLocation, CustomerPortalSubscriptionStats, CustomerPortalUser,
    CustomerRegistrationInviteCreateResponse, CustomerRegistrationInvitePolicy,
    CustomerRegistrationInviteSummary, CustomerRegistrationInviteView,
    CustomerServiceLifecycleRepairResult, CustomerServiceLifecycleReport, CustomerSubscription,
    CustomerSubscriptionOption, CustomerSubscriptionView, CustomerSummary, InstallationWorkOrder,
    InstallationWorkOrderView, Invoice, IspPackage, PaginatedResponse,
    PortalCheckoutSubscriptionRequest, RepairCustomerServiceLifecycleRequest,
    ResetCustomerPortalPasswordRequest, ResetCustomerPortalPasswordResponse,
    UpdateCustomerLocationRequest, UpdateCustomerRegistrationInvitePolicyRequest,
    UpdateCustomerRequest, UpdateCustomerSubscriptionRequest, WorkOrderRescheduleRequestView,
};
use axum::{
    extract::{ConnectInfo, Extension, Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use std::net::SocketAddr;

pub fn router() -> Router<AppState> {
    Router::new()
        // Admin
        .route("/", get(list_customers).post(create_customer))
        .route("/with-portal", post(create_customer_with_portal))
        .route(
            "/orders/installations",
            post(create_backoffice_installation_order),
        )
        .route("/summary", get(get_customer_summary))
        .route(
            "/reconciliation/service-lifecycle",
            get(get_service_lifecycle_report),
        )
        .route(
            "/reconciliation/service-lifecycle/repair",
            post(repair_service_lifecycle_issues),
        )
        .route(
            "/invites",
            get(list_customer_registration_invites).post(create_customer_registration_invite),
        )
        .route(
            "/invites/policy",
            get(get_customer_registration_invite_policy)
                .put(update_customer_registration_invite_policy),
        )
        .route(
            "/invites/summary",
            get(get_customer_registration_invite_summary),
        )
        .route(
            "/invites/{invite_id}",
            delete(revoke_customer_registration_invite),
        )
        .route("/observability/lifecycle", get(get_lifecycle_observability))
        .route(
            "/{id}",
            get(get_customer)
                .put(update_customer)
                .delete(delete_customer),
        )
        .route("/{id}/locations", get(list_locations))
        .route("/{id}/portal-users", get(list_portal_users))
        .route("/subscriptions/options", get(list_subscription_options))
        .route(
            "/{id}/subscriptions",
            get(list_subscriptions).post(create_subscription),
        )
        // Locations (write)
        .route("/locations", post(create_location))
        .route(
            "/locations/{location_id}",
            axum::routing::put(update_location).delete(delete_location),
        )
        // Portal users (write)
        .route("/portal-users/add", post(add_portal_user))
        .route("/portal-users/create", post(create_portal_user))
        .route(
            "/portal-users/{customer_user_id}",
            delete(remove_portal_user),
        )
        .route(
            "/portal-users/{customer_user_id}/reset-password",
            post(reset_portal_user_password),
        )
        .route(
            "/subscriptions/{subscription_id}",
            get(get_subscription)
                .put(update_subscription)
                .delete(delete_subscription),
        )
        // Customer portal
        .route(
            "/portal/my-locations",
            get(list_my_locations).post(create_my_location),
        )
        .route(
            "/portal/my-locations/{location_id}",
            axum::routing::put(update_my_location).delete(delete_my_location),
        )
        .route("/portal/my-packages", get(list_my_packages))
        .route(
            "/portal/my-subscriptions/stats",
            get(get_my_subscription_stats),
        )
        .route("/portal/my-subscriptions", get(list_my_subscriptions))
        .route(
            "/portal/my-subscriptions/{subscription_id}",
            get(get_my_subscription),
        )
        .route(
            "/portal/my-subscriptions/{subscription_id}/installation-tracker",
            get(portal_get_installation_tracker),
        )
        .route(
            "/portal/my-subscriptions/{subscription_id}/reschedule-request",
            post(portal_reschedule_order_request_subscription),
        )
        .route(
            "/portal/my-subscriptions/{subscription_id}/reopen-request",
            post(portal_reopen_order_request_subscription),
        )
        .route(
            "/portal/order-request",
            post(portal_order_request_subscription),
        )
        .route("/portal/checkout", post(portal_checkout_subscription))
        .route("/portal/contact", get(portal_get_contact_info))
        .route("/portal/network-status", get(portal_get_network_status))
}

fn bearer_token(headers: &HeaderMap) -> AppResult<String> {
    crate::http::extract_token(headers)
}

async fn tenant_and_claims(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<(String, crate::services::auth_service::Claims)> {
    let token = bearer_token(headers)?;
    let claims = state.auth_service.validate_token(&token).await?;
    let tenant_id = claims.tenant_id.clone().ok_or(AppError::Unauthorized)?;
    Ok((tenant_id, claims))
}

async fn require_permission(
    state: &AppState,
    claims: &crate::services::auth_service::Claims,
    tenant_id: &str,
    resource: &str,
    action: &str,
) -> AppResult<()> {
    state
        .auth_service
        .check_permission(&claims.sub, tenant_id, resource, action)
        .await
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    q: Option<String>,
    status: Option<String>,
    service: Option<String>,
    installation: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ListSubscriptionQuery {
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ListSubscriptionOptionQuery {
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct LifecycleObservabilityQuery {
    customer_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListMySubscriptionQuery {
    page: Option<u32>,
    per_page: Option<u32>,
    status: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListCustomerInviteQuery {
    include_inactive: Option<bool>,
    limit: Option<u32>,
}

#[derive(Debug, serde::Serialize)]
struct PortalCheckoutResponse {
    subscription: CustomerSubscription,
    invoice: Invoice,
}

#[derive(Debug, serde::Serialize)]
struct PortalOrderRequestResponse {
    subscription: CustomerSubscription,
    work_order: InstallationWorkOrder,
}

#[derive(Debug, serde::Serialize)]
struct PortalInstallationTrackerResponse {
    subscription: CustomerSubscriptionView,
    work_order: Option<InstallationWorkOrderView>,
    reschedule_request: Option<WorkOrderRescheduleRequestView>,
}

// GET /api/customers?q=...&page=1&per_page=25
async fn list_customers(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<PaginatedResponse<CustomerListItem>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    if require_permission(&state, &claims, &tenant_id, "customers", "read")
        .await
        .is_err()
    {
        require_permission(&state, &claims, &tenant_id, "orders", "create").await?;
    }
    let resp = state
        .customer_service
        .list_customers(
            &claims.sub,
            &tenant_id,
            q.q,
            q.status,
            q.service,
            q.installation,
            q.page.unwrap_or(1),
            q.per_page.unwrap_or(25),
        )
        .await?;
    Ok(Json(resp))
}

// GET /api/customers/summary
async fn get_customer_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<CustomerSummary>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "read").await?;
    let summary = state
        .customer_service
        .get_customer_summary(&claims.sub, &tenant_id)
        .await?;
    Ok(Json(summary))
}

#[derive(Debug, Deserialize, Default)]
struct ServiceLifecycleReportQuery {
    q: Option<String>,
    issue_type: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

// GET /api/customers/reconciliation/service-lifecycle
async fn get_service_lifecycle_report(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ServiceLifecycleReportQuery>,
) -> AppResult<Json<CustomerServiceLifecycleReport>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "read").await?;
    let report = state
        .customer_service
        .get_service_lifecycle_report(
            &claims.sub,
            &tenant_id,
            q.q,
            q.issue_type,
            q.page.unwrap_or(1),
            q.per_page.unwrap_or(25),
        )
        .await?;
    Ok(Json(report))
}

// POST /api/customers/reconciliation/service-lifecycle/repair
async fn repair_service_lifecycle_issues(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<RepairCustomerServiceLifecycleRequest>,
) -> AppResult<Json<CustomerServiceLifecycleRepairResult>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "billing", "manage").await?;
    let result = state
        .customer_service
        .repair_service_lifecycle_issues(&claims.sub, &tenant_id, dto)
        .await?;
    Ok(Json(result))
}

// GET /api/customers/observability/lifecycle?customer_id=...
async fn get_lifecycle_observability(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<LifecycleObservabilityQuery>,
) -> AppResult<Json<CustomerLifecycleObservability>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "read").await?;
    let metrics = state
        .customer_service
        .get_lifecycle_observability(&claims.sub, &tenant_id, q.customer_id.as_deref())
        .await?;
    Ok(Json(metrics))
}

// GET /api/customers/{id}
async fn get_customer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<Customer>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    if require_permission(&state, &claims, &tenant_id, "customers", "read")
        .await
        .is_err()
    {
        require_permission(&state, &claims, &tenant_id, "orders", "create").await?;
    }
    let row = state
        .customer_service
        .get_customer(&claims.sub, &tenant_id, &id)
        .await?;
    Ok(Json(row))
}

// POST /api/customers
async fn create_customer(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<CreateCustomerRequest>,
) -> AppResult<Json<Customer>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "manage").await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .create_customer(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

// POST /api/customers/with-portal
async fn create_customer_with_portal(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<CreateCustomerWithPortalRequest>,
) -> AppResult<Json<Customer>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "manage").await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .create_customer_with_portal(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

// PUT /api/customers/{id}
async fn update_customer(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateCustomerRequest>,
) -> AppResult<Json<Customer>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "manage").await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .update_customer(&claims.sub, &tenant_id, &id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

// DELETE /api/customers/{id}
async fn delete_customer(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "manage").await?;
    let ip = extract_ip(&headers, addr);
    state
        .customer_service
        .delete_customer(&claims.sub, &tenant_id, &id, Some(&ip))
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// POST /api/customers/invites
async fn create_customer_registration_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<CreateCustomerRegistrationInviteRequest>,
) -> AppResult<Json<CustomerRegistrationInviteCreateResponse>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "manage").await?;
    let ip = extract_ip(&headers, addr);
    let invite = state
        .customer_service
        .create_customer_registration_invite(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;
    Ok(Json(invite))
}

// GET /api/customers/invites?include_inactive=true&limit=50
async fn list_customer_registration_invites(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListCustomerInviteQuery>,
) -> AppResult<Json<Vec<CustomerRegistrationInviteView>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "read").await?;
    let rows = state
        .customer_service
        .list_customer_registration_invites(
            &claims.sub,
            &tenant_id,
            q.include_inactive.unwrap_or(true),
            q.limit.unwrap_or(50),
        )
        .await?;
    Ok(Json(rows))
}

// GET /api/customers/invites/policy
async fn get_customer_registration_invite_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<CustomerRegistrationInvitePolicy>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "read").await?;
    let policy = state
        .customer_service
        .get_customer_registration_invite_policy(&claims.sub, &tenant_id)
        .await?;
    Ok(Json(policy))
}

// PUT /api/customers/invites/policy
async fn update_customer_registration_invite_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<UpdateCustomerRegistrationInvitePolicyRequest>,
) -> AppResult<Json<CustomerRegistrationInvitePolicy>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "manage").await?;
    let ip = extract_ip(&headers, addr);
    let policy = state
        .customer_service
        .update_customer_registration_invite_policy(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;
    Ok(Json(policy))
}

// GET /api/customers/invites/summary
async fn get_customer_registration_invite_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<CustomerRegistrationInviteSummary>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "read").await?;
    let summary = state
        .customer_service
        .summarize_customer_registration_invites(&claims.sub, &tenant_id)
        .await?;
    Ok(Json(summary))
}

// DELETE /api/customers/invites/{invite_id}
async fn revoke_customer_registration_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(invite_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "manage").await?;
    let ip = extract_ip(&headers, addr);
    state
        .customer_service
        .revoke_customer_registration_invite(&claims.sub, &tenant_id, &invite_id, Some(&ip))
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// GET /api/customers/{id}/locations
async fn list_locations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<CustomerLocation>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    if require_permission(&state, &claims, &tenant_id, "customer_locations", "read")
        .await
        .is_err()
    {
        require_permission(&state, &claims, &tenant_id, "orders", "create").await?;
    }
    let rows = state
        .customer_service
        .list_locations(&claims.sub, &tenant_id, &id)
        .await?;
    Ok(Json(rows))
}

// POST /api/customers/locations
async fn create_location(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<CreateCustomerLocationRequest>,
) -> AppResult<Json<CustomerLocation>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customer_locations", "manage").await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .create_location(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

// PUT /api/customers/locations/{location_id}
async fn update_location(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(location_id): Path<String>,
    Json(dto): Json<UpdateCustomerLocationRequest>,
) -> AppResult<Json<CustomerLocation>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customer_locations", "manage").await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .update_location(&claims.sub, &tenant_id, &location_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

// DELETE /api/customers/locations/{location_id}
async fn delete_location(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(location_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customer_locations", "manage").await?;
    let ip = extract_ip(&headers, addr);
    state
        .customer_service
        .delete_location(&claims.sub, &tenant_id, &location_id, Some(&ip))
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// GET /api/customers/{id}/portal-users
async fn list_portal_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<CustomerPortalUser>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "read").await?;
    let rows = state
        .customer_service
        .list_portal_users(&claims.sub, &tenant_id, &id)
        .await?;
    Ok(Json(rows))
}

// POST /api/customers/portal-users/add
async fn add_portal_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<AddCustomerPortalUserRequest>,
) -> AppResult<Json<CustomerPortalUser>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "manage").await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .add_portal_user(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

// POST /api/customers/portal-users/create
async fn create_portal_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<CreateCustomerPortalUserRequest>,
) -> AppResult<Json<CustomerPortalUser>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "manage").await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .create_portal_user(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

// DELETE /api/customers/portal-users/{customer_user_id}
async fn remove_portal_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(customer_user_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "manage").await?;
    let ip = extract_ip(&headers, addr);
    state
        .customer_service
        .remove_portal_user(&claims.sub, &tenant_id, &customer_user_id, Some(&ip))
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// POST /api/customers/portal-users/{customer_user_id}/reset-password
async fn reset_portal_user_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(customer_user_id): Path<String>,
    Json(body): Json<ResetCustomerPortalPasswordRequest>,
) -> AppResult<Json<ResetCustomerPortalPasswordResponse>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "customers", "manage").await?;
    let ip = extract_ip(&headers, addr);
    let result = state
        .customer_service
        .reset_portal_user_password(
            &claims.sub,
            &tenant_id,
            &customer_user_id,
            body.new_password.as_deref(),
            Some(&ip),
        )
        .await?;
    Ok(Json(result))
}

// GET /api/customers/portal/my-locations
async fn list_my_locations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<CustomerLocation>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let rows = state
        .customer_service
        .list_my_locations(&claims.sub, &tenant_id)
        .await?;
    Ok(Json(rows))
}

// POST /api/customers/portal/my-locations
async fn create_my_location(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<CreateMyCustomerLocationRequest>,
) -> AppResult<Json<CustomerLocation>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .create_my_location(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

// PUT /api/customers/portal/my-locations/{location_id}
async fn update_my_location(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(location_id): Path<String>,
    Json(dto): Json<UpdateCustomerLocationRequest>,
) -> AppResult<Json<CustomerLocation>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .update_my_location(&claims.sub, &tenant_id, &location_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

// DELETE /api/customers/portal/my-locations/{location_id}
async fn delete_my_location(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(location_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    state
        .customer_service
        .delete_my_location(&claims.sub, &tenant_id, &location_id, Some(&ip))
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// GET /api/customers/portal/my-packages
async fn list_my_packages(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<IspPackage>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let rows = state
        .customer_service
        .list_my_packages(&claims.sub, &tenant_id)
        .await?;
    Ok(Json(rows))
}

// GET /api/customers/portal/my-subscriptions
async fn list_my_subscriptions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListMySubscriptionQuery>,
) -> AppResult<Json<PaginatedResponse<CustomerSubscriptionView>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let rows = state
        .customer_service
        .list_my_subscriptions(
            &claims.sub,
            &tenant_id,
            q.page.unwrap_or(1),
            q.per_page.unwrap_or(25),
            q.status,
            q.sort_by,
            q.sort_dir,
        )
        .await?;
    Ok(Json(rows))
}

// GET /api/customers/portal/my-subscriptions/{id}
async fn get_my_subscription(
    State(state): State<AppState>,
    Path(subscription_id): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<CustomerSubscriptionView>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let row = state
        .customer_service
        .get_my_subscription(&claims.sub, &tenant_id, &subscription_id)
        .await?;
    Ok(Json(row))
}

// GET /api/customers/portal/my-subscriptions/stats
async fn get_my_subscription_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<CustomerPortalSubscriptionStats>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let stats = state
        .customer_service
        .get_my_subscription_stats(&claims.sub, &tenant_id)
        .await?;
    Ok(Json(stats))
}

// POST /api/customers/portal/checkout
async fn portal_checkout_subscription(
    State(state): State<AppState>,
    Extension(correlation_id): Extension<CorrelationId>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<PortalCheckoutSubscriptionRequest>,
) -> AppResult<Json<PortalCheckoutResponse>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    tracing::info!(request_id = correlation_id.as_str(), customer_user_id = %claims.sub, "Portal checkout subscription request");

    let subscription = state
        .customer_service
        .create_my_subscription(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;

    let invoice = state
        .payment_service
        .create_invoice_for_customer_subscription(&tenant_id, &subscription.id)
        .await?;

    Ok(Json(PortalCheckoutResponse {
        subscription,
        invoice,
    }))
}

// POST /api/customers/portal/order-request
async fn portal_order_request_subscription(
    State(state): State<AppState>,
    Extension(correlation_id): Extension<CorrelationId>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<PortalCheckoutSubscriptionRequest>,
) -> AppResult<Json<PortalOrderRequestResponse>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    tracing::info!(request_id = correlation_id.as_str(), customer_user_id = %claims.sub, "Portal order request subscription");

    let (subscription, work_order) = state
        .customer_service
        .create_my_subscription_order_request(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;

    Ok(Json(PortalOrderRequestResponse {
        subscription,
        work_order,
    }))
}

#[derive(Debug, Deserialize)]
struct PortalReopenRequestBody {
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PortalRescheduleRequestBody {
    scheduled_at: String,
    reason: Option<String>,
}

// GET /api/customers/portal/my-subscriptions/{subscription_id}/installation-tracker
async fn portal_get_installation_tracker(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(subscription_id): Path<String>,
) -> AppResult<Json<PortalInstallationTrackerResponse>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let (subscription, work_order, reschedule_request) = state
        .customer_service
        .get_my_subscription_installation_tracker(&claims.sub, &tenant_id, &subscription_id)
        .await?;

    Ok(Json(PortalInstallationTrackerResponse {
        subscription,
        work_order,
        reschedule_request,
    }))
}

// POST /api/customers/portal/my-subscriptions/{subscription_id}/reopen-request
async fn portal_reopen_order_request_subscription(
    State(state): State<AppState>,
    Extension(correlation_id): Extension<CorrelationId>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(subscription_id): Path<String>,
    Json(body): Json<PortalReopenRequestBody>,
) -> AppResult<Json<PortalOrderRequestResponse>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    tracing::info!(request_id = correlation_id.as_str(), subscription_id = %subscription_id, customer_user_id = %claims.sub, "Portal reopen order request subscription");

    let (subscription, work_order) = state
        .customer_service
        .reopen_my_subscription_order_request(
            &claims.sub,
            &tenant_id,
            &subscription_id,
            body.notes,
            Some(&ip),
        )
        .await?;

    Ok(Json(PortalOrderRequestResponse {
        subscription,
        work_order,
    }))
}

// POST /api/customers/portal/my-subscriptions/{subscription_id}/reschedule-request
async fn portal_reschedule_order_request_subscription(
    State(state): State<AppState>,
    Extension(correlation_id): Extension<CorrelationId>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(subscription_id): Path<String>,
    Json(body): Json<PortalRescheduleRequestBody>,
) -> AppResult<Json<PortalOrderRequestResponse>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    tracing::info!(request_id = correlation_id.as_str(), subscription_id = %subscription_id, customer_user_id = %claims.sub, "Portal reschedule order request subscription");

    let (subscription, work_order) = state
        .customer_service
        .request_my_subscription_reschedule(
            &claims.sub,
            &tenant_id,
            &subscription_id,
            body.scheduled_at,
            body.reason,
            Some(&ip),
        )
        .await?;

    Ok(Json(PortalOrderRequestResponse {
        subscription,
        work_order,
    }))
}

// GET /api/customers/{id}/subscriptions
async fn list_subscriptions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<ListSubscriptionQuery>,
) -> AppResult<Json<PaginatedResponse<CustomerSubscriptionView>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "billing", "read").await?;
    let rows = state
        .customer_service
        .list_customer_subscriptions(
            &claims.sub,
            &tenant_id,
            &id,
            q.page.unwrap_or(1),
            q.per_page.unwrap_or(25),
        )
        .await?;
    Ok(Json(rows))
}

// GET /api/customers/subscriptions/options
async fn list_subscription_options(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListSubscriptionOptionQuery>,
) -> AppResult<Json<Vec<CustomerSubscriptionOption>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "billing", "read").await?;
    let rows = state
        .customer_service
        .list_customer_subscription_options(&claims.sub, &tenant_id, q.limit.unwrap_or(2000))
        .await?;
    Ok(Json(rows))
}

// POST /api/customers/{id}/subscriptions
async fn create_subscription(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(mut dto): Json<CreateCustomerSubscriptionRequest>,
) -> AppResult<Json<CustomerSubscription>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "billing", "manage").await?;
    let ip = extract_ip(&headers, addr);
    dto.customer_id = Some(id);
    let row = state
        .customer_service
        .create_customer_subscription(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

async fn create_backoffice_installation_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<CreateBackofficeInstallationOrderRequest>,
) -> AppResult<Json<BackofficeInstallationOrderResponse>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "orders", "create").await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .create_backoffice_installation_order(&claims.sub, &tenant_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

// GET /api/customers/subscriptions/{subscription_id}
async fn get_subscription(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(subscription_id): Path<String>,
) -> AppResult<Json<CustomerSubscriptionView>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "billing", "read").await?;
    let row = state
        .customer_service
        .get_customer_subscription(&claims.sub, &tenant_id, &subscription_id)
        .await?;
    Ok(Json(row))
}

// PUT /api/customers/subscriptions/{subscription_id}
async fn update_subscription(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(subscription_id): Path<String>,
    Json(dto): Json<UpdateCustomerSubscriptionRequest>,
) -> AppResult<Json<CustomerSubscription>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "billing", "manage").await?;
    let ip = extract_ip(&headers, addr);
    let row = state
        .customer_service
        .update_customer_subscription(&claims.sub, &tenant_id, &subscription_id, dto, Some(&ip))
        .await?;
    Ok(Json(row))
}

// DELETE /api/customers/subscriptions/{subscription_id}
async fn delete_subscription(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(subscription_id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    require_permission(&state, &claims, &tenant_id, "billing", "manage").await?;
    let ip = extract_ip(&headers, addr);
    state
        .customer_service
        .delete_customer_subscription(&claims.sub, &tenant_id, &subscription_id, Some(&ip))
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// GET /api/customers/portal/contact
async fn portal_get_contact_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, _claims) = tenant_and_claims(&state, &headers).await?;

    let keys = [
        "organization_name",
        "company_phone",
        "company_whatsapp",
        "company_email",
        "company_address",
        "company_website",
    ];

    let mut contact = serde_json::Map::new();
    for key in &keys {
        if let Ok(Some(val)) = state
            .settings_service
            .get_value(Some(&tenant_id), key)
            .await
        {
            if !val.is_empty() {
                contact.insert(key.to_string(), serde_json::Value::String(val));
            }
        }
    }

    Ok(Json(serde_json::Value::Object(contact)))
}

// GET /api/customers/portal/network-status
//
// Returns the current operational status of the ISP's network. Backed by
// the `network_status` setting in the settings table (JSON-encoded) so that
// the admin can update it from the web admin without a code deploy.
//
// Default (when not set) is `{ status: "operational", area: "Semua Area" }`
// so the home banner shows nothing (NetworkStatusBanner hides when normal).
async fn portal_get_network_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let (tenant_id, _claims) = tenant_and_claims(&state, &headers).await?;

    let raw = state
        .settings_service
        .get_value(Some(&tenant_id), "network_status")
        .await
        .ok()
        .flatten()
        .filter(|v| !v.is_empty());

    let payload = match raw.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()) {
        Some(v) => v,
        None => serde_json::json!({
            "status": "operational",
            "area": "Semua Area",
        }),
    };

    Ok(Json(payload))
}
