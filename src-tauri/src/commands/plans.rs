//! Plan Management Commands (Superadmin only)

use crate::services::{AuthService, PlanService};
use crate::models::{
    Plan, PlanWithFeatures, FeatureDefinition, TenantSubscription, FeatureAccess,
    CreatePlanRequest, UpdatePlanRequest, CreateFeatureRequest, SetPlanFeatureRequest,
    AssignPlanRequest, TenantSubscriptionDetails,
};
use tauri::State;

/// Get detailed tenant subscription info (Usage vs Limits)
#[tauri::command]
pub async fn get_tenant_subscription_details(
    token: String,
    tenant_id: Option<String>,
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
) -> Result<TenantSubscriptionDetails, String> {
    let claims = auth_service.validate_token(&token).await
        .map_err(|e| e.to_string())?;
    
    let target_tenant_id = if let Some(tid) = tenant_id {
        if !claims.is_super_admin && claims.tenant_id.as_deref() != Some(&tid) {
            return Err("Unauthorized".to_string());
        }
        tid
    } else {
        claims.tenant_id.ok_or_else(|| "No tenant context".to_string())?
    };

    plan_service.get_tenant_subscription_details(&target_tenant_id).await
        .map_err(|e| e.to_string())
}

/// List all plans
#[tauri::command]
pub async fn list_plans(
    token: String,
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
) -> Result<Vec<Plan>, String> {
    let claims = auth_service.validate_token(&token).await
        .map_err(|e| e.to_string())?;
    
    if claims.is_super_admin {
        plan_service.list_plans().await
            .map_err(|e| e.to_string())
    } else {
        plan_service.list_active_plans().await
            .map_err(|e| e.to_string())
    }
}

/// Get plan with features
#[tauri::command]
pub async fn get_plan(
    token: String,
    plan_id: String,
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
) -> Result<Option<PlanWithFeatures>, String> {
    let claims = auth_service.validate_token(&token).await
        .map_err(|e| e.to_string())?;
    
    if !claims.is_super_admin {
        return Err("Unauthorized: Superadmin access required".to_string());
    }

    plan_service.get_plan_with_features(&plan_id).await
        .map_err(|e| e.to_string())
}

/// Create a new plan
#[tauri::command]
pub async fn create_plan(
    token: String,
    name: String,
    slug: String,
    description: Option<String>,
    price_monthly: Option<f64>,
    price_yearly: Option<f64>,
    is_active: Option<bool>,
    is_default: Option<bool>,
    sort_order: Option<i32>,
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
) -> Result<Plan, String> {
    let claims = auth_service.validate_token(&token).await
        .map_err(|e| e.to_string())?;
    
    if !claims.is_super_admin {
        return Err("Unauthorized: Superadmin access required".to_string());
    }

    let req = CreatePlanRequest {
        name,
        slug,
        description,
        price_monthly,
        price_yearly,
        is_active,
        is_default,
        sort_order,
    };

    plan_service.create_plan(req).await
        .map_err(|e| e.to_string())
}

/// Update a plan
#[tauri::command]
pub async fn update_plan(
    token: String,
    plan_id: String,
    name: Option<String>,
    slug: Option<String>,
    description: Option<String>,
    price_monthly: Option<f64>,
    price_yearly: Option<f64>,
    is_active: Option<bool>,
    is_default: Option<bool>,
    sort_order: Option<i32>,
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
) -> Result<Plan, String> {
    let claims = auth_service.validate_token(&token).await
        .map_err(|e| e.to_string())?;
    
    if !claims.is_super_admin {
        return Err("Unauthorized: Superadmin access required".to_string());
    }

    let req = UpdatePlanRequest {
        name,
        slug,
        description,
        price_monthly,
        price_yearly,
        is_active,
        is_default,
        sort_order,
    };

    plan_service.update_plan(&plan_id, req).await
        .map_err(|e| e.to_string())
}

/// Delete a plan
#[tauri::command]
pub async fn delete_plan(
    token: String,
    plan_id: String,
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
) -> Result<(), String> {
    let claims = auth_service.validate_token(&token).await
        .map_err(|e| e.to_string())?;
    
    if !claims.is_super_admin {
        return Err("Unauthorized: Superadmin access required".to_string());
    }

    plan_service.delete_plan(&plan_id).await
        .map_err(|e| e.to_string())
}

/// List all feature definitions
#[tauri::command]
pub async fn list_features(
    token: String,
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
) -> Result<Vec<FeatureDefinition>, String> {
    let claims = auth_service.validate_token(&token).await
        .map_err(|e| e.to_string())?;
    
    if !claims.is_super_admin {
        return Err("Unauthorized: Superadmin access required".to_string());
    }

    plan_service.list_feature_definitions().await
        .map_err(|e| e.to_string())
}

// Feature creation/deletion commands removed - System Managed only

/// Set a feature value for a plan
#[tauri::command]
pub async fn set_plan_feature(
    token: String,
    plan_id: String,
    feature_id: String,
    value: String,
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
) -> Result<(), String> {
    let claims = auth_service.validate_token(&token).await
        .map_err(|e| e.to_string())?;
    
    if !claims.is_super_admin {
        return Err("Unauthorized: Superadmin access required".to_string());
    }

    plan_service.set_plan_feature(&plan_id, &feature_id, &value).await
        .map_err(|e| e.to_string())
}

/// Assign a plan to a tenant
#[tauri::command]
pub async fn assign_plan_to_tenant(
    token: String,
    tenant_id: String,
    plan_id: String,
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
) -> Result<TenantSubscription, String> {
    let claims = auth_service.validate_token(&token).await
        .map_err(|e| e.to_string())?;
    
    if !claims.is_super_admin {
        return Err("Unauthorized: Superadmin access required".to_string());
    }

    plan_service.assign_plan_to_tenant(&tenant_id, &plan_id).await
        .map_err(|e| e.to_string())
}

/// Get tenant subscription
#[tauri::command]
pub async fn get_tenant_subscription(
    token: String,
    tenant_id: String,
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
) -> Result<Option<TenantSubscription>, String> {
    let claims = auth_service.validate_token(&token).await
        .map_err(|e| e.to_string())?;
    
    // Allow superadmin or if checking own tenant
    if !claims.is_super_admin && claims.tenant_id.as_deref() != Some(&tenant_id) {
        return Err("Unauthorized".to_string());
    }

    plan_service.get_tenant_subscription(&tenant_id).await
        .map_err(|e| e.to_string())
}

/// Check feature access for a tenant
#[tauri::command]
pub async fn check_feature_access(
    token: String,
    tenant_id: String,
    feature_code: String,
    auth_service: State<'_, AuthService>,
    plan_service: State<'_, PlanService>,
) -> Result<FeatureAccess, String> {
    let claims = auth_service.validate_token(&token).await
        .map_err(|e| e.to_string())?;
    
    // Allow superadmin or if checking own tenant
    if !claims.is_super_admin && claims.tenant_id.as_deref() != Some(&tenant_id) {
        return Err("Unauthorized".to_string());
    }

    plan_service.check_feature_access(&tenant_id, &feature_code).await
        .map_err(|e| e.to_string())
}
