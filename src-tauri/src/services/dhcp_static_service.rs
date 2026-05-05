use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    CreateDhcpStaticServiceRequest, DhcpStaticQueueMode, DhcpStaticService,
    DhcpStaticServicePublic, PaginatedResponse, UpdateDhcpStaticServiceRequest,
};
use crate::security::secret::decrypt_secret_opt;
use crate::services::{AuditService, AuthService};
use chrono::Utc;
use mikrotik_rs::{protocol::command::CommandBuilder, protocol::CommandResponse, MikrotikDevice};
use std::net::IpAddr;
use std::str::FromStr;
use tokio::time::{timeout, Duration};

#[derive(Clone)]
pub struct DhcpStaticServiceManager {
    pool: DbPool,
    auth_service: AuthService,
    audit_service: AuditService,
}

#[derive(Debug, Clone)]
struct InstallationDhcpScope {
    subscription_id: String,
    customer_id: String,
    location_id: String,
    package_id: Option<String>,
    router_id: Option<String>,
    status: String,
    assigned_to: Option<String>,
}

impl DhcpStaticServiceManager {
    pub fn new(pool: DbPool, auth_service: AuthService, audit_service: AuditService) -> Self {
        Self {
            pool,
            auth_service,
            audit_service,
        }
    }

    async fn require_read_or_installation_manage(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<()> {
        if self
            .auth_service
            .check_permission(actor_id, tenant_id, "dhcp_static", "read")
            .await
            .is_ok()
        {
            return Ok(());
        }
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await
    }

    async fn is_actor_admin_or_owner(&self, tenant_id: &str, actor_id: &str) -> AppResult<bool> {
        let role_name: Option<String> = sqlx::query_scalar(
            r#"
            SELECT LOWER(COALESCE(r.name, tm.role, ''))
            FROM tenant_members tm
            LEFT JOIN roles r ON r.id = tm.role_id
            WHERE tm.tenant_id = $1 AND tm.user_id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(matches!(
            role_name.as_deref().unwrap_or_default(),
            "admin" | "owner"
        ))
    }

    async fn load_installation_scope(
        &self,
        tenant_id: &str,
        work_order_id: &str,
    ) -> AppResult<InstallationDhcpScope> {
        let row: Option<(
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            Option<String>,
        )> = sqlx::query_as(
            r#"
            SELECT
              wo.subscription_id,
              wo.customer_id,
              wo.location_id,
              cs.package_id,
              COALESCE(wo.router_id, cs.router_id) AS router_id,
              wo.status,
              wo.assigned_to
            FROM installation_work_orders wo
            LEFT JOIN customer_subscriptions cs
              ON cs.tenant_id = wo.tenant_id
             AND cs.id = wo.subscription_id
            WHERE wo.tenant_id = $1
              AND wo.id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let Some((
            subscription_id,
            customer_id,
            location_id,
            package_id,
            router_id,
            status,
            assigned_to,
        )) = row
        else {
            return Err(AppError::NotFound(
                "Installation work order not found".into(),
            ));
        };

        Ok(InstallationDhcpScope {
            subscription_id,
            customer_id,
            location_id,
            package_id,
            router_id,
            status,
            assigned_to,
        })
    }

    async fn require_manage_or_installation_scope(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: Option<&str>,
        service: &DhcpStaticService,
    ) -> AppResult<()> {
        if self
            .auth_service
            .check_permission(actor_id, tenant_id, "dhcp_static", "manage")
            .await
            .is_ok()
        {
            return Ok(());
        }

        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        let work_order_id =
            work_order_id.ok_or_else(|| AppError::Forbidden("work_order_id is required".into()))?;
        let scope = self
            .load_installation_scope(tenant_id, work_order_id)
            .await?;
        let is_admin_owner = self.is_actor_admin_or_owner(tenant_id, actor_id).await?;
        let status = scope.status.trim().to_ascii_lowercase();
        let assigned_to = scope.assigned_to.as_deref().unwrap_or_default();
        if !matches!(status.as_str(), "pending" | "in_progress") {
            return Err(AppError::Forbidden(
                "Installation work order is not open for DHCP provisioning".into(),
            ));
        }
        if !is_admin_owner && assigned_to != actor_id {
            return Err(AppError::Forbidden(
                "DHCP static service is outside the assigned installation work order".into(),
            ));
        }
        if scope.subscription_id != service.subscription_id
            || scope.customer_id != service.customer_id
            || scope.location_id != service.location_id
            || scope.package_id.as_deref() != Some(service.package_id.as_str())
        {
            return Err(AppError::Forbidden(
                "DHCP static service does not match the installation work order scope".into(),
            ));
        }
        if let Some(expected_router_id) =
            scope.router_id.as_deref().filter(|v| !v.trim().is_empty())
        {
            if expected_router_id != service.router_id {
                return Err(AppError::Forbidden(
                    "DHCP static router does not match the installation work order".into(),
                ));
            }
        }
        Ok(())
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

    async fn ensure_subscription_scope(
        &self,
        tenant_id: &str,
        dto: &CreateDhcpStaticServiceRequest,
    ) -> AppResult<()> {
        let row: Option<(String, String, String, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT customer_id, location_id, package_id, status, router_id
            FROM customer_subscriptions
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&dto.subscription_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let Some((customer_id, location_id, package_id, status, router_id)) = row else {
            return Err(AppError::Validation("Subscription not found".into()));
        };

        if customer_id != dto.customer_id
            || location_id != dto.location_id
            || package_id != dto.package_id
        {
            return Err(AppError::Validation(
                "Subscription customer/location/package does not match DHCP request".into(),
            ));
        }
        if let Some(sub_router_id) = router_id.filter(|v| !v.trim().is_empty()) {
            if sub_router_id != dto.router_id {
                return Err(AppError::Validation(
                    "Subscription router does not match DHCP request router".into(),
                ));
            }
        }
        let normalized_status = status.trim().to_ascii_lowercase();
        if !matches!(
            normalized_status.as_str(),
            "active"
                | "pending_installation"
                | "grace_active"
                | "installation_done_awaiting_payment"
        ) {
            return Err(AppError::Validation(
                "Subscription is not in a DHCP-provisionable status".into(),
            ));
        }
        Ok(())
    }

    async fn ensure_package_is_dhcp_static(
        &self,
        tenant_id: &str,
        package_id: &str,
    ) -> AppResult<()> {
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT service_type, provisioning_type FROM isp_packages WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(package_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;
        let Some((service_type, provisioning_type)) = row else {
            return Err(AppError::Validation("Package not found".into()));
        };
        if service_type != "internet_pppoe" || provisioning_type != "dhcp_static" {
            return Err(AppError::Validation(
                "Selected package is not configured for DHCP static provisioning".into(),
            ));
        }
        Ok(())
    }

    fn normalize_mac_address(value: &str) -> AppResult<String> {
        let upper = value.trim().replace('-', ":").to_ascii_uppercase();
        let parts: Vec<&str> = upper.split(':').collect();
        if parts.len() != 6
            || parts
                .iter()
                .any(|part| part.len() != 2 || !part.chars().all(|c| c.is_ascii_hexdigit()))
        {
            return Err(AppError::Validation(
                "mac_address must use a valid MAC format like AA:BB:CC:DD:EE:FF".into(),
            ));
        }
        Ok(parts.join(":"))
    }

    fn normalize_ip_address(value: &str) -> AppResult<String> {
        let trimmed = value.trim();
        let parsed = IpAddr::from_str(trimmed).map_err(|_| {
            AppError::Validation("ip_address must contain a valid IPv4 or IPv6 address".into())
        })?;
        Ok(parsed.to_string())
    }

    fn normalize_required(value: &str, field: &str) -> AppResult<String> {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            return Err(AppError::Validation(format!("{field} is required")));
        }
        Ok(trimmed)
    }

    fn normalize_optional(value: Option<String>) -> Option<String> {
        value.and_then(|raw| {
            let trimmed = raw.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    }

    fn normalize_queue_rate_limit(value: Option<String>) -> Option<String> {
        Self::normalize_optional(value)
    }

    fn build_queue_name(service: &DhcpStaticService) -> String {
        format!("dhcp-{}", &service.id[..12])
    }

    async fn connect_router(&self, tenant_id: &str, router_id: &str) -> AppResult<MikrotikDevice> {
        let row = sqlx::query_as::<_, crate::models::MikrotikRouter>(
            "SELECT * FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2",
        )
        .bind(router_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Router not found".into()))?;

        let password = decrypt_secret_opt(row.password.as_str())?;
        let addr = format!("{}:{}", row.host, row.port);

        timeout(
            Duration::from_secs(5),
            MikrotikDevice::connect(addr, row.username.as_str(), password.as_deref()),
        )
        .await
        .map_err(|_| AppError::Internal("Connection timed out".into()))?
        .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn router_find_lease_id(
        &self,
        dev: &MikrotikDevice,
        server_name: &str,
        mac_address: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let cmd = CommandBuilder::new()
            .command("/ip/dhcp-server/lease/print")
            .build();
        let mut rx = dev.send_command(cmd).await?;
        while let Some(res) = rx.recv().await {
            let reply = res?;
            if let CommandResponse::Reply(row) = reply {
                let server = row.attributes.get("server").and_then(|v| v.clone());
                let mac = row.attributes.get("mac-address").and_then(|v| v.clone());
                if server.as_deref() == Some(server_name) && mac.as_deref() == Some(mac_address) {
                    return Ok(row.attributes.get(".id").and_then(|v| v.clone()));
                }
            }
        }
        Ok(None)
    }

    async fn router_apply_lease(
        &self,
        dev: &MikrotikDevice,
        service: &DhcpStaticService,
    ) -> Result<Option<String>, anyhow::Error> {
        let existing = self
            .router_find_lease_id(dev, &service.dhcp_server_name, &service.mac_address)
            .await?;
        let mut builder = if let Some(id) = existing.clone() {
            CommandBuilder::new()
                .command("/ip/dhcp-server/lease/set")
                .attribute("numbers", Some(id.as_str()))
        } else {
            CommandBuilder::new().command("/ip/dhcp-server/lease/add")
        };
        builder = builder
            .attribute("server", Some(service.dhcp_server_name.as_str()))
            .attribute("mac-address", Some(service.mac_address.as_str()))
            .attribute("address", Some(service.ip_address.as_str()))
            .attribute(
                "disabled",
                Some(if service.disabled { "yes" } else { "no" }),
            );
        if let Some(comment) = service.comment.as_deref() {
            builder = builder.attribute("comment", Some(comment));
        }
        let mut rx = dev.send_command(builder.build()).await?;
        while let Some(res) = rx.recv().await {
            let reply = res?;
            if let CommandResponse::Trap(trap) = reply {
                return Err(anyhow::anyhow!(if trap.message.trim().is_empty() {
                    "RouterOS trap".to_string()
                } else {
                    trap.message
                }));
            }
        }
        if existing.is_some() {
            Ok(existing)
        } else {
            self.router_find_lease_id(dev, &service.dhcp_server_name, &service.mac_address)
                .await
        }
    }

    async fn router_find_queue_id(
        &self,
        dev: &MikrotikDevice,
        queue_name: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let cmd = CommandBuilder::new().command("/queue/simple/print").build();
        let mut rx = dev.send_command(cmd).await?;
        while let Some(res) = rx.recv().await {
            let reply = res?;
            if let CommandResponse::Reply(row) = reply {
                let name = row.attributes.get("name").and_then(|v| v.clone());
                if name.as_deref() == Some(queue_name) {
                    return Ok(row.attributes.get(".id").and_then(|v| v.clone()));
                }
            }
        }
        Ok(None)
    }

    async fn router_apply_queue(
        &self,
        dev: &MikrotikDevice,
        service: &DhcpStaticService,
    ) -> Result<Option<String>, anyhow::Error> {
        if service.queue_mode == DhcpStaticQueueMode::None {
            return Ok(None);
        }
        let queue_name = service
            .queue_name
            .as_deref()
            .filter(|v| !v.is_empty())
            .unwrap_or_default();
        if queue_name.is_empty() {
            return Err(anyhow::anyhow!("Queue name is not configured"));
        }
        let max_limit = service
            .queue_rate_limit
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("queue_rate_limit is required for simple_queue mode"))?;
        let target = service
            .queue_target
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Queue target is missing"))?;
        let existing = self.router_find_queue_id(dev, queue_name).await?;
        let mut builder = if let Some(id) = existing.clone() {
            CommandBuilder::new()
                .command("/queue/simple/set")
                .attribute("numbers", Some(id.as_str()))
        } else {
            CommandBuilder::new().command("/queue/simple/add")
        };
        builder = builder
            .attribute("name", Some(queue_name))
            .attribute("target", Some(target))
            .attribute("max-limit", Some(max_limit))
            .attribute(
                "disabled",
                Some(if service.disabled { "yes" } else { "no" }),
            );
        if let Some(comment) = service.comment.as_deref() {
            builder = builder.attribute("comment", Some(comment));
        }
        let mut rx = dev.send_command(builder.build()).await?;
        while let Some(res) = rx.recv().await {
            let reply = res?;
            if let CommandResponse::Trap(trap) = reply {
                return Err(anyhow::anyhow!(if trap.message.trim().is_empty() {
                    "RouterOS trap".to_string()
                } else {
                    trap.message
                }));
            }
        }
        self.router_find_queue_id(dev, queue_name).await
    }

    async fn update_sync_state(
        &self,
        tenant_id: &str,
        service_id: &str,
        lease_present: bool,
        lease_router_ref: Option<String>,
        lease_last_error: Option<String>,
        queue_present: bool,
        queue_last_error: Option<String>,
    ) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE dhcp_static_services
            SET lease_present = $1,
                lease_router_ref = $2,
                lease_last_sync_at = $3,
                lease_last_error = $4,
                queue_present = $5,
                queue_last_sync_at = $6,
                queue_last_error = $7,
                updated_at = $8
            WHERE tenant_id = $9 AND id = $10
            "#,
        )
        .bind(lease_present)
        .bind(lease_router_ref)
        .bind(now)
        .bind(lease_last_error)
        .bind(queue_present)
        .bind(now)
        .bind(queue_last_error)
        .bind(now)
        .bind(tenant_id)
        .bind(service_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    async fn load_service_row(&self, tenant_id: &str, id: &str) -> AppResult<DhcpStaticService> {
        sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, router_id, customer_id, location_id, package_id,
                   dhcp_server_name, mac_address, ip_address, comment, disabled,
                   lease_present, lease_router_ref, lease_last_sync_at, lease_last_error,
                   queue_mode, queue_name, queue_target, queue_rate_limit,
                   queue_present, queue_last_sync_at, queue_last_error,
                   created_at, updated_at
            FROM dhcp_static_services
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("DHCP static service not found".into()))
    }

    pub async fn list_services(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: Option<String>,
        location_id: Option<String>,
        router_id: Option<String>,
        dhcp_server_name: Option<String>,
        q: Option<String>,
        page: u32,
        per_page: u32,
    ) -> AppResult<PaginatedResponse<DhcpStaticServicePublic>> {
        self.require_read_or_installation_manage(actor_id, tenant_id)
            .await?;
        let offset = (page.saturating_sub(1)) * per_page;
        let q = q.unwrap_or_default();
        let customer_id = customer_id.unwrap_or_default();
        let location_id = location_id.unwrap_or_default();
        let router_id = router_id.unwrap_or_default();
        let dhcp_server_name = dhcp_server_name.unwrap_or_default();

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM dhcp_static_services
            WHERE tenant_id = $1
              AND ($2 = '' OR customer_id = $2)
              AND ($3 = '' OR location_id = $3)
              AND ($4 = '' OR router_id = $4)
              AND ($5 = '' OR dhcp_server_name = $5)
              AND ($6 = '' OR mac_address ILIKE '%' || $6 || '%' OR ip_address ILIKE '%' || $6 || '%' OR comment ILIKE '%' || $6 || '%')
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(&location_id)
        .bind(&router_id)
        .bind(&dhcp_server_name)
        .bind(&q)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let rows: Vec<DhcpStaticService> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, router_id, customer_id, location_id, package_id,
                   dhcp_server_name, mac_address, ip_address, comment, disabled,
                   lease_present, lease_router_ref, lease_last_sync_at, lease_last_error,
                   queue_mode, queue_name, queue_target, queue_rate_limit,
                   queue_present, queue_last_sync_at, queue_last_error,
                   created_at, updated_at
            FROM dhcp_static_services
            WHERE tenant_id = $1
              AND ($2 = '' OR customer_id = $2)
              AND ($3 = '' OR location_id = $3)
              AND ($4 = '' OR router_id = $4)
              AND ($5 = '' OR dhcp_server_name = $5)
              AND ($6 = '' OR mac_address ILIKE '%' || $6 || '%' OR ip_address ILIKE '%' || $6 || '%' OR comment ILIKE '%' || $6 || '%')
            ORDER BY updated_at DESC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(&location_id)
        .bind(&router_id)
        .bind(&dhcp_server_name)
        .bind(&q)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(PaginatedResponse {
            data: rows.into_iter().map(Into::into).collect(),
            total,
            page,
            per_page,
        })
    }

    pub async fn get_service(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<DhcpStaticServicePublic> {
        self.require_read_or_installation_manage(actor_id, tenant_id)
            .await?;
        Ok(self.load_service_row(tenant_id, id).await?.into())
    }

    pub async fn create_service(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateDhcpStaticServiceRequest,
        ip_address: Option<&str>,
    ) -> AppResult<DhcpStaticServicePublic> {
        self.ensure_router_access(tenant_id, &dto.router_id).await?;
        self.ensure_subscription_scope(tenant_id, &dto).await?;
        self.ensure_package_is_dhcp_static(tenant_id, &dto.package_id)
            .await?;

        let mut service = DhcpStaticService::new(
            tenant_id.to_string(),
            dto.subscription_id.trim().to_string(),
            dto.router_id.trim().to_string(),
            dto.customer_id.trim().to_string(),
            dto.location_id.trim().to_string(),
            dto.package_id.trim().to_string(),
            Self::normalize_required(&dto.dhcp_server_name, "dhcp_server_name")?,
            Self::normalize_mac_address(&dto.mac_address)?,
            Self::normalize_ip_address(&dto.ip_address)?,
            Self::normalize_optional(dto.comment),
            dto.disabled,
            dto.queue_mode,
            Self::normalize_queue_rate_limit(dto.queue_rate_limit),
        );
        if service.queue_mode == DhcpStaticQueueMode::SimpleQueue {
            service.queue_name = Some(Self::build_queue_name(&service));
        }

        self.require_manage_or_installation_scope(
            actor_id,
            tenant_id,
            dto.work_order_id.as_deref(),
            &service,
        )
        .await?;

        sqlx::query(
            r#"
            INSERT INTO dhcp_static_services (
                id, tenant_id, subscription_id, router_id, customer_id, location_id, package_id,
                dhcp_server_name, mac_address, ip_address, comment, disabled,
                lease_present, lease_router_ref, lease_last_sync_at, lease_last_error,
                queue_mode, queue_name, queue_target, queue_rate_limit,
                queue_present, queue_last_sync_at, queue_last_error,
                created_at, updated_at
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25
            )
            "#,
        )
        .bind(&service.id)
        .bind(&service.tenant_id)
        .bind(&service.subscription_id)
        .bind(&service.router_id)
        .bind(&service.customer_id)
        .bind(&service.location_id)
        .bind(&service.package_id)
        .bind(&service.dhcp_server_name)
        .bind(&service.mac_address)
        .bind(&service.ip_address)
        .bind(&service.comment)
        .bind(service.disabled)
        .bind(service.lease_present)
        .bind(&service.lease_router_ref)
        .bind(service.lease_last_sync_at)
        .bind(&service.lease_last_error)
        .bind(service.queue_mode)
        .bind(&service.queue_name)
        .bind(&service.queue_target)
        .bind(&service.queue_rate_limit)
        .bind(service.queue_present)
        .bind(service.queue_last_sync_at)
        .bind(&service.queue_last_error)
        .bind(service.created_at)
        .bind(service.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.as_database_error()
                .and_then(|db| db.code().map(|code| code == "23505"))
                .unwrap_or(false)
            {
                AppError::Validation(
                    "Duplicate DHCP static subscription, MAC address, or IP address detected"
                        .into(),
                )
            } else {
                AppError::Database(e)
            }
        })?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "DHCP_STATIC_CREATE",
                "dhcp_static_services",
                Some(&service.id),
                Some(&format!(
                    "Created DHCP static service {} -> {} on {}",
                    service.mac_address, service.ip_address, service.dhcp_server_name
                )),
                ip_address,
            )
            .await;

        Ok(service.into())
    }

    pub async fn update_service(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        dto: UpdateDhcpStaticServiceRequest,
        ip_address: Option<&str>,
    ) -> AppResult<DhcpStaticServicePublic> {
        let mut service = self.load_service_row(tenant_id, id).await?;
        self.require_manage_or_installation_scope(
            actor_id,
            tenant_id,
            dto.work_order_id.as_deref(),
            &service,
        )
        .await?;

        if let Some(router_id) = dto.router_id {
            self.ensure_router_access(tenant_id, &router_id).await?;
            service.router_id = router_id.trim().to_string();
        }
        if let Some(package_id) = dto.package_id {
            self.ensure_package_is_dhcp_static(tenant_id, &package_id)
                .await?;
            service.package_id = package_id.trim().to_string();
        }
        if let Some(name) = dto.dhcp_server_name {
            service.dhcp_server_name = Self::normalize_required(&name, "dhcp_server_name")?;
        }
        if let Some(mac) = dto.mac_address {
            service.mac_address = Self::normalize_mac_address(&mac)?;
        }
        if let Some(ip) = dto.ip_address {
            service.ip_address = Self::normalize_ip_address(&ip)?;
            service.queue_target = Some(format!("{}/32", service.ip_address));
        }
        if let Some(comment) = dto.comment {
            service.comment = Self::normalize_optional(Some(comment));
        }
        if let Some(disabled) = dto.disabled {
            service.disabled = disabled;
        }
        if let Some(queue_mode) = dto.queue_mode {
            service.queue_mode = queue_mode;
        }
        if dto.queue_rate_limit.is_some() {
            service.queue_rate_limit = Self::normalize_queue_rate_limit(dto.queue_rate_limit);
        }
        service.queue_name = if service.queue_mode == DhcpStaticQueueMode::SimpleQueue {
            Some(Self::build_queue_name(&service))
        } else {
            None
        };
        service.updated_at = Utc::now();

        sqlx::query(
            r#"
            UPDATE dhcp_static_services
            SET router_id = $1,
                package_id = $2,
                dhcp_server_name = $3,
                mac_address = $4,
                ip_address = $5,
                comment = $6,
                disabled = $7,
                queue_mode = $8,
                queue_name = $9,
                queue_target = $10,
                queue_rate_limit = $11,
                updated_at = $12
            WHERE tenant_id = $13 AND id = $14
            "#,
        )
        .bind(&service.router_id)
        .bind(&service.package_id)
        .bind(&service.dhcp_server_name)
        .bind(&service.mac_address)
        .bind(&service.ip_address)
        .bind(&service.comment)
        .bind(service.disabled)
        .bind(service.queue_mode)
        .bind(&service.queue_name)
        .bind(&service.queue_target)
        .bind(&service.queue_rate_limit)
        .bind(service.updated_at)
        .bind(tenant_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "DHCP_STATIC_UPDATE",
                "dhcp_static_services",
                Some(id),
                Some("Updated DHCP static service"),
                ip_address,
            )
            .await;

        Ok(service.into())
    }

    pub async fn delete_service(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "dhcp_static", "manage")
            .await?;
        sqlx::query("DELETE FROM dhcp_static_services WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "DHCP_STATIC_DELETE",
                "dhcp_static_services",
                Some(id),
                Some("Deleted DHCP static service"),
                ip_address,
            )
            .await;
        Ok(())
    }

    pub async fn apply_service(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        work_order_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> AppResult<DhcpStaticServicePublic> {
        let service = self.load_service_row(tenant_id, id).await?;
        self.require_manage_or_installation_scope(actor_id, tenant_id, work_order_id, &service)
            .await?;

        let dev = self.connect_router(tenant_id, &service.router_id).await?;
        let lease_result = self.router_apply_lease(&dev, &service).await;
        let (lease_present, lease_router_ref, lease_last_error) = match lease_result {
            Ok(router_ref) => (true, router_ref, None),
            Err(error) => (false, None, Some(error.to_string())),
        };
        let queue_result = self.router_apply_queue(&dev, &service).await;
        let (queue_present, queue_last_error) = match queue_result {
            Ok(_) => (service.queue_mode == DhcpStaticQueueMode::SimpleQueue, None),
            Err(error) => (false, Some(error.to_string())),
        };

        self.update_sync_state(
            tenant_id,
            id,
            lease_present,
            lease_router_ref,
            lease_last_error.clone(),
            queue_present,
            queue_last_error.clone(),
        )
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "DHCP_STATIC_APPLY",
                "dhcp_static_services",
                Some(id),
                Some("Applied DHCP static service to router"),
                ip_address,
            )
            .await;

        let updated = self.load_service_row(tenant_id, id).await?;
        if let Some(error) = updated.lease_last_error.clone() {
            return Err(AppError::Internal(error));
        }
        Ok(updated.into())
    }

    pub async fn reconcile_router(
        &self,
        actor_id: &str,
        tenant_id: &str,
        router_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<serde_json::Value> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "dhcp_static", "manage")
            .await?;
        self.ensure_router_access(tenant_id, router_id).await?;

        let rows: Vec<DhcpStaticService> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, router_id, customer_id, location_id, package_id,
                   dhcp_server_name, mac_address, ip_address, comment, disabled,
                   lease_present, lease_router_ref, lease_last_sync_at, lease_last_error,
                   queue_mode, queue_name, queue_target, queue_rate_limit,
                   queue_present, queue_last_sync_at, queue_last_error,
                   created_at, updated_at
            FROM dhcp_static_services
            WHERE tenant_id = $1 AND router_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let dev = self.connect_router(tenant_id, router_id).await?;
        let mut updated = 0usize;
        for row in rows {
            let lease_router_ref = self
                .router_find_lease_id(&dev, &row.dhcp_server_name, &row.mac_address)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let queue_present = if let Some(queue_name) = row.queue_name.as_deref() {
                self.router_find_queue_id(&dev, queue_name)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?
                    .is_some()
            } else {
                false
            };
            self.update_sync_state(
                tenant_id,
                &row.id,
                lease_router_ref.is_some(),
                lease_router_ref,
                None,
                queue_present,
                None,
            )
            .await?;
            updated += 1;
        }
        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "DHCP_STATIC_RECONCILE",
                "dhcp_static_services",
                Some(router_id),
                Some("Reconciled DHCP static services for router"),
                ip_address,
            )
            .await;
        Ok(serde_json::json!({ "ok": true, "updated": updated }))
    }
}
