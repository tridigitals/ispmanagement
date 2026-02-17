use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    CreateIspPackageRequest, IspPackage, IspPackageRouterMapping, IspPackageRouterMappingView,
    PaginatedResponse, UpsertIspPackageRouterMappingRequest, UpdateIspPackageRequest,
};
use crate::services::{AuditService, AuthService};
use chrono::Utc;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone)]
pub struct IspPackageService {
    pool: DbPool,
    auth_service: AuthService,
    audit_service: AuditService,
}

impl IspPackageService {
    pub fn new(pool: DbPool, auth_service: AuthService, audit_service: AuditService) -> Self {
        Self {
            pool,
            auth_service,
            audit_service,
        }
    }

    async fn ensure_router_access(&self, tenant_id: &str, router_id: &str) -> AppResult<()> {
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2")
                .bind(router_id)
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?;

        if exists.is_none() {
            return Err(AppError::Forbidden("No access to router".into()));
        }
        Ok(())
    }

    async fn ensure_package_access(&self, tenant_id: &str, package_id: &str) -> AppResult<()> {
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM isp_packages WHERE id = $1 AND tenant_id = $2")
                .bind(package_id)
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?;

        if exists.is_none() {
            return Err(AppError::Validation("Package not found".into()));
        }
        Ok(())
    }

    fn normalize_features(features: Option<Vec<String>>) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for raw in features.unwrap_or_default() {
            let trimmed = raw.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            let key = trimmed.to_lowercase();
            if seen.insert(key) {
                out.push(trimmed);
            }
        }
        out
    }

    pub async fn list_packages(
        &self,
        actor_id: &str,
        tenant_id: &str,
        q: Option<String>,
        page: u32,
        per_page: u32,
    ) -> AppResult<PaginatedResponse<IspPackage>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "isp_packages", "read")
            .await?;

        let q = q.unwrap_or_default().trim().to_string();
        let offset = (page.saturating_sub(1)) * per_page;

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM isp_packages
            WHERE tenant_id = $1
              AND ($2 = '' OR name ILIKE '%' || $2 || '%')
            "#,
        )
        .bind(tenant_id)
        .bind(&q)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let rows: Vec<IspPackage> = sqlx::query_as(
            r#"
            SELECT
              id,
              tenant_id,
              name,
              description,
              features,
              is_active,
              price_monthly::float8 AS price_monthly,
              price_yearly::float8 AS price_yearly,
              created_at,
              updated_at
            FROM isp_packages
            WHERE tenant_id = $1
              AND ($2 = '' OR name ILIKE '%' || $2 || '%')
            ORDER BY updated_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(&q)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(PaginatedResponse {
            data: rows,
            total,
            page,
            per_page,
        })
    }

    pub async fn create_package(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateIspPackageRequest,
        ip_address: Option<&str>,
    ) -> AppResult<IspPackage> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "isp_packages", "manage")
            .await?;

        let name = dto.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Validation("name is required".into()));
        }
        let monthly = dto.price_monthly.unwrap_or(0.0);
        if monthly <= 0.0 {
            return Err(AppError::Validation(
                "price_monthly is required and must be greater than 0".into(),
            ));
        }
        let yearly = dto.price_yearly.unwrap_or(0.0);
        if yearly < 0.0 {
            return Err(AppError::Validation(
                "price_yearly must be greater than or equal to 0".into(),
            ));
        }

        let normalized_features = Self::normalize_features(dto.features);

        let pkg = IspPackage::new(
            tenant_id.to_string(),
            name,
            dto.description.and_then(|v| {
                let x = v.trim().to_string();
                if x.is_empty() {
                    None
                } else {
                    Some(x)
                }
            }),
            Some(normalized_features.clone()),
            dto.is_active,
            Some(monthly),
            Some(yearly),
        );

        sqlx::query(
            r#"
            INSERT INTO isp_packages (id, tenant_id, name, description, features, is_active, price_monthly, price_yearly, created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(&pkg.id)
        .bind(&pkg.tenant_id)
        .bind(&pkg.name)
        .bind(&pkg.description)
        .bind(&pkg.features)
        .bind(pkg.is_active)
        .bind(pkg.price_monthly)
        .bind(pkg.price_yearly)
        .bind(pkg.created_at)
        .bind(pkg.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.as_database_error()
                .and_then(|d| d.code().map(|c| c == "23505"))
                .unwrap_or(false)
            {
                AppError::Validation("Package name already exists".into())
            } else {
                AppError::Database(e)
            }
        })?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "ISP_PACKAGE_CREATE",
                "isp_packages",
                Some(&pkg.id),
                Some(&format!(
                    "Created ISP package {} (monthly={}, yearly={}, features={})",
                    pkg.name,
                    pkg.price_monthly,
                    pkg.price_yearly,
                    pkg.features.join(" | ")
                )),
                ip_address,
            )
            .await;

        Ok(pkg)
    }

    pub async fn update_package(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        dto: UpdateIspPackageRequest,
        ip_address: Option<&str>,
    ) -> AppResult<IspPackage> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "isp_packages", "manage")
            .await?;

        let mut pkg: IspPackage = sqlx::query_as(
            r#"
            SELECT
              id,
              tenant_id,
              name,
              description,
              features,
              is_active,
              price_monthly::float8 AS price_monthly,
              price_yearly::float8 AS price_yearly,
              created_at,
              updated_at
            FROM isp_packages
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Package not found".into()))?;

        let old_monthly = pkg.price_monthly;
        let old_yearly = pkg.price_yearly;
        let old_features = pkg.features.clone();
        let old_name = pkg.name.clone();
        let old_description = pkg.description.clone();
        let old_active = pkg.is_active;

        if let Some(v) = dto.name {
            let vv = v.trim().to_string();
            if !vv.is_empty() {
                pkg.name = vv;
            }
        }
        if let Some(v) = dto.description {
            let vv = v.trim().to_string();
            pkg.description = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(items) = dto.features {
            pkg.features = Self::normalize_features(Some(items));
        }
        if let Some(v) = dto.is_active {
            pkg.is_active = v;
        }
        if let Some(v) = dto.price_monthly {
            if v <= 0.0 {
                return Err(AppError::Validation(
                    "price_monthly must be greater than 0".into(),
                ));
            }
            pkg.price_monthly = v;
        }
        if let Some(v) = dto.price_yearly {
            if v < 0.0 {
                return Err(AppError::Validation(
                    "price_yearly must be greater than or equal to 0".into(),
                ));
            }
            pkg.price_yearly = v;
        }
        if pkg.price_monthly <= 0.0 {
            return Err(AppError::Validation(
                "price_monthly is required and must be greater than 0".into(),
            ));
        }

        pkg.updated_at = Utc::now();

        sqlx::query(
            r#"
            UPDATE isp_packages SET
              name = $1,
              description = $2,
              features = $3,
              is_active = $4,
              price_monthly = $5,
              price_yearly = $6,
              updated_at = $7
            WHERE tenant_id = $8 AND id = $9
            "#,
        )
        .bind(&pkg.name)
        .bind(&pkg.description)
        .bind(&pkg.features)
        .bind(pkg.is_active)
        .bind(pkg.price_monthly)
        .bind(pkg.price_yearly)
        .bind(pkg.updated_at)
        .bind(tenant_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.as_database_error()
                .and_then(|d| d.code().map(|c| c == "23505"))
                .unwrap_or(false)
            {
                AppError::Validation("Package name already exists".into())
            } else {
                AppError::Database(e)
            }
        })?;

        let audit_message = {
            let mut changes = Vec::new();
            if old_name != pkg.name {
                changes.push(format!("name: '{}' -> '{}'", old_name, pkg.name));
            }
            if old_description != pkg.description {
                changes.push(format!(
                    "description: '{}' -> '{}'",
                    old_description.as_deref().unwrap_or(""),
                    pkg.description.as_deref().unwrap_or("")
                ));
            }
            if (old_monthly - pkg.price_monthly).abs() > f64::EPSILON {
                changes.push(format!("monthly: {} -> {}", old_monthly, pkg.price_monthly));
            }
            if (old_yearly - pkg.price_yearly).abs() > f64::EPSILON {
                changes.push(format!("yearly: {} -> {}", old_yearly, pkg.price_yearly));
            }
            if old_active != pkg.is_active {
                changes.push(format!("active: {} -> {}", old_active, pkg.is_active));
            }
            if old_features != pkg.features {
                changes.push(format!(
                    "features: [{}] -> [{}]",
                    old_features.join(" | "),
                    pkg.features.join(" | ")
                ));
            }

            if changes.is_empty() {
                "Updated ISP package (no field changes)".to_string()
            } else {
                format!("Updated ISP package: {}", changes.join("; "))
            }
        };

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "ISP_PACKAGE_UPDATE",
                "isp_packages",
                Some(id),
                Some(&audit_message),
                ip_address,
            )
            .await;

        Ok(pkg)
    }

    pub async fn delete_package(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "isp_packages", "manage")
            .await?;

        let name: Option<String> =
            sqlx::query_scalar("SELECT name FROM isp_packages WHERE tenant_id = $1 AND id = $2")
                .bind(tenant_id)
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?;

        sqlx::query("DELETE FROM isp_packages WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "ISP_PACKAGE_DELETE",
                "isp_packages",
                Some(id),
                Some(&format!(
                    "Deleted ISP package {}",
                    name.unwrap_or_else(|| id.to_string())
                )),
                ip_address,
            )
            .await;

        Ok(())
    }

    pub async fn list_router_mappings(
        &self,
        actor_id: &str,
        tenant_id: &str,
        router_id: Option<String>,
    ) -> AppResult<Vec<IspPackageRouterMappingView>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "isp_packages", "read")
            .await?;

        if let Some(ref rid) = router_id {
            self.ensure_router_access(tenant_id, rid).await?;
        }

        let rows: Vec<IspPackageRouterMappingView> = sqlx::query_as(
            r#"
            SELECT
              m.id,
              m.tenant_id,
              m.router_id,
              m.package_id,
              p.name AS package_name,
              m.router_profile_name,
              m.address_pool,
              m.created_at,
              m.updated_at
            FROM isp_package_router_mappings m
            JOIN isp_packages p ON p.id = m.package_id
            WHERE m.tenant_id = $1
              AND ($2 = '' OR m.router_id = $2)
            ORDER BY p.name ASC
            "#,
        )
        .bind(tenant_id)
        .bind(router_id.unwrap_or_default())
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows)
    }

    pub async fn upsert_router_mapping(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: UpsertIspPackageRouterMappingRequest,
        ip_address: Option<&str>,
    ) -> AppResult<IspPackageRouterMapping> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "isp_packages", "manage")
            .await?;

        self.ensure_router_access(tenant_id, &dto.router_id).await?;
        self.ensure_package_access(tenant_id, &dto.package_id).await?;

        let profile = dto.router_profile_name.trim().to_string();
        if profile.is_empty() {
            return Err(AppError::Validation("router_profile_name is required".into()));
        }

        let addr_pool = dto.address_pool.and_then(|v| {
            let vv = v.trim().to_string();
            if vv.is_empty() {
                None
            } else {
                Some(vv)
            }
        });

        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO isp_package_router_mappings
              (id, tenant_id, router_id, package_id, router_profile_name, address_pool, created_at, updated_at)
            VALUES
              ($1,$2,$3,$4,$5,$6,$7,$8)
            ON CONFLICT (tenant_id, router_id, package_id) DO UPDATE SET
              router_profile_name = EXCLUDED.router_profile_name,
              address_pool = EXCLUDED.address_pool,
              updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(&dto.router_id)
        .bind(&dto.package_id)
        .bind(&profile)
        .bind(&addr_pool)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mapping: IspPackageRouterMapping = sqlx::query_as(
            r#"
            SELECT * FROM isp_package_router_mappings
            WHERE tenant_id = $1 AND router_id = $2 AND package_id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(&dto.router_id)
        .bind(&dto.package_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "ISP_PACKAGE_ROUTER_MAP_UPSERT",
                "isp_packages",
                Some(&mapping.id),
                Some("Upserted ISP package router mapping"),
                ip_address,
            )
            .await;

        Ok(mapping)
    }
}

