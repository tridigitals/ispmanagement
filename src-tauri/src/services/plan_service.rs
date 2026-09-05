//! Plan Service - Manages subscription plans and features

use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    CreateFeatureRequest, CreatePlanRequest, FeatureAccess, FeatureDefinition, Plan, PlanFeature,
    PlanFeatureValue, PlanWithFeatures, TenantSubscription, UpdatePlanRequest,
};
use chrono::Utc;
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::Postgres;

/// Validasi slug plan: huruf kecil, angka, dash/underscore, 2-40 karakter.
/// Slug masuk ke URL dan dipakai lookup `free`; dulu apa pun lolos dan
/// tabrakan baru ketahuan sebagai 500 unique-violation mentah.
pub fn validate_plan_slug(slug: &str) -> AppResult<()> {
    let ok = slug.len() >= 2
        && slug.len() <= 40
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(AppError::Validation(
            "slug must be 2-40 chars, lowercase letters, digits, '-' or '_'".to_string(),
        ))
    }
}

/// Nama plan tidak boleh kosong dan harus masuk akal panjangnya.
pub fn validate_plan_name(name: &str) -> AppResult<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 80 {
        return Err(AppError::Validation(
            "plan name must be 1-80 characters".to_string(),
        ));
    }
    Ok(())
}

/// Harga tidak boleh negatif dan harus finite. Harga negatif dulu lolos ke
/// DB dan menghasilkan invoice bernilai minus.
pub fn validate_plan_prices(monthly: Option<f64>, yearly: Option<f64>) -> AppResult<()> {
    for (label, v) in [("price_monthly", monthly), ("price_yearly", yearly)] {
        if let Some(x) = v {
            if !x.is_finite() || x < 0.0 {
                return Err(AppError::Validation(format!(
                    "{label} must be a finite non-negative number"
                )));
            }
        }
    }
    Ok(())
}

#[derive(Clone)]
pub struct PlanService {
    pool: DbPool,
}

impl PlanService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ==================== PLANS ====================

    /// List all plans
    pub async fn list_plans(&self) -> Result<Vec<Plan>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let plans: Vec<Plan> = sqlx::query_as(
            r#"
            SELECT 
                id, name, slug, description, 
                price_monthly::FLOAT8 as price_monthly, 
                price_yearly::FLOAT8 as price_yearly, 
                is_active, is_default, sort_order, created_at, updated_at
            FROM plans 
            ORDER BY sort_order ASC, created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let plans: Vec<Plan> =
            sqlx::query_as("SELECT * FROM plans ORDER BY sort_order ASC, created_at ASC")
                .fetch_all(&self.pool)
                .await?;

        Ok(plans)
    }

    /// List active plans (for public/tenant view)
    pub async fn list_active_plans(&self) -> Result<Vec<Plan>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let plans: Vec<Plan> = sqlx::query_as(
            r#"
            SELECT 
                id, name, slug, description, 
                price_monthly::FLOAT8 as price_monthly, 
                price_yearly::FLOAT8 as price_yearly, 
                is_active, is_default, sort_order, created_at, updated_at
            FROM plans 
            WHERE is_active = true
            ORDER BY sort_order ASC, created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let plans: Vec<Plan> = sqlx::query_as(
            "SELECT * FROM plans WHERE is_active = 1 ORDER BY sort_order ASC, created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(plans)
    }

    /// Get plan by ID with features
    pub async fn get_plan_with_features(
        &self,
        plan_id: &str,
    ) -> Result<Option<PlanWithFeatures>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let plan: Option<Plan> = sqlx::query_as(
            r#"
            SELECT 
                id, name, slug, description, 
                price_monthly::FLOAT8 as price_monthly, 
                price_yearly::FLOAT8 as price_yearly, 
                is_active, is_default, sort_order, created_at, updated_at
            FROM plans 
            WHERE id = $1
            "#,
        )
        .bind(plan_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let plan: Option<Plan> = sqlx::query_as("SELECT * FROM plans WHERE id = ?")
            .bind(plan_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(plan) = plan {
            let features = self.get_plan_features(plan_id).await?;
            Ok(Some(PlanWithFeatures { plan, features }))
        } else {
            Ok(None)
        }
    }

    /// Get features for a plan
    pub async fn get_plan_features(
        &self,
        plan_id: &str,
    ) -> Result<Vec<PlanFeatureValue>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let features: Vec<PlanFeatureValue> = sqlx::query_as(
            r#"
            SELECT 
                pf.feature_id,
                fd.code,
                fd.name,
                fd.value_type,
                pf.value,
                fd.category
            FROM plan_features pf
            JOIN features fd ON fd.id = pf.feature_id
            WHERE pf.plan_id = $1
            ORDER BY fd.sort_order ASC
            "#,
        )
        .bind(plan_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let features: Vec<PlanFeatureValue> = sqlx::query_as(
            r#"
            SELECT 
                pf.feature_id,
                fd.code,
                fd.name,
                fd.value_type,
                pf.value,
                fd.category
            FROM plan_features pf
            JOIN features fd ON fd.id = pf.feature_id
            WHERE pf.plan_id = ?
            ORDER BY fd.sort_order ASC
            "#,
        )
        .bind(plan_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(features)
    }

    /// Create a new plan
    pub async fn create_plan(&self, req: CreatePlanRequest) -> AppResult<Plan> {
        validate_plan_name(&req.name)?;
        validate_plan_slug(&req.slug)?;
        validate_plan_prices(req.price_monthly, req.price_yearly)?;
        if self.find_plan_by_slug(&req.slug).await?.is_some() {
            return Err(AppError::Conflict(format!(
                "a plan with slug '{}' already exists",
                req.slug
            )));
        }
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO plans (id, name, slug, description, price_monthly, price_yearly, is_active, is_default, sort_order, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#
        )
        .bind(&id)
        .bind(&req.name)
        .bind(&req.slug)
        .bind(&req.description)
        .bind(req.price_monthly.unwrap_or(0.0))
        .bind(req.price_yearly.unwrap_or(0.0))
        .bind(req.is_active.unwrap_or(true))
        .bind(req.is_default.unwrap_or(false))
        .bind(req.sort_order.unwrap_or(0))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO plans (id, name, slug, description, price_monthly, price_yearly, is_active, is_default, sort_order, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&id)
        .bind(&req.name)
        .bind(&req.slug)
        .bind(&req.description)
        .bind(req.price_monthly.unwrap_or(0.0))
        .bind(req.price_yearly.unwrap_or(0.0))
        .bind(req.is_active.unwrap_or(true))
        .bind(req.is_default.unwrap_or(false))
        .bind(req.sort_order.unwrap_or(0))
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        // Fetch and return
        self.get_plan(&id).await.map_err(AppError::from)
    }

    /// Cari plan by slug (untuk guard duplikat sebelum INSERT).
    async fn find_plan_by_slug(&self, slug: &str) -> AppResult<Option<Plan>> {
        #[cfg(feature = "postgres")]
        let plan: Option<Plan> = sqlx::query_as(
            r#"
            SELECT
                id, name, slug, description,
                price_monthly::FLOAT8 as price_monthly,
                price_yearly::FLOAT8 as price_yearly,
                is_active, is_default, sort_order, created_at, updated_at
            FROM plans WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let plan: Option<Plan> = sqlx::query_as("SELECT * FROM plans WHERE slug = ?")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?;

        Ok(plan)
    }

    /// Get plan by ID
    pub async fn get_plan(&self, plan_id: &str) -> Result<Plan, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let plan: Plan = sqlx::query_as(
            r#"
            SELECT 
                id, name, slug, description, 
                price_monthly::FLOAT8 as price_monthly, 
                price_yearly::FLOAT8 as price_yearly, 
                is_active, is_default, sort_order, created_at, updated_at
            FROM plans 
            WHERE id = $1
            "#,
        )
        .bind(plan_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let plan: Plan = sqlx::query_as("SELECT * FROM plans WHERE id = ?")
            .bind(plan_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(plan)
    }

    /// Update a plan
    pub async fn update_plan(
        &self,
        plan_id: &str,
        req: UpdatePlanRequest,
    ) -> AppResult<Plan> {
        if let Some(name) = &req.name {
            validate_plan_name(name)?;
        }
        if let Some(slug) = &req.slug {
            validate_plan_slug(slug)?;
            if let Some(existing) = self.find_plan_by_slug(slug).await? {
                if existing.id != plan_id {
                    return Err(AppError::Conflict(format!(
                        "a plan with slug '{slug}' already exists"
                    )));
                }
            }
        }
        validate_plan_prices(req.price_monthly, req.price_yearly)?;
        // Guard existence: UPDATE id tak dikenal dulu = sukses hampa lalu
        // get_plan fetch_one -> 500 "no rows".
        if self.get_plan_opt(plan_id).await?.is_none() {
            return Err(AppError::NotFound("plan not found".to_string()));
        }
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE plans SET
                name = COALESCE($2, name),
                slug = COALESCE($3, slug),
                description = COALESCE($4, description),
                price_monthly = COALESCE($5, price_monthly),
                price_yearly = COALESCE($6, price_yearly),
                is_active = COALESCE($7, is_active),
                is_default = COALESCE($8, is_default),
                sort_order = COALESCE($9, sort_order),
                updated_at = $10
            WHERE id = $1
            "#,
        )
        .bind(plan_id)
        .bind(&req.name)
        .bind(&req.slug)
        .bind(&req.description)
        .bind(req.price_monthly)
        .bind(req.price_yearly)
        .bind(req.is_active)
        .bind(req.is_default)
        .bind(req.sort_order)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        {
            // SQLite doesn't support COALESCE in UPDATE well, so we fetch first
            let existing = self.get_plan(plan_id).await?;
            sqlx::query(
                r#"
                UPDATE plans SET
                    name = ?, slug = ?, description = ?, price_monthly = ?, price_yearly = ?,
                    is_active = ?, is_default = ?, sort_order = ?, updated_at = ?
                WHERE id = ?
                "#,
            )
            .bind(req.name.as_ref().unwrap_or(&existing.name))
            .bind(req.slug.as_ref().unwrap_or(&existing.slug))
            .bind(req.description.as_ref().or(existing.description.as_ref()))
            .bind(req.price_monthly.unwrap_or(existing.price_monthly))
            .bind(req.price_yearly.unwrap_or(existing.price_yearly))
            .bind(req.is_active.unwrap_or(existing.is_active))
            .bind(req.is_default.unwrap_or(existing.is_default))
            .bind(req.sort_order.unwrap_or(existing.sort_order))
            .bind(now.to_rfc3339())
            .bind(plan_id)
            .execute(&self.pool)
            .await?;
        }

        self.get_plan(plan_id).await.map_err(AppError::from)
    }

    /// Ambil plan tanpa membingungkan "tidak ada" dengan error.
    async fn get_plan_opt(&self, plan_id: &str) -> AppResult<Option<Plan>> {
        #[cfg(feature = "postgres")]
        let plan: Option<Plan> = sqlx::query_as(
            r#"
            SELECT
                id, name, slug, description,
                price_monthly::FLOAT8 as price_monthly,
                price_yearly::FLOAT8 as price_yearly,
                is_active, is_default, sort_order, created_at, updated_at
            FROM plans WHERE id = $1
            "#,
        )
        .bind(plan_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let plan: Option<Plan> = sqlx::query_as("SELECT * FROM plans WHERE id = ?")
            .bind(plan_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(plan)
    }

    /// Delete a plan.
    ///
    /// plans dirujuk tenant_subscriptions & plan_features dengan FK
    /// NO ACTION — dulu hapus paket yang masih dipakai tenant membalas
    /// 500 "Database error" mentah. Sekarang: guard existence (404 jujur)
    /// + guard pemakaian (409 dengan daftar penyewa).
    pub async fn delete_plan(&self, plan_id: &str) -> AppResult<()> {
        let plan = self
            .get_plan_opt(plan_id)
            .await?
            .ok_or_else(|| AppError::NotFound("plan not found".to_string()))?;

        #[cfg(feature = "postgres")]
        let used_by: Vec<String> = sqlx::query_scalar(
            "SELECT t.slug FROM tenant_subscriptions ts JOIN tenants t ON t.id = ts.tenant_id WHERE ts.plan_id = $1 ORDER BY t.slug LIMIT 10",
        )
        .bind(plan_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let used_by: Vec<String> = sqlx::query_scalar(
            "SELECT t.slug FROM tenant_subscriptions ts JOIN tenants t ON t.id = ts.tenant_id WHERE ts.plan_id = ? ORDER BY t.slug LIMIT 10",
        )
        .bind(plan_id)
        .fetch_all(&self.pool)
        .await?;

        if !used_by.is_empty() {
            return Err(AppError::Conflict(format!(
                "plan '{}' is still used by: {}",
                plan.slug,
                used_by.join(", ")
            )));
        }

        // Putuskan relasi plan_features (FK NO ACTION juga) sebelum DELETE.
        #[cfg(feature = "postgres")]
        sqlx::query("DELETE FROM plan_features WHERE plan_id = $1")
            .bind(plan_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query("DELETE FROM plan_features WHERE plan_id = ?")
            .bind(plan_id)
            .execute(&self.pool)
            .await?;
        #[cfg(feature = "postgres")]
        sqlx::query("DELETE FROM plans WHERE id = $1")
            .bind(plan_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query("DELETE FROM plans WHERE id = ?")
            .bind(plan_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ==================== FEATURE DEFINITIONS ====================

    /// List all feature definitions
    pub async fn list_feature_definitions(&self) -> Result<Vec<FeatureDefinition>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let features: Vec<FeatureDefinition> =
            sqlx::query_as("SELECT * FROM features ORDER BY category ASC, sort_order ASC")
                .fetch_all(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let features: Vec<FeatureDefinition> =
            sqlx::query_as("SELECT * FROM features ORDER BY category ASC, sort_order ASC")
                .fetch_all(&self.pool)
                .await?;

        Ok(features)
    }

    /// Create a feature definition
    pub async fn create_feature(
        &self,
        req: CreateFeatureRequest,
    ) -> Result<FeatureDefinition, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO features (id, code, name, description, value_type, category, default_value, sort_order, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#
        )
        .bind(&id)
        .bind(&req.code)
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.value_type.as_deref().unwrap_or("boolean"))
        .bind(req.category.as_deref().unwrap_or("general"))
        .bind(req.default_value.as_deref().unwrap_or("false"))
        .bind(req.sort_order.unwrap_or(0))
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO features (id, code, name, description, value_type, category, default_value, sort_order, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&id)
        .bind(&req.code)
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.value_type.as_deref().unwrap_or("boolean"))
        .bind(req.category.as_deref().unwrap_or("general"))
        .bind(req.default_value.as_deref().unwrap_or("false"))
        .bind(req.sort_order.unwrap_or(0))
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.get_feature(&id).await
    }

    /// Get feature by ID
    pub async fn get_feature(&self, feature_id: &str) -> Result<FeatureDefinition, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let feature: FeatureDefinition = sqlx::query_as("SELECT * FROM features WHERE id = $1")
            .bind(feature_id)
            .fetch_one(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let feature: FeatureDefinition = sqlx::query_as("SELECT * FROM features WHERE id = ?")
            .bind(feature_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(feature)
    }

    /// Delete a feature definition
    pub async fn delete_feature(&self, feature_id: &str) -> Result<(), sqlx::Error> {
        #[cfg(feature = "postgres")]
        sqlx::query("DELETE FROM features WHERE id = $1")
            .bind(feature_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query("DELETE FROM features WHERE id = ?")
            .bind(feature_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ==================== PLAN FEATURES ====================

    /// Set a feature value for a plan
    pub async fn set_plan_feature(
        &self,
        plan_id: &str,
        feature_id: &str,
        value: &str,
    ) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO plan_features (id, plan_id, feature_id, value)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (plan_id, feature_id) DO UPDATE SET value = $4
            "#,
        )
        .bind(&id)
        .bind(plan_id)
        .bind(feature_id)
        .bind(value)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO plan_features (id, plan_id, feature_id, value)
            VALUES (?, ?, ?, ?)
            ON CONFLICT (plan_id, feature_id) DO UPDATE SET value = excluded.value
            "#,
        )
        .bind(&id)
        .bind(plan_id)
        .bind(feature_id)
        .bind(value)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Remove a feature from a plan
    #[allow(dead_code)]
    pub async fn remove_plan_feature(
        &self,
        plan_id: &str,
        feature_id: &str,
    ) -> Result<(), sqlx::Error> {
        #[cfg(feature = "postgres")]
        sqlx::query("DELETE FROM plan_features WHERE plan_id = $1 AND feature_id = $2")
            .bind(plan_id)
            .bind(feature_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query("DELETE FROM plan_features WHERE plan_id = ? AND feature_id = ?")
            .bind(plan_id)
            .bind(feature_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ==================== TENANT SUBSCRIPTIONS ====================

    /// Get tenant subscription (Internal raw fetch)
    async fn get_tenant_subscription_raw(
        &self,
        tenant_id: &str,
    ) -> Result<Option<TenantSubscription>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let sub: Option<TenantSubscription> = sqlx::query_as(
            r#"
            SELECT 
                id, tenant_id, plan_id, status, trial_ends_at, 
                current_period_start, current_period_end, 
                feature_overrides::TEXT as feature_overrides, 
                created_at, updated_at 
            FROM tenant_subscriptions WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let sub: Option<TenantSubscription> =
            sqlx::query_as("SELECT * FROM tenant_subscriptions WHERE tenant_id = ?")
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(sub)
    }

    /// Get tenant subscription with auto-expiration check
    pub async fn get_tenant_subscription(
        &self,
        tenant_id: &str,
    ) -> Result<Option<TenantSubscription>, sqlx::Error> {
        let sub = self.get_tenant_subscription_raw(tenant_id).await?;

        if let Some(ref s) = sub {
            // Check for expiration
            if let Some(end_date) = s.current_period_end {
                if end_date < Utc::now() && s.status == "active" {
                    // Expired! Downgrade to Free.
                    self.downgrade_to_free(tenant_id).await?;
                    // Return the new state
                    return self.get_tenant_subscription_raw(tenant_id).await;
                }
            }
        }

        Ok(sub)
    }

    /// Downgrade tenant to Free plan
    async fn downgrade_to_free(&self, tenant_id: &str) -> Result<(), sqlx::Error> {
        // 1. Get Free Plan ID
        #[cfg(feature = "postgres")]
        let free_plan_id: Option<String> =
            sqlx::query_scalar("SELECT id FROM plans WHERE slug = 'free'")
                .fetch_optional(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let free_plan_id: Option<String> =
            sqlx::query_scalar("SELECT id FROM plans WHERE slug = 'free'")
                .fetch_optional(&self.pool)
                .await?;

        if let Some(free_id) = free_plan_id {
            let now = Utc::now();

            // 2. Update Subscription
            #[cfg(feature = "postgres")]
            sqlx::query(
                "UPDATE tenant_subscriptions SET plan_id = $1, status = 'active', current_period_end = NULL, updated_at = $2 WHERE tenant_id = $3"
            )
            .bind(&free_id).bind(now).bind(tenant_id)
            .execute(&self.pool).await?;

            #[cfg(feature = "sqlite")]
            sqlx::query(
                "UPDATE tenant_subscriptions SET plan_id = ?, status = 'active', current_period_end = NULL, updated_at = ? WHERE tenant_id = ?"
            )
            .bind(&free_id).bind(now.to_rfc3339()).bind(tenant_id)
            .execute(&self.pool).await?;
        }
        Ok(())
    }

    /// Get tenant subscription within a transaction (PostgreSQL only)
    #[cfg(feature = "postgres")]
    pub async fn get_tenant_subscription_with_conn<'a>(
        &self,
        tenant_id: &str,
        tx: &mut sqlx::Transaction<'a, Postgres>,
    ) -> Result<Option<TenantSubscription>, sqlx::Error> {
        // 1. Fetch
        let sub: Option<TenantSubscription> = sqlx::query_as(
            r#"
            SELECT 
                id, tenant_id, plan_id, status, trial_ends_at, 
                current_period_start, current_period_end, 
                feature_overrides::TEXT as feature_overrides, 
                created_at, updated_at 
            FROM tenant_subscriptions WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&mut **tx)
        .await?;

        // 2. Check Expiration
        if let Some(ref s) = sub {
            if let Some(end_date) = s.current_period_end {
                if end_date < Utc::now() && s.status == "active" {
                    // Downgrade Logic (Inline for transaction)
                    let free_plan_id: Option<String> =
                        sqlx::query_scalar("SELECT id FROM plans WHERE slug = 'free'")
                            .fetch_optional(&mut **tx)
                            .await?;

                    if let Some(free_id) = free_plan_id {
                        let now = Utc::now();
                        sqlx::query(
                            "UPDATE tenant_subscriptions SET plan_id = $1, status = 'active', current_period_end = NULL, updated_at = $2 WHERE tenant_id = $3"
                        )
                        .bind(free_id).bind(now).bind(tenant_id)
                        .execute(&mut **tx).await?;

                        // Refetch
                        return sqlx::query_as(
                            r#"
                            SELECT 
                                id, tenant_id, plan_id, status, trial_ends_at, 
                                current_period_start, current_period_end, 
                                feature_overrides::TEXT as feature_overrides, 
                                created_at, updated_at 
                            FROM tenant_subscriptions WHERE tenant_id = $1
                            "#,
                        )
                        .bind(tenant_id)
                        .fetch_optional(&mut **tx)
                        .await;
                    }
                }
            }
        }

        Ok(sub)
    }

    /// Assign a plan to a tenant
    pub async fn assign_plan_to_tenant(
        &self,
        tenant_id: &str,
        plan_id: &str,
    ) -> AppResult<TenantSubscription> {
        // Guard plan: FK NO ACTION -> plan_id tak dikenal dulu = 500.
        if self.get_plan_opt(plan_id).await?.is_none() {
            return Err(AppError::NotFound("plan not found".to_string()));
        }
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO tenant_subscriptions (id, tenant_id, plan_id, status, current_period_start, created_at, updated_at)
            VALUES ($1, $2, $3, 'active', $4, $5, $6)
            ON CONFLICT (tenant_id) DO UPDATE SET 
                plan_id = $3, 
                status = 'active',
                current_period_start = $4,
                updated_at = $6
            "#
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(plan_id)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO tenant_subscriptions (id, tenant_id, plan_id, status, current_period_start, created_at, updated_at)
            VALUES (?, ?, ?, 'active', ?, ?, ?)
            ON CONFLICT (tenant_id) DO UPDATE SET 
                plan_id = excluded.plan_id, 
                status = 'active',
                current_period_start = excluded.current_period_start,
                updated_at = excluded.updated_at
            "#
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(plan_id)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.get_tenant_subscription(tenant_id)
            .await?
            // .unwrap() dulu bisa panic kalau baris hilang di antara
            // INSERT dan SELECT (race dengan penghapusan tenant).
            .ok_or_else(|| AppError::NotFound("subscription not found".to_string()))
    }

    // ==================== FEATURE ACCESS CHECKING ====================

    /// Check if a tenant has access to a feature
    pub async fn check_feature_access(
        &self,
        tenant_id: &str,
        feature_code: &str,
    ) -> Result<FeatureAccess, sqlx::Error> {
        // Get tenant's subscription
        let subscription = self.get_tenant_subscription(tenant_id).await?;

        // Get feature definition
        #[cfg(feature = "postgres")]
        let feature: Option<FeatureDefinition> =
            sqlx::query_as("SELECT * FROM features WHERE code = $1")
                .bind(feature_code)
                .fetch_optional(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let feature: Option<FeatureDefinition> =
            sqlx::query_as("SELECT * FROM features WHERE code = ?")
                .bind(feature_code)
                .fetch_optional(&self.pool)
                .await?;

        let feature = match feature {
            Some(f) => f,
            None => {
                return Ok(FeatureAccess {
                    code: feature_code.to_string(),
                    has_access: false,
                    value: "false".to_string(),
                    value_type: "boolean".to_string(),
                })
            }
        };

        // No subscription means use default value
        let subscription = match subscription {
            Some(s) => s,
            None => {
                return Ok(FeatureAccess {
                    code: feature_code.to_string(),
                    has_access: feature.default_value == "true"
                        || feature.default_value == "unlimited",
                    value: feature.default_value.clone(),
                    value_type: feature.value_type.clone(),
                })
            }
        };

        // Check for feature override in subscription
        if let Some(ref overrides_json) = subscription.feature_overrides {
            if let Ok(overrides) = serde_json::from_str::<serde_json::Value>(overrides_json) {
                if let Some(override_value) = overrides.get(feature_code) {
                    let value_str = override_value
                        .as_str()
                        .unwrap_or(&override_value.to_string())
                        .to_string();
                    return Ok(FeatureAccess {
                        code: feature_code.to_string(),
                        has_access: self.is_truthy(&value_str),
                        value: value_str,
                        value_type: feature.value_type,
                    });
                }
            }
        }

        // Get plan feature value
        #[cfg(feature = "postgres")]
        let plan_feature: Option<PlanFeature> =
            sqlx::query_as("SELECT * FROM plan_features WHERE plan_id = $1 AND feature_id = $2")
                .bind(&subscription.plan_id)
                .bind(&feature.id)
                .fetch_optional(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let plan_feature: Option<PlanFeature> =
            sqlx::query_as("SELECT * FROM plan_features WHERE plan_id = ? AND feature_id = ?")
                .bind(&subscription.plan_id)
                .bind(&feature.id)
                .fetch_optional(&self.pool)
                .await?;

        let value = plan_feature
            .map(|pf| pf.value)
            .unwrap_or(feature.default_value.clone());

        Ok(FeatureAccess {
            code: feature_code.to_string(),
            has_access: self.is_truthy(&value),
            value,
            value_type: feature.value_type,
        })
    }

    /// Check if a value is truthy (for boolean features) or positive (for number features)
    fn is_truthy(&self, value: &str) -> bool {
        match value.to_lowercase().as_str() {
            "true" | "yes" | "1" | "unlimited" => true,
            "false" | "no" | "0" => false,
            _ => {
                // Try parsing as number - positive means has access
                value.parse::<i64>().map(|n| n > 0).unwrap_or(false)
            }
        }
    }

    /// Get numeric limit for a feature (for things like max_users)
    pub async fn get_feature_limit(
        &self,
        tenant_id: &str,
        feature_code: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        let access = self.check_feature_access(tenant_id, feature_code).await?;

        if access.value.to_lowercase() == "unlimited" {
            return Ok(None); // None means unlimited
        }

        Ok(access.value.parse::<i64>().ok())
    }

    /// Get detailed subscription info for dashboard (Usage vs Limits)
    pub async fn get_tenant_subscription_details(
        &self,
        tenant_id: &str,
    ) -> AppResult<crate::models::TenantSubscriptionDetails> {
        // 1. Get Subscription & Plan
        let sub = self.get_tenant_subscription(tenant_id).await?;

        let (plan_name, plan_slug, status, period_end, feature_plan_id) = if let Some(s) = sub {
            // Subscription yatim (plan terhapus manual di DB) dulu membuat
            // SELURUH halaman subscription balas 500 lewat fetch_one.
            match self.get_plan_opt(&s.plan_id).await? {
                Some(plan) => (
                    plan.name.clone(),
                    plan.slug.clone(),
                    s.status,
                    s.current_period_end,
                    Some(plan.id.clone()),
                ),
                None => (
                    "Free".to_string(),
                    "free".to_string(),
                    s.status,
                    s.current_period_end,
                    None,
                ),
            }
        } else {
            (
                "Free".to_string(),
                "free".to_string(),
                "active".to_string(),
                None,
                None,
            )
        };

        // Entitlement nyata: pakai plan_id subscription; fallback ke slug
        // 'free' kalau yatim/tanpa subscription.
        let features = match feature_plan_id {
            Some(pid) => self.get_plan_features(&pid).await?,
            None => match self.find_plan_by_slug("free").await? {
                Some(free) => self.get_plan_features(&free.id).await?,
                None => Vec::new(),
            },
        };

        // 2. Get Limits
        let storage_limit_gb = self.get_feature_limit(tenant_id, "max_storage_gb").await?;
        let storage_limit = storage_limit_gb.map(|gb| gb * 1024 * 1024 * 1024); // Convert GB to Bytes

        let member_limit = self.get_feature_limit(tenant_id, "max_members").await?;

        // 3. Get Usage
        // Storage Usage
        #[cfg(feature = "postgres")]
        let storage_usage: i64 =
            sqlx::query_scalar("SELECT storage_usage FROM tenants WHERE id = $1")
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let storage_usage: i64 =
            sqlx::query_scalar("SELECT storage_usage FROM tenants WHERE id = ?")
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await?;

        // Member Usage
        #[cfg(feature = "postgres")]
        let member_usage: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tenant_members WHERE tenant_id = $1")
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let member_usage: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tenant_members WHERE tenant_id = ?")
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(crate::models::TenantSubscriptionDetails {
            plan_name,
            plan_slug,
            status,
            features,
            current_period_end: period_end,
            storage_usage,
            storage_limit,
            member_usage,
            member_limit,
        })
    }

    /// Get numeric limit for a feature (for things like max_users) within a transaction (PostgreSQL only)
    #[cfg(feature = "postgres")]
    pub async fn get_feature_limit_with_conn<'a>(
        &self,
        tenant_id: &str,
        feature_code: &str,
        tx: &mut sqlx::Transaction<'a, Postgres>,
    ) -> Result<Option<i64>, sqlx::Error> {
        let access = self
            .check_feature_access_with_conn(tenant_id, feature_code, tx)
            .await?;

        if access.value.to_lowercase() == "unlimited" {
            return Ok(None); // None means unlimited
        }

        Ok(access.value.parse::<i64>().ok())
    }

    /// Check if a tenant has access to a feature within a transaction (PostgreSQL only)
    #[cfg(feature = "postgres")]
    pub async fn check_feature_access_with_conn<'a>(
        &self,
        tenant_id: &str,
        feature_code: &str,
        tx: &mut sqlx::Transaction<'a, Postgres>,
    ) -> Result<FeatureAccess, sqlx::Error> {
        // Get tenant's subscription
        let subscription = self
            .get_tenant_subscription_with_conn(tenant_id, tx)
            .await?;

        // Get feature definition
        let feature: Option<FeatureDefinition> =
            sqlx::query_as("SELECT * FROM features WHERE code = $1")
                .bind(feature_code)
                .fetch_optional(&mut **tx)
                .await?;

        let feature = match feature {
            Some(f) => f,
            None => {
                return Ok(FeatureAccess {
                    code: feature_code.to_string(),
                    has_access: false,
                    value: "false".to_string(),
                    value_type: "boolean".to_string(),
                })
            }
        };

        // No subscription means use default value
        let subscription = match subscription {
            Some(s) => s,
            None => {
                return Ok(FeatureAccess {
                    code: feature_code.to_string(),
                    has_access: feature.default_value == "true"
                        || feature.default_value == "unlimited",
                    value: feature.default_value.clone(),
                    value_type: feature.value_type.clone(),
                })
            }
        };

        // Check for feature override in subscription
        if let Some(ref overrides_json) = subscription.feature_overrides {
            if let Ok(overrides) = serde_json::from_str::<serde_json::Value>(overrides_json) {
                if let Some(override_value) = overrides.get(feature_code) {
                    let value_str = override_value
                        .as_str()
                        .unwrap_or(&override_value.to_string())
                        .to_string();
                    return Ok(FeatureAccess {
                        code: feature_code.to_string(),
                        has_access: self.is_truthy(&value_str),
                        value: value_str,
                        value_type: feature.value_type,
                    });
                }
            }
        }

        // Get plan feature value
        let plan_feature: Option<PlanFeature> =
            sqlx::query_as("SELECT * FROM plan_features WHERE plan_id = $1 AND feature_id = $2")
                .bind(&subscription.plan_id)
                .bind(&feature.id)
                .fetch_optional(&mut **tx)
                .await?;

        let value = plan_feature
            .map(|pf| pf.value)
            .unwrap_or(feature.default_value.clone());

        Ok(FeatureAccess {
            code: feature_code.to_string(),
            has_access: self.is_truthy(&value),
            value,
            value_type: feature.value_type,
        })
    }
    /// Seed default system features if they don't exist
    pub async fn seed_default_features(&self) -> Result<(), sqlx::Error> {
        // Define standard SaaS features
        let default_features = vec![
            (
                "max_users",
                "Maximum Users",
                "Maximum number of users allowed",
                "number",
                "limits",
                "5",
            ),
            (
                "max_storage_gb",
                "Storage (GB)",
                "Maximum storage in Gigabytes",
                "number",
                "limits",
                "1",
            ),
            (
                "api_access",
                "API Access",
                "Access to developer API",
                "boolean",
                "capabilities",
                "false",
            ),
            (
                "custom_domain",
                "Custom Domain",
                "Ability to use custom domain",
                "boolean",
                "branding",
                "false",
            ),
            (
                "remove_branding",
                "Remove Branding",
                "Remove 'Powered by' branding",
                "boolean",
                "branding",
                "false",
            ),
            (
                "audit_logs",
                "Audit Logs",
                "Access to audit logs",
                "boolean",
                "security",
                "false",
            ),
            (
                "managed_radius",
                "Managed RADIUS",
                "Access to managed RADIUS onboarding and centralized PPP authentication",
                "boolean",
                "network",
                "false",
            ),
            (
                "sso_support",
                "SSO Support",
                "Single Sign-On (SAML/OIDC)",
                "boolean",
                "security",
                "false",
            ),
            (
                "support_level",
                "Support Level",
                "Level of support (Standard, Priority, 24/7)",
                "text",
                "support",
                "Standard",
            ),
        ];

        for (i, (code, name, description, value_type, category, default_value)) in
            default_features.into_iter().enumerate()
        {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM features WHERE code = $1)")
                    .bind(code)
                    .fetch_one(&self.pool)
                    .await?;

            if !exists {
                sqlx::query(
                    r#"
                    INSERT INTO features (id, code, name, description, value_type, category, default_value, sort_order, created_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
                    "#
                )
                .bind(Uuid::new_v4().to_string())
                .bind(code)
                .bind(name)
                .bind(description)
                .bind(value_type)
                .bind(category)
                .bind(default_value)
                .bind(i as i32)
                .execute(&self.pool)
                .await?;
            } else {
                // Feature exists, ensure definition matches code (e.g. value_type fix)
                sqlx::query(
                    r#"
                    UPDATE features 
                    SET name = $2, description = $3, value_type = $4, category = $5, default_value = $6, sort_order = $7
                    WHERE code = $1
                    "#
                )
                .bind(code)
                .bind(name)
                .bind(description)
                .bind(value_type)
                .bind(category)
                .bind(default_value)
                .bind(i as i32)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod plan_validation_tests {
    use super::{validate_plan_name, validate_plan_prices, validate_plan_slug};

    #[test]
    fn slug_valid_diterima() {
        assert!(validate_plan_slug("pro").is_ok());
        assert!(validate_plan_slug("team-20").is_ok());
        assert!(validate_plan_slug("biz_plan").is_ok());
    }

    #[test]
    fn slug_ditolak() {
        assert!(validate_plan_slug("a").is_err(), "terlalu pendek");
        assert!(validate_plan_slug("Pro").is_err(), "huruf besar");
        assert!(validate_plan_slug("pro!").is_err(), "karakter terlarang");
        assert!(validate_plan_slug("").is_err());
        assert!(validate_plan_slug(&"x".repeat(41)).is_err(), "terlalu panjang");
    }

    #[test]
    fn nama_ditolak_kosong_atau_kepanjangan() {
        assert!(validate_plan_name("Pro").is_ok());
        assert!(validate_plan_name("   ").is_err());
        assert!(validate_plan_name(&"n".repeat(81)).is_err());
    }

    #[test]
    fn harga_negatif_dan_non_finite_ditolak() {
        assert!(validate_plan_prices(Some(0.0), Some(1200.0)).is_ok());
        assert!(validate_plan_prices(None, None).is_ok());
        assert!(validate_plan_prices(Some(-1.0), None).is_err());
        assert!(validate_plan_prices(None, Some(f64::NAN)).is_err());
        assert!(validate_plan_prices(None, Some(f64::INFINITY)).is_err());
    }
}
