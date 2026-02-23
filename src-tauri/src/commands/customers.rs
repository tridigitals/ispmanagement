use crate::models::{
    AddCustomerPortalUserRequest, CreateCustomerLocationRequest, CreateCustomerPortalUserRequest,
    CreateCustomerRegistrationInviteRequest, CreateCustomerRequest, CreateCustomerSubscriptionRequest,
    CreateCustomerWithPortalRequest, Customer, CustomerLocation, CustomerPortalUser,
    CustomerRegistrationInviteCreateResponse, CustomerRegistrationInviteView, CustomerSubscription,
    CustomerSubscriptionView, Invoice, IspPackage, PaginatedResponse,
    PortalCheckoutSubscriptionRequest, UpdateCustomerLocationRequest, UpdateCustomerRequest,
    UpdateCustomerSubscriptionRequest,
};
use crate::services::{AuthService, CustomerService, PaymentService};
use tauri::State;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortalCheckoutResponse {
    pub subscription: CustomerSubscription,
    pub invoice: Invoice,
}

#[tauri::command]
pub async fn list_customers(
    token: String,
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<PaginatedResponse<Customer>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .list_customers(
            &claims.sub,
            &tenant_id,
            q,
            page.unwrap_or(1),
            per_page.unwrap_or(25),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_customer(
    token: String,
    customer_id: String,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<Customer, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .get_customer(&claims.sub, &tenant_id, &customer_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_customer(
    token: String,
    dto: CreateCustomerRequest,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<Customer, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .create_customer(&claims.sub, &tenant_id, dto, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_customer_with_portal(
    token: String,
    dto: CreateCustomerWithPortalRequest,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<Customer, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .create_customer_with_portal(&claims.sub, &tenant_id, dto, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_customer(
    token: String,
    customer_id: String,
    dto: UpdateCustomerRequest,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<Customer, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .update_customer(
            &claims.sub,
            &tenant_id,
            &customer_id,
            dto,
            Some("127.0.0.1"),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_customer(
    token: String,
    customer_id: String,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .delete_customer(&claims.sub, &tenant_id, &customer_id, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_customer_registration_invite(
    token: String,
    dto: CreateCustomerRegistrationInviteRequest,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<CustomerRegistrationInviteCreateResponse, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .create_customer_registration_invite(&claims.sub, &tenant_id, dto, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_customer_registration_invites(
    token: String,
    include_inactive: Option<bool>,
    limit: Option<u32>,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<Vec<CustomerRegistrationInviteView>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .list_customer_registration_invites(
            &claims.sub,
            &tenant_id,
            include_inactive.unwrap_or(true),
            limit.unwrap_or(50),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn revoke_customer_registration_invite(
    token: String,
    invite_id: String,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .revoke_customer_registration_invite(
            &claims.sub,
            &tenant_id,
            &invite_id,
            Some("127.0.0.1"),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_customer_locations(
    token: String,
    customer_id: String,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<Vec<CustomerLocation>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .list_locations(&claims.sub, &tenant_id, &customer_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_customer_location(
    token: String,
    dto: CreateCustomerLocationRequest,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<CustomerLocation, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .create_location(&claims.sub, &tenant_id, dto, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_customer_location(
    token: String,
    location_id: String,
    dto: UpdateCustomerLocationRequest,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<CustomerLocation, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .update_location(
            &claims.sub,
            &tenant_id,
            &location_id,
            dto,
            Some("127.0.0.1"),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_customer_location(
    token: String,
    location_id: String,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .delete_location(&claims.sub, &tenant_id, &location_id, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_customer_portal_users(
    token: String,
    customer_id: String,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<Vec<CustomerPortalUser>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .list_portal_users(&claims.sub, &tenant_id, &customer_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_customer_portal_user(
    token: String,
    dto: AddCustomerPortalUserRequest,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<CustomerPortalUser, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .add_portal_user(&claims.sub, &tenant_id, dto, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_customer_portal_user(
    token: String,
    dto: CreateCustomerPortalUserRequest,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<CustomerPortalUser, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .create_portal_user(&claims.sub, &tenant_id, dto, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_customer_portal_user(
    token: String,
    customer_user_id: String,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .remove_portal_user(
            &claims.sub,
            &tenant_id,
            &customer_user_id,
            Some("127.0.0.1"),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_my_customer_locations(
    token: String,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<Vec<CustomerLocation>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .list_my_locations(&claims.sub, &tenant_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_my_customer_packages(
    token: String,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<Vec<IspPackage>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .list_my_packages(&claims.sub, &tenant_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_my_customer_subscriptions(
    token: String,
    page: Option<u32>,
    per_page: Option<u32>,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<PaginatedResponse<CustomerSubscriptionView>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .list_my_subscriptions(
            &claims.sub,
            &tenant_id,
            page.unwrap_or(1),
            per_page.unwrap_or(25),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_my_customer_subscription_invoice(
    token: String,
    dto: PortalCheckoutSubscriptionRequest,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
    payment: State<'_, PaymentService>,
) -> Result<PortalCheckoutResponse, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    let subscription = customers
        .create_my_subscription(&claims.sub, &tenant_id, dto, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())?;

    let invoice = payment
        .create_invoice_for_customer_subscription(&tenant_id, &subscription.id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(PortalCheckoutResponse {
        subscription,
        invoice,
    })
}

#[tauri::command]
pub async fn list_customer_subscriptions(
    token: String,
    customer_id: String,
    page: Option<u32>,
    per_page: Option<u32>,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<PaginatedResponse<CustomerSubscriptionView>, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .list_customer_subscriptions(
            &claims.sub,
            &tenant_id,
            &customer_id,
            page.unwrap_or(1),
            per_page.unwrap_or(25),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_customer_subscription(
    token: String,
    dto: CreateCustomerSubscriptionRequest,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<CustomerSubscription, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .create_customer_subscription(&claims.sub, &tenant_id, dto, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_customer_subscription(
    token: String,
    subscription_id: String,
    dto: UpdateCustomerSubscriptionRequest,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<CustomerSubscription, String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .update_customer_subscription(
            &claims.sub,
            &tenant_id,
            &subscription_id,
            dto,
            Some("127.0.0.1"),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_customer_subscription(
    token: String,
    subscription_id: String,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<(), String> {
    let claims = auth
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .delete_customer_subscription(&claims.sub, &tenant_id, &subscription_id, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}
