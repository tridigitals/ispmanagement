use crate::error::{AppError, AppResult};
use crate::http::auth::extract_ip;
use crate::http::AppState;
use crate::models::{
    AddCustomerPortalUserRequest, CreateCustomerLocationRequest, CreateCustomerPortalUserRequest,
    CreateCustomerRegistrationInviteRequest, CreateCustomerRequest,
    CreateCustomerSubscriptionRequest, CreateCustomerWithPortalRequest,
    CreateMyCustomerLocationRequest, Customer, CustomerLocation, CustomerPortalUser,
    CustomerRegistrationInviteCreateResponse, CustomerRegistrationInvitePolicy,
    CustomerRegistrationInviteSummary, CustomerRegistrationInviteView, CustomerSubscription,
    CustomerSubscriptionView, Invoice, IspPackage, PaginatedResponse,
    PortalCheckoutSubscriptionRequest, UpdateCustomerLocationRequest,
    UpdateCustomerRegistrationInvitePolicyRequest, UpdateCustomerRequest,
    UpdateCustomerSubscriptionRequest,
};
use axum::{
    extract::{ConnectInfo, Path, Query, State},
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
        .route(
            "/{id}",
            get(get_customer)
                .put(update_customer)
                .delete(delete_customer),
        )
        .route("/{id}/locations", get(list_locations))
        .route("/{id}/portal-users", get(list_portal_users))
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
            "/subscriptions/{subscription_id}",
            axum::routing::put(update_subscription).delete(delete_subscription),
        )
        // Customer portal
        .route(
            "/portal/my-locations",
            get(list_my_locations).post(create_my_location),
        )
        .route("/portal/my-packages", get(list_my_packages))
        .route("/portal/my-subscriptions", get(list_my_subscriptions))
        .route("/portal/checkout", post(portal_checkout_subscription))
}

fn bearer_token(headers: &HeaderMap) -> AppResult<String> {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or(AppError::Unauthorized)
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

#[derive(Debug, Deserialize)]
struct ListQuery {
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ListSubscriptionQuery {
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ListMySubscriptionQuery {
    page: Option<u32>,
    per_page: Option<u32>,
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

// GET /api/customers?q=...&page=1&per_page=25
async fn list_customers(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<PaginatedResponse<Customer>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let resp = state
        .customer_service
        .list_customers(
            &claims.sub,
            &tenant_id,
            q.q,
            q.page.unwrap_or(1),
            q.per_page.unwrap_or(25),
        )
        .await?;
    Ok(Json(resp))
}

// GET /api/customers/{id}
async fn get_customer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> AppResult<Json<Customer>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
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
    let ip = extract_ip(&headers, addr);
    state
        .customer_service
        .remove_portal_user(&claims.sub, &tenant_id, &customer_user_id, Some(&ip))
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
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
        )
        .await?;
    Ok(Json(rows))
}

// POST /api/customers/portal/checkout
async fn portal_checkout_subscription(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(dto): Json<PortalCheckoutSubscriptionRequest>,
) -> AppResult<Json<PortalCheckoutResponse>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);

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

// GET /api/customers/{id}/subscriptions
async fn list_subscriptions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<ListSubscriptionQuery>,
) -> AppResult<Json<PaginatedResponse<CustomerSubscriptionView>>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
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

// POST /api/customers/{id}/subscriptions
async fn create_subscription(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(mut dto): Json<CreateCustomerSubscriptionRequest>,
) -> AppResult<Json<CustomerSubscription>> {
    let (tenant_id, claims) = tenant_and_claims(&state, &headers).await?;
    let ip = extract_ip(&headers, addr);
    dto.customer_id = id;
    let row = state
        .customer_service
        .create_customer_subscription(&claims.sub, &tenant_id, dto, Some(&ip))
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
    let ip = extract_ip(&headers, addr);
    state
        .customer_service
        .delete_customer_subscription(&claims.sub, &tenant_id, &subscription_id, Some(&ip))
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
