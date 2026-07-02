//! Role and Permission service for RBAC

use crate::db::DbPool;
use crate::models::{CreateRoleDto, Permission, Role, RoleWithPermissions, UpdateRoleDto};
use crate::services::audit_service::AuditService;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Clone)]
pub struct RoleService {
    pool: DbPool,
    audit_service: AuditService,
}

impl RoleService {
    fn map_legacy_permission_key(key: &str) -> Vec<String> {
        match key {
            "network_routers:read" => vec![
                "network_noc:read",
                "network_alerts:read",
                "network_incidents:read",
                "network_logs:read",
                "router_inventory:read",
                "ppp_profiles:read",
                "ip_pools:read",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            "network_routers:manage" => vec![
                "network_noc:manage",
                "network_alerts:manage",
                "network_incidents:manage",
                "network_logs:manage",
                "router_inventory:manage",
                "ppp_profiles:manage",
                "ip_pools:manage",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            "network_routers:manage_radius_secret" => {
                vec!["router_inventory:manage_radius_secret".to_string()]
            }
            "storage:read" => vec![
                "storage_console:read".to_string(),
                "storage_files:read".to_string(),
            ],
            "storage:upload" => vec!["storage_files:upload".to_string()],
            "storage:delete" => vec!["storage_files:delete".to_string()],
            "pppoe:read" => vec![
                "ppp_profiles:read".to_string(),
                "ip_pools:read".to_string(),
                "pppoe:read".to_string(),
            ],
            "pppoe:manage" => vec![
                "ppp_profiles:manage".to_string(),
                "ip_pools:manage".to_string(),
                "pppoe:manage".to_string(),
            ],
            _ => vec![key.to_string()],
        }
    }

    fn normalize_permission_keys<'a, I>(permissions: I) -> Vec<String>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut normalized = permissions
            .into_iter()
            .flat_map(Self::map_legacy_permission_key)
            .collect::<Vec<_>>();
        normalized.sort();
        normalized.dedup();
        normalized
    }

    fn legacy_permission_keys() -> &'static [&'static str] {
        &[
            "network_routers:read",
            "network_routers:manage",
            "network_routers:manage_radius_secret",
            "storage:read",
            "storage:upload",
            "storage:delete",
            "pppoe:read",
            "pppoe:manage",
        ]
    }

    pub fn new(pool: DbPool, audit_service: AuditService) -> Self {
        Self {
            pool,
            audit_service,
        }
    }

    /// Default permissions to seed
    pub fn get_default_permissions() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            // Team permissions
            ("team", "create", "Create team members"),
            ("team", "read", "View team members"),
            ("team", "update", "Update team members"),
            ("team", "delete", "Remove team members"),
            // Settings permissions
            ("settings", "read", "View settings"),
            ("settings", "update", "Modify settings"),
            // Roles permissions
            ("roles", "create", "Create roles"),
            ("roles", "read", "View roles"),
            ("roles", "update", "Modify roles"),
            ("roles", "delete", "Delete roles"),
            // Dashboard permissions
            ("dashboard", "read", "View dashboard"),
            // Customers (tenant scoped)
            ("customers", "read", "View customers"),
            ("customers", "manage", "Manage customers"),
            ("customers", "read_own", "View own customer portal data"),
            ("customer_locations", "read", "View customer locations"),
            ("customer_locations", "manage", "Manage customer locations"),
            // Network monitoring and inventory (tenant scoped)
            ("network_noc", "read", "View NOC dashboards and wallboards"),
            (
                "network_noc",
                "manage",
                "Manage NOC wallboards and dashboard settings",
            ),
            ("network_alerts", "read", "View network alerts"),
            (
                "network_alerts",
                "manage",
                "Acknowledge and resolve network alerts",
            ),
            ("network_incidents", "read", "View network incidents"),
            (
                "network_incidents",
                "manage",
                "Manage and escalate network incidents",
            ),
            ("network_logs", "read", "View router and network logs"),
            (
                "network_logs",
                "manage",
                "Sync, clear, and manage router logs",
            ),
            ("router_inventory", "read", "View routers and status"),
            ("router_inventory", "manage", "Manage router inventory"),
            (
                "router_inventory",
                "manage_radius_secret",
                "Reveal managed RADIUS shared secrets",
            ),
            ("ftth_assets", "read", "View FTTH assets"),
            ("ftth_assets", "manage", "Manage FTTH assets"),
            // Network Topology / Mapping (tenant scoped)
            ("network_topology", "read", "View network topology map"),
            ("network_topology", "manage", "Manage network topology map"),
            ("service_zones", "read", "View service zones"),
            ("service_zones", "manage", "Manage service zones"),
            ("coverage", "read", "Read coverage checks"),
            ("ppp_profiles", "read", "View PPP profiles"),
            ("ppp_profiles", "manage", "Manage PPP profiles"),
            ("ip_pools", "read", "View IP pools"),
            ("ip_pools", "manage", "Manage IP pools"),
            // PPPoE (tenant scoped)
            ("pppoe", "read", "View PPPoE accounts"),
            ("pppoe", "manage", "Manage PPPoE accounts"),
            // DHCP Static (tenant scoped)
            ("dhcp_static", "read", "View DHCP static services"),
            ("dhcp_static", "manage", "Manage DHCP static services"),
            // ISP Packages (tenant scoped)
            ("isp_packages", "read", "View ISP packages"),
            ("isp_packages", "manage", "Manage ISP packages"),
            // Installation work orders (tenant scoped)
            (
                "orders",
                "create",
                "Create installation orders from backoffice",
            ),
            ("work_orders", "read", "View installation work orders"),
            ("work_orders", "manage", "Manage installation work orders"),
            // Billing / Payments (tenant scoped)
            ("billing", "read", "View billing and subscription data"),
            ("billing", "manage", "Manage billing actions"),
            // Backups permissions
            ("backups", "read", "View backups"),
            ("backups", "create", "Create backups"),
            ("backups", "download", "Download backups"),
            ("backups", "restore", "Restore backups"),
            ("backups", "delete", "Delete backups"),
            // Tenant storage
            ("storage_console", "read", "View tenant storage console"),
            ("storage_files", "read", "Read tenant file contents"),
            ("storage_files", "upload", "Upload tenant files"),
            ("storage_files", "delete", "Delete tenant files"),
            // Support Tickets (tenant scoped)
            ("support", "create", "Create support tickets"),
            ("support", "read", "Read own support tickets"),
            ("support", "read_all", "Read all support tickets in tenant"),
            ("support", "reply", "Reply to support tickets"),
            (
                "support",
                "update",
                "Update support tickets (status/priority)",
            ),
            ("support", "assign", "Assign support tickets"),
            ("support", "internal", "Post internal support notes"),
            // Announcements
            ("announcements", "read", "Read announcements"),
            ("announcements", "manage", "Create/update announcements"),
            // Email Outbox (tenant admin diagnostics)
            ("email_outbox", "read", "View email outbox"),
            ("email_outbox", "retry", "Retry outbox items"),
            ("email_outbox", "delete", "Delete outbox items"),
            // Communication templates
            (
                "communication_templates",
                "read",
                "View WhatsApp and email message templates",
            ),
            (
                "communication_templates",
                "manage",
                "Manage WhatsApp and email message templates",
            ),
            // OLT Monitoring (tenant scoped)
            ("olt", "read", "View OLT devices and ONU status"),
            ("olt", "manage", "Manage OLT inventory, reboot ONU, test connections"),
            ("olt_onu_history", "read", "View ONU signal history and graphs"),
        ]
    }

    /// Default roles with their permissions
    pub fn get_default_roles() -> Vec<(&'static str, &'static str, bool, i32, Vec<&'static str>)> {
        vec![
            // (name, description, is_system, level, permissions)
            (
                "Owner",
                "Full access to all features",
                true,
                100,
                vec![
                    "team:create",
                    "team:read",
                    "team:update",
                    "team:delete",
                    "settings:read",
                    "settings:update",
                    "roles:create",
                    "roles:read",
                    "roles:update",
                    "roles:delete",
                    "dashboard:read",
                    "customers:read",
                    "customers:manage",
                    "customer_locations:read",
                    "customer_locations:manage",
                    "network_noc:read",
                    "network_noc:manage",
                    "network_alerts:read",
                    "network_alerts:manage",
                    "network_incidents:read",
                    "network_incidents:manage",
                    "network_logs:read",
                    "network_logs:manage",
                    "router_inventory:read",
                    "router_inventory:manage",
                    "router_inventory:manage_radius_secret",
                    "ftth_assets:read",
                    "ftth_assets:manage",
                    "network_topology:read",
                    "network_topology:manage",
                    "service_zones:read",
                    "service_zones:manage",
                    "coverage:read",
                    "ppp_profiles:read",
                    "ppp_profiles:manage",
                    "ip_pools:read",
                    "ip_pools:manage",
                    "pppoe:read",
                    "pppoe:manage",
                    "isp_packages:read",
                    "isp_packages:manage",
                    "orders:create",
                    "work_orders:read",
                    "work_orders:manage",
                    "billing:read",
                    "billing:manage",
                    "storage_console:read",
                    "storage_files:read",
                    "storage_files:upload",
                    "storage_files:delete",
                    "backups:read",
                    "backups:create",
                    "backups:download",
                    "backups:restore",
                    "backups:delete",
                    "support:create",
                    "support:read",
                    "support:read_all",
                    "support:reply",
                    "support:update",
                    "support:assign",
                    "support:internal",
                    "announcements:read",
                    "announcements:manage",
                    "email_outbox:read",
                    "email_outbox:retry",
                    "email_outbox:delete",
                    "communication_templates:read",
                    "communication_templates:manage",
                    "olt:read",
                    "olt:manage",
                    "olt_onu_history:read",
                ],
            ),
            (
                "Admin",
                "Manage team and settings",
                true,
                50,
                vec![
                    "team:create",
                    "team:read",
                    "team:update",
                    "team:delete",
                    "settings:read",
                    "settings:update",
                    "roles:read",
                    "dashboard:read",
                    "customers:read",
                    "customers:manage",
                    "customer_locations:read",
                    "customer_locations:manage",
                    "network_noc:read",
                    "network_noc:manage",
                    "network_alerts:read",
                    "network_alerts:manage",
                    "network_incidents:read",
                    "network_incidents:manage",
                    "network_logs:read",
                    "network_logs:manage",
                    "router_inventory:read",
                    "router_inventory:manage",
                    "router_inventory:manage_radius_secret",
                    "ftth_assets:read",
                    "ftth_assets:manage",
                    "network_topology:read",
                    "network_topology:manage",
                    "service_zones:read",
                    "service_zones:manage",
                    "coverage:read",
                    "ppp_profiles:read",
                    "ppp_profiles:manage",
                    "ip_pools:read",
                    "ip_pools:manage",
                    "pppoe:read",
                    "pppoe:manage",
                    "isp_packages:read",
                    "isp_packages:manage",
                    "orders:create",
                    "work_orders:read",
                    "work_orders:manage",
                    "billing:read",
                    "billing:manage",
                    "storage_console:read",
                    "storage_files:read",
                    "storage_files:upload",
                    "storage_files:delete",
                    "backups:read",
                    "backups:create",
                    "backups:download",
                    "backups:restore",
                    "backups:delete",
                    "support:create",
                    "support:read",
                    "support:read_all",
                    "support:reply",
                    "support:update",
                    "support:assign",
                    "support:internal",
                    "announcements:read",
                    "announcements:manage",
                    "email_outbox:read",
                    "email_outbox:retry",
                    "email_outbox:delete",
                    "communication_templates:read",
                    "communication_templates:manage",
                    "olt:read",
                    "olt:manage",
                    "olt_onu_history:read",
                ],
            ),
            (
                "Planner",
                "Plan network topology, zones, and coverage",
                true,
                30,
                vec![
                    "dashboard:read",
                    "customers:read",
                    "customer_locations:read",
                    "router_inventory:read",
                    "ftth_assets:read",
                    "network_topology:read",
                    "network_topology:manage",
                    "service_zones:read",
                    "service_zones:manage",
                    "coverage:read",
                    "isp_packages:read",
                    "work_orders:read",
                    "announcements:read",
                    "olt:read",
                ],
            ),
            (
                "NOC",
                "Network operations center access for monitoring and provisioning",
                true,
                35,
                vec![
                    "dashboard:read",
                    "customers:read",
                    "customer_locations:read",
                    "network_noc:read",
                    "network_alerts:read",
                    "network_alerts:manage",
                    "network_incidents:read",
                    "network_incidents:manage",
                    "network_logs:read",
                    "network_logs:manage",
                    "router_inventory:read",
                    "ftth_assets:read",
                    "ppp_profiles:read",
                    "ppp_profiles:manage",
                    "ip_pools:read",
                    "ip_pools:manage",
                    "pppoe:read",
                    "pppoe:manage",
                    "isp_packages:read",
                    "work_orders:read",
                    "work_orders:manage",
                    "billing:read",
                    "support:read",
                    "support:read_all",
                    "support:reply",
                    "support:update",
                    "support:internal",
                    "announcements:read",
                    "olt:read",
                    "olt_onu_history:read",
                ],
            ),
            (
                "Customer Service",
                "Handle customers, tickets, and billing communication",
                true,
                25,
                vec![
                    "dashboard:read",
                    "customers:read",
                    "customers:manage",
                    "customer_locations:read",
                    "customer_locations:manage",
                    "orders:create",
                    "work_orders:read",
                    "storage_files:read",
                    "storage_files:upload",
                    "billing:read",
                    "billing:manage",
                    "support:create",
                    "support:read",
                    "support:read_all",
                    "support:reply",
                    "support:update",
                    "support:assign",
                    "support:internal",
                    "announcements:read",
                    "communication_templates:read",
                    "communication_templates:manage",
                ],
            ),
            (
                "Technician",
                "Field technician for installation and service activation tasks",
                true,
                20,
                vec![
                    "dashboard:read",
                    "customers:read",
                    "customer_locations:read",
                    "router_inventory:read",
                    "ftth_assets:read",
                    "ftth_assets:manage",
                    "ppp_profiles:read",
                    "ip_pools:read",
                    "pppoe:read",
                    "pppoe:manage",
                    "isp_packages:read",
                    "work_orders:read",
                    "work_orders:manage",
                    "support:read",
                    "support:reply",
                    "support:internal",
                    "storage_files:read",
                    "storage_files:upload",
                    "announcements:read",
                    "olt:read",
                    "olt_onu_history:read",
                    "billing:read",
                ],
            ),
            (
                "Member",
                "Standard team member",
                true,
                10,
                vec![
                    "team:read",
                    "dashboard:read",
                    "storage_files:read",
                    "storage_files:upload",
                    "support:create",
                    "support:read",
                    "support:reply",
                    "announcements:read",
                ],
            ),
            (
                "Viewer",
                "Read-only access",
                true,
                0,
                vec!["dashboard:read"],
            ),
            (
                "Customer",
                "Customer portal access (dashboard only)",
                true,
                0,
                vec![
                    "dashboard:read",
                    "announcements:read",
                    "storage_files:read",
                    "storage_files:upload",
                    "support:create",
                    "support:read",
                    "support:reply",
                    "customers:read_own",
                    "coverage:read",
                ],
            ),
        ]
    }

    pub fn get_role_permission_keys(role_name: &str) -> Option<Vec<&'static str>> {
        Self::get_default_roles()
            .into_iter()
            .find(|(name, _, _, _, _)| *name == role_name)
            .map(|(_, _, _, _, permissions)| permissions)
    }

    /// Seed default permissions into database
    pub async fn seed_permissions(&self) -> Result<(), sqlx::Error> {
        let permissions = Self::get_default_permissions();

        for (resource, action, description) in permissions {
            let id = format!("{}:{}", resource, action);

            #[cfg(feature = "postgres")]
            {
                sqlx::query(
                    r#"
                    INSERT INTO permissions (id, resource, action, description)
                    VALUES ($1, $2, $3, $4)
                    ON CONFLICT (resource, action) DO NOTHING
                "#,
                )
                .bind(&id)
                .bind(resource)
                .bind(action)
                .bind(description)
                .execute(&self.pool)
                .await?;
            }

            #[cfg(feature = "sqlite")]
            {
                sqlx::query(
                    r#"
                    INSERT OR IGNORE INTO permissions (id, resource, action, description)
                    VALUES (?, ?, ?, ?)
                "#,
                )
                .bind(&id)
                .bind(resource)
                .bind(action)
                .bind(description)
                .execute(&self.pool)
                .await?;
            }
        }

        tracing::info!("Default permissions seeded");
        Ok(())
    }

    async fn migrate_legacy_role_permissions(&self) -> Result<(), sqlx::Error> {
        let legacy_keys = Self::legacy_permission_keys();

        #[cfg(feature = "postgres")]
        let roles: Vec<(String,)> = sqlx::query_as("SELECT id FROM roles")
            .fetch_all(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let roles: Vec<(String,)> = sqlx::query_as("SELECT id FROM roles")
            .fetch_all(&self.pool)
            .await?;

        for (role_id,) in roles {
            let existing = self.get_role_permissions(&role_id).await?;
            let existing_set = existing.iter().cloned().collect::<HashSet<_>>();
            let normalized = Self::normalize_permission_keys(existing.iter().map(String::as_str));

            for perm_key in normalized {
                if existing_set.contains(&perm_key) {
                    continue;
                }
                let parts: Vec<&str> = perm_key.split(':').collect();
                if parts.len() != 2 {
                    continue;
                }
                let (resource, action) = (parts[0], parts[1]);

                #[cfg(feature = "postgres")]
                sqlx::query(
                    r#"
                    INSERT INTO role_permissions (role_id, permission_id)
                    SELECT $1, id FROM permissions WHERE resource = $2 AND action = $3
                    ON CONFLICT DO NOTHING
                "#,
                )
                .bind(&role_id)
                .bind(resource)
                .bind(action)
                .execute(&self.pool)
                .await?;

                #[cfg(feature = "sqlite")]
                sqlx::query(
                    r#"
                    INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
                    SELECT ?, id FROM permissions WHERE resource = ? AND action = ?
                "#,
                )
                .bind(&role_id)
                .bind(resource)
                .bind(action)
                .execute(&self.pool)
                .await?;
            }

            for legacy_key in legacy_keys {
                #[cfg(feature = "postgres")]
                sqlx::query(
                    r#"
                    DELETE FROM role_permissions
                    WHERE role_id = $1
                      AND permission_id IN (SELECT id FROM permissions WHERE id = $2)
                "#,
                )
                .bind(&role_id)
                .bind(legacy_key)
                .execute(&self.pool)
                .await?;

                #[cfg(feature = "sqlite")]
                sqlx::query(
                    r#"
                    DELETE FROM role_permissions
                    WHERE role_id = ?
                      AND permission_id IN (SELECT id FROM permissions WHERE id = ?)
                "#,
                )
                .bind(&role_id)
                .bind(legacy_key)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Seed default roles into database (global roles, tenant_id = NULL)
    pub async fn seed_roles(&self) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let roles = Self::get_default_roles();

        for (name, description, is_system, level, permission_keys) in roles {
            // Check if role already exists
            #[cfg(feature = "postgres")]
            let existing: Option<(String,)> =
                sqlx::query_as("SELECT id FROM roles WHERE name = $1 AND tenant_id IS NULL")
                    .bind(name)
                    .fetch_optional(&self.pool)
                    .await?;

            #[cfg(feature = "sqlite")]
            let existing: Option<(String,)> =
                sqlx::query_as("SELECT id FROM roles WHERE name = ? AND tenant_id IS NULL")
                    .bind(name)
                    .fetch_optional(&self.pool)
                    .await?;

            let role_id = if let Some((rid,)) = existing {
                // Keep existing system roles, but still ensure metadata/level stays consistent.
                #[cfg(feature = "postgres")]
                sqlx::query(
                    "UPDATE roles SET description = $1, is_system = $2, level = $3, updated_at = $4 WHERE id = $5",
                )
                .bind(description)
                .bind(is_system)
                .bind(level)
                .bind(now)
                .bind(&rid)
                .execute(&self.pool)
                .await?;

                #[cfg(feature = "sqlite")]
                sqlx::query(
                    "UPDATE roles SET description = ?, is_system = ?, level = ?, updated_at = ? WHERE id = ?",
                )
                .bind(description)
                .bind(is_system as i32)
                .bind(level)
                .bind(now.to_rfc3339())
                .bind(&rid)
                .execute(&self.pool)
                .await?;
                rid
            } else {
                let role_id = Uuid::new_v4().to_string();

                // Insert role
                #[cfg(feature = "postgres")]
                {
                    sqlx::query(r#"
                        INSERT INTO roles (id, tenant_id, name, description, is_system, level, created_at, updated_at)
                        VALUES ($1, NULL, $2, $3, $4, $5, $6, $7)
                    "#)
                    .bind(&role_id)
                    .bind(name)
                    .bind(description)
                    .bind(is_system)
                    .bind(level)
                    .bind(now)
                    .bind(now)
                    .execute(&self.pool)
                    .await?;
                }

                #[cfg(feature = "sqlite")]
                {
                    let now_str = now.to_rfc3339();
                    sqlx::query(r#"
                        INSERT INTO roles (id, tenant_id, name, description, is_system, level, created_at, updated_at)
                        VALUES (?, NULL, ?, ?, ?, ?, ?, ?)
                    "#)
                    .bind(&role_id)
                    .bind(name)
                    .bind(description)
                    .bind(is_system as i32)
                    .bind(level)
                    .bind(&now_str)
                    .bind(&now_str)
                    .execute(&self.pool)
                    .await?;
                }

                role_id
            };

            // Link permissions to role
            for perm_key in permission_keys {
                let parts: Vec<&str> = perm_key.split(':').collect();
                if parts.len() != 2 {
                    continue;
                }
                let (resource, action) = (parts[0], parts[1]);

                #[cfg(feature = "postgres")]
                {
                    sqlx::query(
                        r#"
                        INSERT INTO role_permissions (role_id, permission_id)
                        SELECT $1, id FROM permissions WHERE resource = $2 AND action = $3
                        ON CONFLICT DO NOTHING
                    "#,
                    )
                    .bind(&role_id)
                    .bind(resource)
                    .bind(action)
                    .execute(&self.pool)
                    .await?;
                }

                #[cfg(feature = "sqlite")]
                {
                    sqlx::query(
                        r#"
                        INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
                        SELECT ?, id FROM permissions WHERE resource = ? AND action = ?
                    "#,
                    )
                    .bind(&role_id)
                    .bind(resource)
                    .bind(action)
                    .execute(&self.pool)
                    .await?;
                }
            }
        }

        self.migrate_legacy_role_permissions().await?;

        tracing::info!("Default roles seeded");
        Ok(())
    }

    /// Get all roles for a tenant (includes global roles where tenant_id IS NULL)
    pub async fn list_roles(
        &self,
        tenant_id: Option<&str>,
    ) -> Result<Vec<RoleWithPermissions>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let roles: Vec<Role> = if let Some(tid) = tenant_id {
            sqlx::query_as(
                "SELECT * FROM roles WHERE tenant_id IS NULL OR tenant_id = $1 ORDER BY is_system DESC, name"
            )
            .bind(tid)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                "SELECT * FROM roles WHERE tenant_id IS NULL ORDER BY is_system DESC, name",
            )
            .fetch_all(&self.pool)
            .await?
        };

        #[cfg(feature = "sqlite")]
        let roles: Vec<Role> = if let Some(tid) = tenant_id {
            sqlx::query_as(
                "SELECT * FROM roles WHERE tenant_id IS NULL OR tenant_id = ? ORDER BY is_system DESC, name"
            )
            .bind(tid)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                "SELECT * FROM roles WHERE tenant_id IS NULL ORDER BY is_system DESC, name",
            )
            .fetch_all(&self.pool)
            .await?
        };

        let role_ids: Vec<String> = roles.iter().map(|role| role.id.clone()).collect();
        let mut permissions_by_role: HashMap<String, Vec<String>> = HashMap::new();

        if !role_ids.is_empty() {
            #[cfg(feature = "postgres")]
            let rows: Vec<(String, String, String)> = sqlx::query_as(
                r#"
                SELECT rp.role_id, p.resource, p.action
                FROM role_permissions rp
                JOIN permissions p ON p.id = rp.permission_id
                WHERE rp.role_id = ANY($1)
                ORDER BY rp.role_id, p.resource, p.action
                "#,
            )
            .bind(&role_ids)
            .fetch_all(&self.pool)
            .await?;

            #[cfg(feature = "sqlite")]
            let rows: Vec<(String, String, String)> = {
                use sqlx::{QueryBuilder, Sqlite};

                let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
                    r#"
                    SELECT rp.role_id, p.resource, p.action
                    FROM role_permissions rp
                    JOIN permissions p ON p.id = rp.permission_id
                    WHERE rp.role_id IN (
                    "#,
                );
                {
                    let mut separated = qb.separated(", ");
                    for role_id in &role_ids {
                        separated.push_bind(role_id);
                    }
                }
                qb.push(") ORDER BY rp.role_id, p.resource, p.action");
                qb.build_query_as().fetch_all(&self.pool).await?
            };

            for (role_id, resource, action) in rows {
                permissions_by_role
                    .entry(role_id)
                    .or_default()
                    .push(format!("{}:{}", resource, action));
            }
        }

        let mut result = Vec::new();
        for role in roles {
            let permissions = permissions_by_role.remove(&role.id).unwrap_or_default();
            result.push(RoleWithPermissions::from_role(role, permissions));
        }

        Ok(result)
    }

    /// Get permissions for a role
    pub async fn get_role_permissions(&self, role_id: &str) -> Result<Vec<String>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let perms: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT p.resource, p.action 
            FROM permissions p
            JOIN role_permissions rp ON p.id = rp.permission_id
            WHERE rp.role_id = $1
        "#,
        )
        .bind(role_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let perms: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT p.resource, p.action 
            FROM permissions p
            JOIN role_permissions rp ON p.id = rp.permission_id
            WHERE rp.role_id = ?
        "#,
        )
        .bind(role_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(perms
            .into_iter()
            .map(|(r, a)| format!("{}:{}", r, a))
            .collect())
    }

    /// Get all available permissions
    pub async fn list_permissions(&self) -> Result<Vec<Permission>, sqlx::Error> {
        let perms: Vec<Permission> =
            sqlx::query_as("SELECT * FROM permissions ORDER BY resource, action")
                .fetch_all(&self.pool)
                .await?;
        Ok(perms)
    }

    /// Create a new role
    pub async fn create_role(
        &self,
        tenant_id: Option<&str>,
        dto: CreateRoleDto,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<RoleWithPermissions, sqlx::Error> {
        let now = Utc::now();
        let role_id = Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        {
            sqlx::query(r#"
                INSERT INTO roles (id, tenant_id, name, description, is_system, level, created_at, updated_at)
                VALUES ($1, $2, $3, $4, false, $5, $6, $7)
            "#)
            .bind(&role_id)
            .bind(tenant_id)
            .bind(&dto.name)
            .bind(&dto.description)
            .bind(dto.level.unwrap_or(0))
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await?;
        }

        #[cfg(feature = "sqlite")]
        {
            let now_str = now.to_rfc3339();
            sqlx::query(r#"
                INSERT INTO roles (id, tenant_id, name, description, is_system, level, created_at, updated_at)
                VALUES (?, ?, ?, ?, 0, ?, ?, ?)
            "#)
            .bind(&role_id)
            .bind(tenant_id)
            .bind(&dto.name)
            .bind(&dto.description)
            .bind(dto.level.unwrap_or(0))
            .bind(&now_str)
            .bind(&now_str)
            .execute(&self.pool)
            .await?;
        }

        // Assign permissions
        for perm_key in Self::normalize_permission_keys(dto.permissions.iter().map(String::as_str))
        {
            let parts: Vec<&str> = perm_key.split(':').collect();
            if parts.len() != 2 {
                continue;
            }
            let (resource, action) = (parts[0], parts[1]);

            #[cfg(feature = "postgres")]
            {
                sqlx::query(
                    r#"
                    INSERT INTO role_permissions (role_id, permission_id)
                    SELECT $1, id FROM permissions WHERE resource = $2 AND action = $3
                "#,
                )
                .bind(&role_id)
                .bind(resource)
                .bind(action)
                .execute(&self.pool)
                .await?;
            }

            #[cfg(feature = "sqlite")]
            {
                sqlx::query(
                    r#"
                    INSERT INTO role_permissions (role_id, permission_id)
                    SELECT ?, id FROM permissions WHERE resource = ? AND action = ?
                "#,
                )
                .bind(&role_id)
                .bind(resource)
                .bind(action)
                .execute(&self.pool)
                .await?;
            }
        }

        // Audit
        self.audit_service
            .log(
                actor_id,
                tenant_id,
                "ROLE_CREATE",
                "roles",
                Some(&role_id),
                Some(&format!("Created role {}", dto.name)),
                ip_address,
            )
            .await;

        // Fetch the created role
        let role = self
            .get_role_by_id(&role_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        Ok(role)
    }

    /// Get role by ID
    pub async fn get_role_by_id(
        &self,
        role_id: &str,
    ) -> Result<Option<RoleWithPermissions>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let role: Option<Role> = sqlx::query_as("SELECT * FROM roles WHERE id = $1")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let role: Option<Role> = sqlx::query_as("SELECT * FROM roles WHERE id = ?")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await?;

        match role {
            Some(r) => {
                let permissions = self.get_role_permissions(&r.id).await?;
                Ok(Some(RoleWithPermissions::from_role(r, permissions)))
            }
            None => Ok(None),
        }
    }

    /// Update a role
    pub async fn update_role(
        &self,
        role_id: &str,
        dto: UpdateRoleDto,
        is_super_admin: bool,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<RoleWithPermissions, sqlx::Error> {
        let now = Utc::now();

        // Check if role is system role
        #[cfg(feature = "postgres")]
        let role: Option<Role> = sqlx::query_as("SELECT * FROM roles WHERE id = $1")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let role: Option<Role> = sqlx::query_as("SELECT * FROM roles WHERE id = ?")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await?;

        let role = role.ok_or_else(|| sqlx::Error::RowNotFound)?;
        let role_name_before = role.name.clone();
        let role_description_before = role.description.clone();
        let role_level_before = role.level;

        // Capture existing permissions for diffing (only if caller is changing permissions).
        let existing_permissions: Vec<String> = if dto.permissions.is_some() {
            self.get_role_permissions(role_id).await?
        } else {
            vec![]
        };

        // Only Superadmins can modify system roles
        if role.is_system && !is_super_admin {
            return Err(sqlx::Error::Protocol(
                "System roles can only be modified by Super Admin".to_string(),
            ));
        }

        if role.is_system && !is_super_admin {
            // This branch is now unreachable due to above check, but kept for clarity
        } else {
            if let Some(name) = &dto.name {
                #[cfg(feature = "postgres")]
                sqlx::query("UPDATE roles SET name = $1, updated_at = $2 WHERE id = $3")
                    .bind(name)
                    .bind(now)
                    .bind(role_id)
                    .execute(&self.pool)
                    .await?;

                #[cfg(feature = "sqlite")]
                sqlx::query("UPDATE roles SET name = ?, updated_at = ? WHERE id = ?")
                    .bind(name)
                    .bind(now.to_rfc3339())
                    .bind(role_id)
                    .execute(&self.pool)
                    .await?;
            }

            if let Some(level) = dto.level {
                #[cfg(feature = "postgres")]
                sqlx::query("UPDATE roles SET level = $1, updated_at = $2 WHERE id = $3")
                    .bind(level)
                    .bind(now)
                    .bind(role_id)
                    .execute(&self.pool)
                    .await?;

                #[cfg(feature = "sqlite")]
                sqlx::query("UPDATE roles SET level = ?, updated_at = ? WHERE id = ?")
                    .bind(level)
                    .bind(now.to_rfc3339())
                    .bind(role_id)
                    .execute(&self.pool)
                    .await?;
            }

            if let Some(description) = &dto.description {
                #[cfg(feature = "postgres")]
                sqlx::query("UPDATE roles SET description = $1, updated_at = $2 WHERE id = $3")
                    .bind(description)
                    .bind(now)
                    .bind(role_id)
                    .execute(&self.pool)
                    .await?;

                #[cfg(feature = "sqlite")]
                sqlx::query("UPDATE roles SET description = ?, updated_at = ? WHERE id = ?")
                    .bind(description)
                    .bind(now.to_rfc3339())
                    .bind(role_id)
                    .execute(&self.pool)
                    .await?;
            }
        }

        // Update permissions if provided
        let mut perms_added: Vec<String> = vec![];
        let mut perms_removed: Vec<String> = vec![];
        if let Some(permissions) = &dto.permissions {
            let existing_set: HashSet<String> = existing_permissions
                .into_iter()
                .collect::<HashSet<String>>();
            let requested_set: HashSet<String> =
                Self::normalize_permission_keys(permissions.iter().map(String::as_str))
                    .into_iter()
                    .collect();

            perms_added = requested_set
                .difference(&existing_set)
                .cloned()
                .collect::<Vec<_>>();
            perms_removed = existing_set
                .difference(&requested_set)
                .cloned()
                .collect::<Vec<_>>();
            perms_added.sort();
            perms_removed.sort();

            // Clear existing permissions
            #[cfg(feature = "postgres")]
            sqlx::query("DELETE FROM role_permissions WHERE role_id = $1")
                .bind(role_id)
                .execute(&self.pool)
                .await?;

            #[cfg(feature = "sqlite")]
            sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
                .bind(role_id)
                .execute(&self.pool)
                .await?;

            // Add new permissions
            for perm_key in Self::normalize_permission_keys(permissions.iter().map(String::as_str))
            {
                let parts: Vec<&str> = perm_key.split(':').collect();
                if parts.len() != 2 {
                    continue;
                }
                let (resource, action) = (parts[0], parts[1]);

                #[cfg(feature = "postgres")]
                {
                    sqlx::query(
                        r#"
                        INSERT INTO role_permissions (role_id, permission_id)
                        SELECT $1, id FROM permissions WHERE resource = $2 AND action = $3
                    "#,
                    )
                    .bind(role_id)
                    .bind(resource)
                    .bind(action)
                    .execute(&self.pool)
                    .await?;
                }

                #[cfg(feature = "sqlite")]
                {
                    sqlx::query(
                        r#"
                        INSERT INTO role_permissions (role_id, permission_id)
                        SELECT ?, id FROM permissions WHERE resource = ? AND action = ?
                    "#,
                    )
                    .bind(role_id)
                    .bind(resource)
                    .bind(action)
                    .execute(&self.pool)
                    .await?;
                }
            }
        }

        let role_name_after = dto.name.clone().unwrap_or(role_name_before.clone());
        let role_description_after = dto
            .description
            .clone()
            .or_else(|| role_description_before.clone());
        let role_level_after = dto.level.unwrap_or(role_level_before);

        let details = serde_json::json!({
            "message": "Updated role",
            "name_before": role_name_before,
            "name_after": role_name_after,
            "description_before": role_description_before,
            "description_after": role_description_after,
            "level_before": role_level_before,
            "level_after": role_level_after,
            "perms_added": perms_added,
            "perms_removed": perms_removed
        })
        .to_string();

        // Audit
        self.audit_service
            .log(
                actor_id,
                role.tenant_id.as_deref(),
                "ROLE_UPDATE",
                "roles",
                Some(role_id),
                Some(details.as_str()),
                ip_address,
            )
            .await;

        self.get_role_by_id(role_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    /// Delete a role (system roles can only be deleted by Superadmins)
    pub async fn delete_role(
        &self,
        role_id: &str,
        is_super_admin: bool,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        // Check if system role
        #[cfg(feature = "postgres")]
        let role_info: Option<(bool, Option<String>)> =
            sqlx::query_as("SELECT is_system, tenant_id FROM roles WHERE id = $1")
                .bind(role_id)
                .fetch_optional(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let role_info: Option<(bool, Option<String>)> =
            sqlx::query_as("SELECT is_system, tenant_id FROM roles WHERE id = ?")
                .bind(role_id)
                .fetch_optional(&self.pool)
                .await?;

        if let Some((is_system_role, tid)) = role_info {
            // Only Superadmins can delete system roles
            if is_system_role && !is_super_admin {
                return Ok(false); // Cannot delete system role
            }

            let tenant_id_str = tid.map(|t| t.to_string());

            #[cfg(feature = "postgres")]
            sqlx::query("DELETE FROM roles WHERE id = $1")
                .bind(role_id)
                .execute(&self.pool)
                .await?;

            #[cfg(feature = "sqlite")]
            sqlx::query("DELETE FROM roles WHERE id = ?")
                .bind(role_id)
                .execute(&self.pool)
                .await?;

            // Audit
            self.audit_service
                .log(
                    actor_id,
                    tenant_id_str.as_deref(),
                    "ROLE_DELETE",
                    "roles",
                    Some(role_id),
                    Some("Deleted role"),
                    ip_address,
                )
                .await;

            Ok(true)
        } else {
            // Role not found, treat as success or error?
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RoleService;
    use std::collections::HashSet;

    fn perms(role_name: &str) -> HashSet<&'static str> {
        RoleService::get_role_permission_keys(role_name)
            .unwrap_or_default()
            .into_iter()
            .collect()
    }

    #[test]
    fn technician_role_is_limited_to_installation_focused_access() {
        let technician = perms("Technician");

        assert!(technician.contains("work_orders:manage"));
        assert!(technician.contains("pppoe:manage"));
        assert!(technician.contains("router_inventory:read"));
        assert!(technician.contains("ppp_profiles:read"));
        assert!(technician.contains("ip_pools:read"));
        assert!(!technician.contains("network_noc:read"));
        assert!(!technician.contains("network_alerts:read"));
        assert!(!technician.contains("network_incidents:manage"));
        assert!(!technician.contains("network_logs:read"));
        assert!(!technician.contains("storage_console:read"));
        assert!(!technician.contains("support:read_all"));
    }

    #[test]
    fn noc_role_can_operate_network_without_router_inventory_mutation() {
        let noc = perms("NOC");

        assert!(noc.contains("network_noc:read"));
        assert!(noc.contains("network_alerts:manage"));
        assert!(noc.contains("network_incidents:manage"));
        assert!(noc.contains("network_logs:manage"));
        assert!(noc.contains("ppp_profiles:manage"));
        assert!(noc.contains("ip_pools:manage"));
        assert!(noc.contains("pppoe:manage"));
        assert!(noc.contains("router_inventory:read"));
        assert!(!noc.contains("router_inventory:manage"));
        assert!(!noc.contains("storage_console:read"));
    }

    #[test]
    fn admin_role_receives_full_granular_admin_surface() {
        let admin = perms("Admin");

        assert!(admin.contains("network_noc:manage"));
        assert!(admin.contains("network_alerts:manage"));
        assert!(admin.contains("network_incidents:manage"));
        assert!(admin.contains("network_logs:manage"));
        assert!(admin.contains("router_inventory:manage"));
        assert!(admin.contains("router_inventory:manage_radius_secret"));
        assert!(admin.contains("ppp_profiles:manage"));
        assert!(admin.contains("ip_pools:manage"));
        assert!(admin.contains("storage_console:read"));
        assert!(admin.contains("storage_files:delete"));
        assert!(admin.contains("orders:create"));
    }

    #[test]
    fn customer_service_role_can_create_orders_without_full_customer_manage() {
        let customer_service = perms("Customer Service");

        assert!(customer_service.contains("orders:create"));
        assert!(customer_service.contains("customers:read"));
    }

    #[test]
    fn legacy_permissions_are_normalized_to_granular_keys() {
        let normalized = RoleService::normalize_permission_keys([
            "network_routers:read",
            "storage:read",
            "pppoe:manage",
        ]);
        let normalized = normalized.into_iter().collect::<HashSet<_>>();

        assert!(normalized.contains("network_noc:read"));
        assert!(normalized.contains("router_inventory:read"));
        assert!(normalized.contains("storage_console:read"));
        assert!(normalized.contains("storage_files:read"));
        assert!(normalized.contains("ppp_profiles:manage"));
        assert!(normalized.contains("ip_pools:manage"));
        assert!(normalized.contains("pppoe:manage"));
    }

    #[test]
    fn legacy_permission_catalog_stays_complete_for_migration_cleanup() {
        let legacy = RoleService::legacy_permission_keys()
            .iter()
            .copied()
            .collect::<HashSet<_>>();

        assert!(legacy.contains("network_routers:read"));
        assert!(legacy.contains("network_routers:manage"));
        assert!(legacy.contains("network_routers:manage_radius_secret"));
        assert!(legacy.contains("storage:read"));
        assert!(legacy.contains("storage:upload"));
        assert!(legacy.contains("storage:delete"));
        assert!(legacy.contains("pppoe:read"));
        assert!(legacy.contains("pppoe:manage"));
        assert_eq!(legacy.len(), 8);
    }
}
