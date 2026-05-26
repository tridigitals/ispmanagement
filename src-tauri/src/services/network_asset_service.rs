use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    CreateNetworkAssetRequest, ListNetworkAssetsParams, NetworkAsset, NetworkAssetListItem,
    PaginatedResponse, UpdateNetworkAssetRequest,
};
use crate::services::network_asset_port_cache::refresh_port_usage_cache_for_tenant;
use crate::services::{AuditService, AuthService};
use std::net::IpAddr;

#[derive(Clone)]
pub struct NetworkAssetService {
    pool: DbPool,
    auth_service: AuthService,
    audit_service: AuditService,
}

impl NetworkAssetService {
    pub fn new(pool: DbPool, auth_service: AuthService, audit_service: AuditService) -> Self {
        Self {
            pool,
            auth_service,
            audit_service,
        }
    }

    async fn require_read(&self, actor_id: &str, tenant_id: &str) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "ftth_assets", "read")
            .await
    }

    async fn require_manage(&self, actor_id: &str, tenant_id: &str) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "ftth_assets", "manage")
            .await
    }

    fn validate_asset_type(asset_type: &str) -> AppResult<()> {
        if matches!(
            asset_type,
            "olt"
                | "odc"
                | "odp"
                | "splitter"
                | "ont"
                | "onu"
                | "fat"
                | "nap"
                | "switch"
                | "router"
                | "media_converter"
                | "odf"
                | "ups"
        ) {
            Ok(())
        } else {
            Err(AppError::Validation("Invalid asset type".into()))
        }
    }

    fn validate_status(status: &str) -> AppResult<()> {
        if matches!(
            status,
            "available" | "reserved" | "installed" | "faulty" | "retired"
        ) {
            Ok(())
        } else {
            Err(AppError::Validation("Invalid asset status".into()))
        }
    }

    fn clean_text(value: Option<String>) -> Option<String> {
        value.and_then(|raw| {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    }

    fn clean_required_text(value: String, field: &str) -> AppResult<String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(AppError::Validation(format!("{field} is required")));
        }
        Ok(trimmed.to_string())
    }

    fn validate_coordinates(latitude: Option<f64>, longitude: Option<f64>) -> AppResult<()> {
        match (latitude, longitude) {
            (None, None) => Ok(()),
            (Some(_), None) | (None, Some(_)) => Err(AppError::Validation(
                "Latitude and longitude must be filled together".into(),
            )),
            (Some(lat), Some(lng)) => {
                if !(-90.0..=90.0).contains(&lat) {
                    return Err(AppError::Validation(
                        "Latitude must be between -90 and 90".into(),
                    ));
                }
                if !(-180.0..=180.0).contains(&lng) {
                    return Err(AppError::Validation(
                        "Longitude must be between -180 and 180".into(),
                    ));
                }
                Ok(())
            }
        }
    }

    fn validate_detail_metadata(asset_type: &str, metadata: &serde_json::Value) -> AppResult<()> {
        let Some(map) = metadata.as_object() else {
            return Ok(());
        };

        let validate_positive_integer = |key: &str, label: &str| -> AppResult<()> {
            let Some(value) = map.get(key) else {
                return Ok(());
            };
            let normalized = match value {
                serde_json::Value::String(text) => text.trim().to_string(),
                serde_json::Value::Number(number) => number.to_string(),
                _ => {
                    return Err(AppError::Validation(format!(
                        "{label} must be a positive whole number"
                    )))
                }
            };

            if normalized
                .parse::<u32>()
                .ok()
                .filter(|value| *value > 0)
                .is_some()
            {
                Ok(())
            } else {
                Err(AppError::Validation(format!(
                    "{label} must be a positive whole number"
                )))
            }
        };

        match asset_type {
            "switch" | "router" | "media_converter" => {
                if let Some(value) = map.get("management_ip").and_then(|value| value.as_str()) {
                    if value.trim().parse::<IpAddr>().is_err() {
                        return Err(AppError::Validation(
                            "Management IP must be a valid IP address".into(),
                        ));
                    }
                }
            }
            "ont" | "onu" => {
                if let Some(value) = map.get("mac_address").and_then(|value| value.as_str()) {
                    let octets: Vec<&str> = value.trim().split([':', '-']).collect();
                    let valid = octets.len() == 6
                        && octets.iter().all(|part| {
                            part.len() == 2 && part.chars().all(|ch| ch.is_ascii_hexdigit())
                        });
                    if !valid {
                        return Err(AppError::Validation(
                            "MAC Address must use format like AA:BB:CC:DD:EE:FF".into(),
                        ));
                    }
                }
            }
            _ => {}
        }

        validate_positive_integer("fiber_core_count", "Fiber Core Count")?;
        validate_positive_integer("output_ports", "Output Ports")?;
        validate_positive_integer("battery_capacity_ah", "Battery Capacity (Ah)")?;
        validate_positive_integer("backup_runtime_minutes", "Backup Runtime (Minutes)")?;

        Ok(())
    }

    async fn ensure_relation_exists(
        &self,
        tenant_id: &str,
        table: &str,
        id: Option<&str>,
        label: &str,
    ) -> AppResult<()> {
        let Some(id) = id else { return Ok(()) };

        #[cfg(feature = "postgres")]
        let query = format!("SELECT id FROM {table} WHERE tenant_id = $1 AND id = $2 LIMIT 1");
        #[cfg(feature = "sqlite")]
        let query = format!("SELECT id FROM {table} WHERE tenant_id = ?1 AND id = ?2 LIMIT 1");

        let found: Option<String> = sqlx::query_scalar(&query)
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::Database)?;

        if found.is_none() {
            return Err(AppError::NotFound(format!("{label} not found")));
        }

        Ok(())
    }

    async fn ensure_unique_identifiers(
        &self,
        tenant_id: &str,
        code: Option<&str>,
        serial_number: Option<&str>,
        exclude_id: Option<&str>,
    ) -> AppResult<()> {
        #[cfg(feature = "postgres")]
        let existing: Option<String> = sqlx::query_scalar(
            r#"
            SELECT id::text
            FROM network_assets
            WHERE tenant_id = $1
              AND ($2::text IS NULL OR id <> $2)
              AND (
                ($3::text IS NOT NULL AND lower(code) = lower($3))
                OR ($4::text IS NOT NULL AND lower(serial_number) = lower($4))
              )
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(exclude_id)
        .bind(code)
        .bind(serial_number)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        #[cfg(feature = "sqlite")]
        let existing: Option<String> = sqlx::query_scalar(
            r#"
            SELECT id
            FROM network_assets
            WHERE tenant_id = ?1
              AND (?2 IS NULL OR id <> ?2)
              AND (
                (?3 IS NOT NULL AND lower(code) = lower(?3))
                OR (?4 IS NOT NULL AND lower(serial_number) = lower(?4))
              )
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(exclude_id)
        .bind(code)
        .bind(serial_number)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if existing.is_some() {
            return Err(AppError::Conflict(
                "Asset code or serial number already exists".into(),
            ));
        }

        Ok(())
    }

    async fn load_asset(&self, tenant_id: &str, id: &str) -> AppResult<NetworkAsset> {
        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT
              id,
              tenant_id,
              asset_group,
              asset_type,
              name,
              code,
              vendor,
              model,
              serial_number,
              status,
              customer_id,
              location_id,
              work_order_id,
              parent_asset_id,
              latitude,
              longitude,
              notes,
              metadata,
              created_at,
              updated_at
            FROM network_assets
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
        "#;
        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT
              id,
              tenant_id,
              asset_group,
              asset_type,
              name,
              code,
              vendor,
              model,
              serial_number,
              status,
              customer_id,
              location_id,
              work_order_id,
              parent_asset_id,
              latitude,
              longitude,
              notes,
              metadata,
              created_at,
              updated_at
            FROM network_assets
            WHERE tenant_id = ?1 AND id = ?2
            LIMIT 1
        "#;

        sqlx::query_as::<_, NetworkAsset>(query)
            .bind(tenant_id)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("Network asset not found".into()))
    }

    pub async fn list_assets(
        &self,
        actor_id: &str,
        tenant_id: &str,
        params: ListNetworkAssetsParams,
    ) -> AppResult<PaginatedResponse<NetworkAssetListItem>> {
        self.require_read(actor_id, tenant_id).await?;
        refresh_port_usage_cache_for_tenant(&self.pool, tenant_id).await?;

        let q = params.q.unwrap_or_default().trim().to_string();
        let page = params.page.unwrap_or(1).max(1);
        let per_page = params.per_page.unwrap_or(25).clamp(1, 200);
        let offset = (page - 1) * per_page;

        #[cfg(feature = "postgres")]
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM network_assets a
            WHERE a.tenant_id = $1
              AND ($2 = '' OR a.name ILIKE '%' || $2 || '%' OR COALESCE(a.code, '') ILIKE '%' || $2 || '%' OR COALESCE(a.serial_number, '') ILIKE '%' || $2 || '%')
              AND ($3::text IS NULL OR a.asset_type = $3)
              AND ($4::text IS NULL OR a.status = $4)
              AND ($5::text IS NULL OR a.customer_id = $5)
              AND ($6::text IS NULL OR a.location_id = $6)
              AND ($7::text IS NULL OR a.parent_asset_id = $7)
            "#,
        )
        .bind(tenant_id)
        .bind(&q)
        .bind(&params.asset_type)
        .bind(&params.status)
        .bind(&params.customer_id)
        .bind(&params.location_id)
        .bind(&params.parent_asset_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        #[cfg(feature = "sqlite")]
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM network_assets a
            WHERE a.tenant_id = ?1
              AND (?2 = '' OR a.name LIKE '%' || ?2 || '%' OR COALESCE(a.code, '') LIKE '%' || ?2 || '%' OR COALESCE(a.serial_number, '') LIKE '%' || ?2 || '%')
              AND (?3 IS NULL OR a.asset_type = ?3)
              AND (?4 IS NULL OR a.status = ?4)
              AND (?5 IS NULL OR a.customer_id = ?5)
              AND (?6 IS NULL OR a.location_id = ?6)
              AND (?7 IS NULL OR a.parent_asset_id = ?7)
            "#,
        )
        .bind(tenant_id)
        .bind(&q)
        .bind(&params.asset_type)
        .bind(&params.status)
        .bind(&params.customer_id)
        .bind(&params.location_id)
        .bind(&params.parent_asset_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        #[cfg(feature = "postgres")]
        let data = sqlx::query_as::<_, NetworkAssetListItem>(
            r#"
            SELECT
              a.id,
              a.tenant_id,
              a.asset_group,
              a.asset_type,
              a.name,
              a.code,
              a.vendor,
              a.model,
              a.serial_number,
              a.status,
              a.customer_id,
              a.location_id,
              a.work_order_id,
              a.parent_asset_id,
              a.latitude,
              a.longitude,
              a.notes,
              a.metadata,
              a.created_at,
              a.updated_at,
              c.name AS customer_name,
              cl.label AS location_label,
              wo.status AS work_order_status,
              parent.name AS parent_asset_name
            FROM network_assets a
            LEFT JOIN customers c ON c.id = a.customer_id
            LEFT JOIN customer_locations cl ON cl.id = a.location_id
            LEFT JOIN installation_work_orders wo ON wo.id = a.work_order_id
            LEFT JOIN network_assets parent ON parent.id = a.parent_asset_id
            WHERE a.tenant_id = $1
              AND ($2 = '' OR a.name ILIKE '%' || $2 || '%' OR COALESCE(a.code, '') ILIKE '%' || $2 || '%' OR COALESCE(a.serial_number, '') ILIKE '%' || $2 || '%')
              AND ($3::text IS NULL OR a.asset_type = $3)
              AND ($4::text IS NULL OR a.status = $4)
              AND ($5::text IS NULL OR a.customer_id = $5)
              AND ($6::text IS NULL OR a.location_id = $6)
              AND ($7::text IS NULL OR a.parent_asset_id = $7)
            ORDER BY a.updated_at DESC
            LIMIT $8 OFFSET $9
            "#,
        )
        .bind(tenant_id)
        .bind(&q)
        .bind(&params.asset_type)
        .bind(&params.status)
        .bind(&params.customer_id)
        .bind(&params.location_id)
        .bind(&params.parent_asset_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        #[cfg(feature = "sqlite")]
        let data = sqlx::query_as::<_, NetworkAssetListItem>(
            r#"
            SELECT
              a.id,
              a.tenant_id,
              a.asset_group,
              a.asset_type,
              a.name,
              a.code,
              a.vendor,
              a.model,
              a.serial_number,
              a.status,
              a.customer_id,
              a.location_id,
              a.work_order_id,
              a.parent_asset_id,
              a.latitude,
              a.longitude,
              a.notes,
              a.metadata,
              a.created_at,
              a.updated_at,
              c.name AS customer_name,
              cl.label AS location_label,
              wo.status AS work_order_status,
              parent.name AS parent_asset_name
            FROM network_assets a
            LEFT JOIN customers c ON c.id = a.customer_id
            LEFT JOIN customer_locations cl ON cl.id = a.location_id
            LEFT JOIN installation_work_orders wo ON wo.id = a.work_order_id
            LEFT JOIN network_assets parent ON parent.id = a.parent_asset_id
            WHERE a.tenant_id = ?1
              AND (?2 = '' OR a.name LIKE '%' || ?2 || '%' OR COALESCE(a.code, '') LIKE '%' || ?2 || '%' OR COALESCE(a.serial_number, '') LIKE '%' || ?2 || '%')
              AND (?3 IS NULL OR a.asset_type = ?3)
              AND (?4 IS NULL OR a.status = ?4)
              AND (?5 IS NULL OR a.customer_id = ?5)
              AND (?6 IS NULL OR a.location_id = ?6)
              AND (?7 IS NULL OR a.parent_asset_id = ?7)
            ORDER BY a.updated_at DESC
            LIMIT ?8 OFFSET ?9
            "#,
        )
        .bind(tenant_id)
        .bind(&q)
        .bind(&params.asset_type)
        .bind(&params.status)
        .bind(&params.customer_id)
        .bind(&params.location_id)
        .bind(&params.parent_asset_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(PaginatedResponse {
            data,
            total,
            page,
            per_page,
        })
    }

    pub async fn get_asset(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<NetworkAsset> {
        self.require_read(actor_id, tenant_id).await?;
        self.load_asset(tenant_id, id).await
    }

    pub async fn create_asset(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateNetworkAssetRequest,
    ) -> AppResult<NetworkAsset> {
        self.require_manage(actor_id, tenant_id).await?;

        let asset_type = Self::clean_required_text(dto.asset_type, "asset_type")?;
        Self::validate_asset_type(&asset_type)?;
        let name = Self::clean_required_text(dto.name, "name")?;
        let status = Self::clean_text(dto.status).unwrap_or_else(|| "available".to_string());
        Self::validate_status(&status)?;
        let code = Self::clean_text(dto.code);
        let vendor = Self::clean_text(dto.vendor);
        let model = Self::clean_text(dto.model);
        let serial_number = Self::clean_text(dto.serial_number);
        let notes = Self::clean_text(dto.notes);
        let customer_id = Self::clean_text(dto.customer_id);
        let location_id = Self::clean_text(dto.location_id);
        let work_order_id = Self::clean_text(dto.work_order_id);
        let parent_asset_id = Self::clean_text(dto.parent_asset_id);
        let latitude = dto.latitude;
        let longitude = dto.longitude;
        let metadata = dto.metadata.unwrap_or_else(|| serde_json::json!({}));

        self.ensure_unique_identifiers(tenant_id, code.as_deref(), serial_number.as_deref(), None)
            .await?;
        self.ensure_relation_exists(tenant_id, "customers", customer_id.as_deref(), "Customer")
            .await?;
        self.ensure_relation_exists(
            tenant_id,
            "customer_locations",
            location_id.as_deref(),
            "Customer location",
        )
        .await?;
        self.ensure_relation_exists(
            tenant_id,
            "installation_work_orders",
            work_order_id.as_deref(),
            "Installation work order",
        )
        .await?;
        self.ensure_relation_exists(
            tenant_id,
            "network_assets",
            parent_asset_id.as_deref(),
            "Parent asset",
        )
        .await?;
        Self::validate_coordinates(latitude, longitude)?;
        Self::validate_detail_metadata(&asset_type, &metadata)?;

        let asset = NetworkAsset::new(
            tenant_id.to_string(),
            asset_type,
            name,
            code,
            vendor,
            model,
            serial_number,
            Some(status),
            customer_id,
            location_id,
            work_order_id,
            parent_asset_id,
            latitude,
            longitude,
            notes,
            Some(metadata),
        );

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO network_assets (
              id, tenant_id, asset_group, asset_type, name, code, vendor, model, serial_number, status,
              customer_id, location_id, work_order_id, parent_asset_id, latitude, longitude, notes, metadata, created_at, updated_at
            )
            VALUES (
              $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
              $11, $12, $13, $14, $15, $16, $17, $18, $19, $20
            )
            "#,
        )
        .bind(&asset.id)
        .bind(&asset.tenant_id)
        .bind(&asset.asset_group)
        .bind(&asset.asset_type)
        .bind(&asset.name)
        .bind(&asset.code)
        .bind(&asset.vendor)
        .bind(&asset.model)
        .bind(&asset.serial_number)
        .bind(&asset.status)
        .bind(&asset.customer_id)
        .bind(&asset.location_id)
        .bind(&asset.work_order_id)
        .bind(&asset.parent_asset_id)
        .bind(&asset.latitude)
        .bind(&asset.longitude)
        .bind(&asset.notes)
        .bind(&asset.metadata)
        .bind(asset.created_at)
        .bind(asset.updated_at)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO network_assets (
              id, tenant_id, asset_group, asset_type, name, code, vendor, model, serial_number, status,
              customer_id, location_id, work_order_id, parent_asset_id, latitude, longitude, notes, metadata, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&asset.id)
        .bind(&asset.tenant_id)
        .bind(&asset.asset_group)
        .bind(&asset.asset_type)
        .bind(&asset.name)
        .bind(&asset.code)
        .bind(&asset.vendor)
        .bind(&asset.model)
        .bind(&asset.serial_number)
        .bind(&asset.status)
        .bind(&asset.customer_id)
        .bind(&asset.location_id)
        .bind(&asset.work_order_id)
        .bind(&asset.parent_asset_id)
        .bind(&asset.latitude)
        .bind(&asset.longitude)
        .bind(&asset.notes)
        .bind(&asset.metadata)
        .bind(asset.created_at.to_rfc3339())
        .bind(asset.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "create",
                "ftth_assets",
                Some(&asset.id),
                Some(&format!(
                    "Created {} asset {}",
                    asset.asset_type, asset.name
                )),
                Some("127.0.0.1"),
            )
            .await;

        refresh_port_usage_cache_for_tenant(&self.pool, tenant_id).await?;
        self.load_asset(tenant_id, &asset.id).await
    }

    pub async fn update_asset(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        dto: UpdateNetworkAssetRequest,
    ) -> AppResult<NetworkAsset> {
        self.require_manage(actor_id, tenant_id).await?;

        let current = self.load_asset(tenant_id, id).await?;
        let asset_type =
            Self::clean_text(dto.asset_type).unwrap_or_else(|| current.asset_type.clone());
        Self::validate_asset_type(&asset_type)?;
        let asset_group = NetworkAsset::asset_group_for_type(&asset_type).to_string();
        let name = match dto.name {
            Some(name) => Self::clean_required_text(name, "name")?,
            None => current.name.clone(),
        };
        let status = Self::clean_text(dto.status).unwrap_or_else(|| current.status.clone());
        Self::validate_status(&status)?;
        let code = if dto.code.is_some() {
            Self::clean_text(dto.code)
        } else {
            current.code.clone()
        };
        let vendor = if dto.vendor.is_some() {
            Self::clean_text(dto.vendor)
        } else {
            current.vendor.clone()
        };
        let model = if dto.model.is_some() {
            Self::clean_text(dto.model)
        } else {
            current.model.clone()
        };
        let serial_number = if dto.serial_number.is_some() {
            Self::clean_text(dto.serial_number)
        } else {
            current.serial_number.clone()
        };
        let customer_id = if dto.customer_id.is_some() {
            Self::clean_text(dto.customer_id)
        } else {
            current.customer_id.clone()
        };
        let location_id = if dto.location_id.is_some() {
            Self::clean_text(dto.location_id)
        } else {
            current.location_id.clone()
        };
        let work_order_id = if dto.work_order_id.is_some() {
            Self::clean_text(dto.work_order_id)
        } else {
            current.work_order_id.clone()
        };
        let parent_asset_id = if dto.parent_asset_id.is_some() {
            Self::clean_text(dto.parent_asset_id)
        } else {
            current.parent_asset_id.clone()
        };
        let latitude = if dto.latitude.is_some() {
            dto.latitude
        } else {
            current.latitude
        };
        let longitude = if dto.longitude.is_some() {
            dto.longitude
        } else {
            current.longitude
        };
        if parent_asset_id.as_deref() == Some(id) {
            return Err(AppError::Validation("Asset cannot parent itself".into()));
        }
        let notes = if dto.notes.is_some() {
            Self::clean_text(dto.notes)
        } else {
            current.notes.clone()
        };
        let metadata = dto.metadata.unwrap_or_else(|| current.metadata.clone());

        self.ensure_unique_identifiers(
            tenant_id,
            code.as_deref(),
            serial_number.as_deref(),
            Some(id),
        )
        .await?;
        self.ensure_relation_exists(tenant_id, "customers", customer_id.as_deref(), "Customer")
            .await?;
        self.ensure_relation_exists(
            tenant_id,
            "customer_locations",
            location_id.as_deref(),
            "Customer location",
        )
        .await?;
        self.ensure_relation_exists(
            tenant_id,
            "installation_work_orders",
            work_order_id.as_deref(),
            "Installation work order",
        )
        .await?;
        self.ensure_relation_exists(
            tenant_id,
            "network_assets",
            parent_asset_id.as_deref(),
            "Parent asset",
        )
        .await?;
        Self::validate_coordinates(latitude, longitude)?;
        Self::validate_detail_metadata(&asset_type, &metadata)?;

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE network_assets
            SET asset_group = $3,
                asset_type = $4,
                name = $5,
                code = $6,
                vendor = $7,
                model = $8,
                serial_number = $9,
                status = $10,
                customer_id = $11,
                location_id = $12,
                work_order_id = $13,
                parent_asset_id = $14,
                latitude = $15,
                longitude = $16,
                notes = $17,
                metadata = $18
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .bind(&asset_group)
        .bind(&asset_type)
        .bind(&name)
        .bind(&code)
        .bind(&vendor)
        .bind(&model)
        .bind(&serial_number)
        .bind(&status)
        .bind(&customer_id)
        .bind(&location_id)
        .bind(&work_order_id)
        .bind(&parent_asset_id)
        .bind(&latitude)
        .bind(&longitude)
        .bind(&notes)
        .bind(&metadata)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE network_assets
            SET asset_group = ?3,
                asset_type = ?4,
                name = ?5,
                code = ?6,
                vendor = ?7,
                model = ?8,
                serial_number = ?9,
                status = ?10,
                customer_id = ?11,
                location_id = ?12,
                work_order_id = ?13,
                parent_asset_id = ?14,
                latitude = ?15,
                longitude = ?16,
                notes = ?17,
                metadata = ?18
            WHERE tenant_id = ?1 AND id = ?2
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .bind(&asset_group)
        .bind(&asset_type)
        .bind(&name)
        .bind(&code)
        .bind(&vendor)
        .bind(&model)
        .bind(&serial_number)
        .bind(&status)
        .bind(&customer_id)
        .bind(&location_id)
        .bind(&work_order_id)
        .bind(&parent_asset_id)
        .bind(&latitude)
        .bind(&longitude)
        .bind(&notes)
        .bind(&metadata)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "update",
                "ftth_assets",
                Some(id),
                Some(&format!("Updated {} asset {}", asset_type, name)),
                Some("127.0.0.1"),
            )
            .await;

        refresh_port_usage_cache_for_tenant(&self.pool, tenant_id).await?;
        self.load_asset(tenant_id, id).await
    }

    pub async fn delete_asset(&self, actor_id: &str, tenant_id: &str, id: &str) -> AppResult<()> {
        self.require_manage(actor_id, tenant_id).await?;
        let current = self.load_asset(tenant_id, id).await?;

        #[cfg(feature = "postgres")]
        let linked_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM network_assets
            WHERE tenant_id = $1 AND parent_asset_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        #[cfg(feature = "sqlite")]
        let linked_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM network_assets
            WHERE tenant_id = ?1 AND parent_asset_id = ?2
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if linked_count > 0 {
            return Err(AppError::Conflict(
                "Cannot delete asset that still has child assets".into(),
            ));
        }

        #[cfg(feature = "postgres")]
        sqlx::query("DELETE FROM network_assets WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        #[cfg(feature = "sqlite")]
        sqlx::query("DELETE FROM network_assets WHERE tenant_id = ?1 AND id = ?2")
            .bind(tenant_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "delete",
                "ftth_assets",
                Some(id),
                Some(&format!(
                    "Deleted {} asset {}",
                    current.asset_type, current.name
                )),
                Some("127.0.0.1"),
            )
            .await;

        refresh_port_usage_cache_for_tenant(&self.pool, tenant_id).await?;
        Ok(())
    }

    async fn update_relation(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        field: &str,
        value: Option<&str>,
        table: Option<&str>,
        label: &str,
    ) -> AppResult<NetworkAsset> {
        self.require_manage(actor_id, tenant_id).await?;
        self.load_asset(tenant_id, id).await?;
        if field == "parent_asset_id" && value == Some(id) {
            return Err(AppError::Validation("Asset cannot parent itself".into()));
        }
        if let Some(table) = table {
            self.ensure_relation_exists(tenant_id, table, value, label)
                .await?;
        }

        #[cfg(feature = "postgres")]
        let query =
            format!("UPDATE network_assets SET {field} = $3 WHERE tenant_id = $1 AND id = $2");
        #[cfg(feature = "sqlite")]
        let query =
            format!("UPDATE network_assets SET {field} = ?3 WHERE tenant_id = ?1 AND id = ?2");

        sqlx::query(&query)
            .bind(tenant_id)
            .bind(id)
            .bind(value)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "update",
                "ftth_assets",
                Some(id),
                Some(&format!("Updated asset {field} relation")),
                Some("127.0.0.1"),
            )
            .await;

        refresh_port_usage_cache_for_tenant(&self.pool, tenant_id).await?;
        self.load_asset(tenant_id, id).await
    }

    pub async fn assign_customer(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        customer_id: Option<&str>,
    ) -> AppResult<NetworkAsset> {
        self.update_relation(
            actor_id,
            tenant_id,
            id,
            "customer_id",
            customer_id,
            Some("customers"),
            "Customer",
        )
        .await
    }

    pub async fn assign_location(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        location_id: Option<&str>,
    ) -> AppResult<NetworkAsset> {
        self.update_relation(
            actor_id,
            tenant_id,
            id,
            "location_id",
            location_id,
            Some("customer_locations"),
            "Customer location",
        )
        .await
    }

    pub async fn assign_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        work_order_id: Option<&str>,
    ) -> AppResult<NetworkAsset> {
        self.update_relation(
            actor_id,
            tenant_id,
            id,
            "work_order_id",
            work_order_id,
            Some("installation_work_orders"),
            "Installation work order",
        )
        .await
    }

    pub async fn link_parent_asset(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        parent_asset_id: Option<&str>,
    ) -> AppResult<NetworkAsset> {
        self.update_relation(
            actor_id,
            tenant_id,
            id,
            "parent_asset_id",
            parent_asset_id,
            Some("network_assets"),
            "Parent asset",
        )
        .await
    }

    pub async fn list_customer_assets(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
    ) -> AppResult<Vec<NetworkAssetListItem>> {
        self.require_read(actor_id, tenant_id).await?;
        self.ensure_relation_exists(tenant_id, "customers", Some(customer_id), "Customer")
            .await?;

        let result = self
            .list_assets(
                actor_id,
                tenant_id,
                ListNetworkAssetsParams {
                    q: None,
                    asset_type: None,
                    status: None,
                    customer_id: Some(customer_id.to_string()),
                    location_id: None,
                    parent_asset_id: None,
                    page: Some(1),
                    per_page: Some(500),
                },
            )
            .await?;

        Ok(result.data)
    }
}
