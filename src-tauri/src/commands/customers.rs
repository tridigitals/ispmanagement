
use crate::models::{
    AddCustomerPortalUserRequest, CreateCustomerLocationRequest, CreateCustomerPortalUserRequest,
    CreateCustomerRequest, Customer, CustomerLocation, CustomerPortalUser, PaginatedResponse,
    UpdateCustomerLocationRequest, UpdateCustomerRequest,
};
use crate::services::{AuthService, CustomerService};
use tauri::State;

#[tauri::command]
pub async fn list_customers(
    token: String,
    q: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<PaginatedResponse<Customer>, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .list_customers(&claims.sub, &tenant_id, q, page.unwrap_or(1), per_page.unwrap_or(25))
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
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
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
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .create_customer(&claims.sub, &tenant_id, dto, Some("127.0.0.1"))
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
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
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
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .delete_customer(&claims.sub, &tenant_id, &customer_id, Some("127.0.0.1"))
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
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
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
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
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
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
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
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
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
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
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
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
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
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
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
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .remove_portal_user(&claims.sub, &tenant_id, &customer_user_id, Some("127.0.0.1"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_my_customer_locations(
    token: String,
    auth: State<'_, AuthService>,
    customers: State<'_, CustomerService>,
) -> Result<Vec<CustomerLocation>, String> {
    let claims = auth.validate_token(&token).await.map_err(|e| e.to_string())?;
    let tenant_id = claims
        .tenant_id
        .ok_or_else(|| "No tenant ID in token".to_string())?;

    customers
        .list_my_locations(&claims.sub, &tenant_id)
        .await
        .map_err(|e| e.to_string())
}

