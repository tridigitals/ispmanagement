use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    CreateIspPackageRequest, IspPackage, IspPackageRouterMapping, IspPackageRouterMappingView,
    PaginatedResponse, UpdateIspPackageRequest, UpsertIspPackageRouterMappingRequest,
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

    fn normalize_service_type(input: Option<String>) -> AppResult<String> {
        let value = input
            .unwrap_or_else(|| "internet_pppoe".to_string())
            .trim()
            .to_lowercase();
        match value.as_str() {
            "internet_pppoe" | "hotspot" | "vpn" => Ok(value),
            _ => Err(AppError::Validation(
                "service_type must be one of: internet_pppoe, hotspot, vpn".into(),
            )),
        }
    }

    fn normalize_provisioning_type(service_type: &str, input: Option<String>) -> AppResult<String> {
        if service_type != "internet_pppoe" {
            return Ok("pppoe".to_string());
        }

        let value = input
            .unwrap_or_else(|| "pppoe".to_string())
            .trim()
            .to_lowercase();
        match value.as_str() {
            "pppoe" | "dhcp_static" => Ok(value),
            _ => Err(AppError::Validation(
                "provisioning_type must be one of: pppoe, dhcp_static".into(),
            )),
        }
    }

    fn normalize_router_mapping_fields(
        router_profile_name: &str,
        address_pool: Option<String>,
    ) -> AppResult<(String, Option<String>)> {
        let profile = router_profile_name.trim().to_string();
        if profile.is_empty() {
            return Err(AppError::Validation(
                "router_profile_name is required".into(),
            ));
        }

        let addr_pool = address_pool.and_then(|v| {
            let vv = v.trim().to_string();
            if vv.is_empty() {
                None
            } else {
                Some(vv)
            }
        });

        Ok((profile, addr_pool))
    }

    fn validate_router_mapping_references(
        router_profile_exists: bool,
        address_pool: Option<&str>,
        address_pool_exists: bool,
    ) -> AppResult<()> {
        if !router_profile_exists {
            return Err(AppError::Validation(
                "Selected PPP profile does not exist on this router. Sync PPP profiles and choose a valid profile.".into(),
            ));
        }

        if let Some(pool) = address_pool {
            if !address_pool_exists {
                return Err(AppError::Validation(format!(
                    "Selected IP pool '{}' does not exist on this router. Sync IP pools and choose a valid pool.",
                    pool
                )));
            }
        }

        Ok(())
    }

    pub async fn list_packages(
        &self,
        actor_id: &str,
        tenant_id: &str,
        q: Option<String>,
        page: u32,
        per_page: u32,
        sort_by: Option<String>,
        sort_dir: Option<String>,
    ) -> AppResult<PaginatedResponse<IspPackage>> {
        if self
            .auth_service
            .check_permission(actor_id, tenant_id, "isp_packages", "read")
            .await
            .is_err()
        {
            self.auth_service
                .check_permission(actor_id, tenant_id, "orders", "create")
                .await?;
        }

        let q = q.unwrap_or_default().trim().to_string();
        let offset = (page.saturating_sub(1)) * per_page;
        let sort_column = match sort_by
            .unwrap_or_else(|| "updated_at".to_string())
            .trim()
            .to_lowercase()
            .as_str()
        {
            "name" => "name",
            "type" => "service_type",
            "price" => "price_monthly",
            "status" => "is_active",
            "mappings" => {
                "(SELECT COUNT(*) FROM isp_package_router_mappings m WHERE m.package_id = isp_packages.id)"
            }
            "created_at" => "created_at",
            "updated_at" => "updated_at",
            _ => "updated_at",
        };
        let sort_direction = match sort_dir
            .unwrap_or_else(|| "desc".to_string())
            .trim()
            .to_lowercase()
            .as_str()
        {
            "asc" => "ASC",
            _ => "DESC",
        };

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

        let list_sql = format!(
            r#"
            SELECT
              id,
              tenant_id,
              service_type,
              provisioning_type,
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
            ORDER BY {sort_column} {sort_direction}
            LIMIT $3 OFFSET $4
            "#
        );

        let rows: Vec<IspPackage> = sqlx::query_as(&list_sql)
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
        let service_type = Self::normalize_service_type(dto.service_type)?;
        let provisioning_type =
            Self::normalize_provisioning_type(&service_type, dto.provisioning_type)?;

        let pkg = IspPackage::new(
            tenant_id.to_string(),
            Some(service_type),
            Some(provisioning_type),
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
            INSERT INTO isp_packages (id, tenant_id, service_type, provisioning_type, name, description, features, is_active, price_monthly, price_yearly, created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            "#,
        )
        .bind(&pkg.id)
        .bind(&pkg.tenant_id)
        .bind(&pkg.service_type)
        .bind(&pkg.provisioning_type)
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
                    "Created ISP package {} (type={}, provisioning={}, monthly={}, yearly={}, features={})",
                    pkg.name,
                    pkg.service_type,
                    pkg.provisioning_type,
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
              service_type,
              provisioning_type,
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
        let old_service_type = pkg.service_type.clone();
        let old_provisioning_type = pkg.provisioning_type.clone();

        if let Some(v) = dto.name {
            let vv = v.trim().to_string();
            if !vv.is_empty() {
                pkg.name = vv;
            }
        }
        if dto.service_type.is_some() {
            pkg.service_type = Self::normalize_service_type(dto.service_type)?;
        }
        if dto.provisioning_type.is_some() || old_service_type != pkg.service_type {
            pkg.provisioning_type =
                Self::normalize_provisioning_type(&pkg.service_type, dto.provisioning_type)?;
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
              service_type = $1,
              provisioning_type = $2,
              name = $3,
              description = $4,
              features = $5,
              is_active = $6,
              price_monthly = $7,
              price_yearly = $8,
              updated_at = $9
            WHERE tenant_id = $10 AND id = $11
            "#,
        )
        .bind(&pkg.service_type)
        .bind(&pkg.provisioning_type)
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
            if old_service_type != pkg.service_type {
                changes.push(format!(
                    "service_type: '{}' -> '{}'",
                    old_service_type, pkg.service_type
                ));
            }
            if old_provisioning_type != pkg.provisioning_type {
                changes.push(format!(
                    "provisioning_type: '{}' -> '{}'",
                    old_provisioning_type, pkg.provisioning_type
                ));
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
        if self
            .auth_service
            .check_permission(actor_id, tenant_id, "isp_packages", "read")
            .await
            .is_err()
        {
            self.auth_service
                .check_permission(actor_id, tenant_id, "work_orders", "manage")
                .await?;
        }

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
              r.name AS router_name,
              m.router_profile_name,
              m.address_pool,
              m.created_at,
              m.updated_at
            FROM isp_package_router_mappings m
            JOIN isp_packages p ON p.id = m.package_id
            LEFT JOIN mikrotik_routers r ON r.tenant_id = m.tenant_id AND r.id = m.router_id
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
        self.ensure_package_access(tenant_id, &dto.package_id)
            .await?;

        let package_provisioning_type: Option<String> = sqlx::query_scalar(
            "SELECT provisioning_type FROM isp_packages WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(&dto.package_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;
        if package_provisioning_type.as_deref() != Some("pppoe") {
            return Err(AppError::Validation(
                "Router mapping is only available for PPPoE provisioning".into(),
            ));
        }

        let (profile, addr_pool) =
            Self::normalize_router_mapping_fields(&dto.router_profile_name, dto.address_pool)?;

        let router_profile_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM mikrotik_ppp_profiles
              WHERE tenant_id = $1
                AND router_id = $2
                AND name = $3
                AND router_present = TRUE
            )
            "#,
        )
        .bind(tenant_id)
        .bind(&dto.router_id)
        .bind(&profile)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let address_pool_exists: bool = if let Some(pool_name) = addr_pool.as_deref() {
            sqlx::query_scalar(
                r#"
                SELECT EXISTS(
                  SELECT 1
                  FROM mikrotik_ip_pools
                  WHERE tenant_id = $1
                    AND router_id = $2
                    AND name = $3
                    AND router_present = TRUE
                )
                "#,
            )
            .bind(tenant_id)
            .bind(&dto.router_id)
            .bind(pool_name)
            .fetch_one(&self.pool)
            .await
            .map_err(AppError::Database)?
        } else {
            true
        };

        Self::validate_router_mapping_references(
            router_profile_exists,
            addr_pool.as_deref(),
            address_pool_exists,
        )?;

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

#[cfg(test)]
mod tests {
    use super::IspPackageService;
    use crate::error::AppError;

    #[test]
    fn normalize_router_mapping_fields_requires_profile_name() {
        let err =
            IspPackageService::normalize_router_mapping_fields("   ", Some(" pool-a ".into()))
                .expect_err("blank profile should be rejected");

        match err {
            AppError::Validation(message) => {
                assert_eq!(message, "router_profile_name is required");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn normalize_router_mapping_fields_trims_values() {
        let (profile, pool) = IspPackageService::normalize_router_mapping_fields(
            " basic-10m ",
            Some(" pool-basic ".into()),
        )
        .expect("trimmed values should be accepted");

        assert_eq!(profile, "basic-10m");
        assert_eq!(pool.as_deref(), Some("pool-basic"));
    }

    #[test]
    fn validate_router_mapping_references_rejects_missing_profile() {
        let err = IspPackageService::validate_router_mapping_references(false, None, true)
            .expect_err("missing profile should be rejected");

        match err {
            AppError::Validation(message) => {
                assert_eq!(
                    message,
                    "Selected PPP profile does not exist on this router. Sync PPP profiles and choose a valid profile."
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn validate_router_mapping_references_rejects_missing_pool() {
        let err = IspPackageService::validate_router_mapping_references(
            true,
            Some("pool-missing"),
            false,
        )
        .expect_err("missing pool should be rejected");

        match err {
            AppError::Validation(message) => {
                assert_eq!(
                    message,
                    "Selected IP pool 'pool-missing' does not exist on this router. Sync IP pools and choose a valid pool."
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn validate_router_mapping_references_accepts_existing_profile_and_optional_pool() {
        IspPackageService::validate_router_mapping_references(true, None, true)
            .expect("valid references should pass");
        IspPackageService::validate_router_mapping_references(true, Some("pool-a"), true)
            .expect("valid references with pool should pass");
    }

    #[test]
    fn normalize_service_type_defaults_to_internet_pppoe() {
        let value = IspPackageService::normalize_service_type(None)
            .expect("default service type should work");

        assert_eq!(value, "internet_pppoe");
    }

    #[test]
    fn normalize_service_type_rejects_unknown_values() {
        let err = IspPackageService::normalize_service_type(Some("dhcp_static".into()))
            .expect_err("unknown service type should fail");

        match err {
            AppError::Validation(message) => {
                assert_eq!(
                    message,
                    "service_type must be one of: internet_pppoe, hotspot, vpn"
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn normalize_provisioning_type_defaults_to_pppoe_for_internet_packages() {
        let value = IspPackageService::normalize_provisioning_type("internet_pppoe", None)
            .expect("default provisioning type should work");

        assert_eq!(value, "pppoe");
    }

    #[test]
    fn normalize_provisioning_type_accepts_dhcp_static_for_internet_packages() {
        let value = IspPackageService::normalize_provisioning_type(
            "internet_pppoe",
            Some("dhcp_static".into()),
        )
        .expect("dhcp static provisioning should be valid for internet packages");

        assert_eq!(value, "dhcp_static");
    }

    #[test]
    fn normalize_provisioning_type_forces_pppoe_for_non_internet_packages() {
        let value =
            IspPackageService::normalize_provisioning_type("hotspot", Some("dhcp_static".into()))
                .expect("non-internet packages should normalize to pppoe");

        assert_eq!(value, "pppoe");
    }
}
