use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    AddCustomerPortalUserRequest, CreateCustomerLocationRequest, CreateCustomerPortalUserRequest,
    CreateCustomerRegistrationInviteRequest, CreateCustomerRequest,
    CreateCustomerSubscriptionRequest, CreateCustomerWithPortalRequest,
    CreateMyCustomerLocationRequest, Customer, CustomerLocation, CustomerPortalSubscriptionStats,
    CustomerPortalUser, CustomerRegistrationInviteCreateResponse, CustomerRegistrationInvitePolicy,
    CustomerRegistrationInviteSummary, CustomerRegistrationInviteValidationView,
    CustomerRegistrationInviteView, CustomerSubscription, CustomerSubscriptionView, CustomerUser,
    InstallationWorkOrder, InstallationWorkOrderView, IspPackage, PaginatedResponse,
    PortalCheckoutSubscriptionRequest, TeamMemberWithUser, UpdateCustomerLocationRequest,
    UpdateCustomerRegistrationInvitePolicyRequest, UpdateCustomerRequest,
    UpdateCustomerSubscriptionRequest, WorkOrderRescheduleDecisionRequest,
    WorkOrderRescheduleRequestView,
};
use crate::security::secret::encrypt_secret_for;
use crate::services::{AuditService, AuthService, NotificationService, PppoeService, UserService};
use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use tracing::warn;
use uuid::Uuid;

const PURPOSE_PPPOE: &str = "pppoe_secrets";
const INVITE_DEFAULT_EXPIRES_HOURS: u32 = 24;
const INVITE_DEFAULT_MAX_USES: u32 = 1;
const INVITE_DEFAULT_EXPIRES_KEY: &str = "customer_invite_default_expires_hours";
const INVITE_DEFAULT_MAX_USES_KEY: &str = "customer_invite_default_max_uses";
const CUSTOMER_PACKAGE_INVOICE_PREFIX: &str = "pkgsub:";
const INSTALLATION_SLA_REMINDER_ENABLED_KEY: &str = "installation_sla_reminder_enabled";
const INSTALLATION_SLA_OVERDUE_MINUTES_KEY: &str = "installation_sla_overdue_minutes";
const INSTALLATION_SLA_REMINDER_COOLDOWN_MINUTES_KEY: &str =
    "installation_sla_reminder_cooldown_minutes";
const INSTALLATION_SLA_SCHEDULER_INTERVAL_MINUTES_KEY: &str =
    "installation_sla_scheduler_interval_minutes";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallationSlaBreachType {
    ScheduledOverdue,
    PendingUnscheduled,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct OverdueInstallationReminderRow {
    work_order_id: String,
    status: String,
    scheduled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    customer_name: Option<String>,
    location_label: Option<String>,
    package_name: Option<String>,
}

#[derive(sqlx::FromRow)]
struct InviteSummaryRow {
    total: i64,
    active: i64,
    revoked: i64,
    expired: i64,
    used_up: i64,
    total_uses: i64,
    total_capacity: i64,
    created_last_30d: i64,
    used_last_30d: i64,
}

#[derive(Clone)]
pub struct CustomerService {
    pool: DbPool,
    auth_service: AuthService,
    audit_service: AuditService,
    notification_service: NotificationService,
    pppoe_service: PppoeService,
    user_service: UserService,
}

impl CustomerService {
    async fn get_installation_work_order_row(
        &self,
        tenant_id: &str,
        work_order_id: &str,
    ) -> AppResult<InstallationWorkOrder> {
        #[cfg(feature = "postgres")]
        let row: Option<InstallationWorkOrder> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: Option<InstallationWorkOrder> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = ? AND id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_optional(&self.pool)
        .await?;

        row.ok_or_else(|| AppError::NotFound("Work order not found".to_string()))
    }

    async fn is_actor_admin_or_owner(&self, tenant_id: &str, actor_id: &str) -> AppResult<bool> {
        #[cfg(feature = "postgres")]
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
        .await?;

        #[cfg(feature = "sqlite")]
        let role_name: Option<String> = sqlx::query_scalar(
            r#"
            SELECT LOWER(COALESCE(r.name, tm.role, ''))
            FROM tenant_members tm
            LEFT JOIN roles r ON r.id = tm.role_id
            WHERE tm.tenant_id = ? AND tm.user_id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(matches!(
            role_name.as_deref(),
            Some("owner") | Some("admin")
        ))
    }

    async fn is_installation_assignee_eligible(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> AppResult<bool> {
        #[cfg(feature = "postgres")]
        let eligible: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM tenant_members tm
              JOIN users u ON u.id = tm.user_id
              LEFT JOIN roles r ON r.id = tm.role_id
              WHERE tm.tenant_id = $1
                AND tm.user_id = $2
                AND u.is_active = TRUE
                AND (
                  EXISTS(
                    SELECT 1
                    FROM role_permissions rp
                    JOIN permissions p ON p.id = rp.permission_id
                    WHERE rp.role_id = tm.role_id
                      AND p.resource = 'work_orders'
                      AND p.action = 'manage'
                  )
                  OR LOWER(COALESCE(r.name, tm.role, '')) IN ('owner', 'admin', 'technician', 'teknisi')
                )
            )
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let eligible: bool = {
            let raw: i64 = sqlx::query_scalar(
                r#"
                SELECT EXISTS(
                  SELECT 1
                  FROM tenant_members tm
                  JOIN users u ON u.id = tm.user_id
                  LEFT JOIN roles r ON r.id = tm.role_id
                  WHERE tm.tenant_id = ?
                    AND tm.user_id = ?
                    AND u.is_active = 1
                    AND (
                      EXISTS(
                        SELECT 1
                        FROM role_permissions rp
                        JOIN permissions p ON p.id = rp.permission_id
                        WHERE rp.role_id = tm.role_id
                          AND p.resource = 'work_orders'
                          AND p.action = 'manage'
                      )
                      OR LOWER(COALESCE(r.name, tm.role, '')) IN ('owner', 'admin', 'technician', 'teknisi')
                    )
                )
                "#,
            )
            .bind(tenant_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
            raw != 0
        };

        Ok(eligible)
    }

    pub async fn list_installation_assignees(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<Vec<TeamMemberWithUser>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<TeamMemberWithUser> = sqlx::query_as(
            r#"
            SELECT
              tm.id,
              tm.user_id,
              u.name,
              u.email,
              tm.role,
              tm.role_id,
              r.name AS role_name,
              u.is_active,
              tm.created_at
            FROM tenant_members tm
            JOIN users u ON tm.user_id = u.id
            LEFT JOIN roles r ON tm.role_id = r.id
            WHERE tm.tenant_id = $1
              AND u.is_active = TRUE
              AND (
                EXISTS(
                  SELECT 1
                  FROM role_permissions rp
                  JOIN permissions p ON p.id = rp.permission_id
                  WHERE rp.role_id = tm.role_id
                    AND p.resource = 'work_orders'
                    AND p.action = 'manage'
                )
                OR LOWER(COALESCE(r.name, tm.role, '')) IN ('owner', 'admin', 'technician', 'teknisi')
              )
            ORDER BY LOWER(u.name), LOWER(u.email)
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<TeamMemberWithUser> = sqlx::query_as(
            r#"
            SELECT
              tm.id,
              tm.user_id,
              u.name,
              u.email,
              tm.role,
              tm.role_id,
              r.name AS role_name,
              u.is_active,
              tm.created_at
            FROM tenant_members tm
            JOIN users u ON tm.user_id = u.id
            LEFT JOIN roles r ON tm.role_id = r.id
            WHERE tm.tenant_id = ?
              AND u.is_active = 1
              AND (
                EXISTS(
                  SELECT 1
                  FROM role_permissions rp
                  JOIN permissions p ON p.id = rp.permission_id
                  WHERE rp.role_id = tm.role_id
                    AND p.resource = 'work_orders'
                    AND p.action = 'manage'
                )
                OR LOWER(COALESCE(r.name, tm.role, '')) IN ('owner', 'admin', 'technician', 'teknisi')
              )
            ORDER BY LOWER(u.name), LOWER(u.email)
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    #[cfg(feature = "postgres")]
    async fn ensure_pending_installation_status_supported(&self) -> AppResult<()> {
        let current_def: Option<String> = sqlx::query_scalar(
            r#"
            SELECT pg_get_constraintdef(c.oid)
            FROM pg_constraint c
            WHERE c.conrelid = 'public.customer_subscriptions'::regclass
              AND c.conname = 'customer_subscriptions_status_check'
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(definition) = current_def else {
            return Ok(());
        };
        if definition.contains("pending_installation") {
            return Ok(());
        }

        sqlx::query(
            r#"
            ALTER TABLE public.customer_subscriptions
              DROP CONSTRAINT IF EXISTS customer_subscriptions_status_check
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            ALTER TABLE public.customer_subscriptions
              ADD CONSTRAINT customer_subscriptions_status_check
              CHECK (status IN ('active', 'pending_installation', 'suspended', 'cancelled'))
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub fn new(
        pool: DbPool,
        auth_service: AuthService,
        audit_service: AuditService,
        notification_service: NotificationService,
        pppoe_service: PppoeService,
        user_service: UserService,
    ) -> Self {
        Self {
            pool,
            auth_service,
            audit_service,
            notification_service,
            pppoe_service,
            user_service,
        }
    }

    pub fn start_installation_sla_scheduler(&self) {
        let svc = self.clone();
        tokio::spawn(async move {
            tracing::info!("Installation SLA reminder scheduler started.");
            loop {
                if let Err(err) = svc.run_installation_sla_reminders_for_all_tenants().await {
                    tracing::warn!("installation SLA reminder scheduler failed: {}", err);
                }
                let interval_minutes = svc
                    .resolve_installation_sla_scheduler_interval_minutes()
                    .await;
                let sleep_secs = (interval_minutes.max(5) as u64) * 60;
                tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
            }
        });
    }

    pub async fn run_installation_sla_reminders_for_all_tenants(&self) -> AppResult<u64> {
        if !self.resolve_installation_sla_reminder_enabled().await {
            return Ok(0);
        }

        let overdue_minutes = self.resolve_installation_sla_overdue_minutes().await;
        let unscheduled_minutes = (overdue_minutes * 2).max(120);
        let cooldown_minutes = self
            .resolve_installation_sla_reminder_cooldown_minutes()
            .await;

        #[cfg(feature = "postgres")]
        let tenant_ids: Vec<String> =
            sqlx::query_scalar("SELECT id FROM tenants WHERE is_active = true")
                .fetch_all(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let tenant_ids: Vec<String> =
            sqlx::query_scalar("SELECT id FROM tenants WHERE is_active = 1")
                .fetch_all(&self.pool)
                .await?;

        let mut sent = 0_u64;
        for tenant_id in tenant_ids {
            sent += self
                .run_installation_sla_reminders_for_tenant(
                    &tenant_id,
                    overdue_minutes,
                    unscheduled_minutes,
                    cooldown_minutes,
                )
                .await?;
        }

        Ok(sent)
    }

    async fn get_system_role_id_by_name(&self, name: &str) -> AppResult<String> {
        #[cfg(feature = "postgres")]
        let row: Option<(String,)> =
            sqlx::query_as("SELECT id FROM roles WHERE tenant_id IS NULL AND name = $1")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let row: Option<(String,)> =
            sqlx::query_as("SELECT id FROM roles WHERE tenant_id IS NULL AND name = ?")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;

        row.map(|(id,)| id).ok_or_else(|| {
            AppError::Internal(format!(
                "Missing system role '{}'. Ensure RoleService seeds default roles.",
                name
            ))
        })
    }

    async fn ensure_tenant_member_role(
        &self,
        tenant_id: &str,
        user_id: &str,
        role_id: &str,
    ) -> AppResult<()> {
        // If user already has membership in this tenant, do not overwrite role.
        #[cfg(feature = "postgres")]
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM tenant_members WHERE tenant_id = $1 AND user_id = $2)",
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM tenant_members WHERE tenant_id = ? AND user_id = ?)",
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        if exists {
            return Ok(());
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(user_id)
        .bind("customer")
        .bind(role_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(user_id)
        .bind("customer")
        .bind(role_id)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    fn normalize_billing_cycle(v: &str) -> AppResult<String> {
        let x = v.trim().to_lowercase();
        match x.as_str() {
            "monthly" | "yearly" => Ok(x),
            _ => Err(AppError::Validation(
                "billing_cycle must be monthly or yearly".to_string(),
            )),
        }
    }

    fn normalize_subscription_status(v: &str) -> AppResult<String> {
        let x = v.trim().to_lowercase();
        match x.as_str() {
            "active" | "pending_installation" | "suspended" | "cancelled" => Ok(x),
            _ => Err(AppError::Validation(
                "status must be active, pending_installation, suspended, or cancelled".to_string(),
            )),
        }
    }

    fn normalize_work_order_status(v: &str) -> AppResult<String> {
        let x = v.trim().to_lowercase();
        match x.as_str() {
            "pending" | "in_progress" | "completed" | "cancelled" => Ok(x),
            _ => Err(AppError::Validation(
                "status must be pending, in_progress, completed, or cancelled".to_string(),
            )),
        }
    }

    fn parse_setting_bool(raw: Option<String>, default_value: bool) -> bool {
        let Some(value) = raw else {
            return default_value;
        };
        match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default_value,
        }
    }

    fn parse_setting_i64(raw: Option<String>, default_value: i64, min: i64, max: i64) -> i64 {
        raw.and_then(|v| v.trim().parse::<i64>().ok())
            .map(|v| v.clamp(min, max))
            .unwrap_or(default_value)
    }

    fn detect_installation_sla_breach(
        status: &str,
        scheduled_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        now: DateTime<Utc>,
        overdue_minutes: i64,
        unscheduled_minutes: i64,
    ) -> Option<InstallationSlaBreachType> {
        let normalized_status = status.trim().to_ascii_lowercase();
        if normalized_status != "pending" && normalized_status != "in_progress" {
            return None;
        }

        let scheduled_cutoff = now - Duration::minutes(overdue_minutes.max(1));
        if let Some(schedule_at) = scheduled_at {
            if schedule_at <= scheduled_cutoff {
                return Some(InstallationSlaBreachType::ScheduledOverdue);
            }
        }

        if normalized_status == "pending" && scheduled_at.is_none() {
            let unscheduled_cutoff = now - Duration::minutes(unscheduled_minutes.max(1));
            if created_at <= unscheduled_cutoff {
                return Some(InstallationSlaBreachType::PendingUnscheduled);
            }
        }

        None
    }

    fn format_elapsed_duration(minutes: i64) -> String {
        let total_minutes = minutes.max(0);
        if total_minutes < 60 {
            return format!("{}m", total_minutes);
        }

        let hours = total_minutes / 60;
        let rem_minutes = total_minutes % 60;
        if hours < 24 {
            if rem_minutes == 0 {
                return format!("{}h", hours);
            }
            return format!("{}h {}m", hours, rem_minutes);
        }

        let days = hours / 24;
        let rem_hours = hours % 24;
        if rem_hours == 0 {
            return format!("{}d", days);
        }
        format!("{}d {}h", days, rem_hours)
    }

    fn is_owner_admin_or_technician_role(role: Option<&str>) -> bool {
        role.map(|r| {
            let normalized = r.trim().to_ascii_lowercase();
            normalized == "owner" || normalized == "admin" || normalized == "technician"
        })
        .unwrap_or(false)
    }

    fn filter_installation_request_user_ids(rows: Vec<(String, Option<String>)>) -> Vec<String> {
        let mut set = HashSet::new();
        for (user_id, role) in rows {
            if Self::is_owner_admin_or_technician_role(role.as_deref()) {
                set.insert(user_id);
            }
        }
        set.into_iter().collect()
    }

    fn merge_work_order_notes(
        existing: Option<String>,
        actor_id: &str,
        note: Option<&str>,
    ) -> Option<String> {
        let mut out = existing.unwrap_or_default();
        let incoming = note.unwrap_or("").trim();
        if incoming.is_empty() {
            return if out.trim().is_empty() {
                None
            } else {
                Some(out)
            };
        }

        if !out.trim().is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&format!(
            "[{}] {}: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            actor_id,
            incoming
        ));
        Some(out)
    }

    fn parse_optional_datetime(input: Option<String>) -> AppResult<Option<DateTime<Utc>>> {
        let Some(raw) = input else {
            return Ok(None);
        };
        let v = raw.trim();
        if v.is_empty() {
            return Ok(None);
        }

        if let Ok(dt) = DateTime::parse_from_rfc3339(v) {
            return Ok(Some(dt.with_timezone(&Utc)));
        }

        if let Ok(d) = chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d") {
            if let Some(ndt) = d.and_hms_opt(0, 0, 0) {
                return Ok(Some(DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)));
            }
        }

        Err(AppError::Validation(
            "Invalid date format. Use RFC3339 or YYYY-MM-DD".to_string(),
        ))
    }

    fn hash_registration_invite_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn build_registration_invite_token() -> String {
        format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
    }

    fn parse_invite_policy_u32(raw: Option<String>, default_value: u32, min: u32, max: u32) -> u32 {
        raw.and_then(|v| v.trim().parse::<u32>().ok())
            .map(|v| v.clamp(min, max))
            .unwrap_or(default_value)
    }

    async fn read_tenant_setting_value(
        &self,
        tenant_id: &str,
        key: &str,
    ) -> AppResult<Option<String>> {
        #[cfg(feature = "postgres")]
        let value: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE tenant_id = $1 AND key = $2 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let value: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE tenant_id = ? AND key = ? LIMIT 1",
        )
        .bind(tenant_id)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(value)
    }

    async fn read_global_setting_value(&self, key: &str) -> AppResult<Option<String>> {
        #[cfg(feature = "postgres")]
        let value: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE tenant_id IS NULL AND key = $1 LIMIT 1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let value: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE tenant_id IS NULL AND key = ? LIMIT 1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(value)
    }

    async fn resolve_installation_sla_reminder_enabled(&self) -> bool {
        let raw = self
            .read_global_setting_value(INSTALLATION_SLA_REMINDER_ENABLED_KEY)
            .await
            .ok()
            .flatten();
        Self::parse_setting_bool(raw, true)
    }

    async fn resolve_installation_sla_overdue_minutes(&self) -> i64 {
        let raw = self
            .read_global_setting_value(INSTALLATION_SLA_OVERDUE_MINUTES_KEY)
            .await
            .ok()
            .flatten();
        Self::parse_setting_i64(raw, 120, 15, 7 * 24 * 60)
    }

    async fn resolve_installation_sla_reminder_cooldown_minutes(&self) -> i64 {
        let raw = self
            .read_global_setting_value(INSTALLATION_SLA_REMINDER_COOLDOWN_MINUTES_KEY)
            .await
            .ok()
            .flatten();
        Self::parse_setting_i64(raw, 180, 15, 7 * 24 * 60)
    }

    async fn resolve_installation_sla_scheduler_interval_minutes(&self) -> i64 {
        let default_global = self
            .read_global_setting_value(INSTALLATION_SLA_SCHEDULER_INTERVAL_MINUTES_KEY)
            .await
            .ok()
            .flatten();
        let default_global = Self::parse_setting_i64(default_global, 15, 5, 24 * 60);

        #[cfg(feature = "postgres")]
        let tenant_values: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT s.value
            FROM settings s
            INNER JOIN tenants t ON t.id = s.tenant_id
            WHERE s.key = $1
              AND t.is_active = true
            "#,
        )
        .bind(INSTALLATION_SLA_SCHEDULER_INTERVAL_MINUTES_KEY)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        #[cfg(feature = "sqlite")]
        let tenant_values: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT s.value
            FROM settings s
            INNER JOIN tenants t ON t.id = s.tenant_id
            WHERE s.key = ?
              AND t.is_active = 1
            "#,
        )
        .bind(INSTALLATION_SLA_SCHEDULER_INTERVAL_MINUTES_KEY)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        tenant_values
            .into_iter()
            .filter_map(|v| v.parse::<i64>().ok())
            .map(|v| v.clamp(5, 24 * 60))
            .min()
            .unwrap_or(default_global)
    }

    async fn upsert_tenant_setting_value(
        &self,
        tenant_id: &str,
        key: &str,
        value: &str,
        description: &str,
    ) -> AppResult<()> {
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let update_res = sqlx::query(
            "UPDATE settings SET value = $1, description = $2, updated_at = $3 WHERE tenant_id = $4 AND key = $5",
        )
        .bind(value)
        .bind(description)
        .bind(now)
        .bind(tenant_id)
        .bind(key)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let update_res = sqlx::query(
            "UPDATE settings SET value = ?, description = ?, updated_at = ? WHERE tenant_id = ? AND key = ?",
        )
        .bind(value)
        .bind(description)
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(key)
        .execute(&self.pool)
        .await?;

        if update_res.rows_affected() == 0 {
            let id = Uuid::new_v4().to_string();

            #[cfg(feature = "postgres")]
            sqlx::query(
                "INSERT INTO settings (id, tenant_id, key, value, description, created_at, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$6)",
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(key)
            .bind(value)
            .bind(description)
            .bind(now)
            .execute(&self.pool)
            .await?;

            #[cfg(feature = "sqlite")]
            sqlx::query(
                "INSERT INTO settings (id, tenant_id, key, value, description, created_at, updated_at) VALUES (?,?,?,?,?,?,?)",
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(key)
            .bind(value)
            .bind(description)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn resolve_invite_policy_for_tenant(
        &self,
        tenant_id: &str,
    ) -> AppResult<CustomerRegistrationInvitePolicy> {
        let expires_raw = self
            .read_tenant_setting_value(tenant_id, INVITE_DEFAULT_EXPIRES_KEY)
            .await?;
        let max_uses_raw = self
            .read_tenant_setting_value(tenant_id, INVITE_DEFAULT_MAX_USES_KEY)
            .await?;

        Ok(CustomerRegistrationInvitePolicy {
            default_expires_in_hours: Self::parse_invite_policy_u32(
                expires_raw,
                INVITE_DEFAULT_EXPIRES_HOURS,
                1,
                24 * 30,
            ),
            default_max_uses: Self::parse_invite_policy_u32(
                max_uses_raw,
                INVITE_DEFAULT_MAX_USES,
                1,
                100,
            ),
        })
    }

    fn build_auto_pppoe_username(
        customer_name: &str,
        customer_id: &str,
        location_id: &str,
    ) -> String {
        let mut slug = String::new();
        for ch in customer_name.trim().chars() {
            if ch.is_ascii_alphanumeric() {
                slug.push(ch.to_ascii_lowercase());
            } else if (ch.is_ascii_whitespace() || ch == '-' || ch == '_')
                && !slug.ends_with('-')
                && !slug.is_empty()
            {
                slug.push('-');
            }
            if slug.len() >= 14 {
                break;
            }
        }
        let slug = slug.trim_matches('-');
        let base = if slug.is_empty() { "cust" } else { slug };
        let c4 = customer_id.chars().rev().take(4).collect::<String>();
        let l4 = location_id.chars().rev().take(4).collect::<String>();
        format!(
            "{}-{}{}",
            base,
            c4.chars().rev().collect::<String>(),
            l4.chars().rev().collect::<String>()
        )
    }

    async fn auto_provision_pppoe_for_subscription(
        &self,
        actor_id: &str,
        tenant_id: &str,
        sub: &CustomerSubscription,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        if sub.status != "active" {
            return Ok(());
        }
        let Some(router_id) = sub.router_id.as_deref() else {
            return Ok(());
        };
        if router_id.trim().is_empty() {
            return Ok(());
        }

        #[derive(sqlx::FromRow)]
        struct MappingRow {
            router_profile_name: String,
            address_pool: Option<String>,
        }

        #[cfg(feature = "postgres")]
        let mapping: Option<MappingRow> = sqlx::query_as(
            r#"
            SELECT router_profile_name, address_pool
            FROM isp_package_router_mappings
            WHERE tenant_id = $1 AND router_id = $2 AND package_id = $3
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(&sub.package_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let mapping: Option<MappingRow> = sqlx::query_as(
            r#"
            SELECT router_profile_name, address_pool
            FROM isp_package_router_mappings
            WHERE tenant_id = ? AND router_id = ? AND package_id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(&sub.package_id)
        .fetch_optional(&self.pool)
        .await?;

        let mapping = mapping.ok_or_else(|| {
            AppError::Validation(
                "PPPoE auto-provision requires package mapping (router profile) for selected router"
                    .to_string(),
            )
        })?;

        #[cfg(feature = "postgres")]
        let customer_name: String =
            sqlx::query_scalar("SELECT name FROM customers WHERE tenant_id = $1 AND id = $2")
                .bind(tenant_id)
                .bind(&sub.customer_id)
                .fetch_optional(&self.pool)
                .await?
                .unwrap_or_else(|| "customer".to_string());

        #[cfg(feature = "sqlite")]
        let customer_name: String =
            sqlx::query_scalar("SELECT name FROM customers WHERE tenant_id = ? AND id = ?")
                .bind(tenant_id)
                .bind(&sub.customer_id)
                .fetch_optional(&self.pool)
                .await?
                .unwrap_or_else(|| "customer".to_string());

        let username =
            Self::build_auto_pppoe_username(&customer_name, &sub.customer_id, &sub.location_id);

        #[cfg(feature = "postgres")]
        let username_conflict: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1 FROM pppoe_accounts
              WHERE tenant_id = $1
                AND username = $2
                AND (customer_id <> $3 OR location_id <> $4 OR router_id <> $5)
            )
            "#,
        )
        .bind(tenant_id)
        .bind(&username)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(router_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let username_conflict: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1 FROM pppoe_accounts
              WHERE tenant_id = ?
                AND username = ?
                AND (customer_id <> ? OR location_id <> ? OR router_id <> ?)
            )
            "#,
        )
        .bind(tenant_id)
        .bind(&username)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(router_id)
        .fetch_one(&self.pool)
        .await?;

        if username_conflict {
            return Err(AppError::Validation(format!(
                "PPPoE username conflict detected across tenant routers: {}",
                username
            )));
        }

        #[derive(sqlx::FromRow)]
        struct ExistingPppoe {
            id: String,
        }

        #[cfg(feature = "postgres")]
        let existing: Option<ExistingPppoe> = sqlx::query_as(
            r#"
            SELECT id FROM pppoe_accounts
            WHERE tenant_id = $1
              AND customer_id = $2
              AND location_id = $3
              AND router_id = $4
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(router_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let existing: Option<ExistingPppoe> = sqlx::query_as(
            r#"
            SELECT id FROM pppoe_accounts
            WHERE tenant_id = ?
              AND customer_id = ?
              AND location_id = ?
              AND router_id = ?
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(router_id)
        .fetch_optional(&self.pool)
        .await?;

        let now = Utc::now();
        let note = format!(
            "Auto-provisioned from active subscription {}. Pending apply.",
            sub.id
        );

        if let Some(ex) = existing {
            #[cfg(feature = "postgres")]
            sqlx::query(
                r#"
                UPDATE pppoe_accounts
                SET username = $1,
                    package_id = $2,
                    router_profile_name = $3,
                    remote_address = NULL,
                    address_pool = $4,
                    disabled = true,
                    comment = $5,
                    updated_at = $6
                WHERE tenant_id = $7 AND id = $8
                "#,
            )
            .bind(&username)
            .bind(&sub.package_id)
            .bind(&mapping.router_profile_name)
            .bind(&mapping.address_pool)
            .bind(&note)
            .bind(now)
            .bind(tenant_id)
            .bind(&ex.id)
            .execute(&self.pool)
            .await?;

            #[cfg(feature = "sqlite")]
            sqlx::query(
                r#"
                UPDATE pppoe_accounts
                SET username = ?,
                    package_id = ?,
                    router_profile_name = ?,
                    remote_address = NULL,
                    address_pool = ?,
                    disabled = 1,
                    comment = ?,
                    updated_at = ?
                WHERE tenant_id = ? AND id = ?
                "#,
            )
            .bind(&username)
            .bind(&sub.package_id)
            .bind(&mapping.router_profile_name)
            .bind(&mapping.address_pool)
            .bind(&note)
            .bind(now)
            .bind(tenant_id)
            .bind(&ex.id)
            .execute(&self.pool)
            .await?;
        } else {
            let pwd_seed = Uuid::new_v4().simple().to_string();
            let password_raw = format!("Pppoe#{}", &pwd_seed[..10]);
            let password_enc = encrypt_secret_for(PURPOSE_PPPOE, &password_raw)?;
            let id = Uuid::new_v4().to_string();

            #[cfg(feature = "postgres")]
            sqlx::query(
                r#"
                INSERT INTO pppoe_accounts
                  (id, tenant_id, router_id, customer_id, location_id, username, password_enc, package_id, profile_id, router_profile_name,
                   remote_address, address_pool, disabled, comment, router_present, router_secret_id, last_sync_at, last_error, created_at, updated_at)
                VALUES
                  ($1,$2,$3,$4,$5,$6,$7,$8,NULL,$9,NULL,$10,true,$11,false,NULL,NULL,NULL,$12,$13)
                "#,
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(router_id)
            .bind(&sub.customer_id)
            .bind(&sub.location_id)
            .bind(&username)
            .bind(&password_enc)
            .bind(&sub.package_id)
            .bind(&mapping.router_profile_name)
            .bind(&mapping.address_pool)
            .bind(&note)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await?;

            #[cfg(feature = "sqlite")]
            sqlx::query(
                r#"
                INSERT INTO pppoe_accounts
                  (id, tenant_id, router_id, customer_id, location_id, username, password_enc, package_id, profile_id, router_profile_name,
                   remote_address, address_pool, disabled, comment, router_present, router_secret_id, last_sync_at, last_error, created_at, updated_at)
                VALUES
                  (?,?,?,?,?,?,?,?,NULL,?,NULL,?,1,?,0,NULL,NULL,NULL,?,?)
                "#,
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(router_id)
            .bind(&sub.customer_id)
            .bind(&sub.location_id)
            .bind(&username)
            .bind(&password_enc)
            .bind(&sub.package_id)
            .bind(&mapping.router_profile_name)
            .bind(&mapping.address_pool)
            .bind(&note)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await?;
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "PPPOE_AUTO_PROVISION",
                "pppoe",
                Some(&sub.id),
                Some("Auto provisioned PPPoE draft from active subscription"),
                ip_address,
            )
            .await;

        Ok(())
    }

    // =========================
    // Admin: Customers
    // =========================

    pub async fn list_customers(
        &self,
        actor_id: &str,
        tenant_id: &str,
        q: Option<String>,
        page: u32,
        per_page: u32,
    ) -> AppResult<PaginatedResponse<Customer>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read")
            .await?;

        let q = q.unwrap_or_default().trim().to_string();
        let offset = (page.saturating_sub(1)) * per_page;

        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT
                c.*,
                COUNT(*) OVER() AS total_count
            FROM customers c
            WHERE c.tenant_id = $1
              AND ($2 = '' OR c.name ILIKE '%' || $2 || '%' OR c.email ILIKE '%' || $2 || '%')
            ORDER BY c.created_at DESC
            LIMIT $3 OFFSET $4
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT
                c.*,
                (SELECT COUNT(*) FROM customers cc WHERE cc.tenant_id = ? AND (? = '' OR cc.name LIKE '%' || ? || '%' OR cc.email LIKE '%' || ? || '%')) AS total_count
            FROM customers c
            WHERE c.tenant_id = ?
              AND (? = '' OR c.name LIKE '%' || ? || '%' OR c.email LIKE '%' || ? || '%')
            ORDER BY c.created_at DESC
            LIMIT ? OFFSET ?
        "#;

        #[derive(sqlx::FromRow)]
        struct Row {
            #[sqlx(flatten)]
            customer: Customer,
            total_count: i64,
        }

        #[cfg(feature = "postgres")]
        let rows: Vec<Row> = sqlx::query_as(query)
            .bind(tenant_id)
            .bind(&q)
            .bind(per_page as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<Row> = sqlx::query_as(query)
            .bind(tenant_id)
            .bind(&q)
            .bind(&q)
            .bind(&q)
            .bind(tenant_id)
            .bind(&q)
            .bind(&q)
            .bind(&q)
            .bind(per_page as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await?;

        let total = rows.first().map(|r| r.total_count).unwrap_or(0);
        Ok(PaginatedResponse {
            data: rows.into_iter().map(|r| r.customer).collect(),
            total,
            page,
            per_page,
        })
    }

    pub async fn get_customer(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
    ) -> AppResult<Customer> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read")
            .await?;

        #[cfg(feature = "postgres")]
        let customer: Option<Customer> =
            sqlx::query_as("SELECT * FROM customers WHERE tenant_id = $1 AND id = $2")
                .bind(tenant_id)
                .bind(customer_id)
                .fetch_optional(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let customer: Option<Customer> =
            sqlx::query_as("SELECT * FROM customers WHERE tenant_id = ? AND id = ?")
                .bind(tenant_id)
                .bind(customer_id)
                .fetch_optional(&self.pool)
                .await?;

        customer.ok_or_else(|| AppError::NotFound("Customer not found".to_string()))
    }

    pub async fn create_customer(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateCustomerRequest,
        ip_address: Option<&str>,
    ) -> AppResult<Customer> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let customer = Customer::new(
            tenant_id.to_string(),
            dto.name,
            dto.email,
            dto.phone,
            dto.notes,
            dto.is_active,
        );

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customers
                (id, tenant_id, name, email, phone, notes, is_active, created_at, updated_at)
            VALUES
                ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(&customer.id)
        .bind(&customer.tenant_id)
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.phone)
        .bind(&customer.notes)
        .bind(customer.is_active)
        .bind(customer.created_at)
        .bind(customer.updated_at)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO customers
                (id, tenant_id, name, email, phone, notes, is_active, created_at, updated_at)
            VALUES
                (?,?,?,?,?,?,?,?,?)
            "#,
        )
        .bind(&customer.id)
        .bind(&customer.tenant_id)
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.phone)
        .bind(&customer.notes)
        .bind(customer.is_active)
        .bind(customer.created_at.to_rfc3339())
        .bind(customer.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_CREATE",
                "customers",
                Some(&customer.id),
                Some(&format!("Created customer {}", customer.name)),
                ip_address,
            )
            .await;

        Ok(customer)
    }

    pub async fn create_customer_with_portal(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateCustomerWithPortalRequest,
        ip_address: Option<&str>,
    ) -> AppResult<Customer> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let portal_email = dto.portal_email.trim().to_lowercase();
        if portal_email.is_empty() {
            return Err(AppError::Validation("portal_email is required".to_string()));
        }
        if dto.portal_password.trim().len() < 6 {
            return Err(AppError::Validation(
                "portal_password must be at least 6 characters".to_string(),
            ));
        }

        let customer = Customer::new(
            tenant_id.to_string(),
            dto.name,
            dto.email,
            dto.phone,
            dto.notes,
            dto.is_active,
        );

        let portal_user_name = dto
            .portal_name
            .unwrap_or_else(|| customer.name.clone())
            .trim()
            .to_string();
        if portal_user_name.is_empty() {
            return Err(AppError::Validation("portal_name is required".to_string()));
        }

        let user_id = Uuid::new_v4().to_string();
        let customer_user_id = Uuid::new_v4().to_string();
        let tenant_member_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let role_id = self.get_system_role_id_by_name("Customer").await?;
        let password_hash = AuthService::hash_password(&dto.portal_password)?;

        let mut tx = self.pool.begin().await?;
        self.auth_service
            .apply_rls_context_tx_values(&mut tx, Some(tenant_id), Some(actor_id), false)
            .await?;

        #[cfg(feature = "postgres")]
        {
            let existing: Option<String> =
                sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
                    .bind(&portal_email)
                    .fetch_optional(&mut *tx)
                    .await?;
            if existing.is_some() {
                return Err(AppError::UserAlreadyExists);
            }

            sqlx::query(
                r#"
                INSERT INTO customers
                    (id, tenant_id, name, email, phone, notes, is_active, created_at, updated_at)
                VALUES
                    ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                "#,
            )
            .bind(&customer.id)
            .bind(&customer.tenant_id)
            .bind(&customer.name)
            .bind(&customer.email)
            .bind(&customer.phone)
            .bind(&customer.notes)
            .bind(customer.is_active)
            .bind(customer.created_at)
            .bind(customer.updated_at)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO users (id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(&user_id)
            .bind(&portal_email)
            .bind(&password_hash)
            .bind(&portal_user_name)
            .bind("user")
            .bind(false)
            .bind(true)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO customer_users (id, tenant_id, customer_id, user_id, created_at) VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(&customer_user_id)
            .bind(tenant_id)
            .bind(&customer.id)
            .bind(&user_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&tenant_member_id)
            .bind(tenant_id)
            .bind(&user_id)
            .bind("customer")
            .bind(&role_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }

        #[cfg(feature = "sqlite")]
        {
            let existing: Option<String> =
                sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
                    .bind(&portal_email)
                    .fetch_optional(&mut *tx)
                    .await?;
            if existing.is_some() {
                return Err(AppError::UserAlreadyExists);
            }

            sqlx::query(
                r#"
                INSERT INTO customers
                    (id, tenant_id, name, email, phone, notes, is_active, created_at, updated_at)
                VALUES
                    (?,?,?,?,?,?,?,?,?)
                "#,
            )
            .bind(&customer.id)
            .bind(&customer.tenant_id)
            .bind(&customer.name)
            .bind(&customer.email)
            .bind(&customer.phone)
            .bind(&customer.notes)
            .bind(customer.is_active)
            .bind(customer.created_at.to_rfc3339())
            .bind(customer.updated_at.to_rfc3339())
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO users (id, email, password_hash, name, role, is_super_admin, is_active, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&user_id)
            .bind(&portal_email)
            .bind(&password_hash)
            .bind(&portal_user_name)
            .bind("user")
            .bind(false)
            .bind(true)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO customer_users (id, tenant_id, customer_id, user_id, created_at) VALUES (?,?,?,?,?)",
            )
            .bind(&customer_user_id)
            .bind(tenant_id)
            .bind(&customer.id)
            .bind(&user_id)
            .bind(now.to_rfc3339())
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(&tenant_member_id)
            .bind(tenant_id)
            .bind(&user_id)
            .bind("customer")
            .bind(&role_id)
            .bind(now.to_rfc3339())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_CREATE",
                "customers",
                Some(&customer.id),
                Some(&format!("Created customer {}", customer.name)),
                ip_address,
            )
            .await;
        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_USER_CREATE",
                "customer_users",
                Some(&customer_user_id),
                Some("Created portal login while creating customer"),
                ip_address,
            )
            .await;

        Ok(customer)
    }

    pub async fn create_customer_from_public_registration(
        &self,
        tenant_id: &str,
        user_id: &str,
        customer_name: &str,
        customer_email: &str,
        ip_address: Option<&str>,
    ) -> AppResult<Customer> {
        let name = customer_name.trim().to_string();
        if name.len() < 2 {
            return Err(AppError::Validation(
                "Customer name must be at least 2 characters".to_string(),
            ));
        }
        let email = customer_email.trim().to_lowercase();
        if email.is_empty() {
            return Err(AppError::Validation(
                "Customer email is required".to_string(),
            ));
        }

        #[cfg(feature = "postgres")]
        let existing_customer: Option<Customer> = sqlx::query_as(
            r#"
            SELECT c.*
            FROM customers c
            JOIN customer_users cu ON cu.customer_id = c.id AND cu.tenant_id = c.tenant_id
            WHERE cu.tenant_id = $1 AND cu.user_id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let existing_customer: Option<Customer> = sqlx::query_as(
            r#"
            SELECT c.*
            FROM customers c
            JOIN customer_users cu ON cu.customer_id = c.id AND cu.tenant_id = c.tenant_id
            WHERE cu.tenant_id = ? AND cu.user_id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(existing) = existing_customer {
            return Ok(existing);
        }

        let customer = Customer::new(
            tenant_id.to_string(),
            name,
            Some(email),
            None,
            None,
            Some(true),
        );
        let customer_user_id = Uuid::new_v4().to_string();
        let tenant_member_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let role_id = self.get_system_role_id_by_name("Customer").await?;

        let mut tx = self.pool.begin().await?;
        self.auth_service
            .apply_rls_context_tx_values(&mut tx, Some(tenant_id), Some(user_id), false)
            .await?;

        #[cfg(feature = "postgres")]
        {
            sqlx::query(
                r#"
                INSERT INTO customers
                    (id, tenant_id, name, email, phone, notes, is_active, created_at, updated_at)
                VALUES
                    ($1,$2,$3,$4,$5,$6,$7,$8,$9)
                "#,
            )
            .bind(&customer.id)
            .bind(&customer.tenant_id)
            .bind(&customer.name)
            .bind(&customer.email)
            .bind(&customer.phone)
            .bind(&customer.notes)
            .bind(customer.is_active)
            .bind(customer.created_at)
            .bind(customer.updated_at)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO customer_users (id, tenant_id, customer_id, user_id, created_at) VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(&customer_user_id)
            .bind(tenant_id)
            .bind(&customer.id)
            .bind(user_id)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            let member_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM tenant_members WHERE tenant_id = $1 AND user_id = $2)",
            )
            .bind(tenant_id)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

            if !member_exists {
                sqlx::query(
                    "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(&tenant_member_id)
                .bind(tenant_id)
                .bind(user_id)
                .bind("customer")
                .bind(&role_id)
                .bind(now)
                .execute(&mut *tx)
                .await?;
            }
        }

        #[cfg(feature = "sqlite")]
        {
            sqlx::query(
                r#"
                INSERT INTO customers
                    (id, tenant_id, name, email, phone, notes, is_active, created_at, updated_at)
                VALUES
                    (?,?,?,?,?,?,?,?,?)
                "#,
            )
            .bind(&customer.id)
            .bind(&customer.tenant_id)
            .bind(&customer.name)
            .bind(&customer.email)
            .bind(&customer.phone)
            .bind(&customer.notes)
            .bind(customer.is_active)
            .bind(customer.created_at.to_rfc3339())
            .bind(customer.updated_at.to_rfc3339())
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO customer_users (id, tenant_id, customer_id, user_id, created_at) VALUES (?,?,?,?,?)",
            )
            .bind(&customer_user_id)
            .bind(tenant_id)
            .bind(&customer.id)
            .bind(user_id)
            .bind(now.to_rfc3339())
            .execute(&mut *tx)
            .await?;

            let member_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM tenant_members WHERE tenant_id = ? AND user_id = ?)",
            )
            .bind(tenant_id)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

            if !member_exists {
                sqlx::query(
                    "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(&tenant_member_id)
                .bind(tenant_id)
                .bind(user_id)
                .bind("customer")
                .bind(&role_id)
                .bind(now.to_rfc3339())
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        self.audit_service
            .log(
                Some(user_id),
                Some(tenant_id),
                "CUSTOMER_SELF_REGISTER",
                "customers",
                Some(&customer.id),
                Some("Created customer via custom-domain public registration"),
                ip_address,
            )
            .await;
        self.audit_service
            .log(
                Some(user_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_USER_CREATE",
                "customer_users",
                Some(&customer_user_id),
                Some("Linked self-registered user as customer portal user"),
                ip_address,
            )
            .await;

        Ok(customer)
    }

    pub async fn create_customer_registration_invite(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateCustomerRegistrationInviteRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerRegistrationInviteCreateResponse> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let policy = self.resolve_invite_policy_for_tenant(tenant_id).await?;
        let expires_in_hours = dto
            .expires_in_hours
            .unwrap_or(policy.default_expires_in_hours)
            .clamp(1, 24 * 30);
        let max_uses = dto
            .max_uses
            .unwrap_or(policy.default_max_uses)
            .clamp(1, 100);
        let note = dto.note.and_then(|v| {
            let trimmed = v.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.chars().take(500).collect::<String>())
            }
        });

        #[cfg(feature = "postgres")]
        let tenant_domain: Option<Option<String>> = sqlx::query_scalar(
            "SELECT custom_domain FROM tenants WHERE id = $1 AND is_active = true",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let tenant_domain: Option<Option<String>> =
            sqlx::query_scalar("SELECT custom_domain FROM tenants WHERE id = ? AND is_active = 1")
                .bind(tenant_id)
                .fetch_optional(&self.pool)
                .await?;

        let tenant_domain = tenant_domain.flatten();

        let domain = tenant_domain
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                AppError::Validation(
                    "Tenant custom domain is required before generating customer invite link"
                        .to_string(),
                )
            })?;

        let now = Utc::now();
        let expires_at = now + chrono::Duration::hours(expires_in_hours as i64);
        let invite_token = Self::build_registration_invite_token();
        let token_hash = Self::hash_registration_invite_token(&invite_token);
        let invite_id = Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customer_registration_invites
                (id, tenant_id, token_hash, created_by, max_uses, used_count, expires_at, is_revoked, revoked_at, last_used_at, note, created_at)
            VALUES
                ($1,$2,$3,$4,$5,0,$6,false,NULL,NULL,$7,$8)
            "#,
        )
        .bind(&invite_id)
        .bind(tenant_id)
        .bind(&token_hash)
        .bind(actor_id)
        .bind(max_uses as i64)
        .bind(expires_at)
        .bind(&note)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO customer_registration_invites
                (id, tenant_id, token_hash, created_by, max_uses, used_count, expires_at, is_revoked, revoked_at, last_used_at, note, created_at)
            VALUES
                (?,?,?,?,?,0,?,0,NULL,NULL,?,?)
            "#,
        )
        .bind(&invite_id)
        .bind(tenant_id)
        .bind(&token_hash)
        .bind(actor_id)
        .bind(max_uses as i64)
        .bind(expires_at.to_rfc3339())
        .bind(&note)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        let invite = CustomerRegistrationInviteView {
            id: invite_id.clone(),
            tenant_id: tenant_id.to_string(),
            created_by: Some(actor_id.to_string()),
            max_uses: max_uses as i64,
            used_count: 0,
            expires_at,
            is_revoked: false,
            revoked_at: None,
            last_used_at: None,
            note,
            created_at: now,
        };
        let invite_url = format!("https://{domain}/register?invite={invite_token}");

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_INVITE_CREATE",
                "customer_registration_invites",
                Some(&invite_id),
                Some(&format!(
                    "Generated customer registration invite (expires in {}h, max uses {})",
                    expires_in_hours, max_uses
                )),
                ip_address,
            )
            .await;

        Ok(CustomerRegistrationInviteCreateResponse {
            invite,
            invite_token,
            invite_url,
        })
    }

    pub async fn get_customer_registration_invite_policy(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<CustomerRegistrationInvitePolicy> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;
        self.resolve_invite_policy_for_tenant(tenant_id).await
    }

    pub async fn update_customer_registration_invite_policy(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: UpdateCustomerRegistrationInvitePolicyRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerRegistrationInvitePolicy> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let current = self.resolve_invite_policy_for_tenant(tenant_id).await?;
        let expires_in_hours = dto
            .default_expires_in_hours
            .unwrap_or(current.default_expires_in_hours)
            .clamp(1, 24 * 30);
        let max_uses = dto
            .default_max_uses
            .unwrap_or(current.default_max_uses)
            .clamp(1, 100);

        self.upsert_tenant_setting_value(
            tenant_id,
            INVITE_DEFAULT_EXPIRES_KEY,
            &expires_in_hours.to_string(),
            "Default invite expiry (hours) for customer registration links",
        )
        .await?;
        self.upsert_tenant_setting_value(
            tenant_id,
            INVITE_DEFAULT_MAX_USES_KEY,
            &max_uses.to_string(),
            "Default max uses for customer registration invite links",
        )
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_INVITE_POLICY_UPDATE",
                "settings",
                None,
                Some(&format!(
                    "Updated customer invite policy defaults (expires={}h, max_uses={})",
                    expires_in_hours, max_uses
                )),
                ip_address,
            )
            .await;

        Ok(CustomerRegistrationInvitePolicy {
            default_expires_in_hours: expires_in_hours,
            default_max_uses: max_uses,
        })
    }

    pub async fn summarize_customer_registration_invites(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<CustomerRegistrationInviteSummary> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let now = Utc::now();
        let since_30d = now - chrono::Duration::days(30);

        #[cfg(feature = "postgres")]
        let row: InviteSummaryRow = sqlx::query_as(
            r#"
            SELECT
                COUNT(*)::bigint AS total,
                COALESCE(SUM(CASE WHEN is_revoked = false AND expires_at > $2 AND used_count < max_uses THEN 1 ELSE 0 END), 0)::bigint AS active,
                COALESCE(SUM(CASE WHEN is_revoked = true THEN 1 ELSE 0 END), 0)::bigint AS revoked,
                COALESCE(SUM(CASE WHEN is_revoked = false AND expires_at <= $2 AND used_count < max_uses THEN 1 ELSE 0 END), 0)::bigint AS expired,
                COALESCE(SUM(CASE WHEN is_revoked = false AND used_count >= max_uses THEN 1 ELSE 0 END), 0)::bigint AS used_up,
                COALESCE(SUM(used_count), 0)::bigint AS total_uses,
                COALESCE(SUM(max_uses), 0)::bigint AS total_capacity,
                COALESCE(SUM(CASE WHEN created_at >= $3 THEN 1 ELSE 0 END), 0)::bigint AS created_last_30d,
                COALESCE(SUM(CASE WHEN last_used_at >= $3 THEN 1 ELSE 0 END), 0)::bigint AS used_last_30d
            FROM customer_registration_invites
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .bind(now)
        .bind(since_30d)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: InviteSummaryRow = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) AS total,
                COALESCE(SUM(CASE WHEN is_revoked = 0 AND expires_at > ? AND used_count < max_uses THEN 1 ELSE 0 END), 0) AS active,
                COALESCE(SUM(CASE WHEN is_revoked = 1 THEN 1 ELSE 0 END), 0) AS revoked,
                COALESCE(SUM(CASE WHEN is_revoked = 0 AND expires_at <= ? AND used_count < max_uses THEN 1 ELSE 0 END), 0) AS expired,
                COALESCE(SUM(CASE WHEN is_revoked = 0 AND used_count >= max_uses THEN 1 ELSE 0 END), 0) AS used_up,
                COALESCE(SUM(used_count), 0) AS total_uses,
                COALESCE(SUM(max_uses), 0) AS total_capacity,
                COALESCE(SUM(CASE WHEN created_at >= ? THEN 1 ELSE 0 END), 0) AS created_last_30d,
                COALESCE(SUM(CASE WHEN last_used_at >= ? THEN 1 ELSE 0 END), 0) AS used_last_30d
            FROM customer_registration_invites
            WHERE tenant_id = ?
            "#,
        )
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(since_30d.to_rfc3339())
        .bind(since_30d.to_rfc3339())
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        let utilization_percent = if row.total_capacity > 0 {
            (row.total_uses as f64 / row.total_capacity as f64) * 100.0
        } else {
            0.0
        };

        Ok(CustomerRegistrationInviteSummary {
            total: row.total,
            active: row.active,
            revoked: row.revoked,
            expired: row.expired,
            used_up: row.used_up,
            total_uses: row.total_uses,
            total_capacity: row.total_capacity,
            utilization_percent,
            created_last_30d: row.created_last_30d,
            used_last_30d: row.used_last_30d,
        })
    }

    pub async fn validate_customer_registration_invite(
        &self,
        tenant_id: &str,
        invite_token: &str,
    ) -> AppResult<CustomerRegistrationInviteValidationView> {
        let token = invite_token.trim();
        if token.len() < 20 {
            return Ok(CustomerRegistrationInviteValidationView {
                valid: false,
                status: "invalid".to_string(),
                message: "Invite token is missing or malformed".to_string(),
                expires_at: None,
                max_uses: None,
                used_count: None,
                remaining_uses: None,
            });
        }

        let token_hash = Self::hash_registration_invite_token(token);
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let invite: Option<CustomerRegistrationInviteView> = sqlx::query_as(
            r#"
            SELECT
                id,
                tenant_id,
                created_by,
                max_uses,
                used_count,
                expires_at,
                is_revoked,
                revoked_at,
                last_used_at,
                note,
                created_at
            FROM customer_registration_invites
            WHERE tenant_id = $1 AND token_hash = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let invite: Option<CustomerRegistrationInviteView> = sqlx::query_as(
            r#"
            SELECT
                id,
                tenant_id,
                created_by,
                max_uses,
                used_count,
                expires_at,
                is_revoked,
                revoked_at,
                last_used_at,
                note,
                created_at
            FROM customer_registration_invites
            WHERE tenant_id = ? AND token_hash = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        let Some(invite) = invite else {
            return Ok(CustomerRegistrationInviteValidationView {
                valid: false,
                status: "invalid".to_string(),
                message: "Invite link is invalid or no longer available".to_string(),
                expires_at: None,
                max_uses: None,
                used_count: None,
                remaining_uses: None,
            });
        };

        let remaining = (invite.max_uses - invite.used_count).max(0);
        let (valid, status, message) = if invite.is_revoked {
            (
                false,
                "revoked".to_string(),
                "Invite link has been revoked".to_string(),
            )
        } else if invite.expires_at <= now {
            (
                false,
                "expired".to_string(),
                "Invite link has expired".to_string(),
            )
        } else if invite.used_count >= invite.max_uses {
            (
                false,
                "used_up".to_string(),
                "Invite link has reached the maximum usage".to_string(),
            )
        } else {
            (
                true,
                "valid".to_string(),
                "Invite link is valid".to_string(),
            )
        };

        Ok(CustomerRegistrationInviteValidationView {
            valid,
            status,
            message,
            expires_at: Some(invite.expires_at),
            max_uses: Some(invite.max_uses),
            used_count: Some(invite.used_count),
            remaining_uses: Some(remaining),
        })
    }

    pub async fn list_customer_registration_invites(
        &self,
        actor_id: &str,
        tenant_id: &str,
        include_inactive: bool,
        limit: u32,
    ) -> AppResult<Vec<CustomerRegistrationInviteView>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let limit = (limit as i64).clamp(1, 500);
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let rows: Vec<CustomerRegistrationInviteView> = sqlx::query_as(
            r#"
            SELECT
                id,
                tenant_id,
                created_by,
                max_uses,
                used_count,
                expires_at,
                is_revoked,
                revoked_at,
                last_used_at,
                note,
                created_at
            FROM customer_registration_invites
            WHERE tenant_id = $1
              AND (
                    $2::bool = true
                 OR (is_revoked = false AND expires_at > $3 AND used_count < max_uses)
              )
            ORDER BY created_at DESC
            LIMIT $4
            "#,
        )
        .bind(tenant_id)
        .bind(include_inactive)
        .bind(now)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<CustomerRegistrationInviteView> = sqlx::query_as(
            r#"
            SELECT
                id,
                tenant_id,
                created_by,
                max_uses,
                used_count,
                expires_at,
                is_revoked,
                revoked_at,
                last_used_at,
                note,
                created_at
            FROM customer_registration_invites
            WHERE tenant_id = ?
              AND (
                    ? = 1
                 OR (is_revoked = 0 AND expires_at > ? AND used_count < max_uses)
              )
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(tenant_id)
        .bind(if include_inactive { 1 } else { 0 })
        .bind(now.to_rfc3339())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn revoke_customer_registration_invite(
        &self,
        actor_id: &str,
        tenant_id: &str,
        invite_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let res = sqlx::query(
            r#"
            UPDATE customer_registration_invites
            SET is_revoked = true, revoked_at = $1
            WHERE tenant_id = $2 AND id = $3 AND is_revoked = false
            "#,
        )
        .bind(now)
        .bind(tenant_id)
        .bind(invite_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let res = sqlx::query(
            r#"
            UPDATE customer_registration_invites
            SET is_revoked = 1, revoked_at = ?
            WHERE tenant_id = ? AND id = ? AND is_revoked = 0
            "#,
        )
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(invite_id)
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Customer invite link not found or already revoked".to_string(),
            ));
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_INVITE_REVOKE",
                "customer_registration_invites",
                Some(invite_id),
                Some("Revoked customer registration invite"),
                ip_address,
            )
            .await;

        Ok(())
    }

    pub async fn consume_customer_registration_invite(
        &self,
        tenant_id: &str,
        invite_token: &str,
    ) -> AppResult<()> {
        let token = invite_token.trim();
        if token.len() < 20 {
            return Err(AppError::Validation(
                "Invalid customer invite token".to_string(),
            ));
        }
        let token_hash = Self::hash_registration_invite_token(token);
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let row: Option<String> = sqlx::query_scalar(
            r#"
            UPDATE customer_registration_invites
            SET used_count = used_count + 1, last_used_at = $1
            WHERE tenant_id = $2
              AND token_hash = $3
              AND is_revoked = false
              AND expires_at > $1
              AND used_count < max_uses
            RETURNING id
            "#,
        )
        .bind(now)
        .bind(tenant_id)
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let affected = sqlx::query(
            r#"
            UPDATE customer_registration_invites
            SET used_count = used_count + 1, last_used_at = ?
            WHERE tenant_id = ?
              AND token_hash = ?
              AND is_revoked = 0
              AND expires_at > ?
              AND used_count < max_uses
            "#,
        )
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(&token_hash)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?
        .rows_affected();

        #[cfg(feature = "postgres")]
        if row.is_none() {
            return Err(AppError::Validation(
                "Invite link is invalid, expired, or already used".to_string(),
            ));
        }

        #[cfg(feature = "sqlite")]
        if affected == 0 {
            return Err(AppError::Validation(
                "Invite link is invalid, expired, or already used".to_string(),
            ));
        }

        Ok(())
    }

    pub async fn update_customer(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
        dto: UpdateCustomerRequest,
        ip_address: Option<&str>,
    ) -> AppResult<Customer> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let mut customer = self.get_customer(actor_id, tenant_id, customer_id).await?;
        if let Some(name) = dto.name {
            customer.name = name;
        }
        if let Some(email) = dto.email {
            let v = email.trim().to_string();
            customer.email = if v.is_empty() { None } else { Some(v) };
        }
        if let Some(phone) = dto.phone {
            let v = phone.trim().to_string();
            customer.phone = if v.is_empty() { None } else { Some(v) };
        }
        if let Some(notes) = dto.notes {
            let v = notes.trim().to_string();
            customer.notes = if v.is_empty() { None } else { Some(v) };
        }
        if let Some(is_active) = dto.is_active {
            customer.is_active = is_active;
        }
        customer.updated_at = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE customers
            SET name=$1, email=$2, phone=$3, notes=$4, is_active=$5, updated_at=$6
            WHERE tenant_id=$7 AND id=$8
            "#,
        )
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.phone)
        .bind(&customer.notes)
        .bind(customer.is_active)
        .bind(customer.updated_at)
        .bind(tenant_id)
        .bind(customer_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE customers
            SET name=?, email=?, phone=?, notes=?, is_active=?, updated_at=?
            WHERE tenant_id=? AND id=?
            "#,
        )
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.phone)
        .bind(&customer.notes)
        .bind(customer.is_active)
        .bind(customer.updated_at.to_rfc3339())
        .bind(tenant_id)
        .bind(customer_id)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_UPDATE",
                "customers",
                Some(customer_id),
                Some("Updated customer"),
                ip_address,
            )
            .await;

        Ok(customer)
    }

    pub async fn delete_customer(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        #[cfg(feature = "postgres")]
        let res = sqlx::query("DELETE FROM customers WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(customer_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let res = sqlx::query("DELETE FROM customers WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(customer_id)
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Customer not found".to_string()));
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_DELETE",
                "customers",
                Some(customer_id),
                Some("Deleted customer"),
                ip_address,
            )
            .await;

        Ok(())
    }

    // =========================
    // Admin: Locations
    // =========================

    pub async fn list_locations(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
    ) -> AppResult<Vec<CustomerLocation>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customer_locations", "read")
            .await?;

        // Ensure customer is within tenant
        let _ = self.get_customer(actor_id, tenant_id, customer_id).await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<CustomerLocation> = sqlx::query_as(
            r#"
            SELECT
                id,
                tenant_id,
                customer_id,
                label,
                address_line1,
                address_line2,
                city,
                state,
                postal_code,
                country,
                latitude::float8 AS latitude,
                longitude::float8 AS longitude,
                notes,
                created_at,
                updated_at
            FROM customer_locations
            WHERE tenant_id = $1 AND customer_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<CustomerLocation> = sqlx::query_as(
            "SELECT * FROM customer_locations WHERE tenant_id = ? AND customer_id = ? ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn create_location(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateCustomerLocationRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerLocation> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customer_locations", "manage")
            .await?;

        let _ = self
            .get_customer(actor_id, tenant_id, &dto.customer_id)
            .await?;

        let loc = CustomerLocation::new(
            tenant_id.to_string(),
            dto.customer_id,
            dto.label,
            dto.address_line1,
            dto.address_line2,
            dto.city,
            dto.state,
            dto.postal_code,
            dto.country,
            dto.latitude,
            dto.longitude,
            dto.notes,
        );

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customer_locations
                (id, tenant_id, customer_id, label, address_line1, address_line2, city, state, postal_code, country, latitude, longitude, notes, created_at, updated_at)
            VALUES
                ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            "#,
        )
        .bind(&loc.id)
        .bind(&loc.tenant_id)
        .bind(&loc.customer_id)
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.created_at)
        .bind(loc.updated_at)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO customer_locations
                (id, tenant_id, customer_id, label, address_line1, address_line2, city, state, postal_code, country, latitude, longitude, notes, created_at, updated_at)
            VALUES
                (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
            "#,
        )
        .bind(&loc.id)
        .bind(&loc.tenant_id)
        .bind(&loc.customer_id)
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.created_at.to_rfc3339())
        .bind(loc.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_LOCATION_CREATE",
                "customer_locations",
                Some(&loc.id),
                Some("Created customer location"),
                ip_address,
            )
            .await;

        Ok(loc)
    }

    pub async fn update_location(
        &self,
        actor_id: &str,
        tenant_id: &str,
        location_id: &str,
        dto: UpdateCustomerLocationRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerLocation> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customer_locations", "manage")
            .await?;

        #[cfg(feature = "postgres")]
        let mut loc: CustomerLocation = sqlx::query_as(
            r#"
            SELECT
                id,
                tenant_id,
                customer_id,
                label,
                address_line1,
                address_line2,
                city,
                state,
                postal_code,
                country,
                latitude::float8 AS latitude,
                longitude::float8 AS longitude,
                notes,
                created_at,
                updated_at
            FROM customer_locations
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Location not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let mut loc: CustomerLocation =
            sqlx::query_as("SELECT * FROM customer_locations WHERE tenant_id = ? AND id = ?")
                .bind(tenant_id)
                .bind(location_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| AppError::NotFound("Location not found".to_string()))?;

        if let Some(v) = dto.label {
            let vv = v.trim().to_string();
            if !vv.is_empty() {
                loc.label = vv;
            }
        }
        if let Some(v) = dto.address_line1 {
            let vv = v.trim().to_string();
            loc.address_line1 = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.address_line2 {
            let vv = v.trim().to_string();
            loc.address_line2 = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.city {
            let vv = v.trim().to_string();
            loc.city = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.state {
            let vv = v.trim().to_string();
            loc.state = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.postal_code {
            let vv = v.trim().to_string();
            loc.postal_code = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.country {
            let vv = v.trim().to_string();
            loc.country = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.latitude {
            loc.latitude = Some(v);
        }
        if let Some(v) = dto.longitude {
            loc.longitude = Some(v);
        }
        if let Some(v) = dto.notes {
            let vv = v.trim().to_string();
            loc.notes = if vv.is_empty() { None } else { Some(vv) };
        }
        loc.updated_at = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE customer_locations
            SET label=$1, address_line1=$2, address_line2=$3, city=$4, state=$5, postal_code=$6, country=$7,
                latitude=$8, longitude=$9, notes=$10, updated_at=$11
            WHERE tenant_id=$12 AND id=$13
            "#,
        )
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.updated_at)
        .bind(tenant_id)
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE customer_locations
            SET label=?, address_line1=?, address_line2=?, city=?, state=?, postal_code=?, country=?,
                latitude=?, longitude=?, notes=?, updated_at=?
            WHERE tenant_id=? AND id=?
            "#,
        )
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.updated_at.to_rfc3339())
        .bind(tenant_id)
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_LOCATION_UPDATE",
                "customer_locations",
                Some(location_id),
                Some("Updated customer location"),
                ip_address,
            )
            .await;

        Ok(loc)
    }

    pub async fn delete_location(
        &self,
        actor_id: &str,
        tenant_id: &str,
        location_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customer_locations", "manage")
            .await?;

        #[cfg(feature = "postgres")]
        let res = sqlx::query("DELETE FROM customer_locations WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(location_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let res = sqlx::query("DELETE FROM customer_locations WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(location_id)
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Location not found".to_string()));
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_LOCATION_DELETE",
                "customer_locations",
                Some(location_id),
                Some("Deleted customer location"),
                ip_address,
            )
            .await;

        Ok(())
    }

    // =========================
    // Admin: Portal Users
    // =========================

    pub async fn list_portal_users(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
    ) -> AppResult<Vec<CustomerPortalUser>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read")
            .await?;

        let _ = self.get_customer(actor_id, tenant_id, customer_id).await?;

        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT
                cu.id as customer_user_id,
                u.id as user_id,
                u.email as email,
                u.name as name,
                cu.created_at as created_at
            FROM customer_users cu
            JOIN users u ON u.id = cu.user_id
            WHERE cu.tenant_id = $1 AND cu.customer_id = $2
            ORDER BY cu.created_at DESC
        "#;

        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT
                cu.id as customer_user_id,
                u.id as user_id,
                u.email as email,
                u.name as name,
                cu.created_at as created_at
            FROM customer_users cu
            JOIN users u ON u.id = cu.user_id
            WHERE cu.tenant_id = ? AND cu.customer_id = ?
            ORDER BY cu.created_at DESC
        "#;

        let rows: Vec<CustomerPortalUser> = sqlx::query_as(query)
            .bind(tenant_id)
            .bind(customer_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows)
    }

    pub async fn add_portal_user(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: AddCustomerPortalUserRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerPortalUser> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let _ = self
            .get_customer(actor_id, tenant_id, &dto.customer_id)
            .await?;

        let cu = CustomerUser::new(tenant_id.to_string(), dto.customer_id, dto.user_id);

        #[cfg(feature = "postgres")]
        {
            let res = sqlx::query(
                "INSERT INTO customer_users (id, tenant_id, customer_id, user_id, created_at) VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(&cu.id)
            .bind(&cu.tenant_id)
            .bind(&cu.customer_id)
            .bind(&cu.user_id)
            .bind(cu.created_at)
            .execute(&self.pool)
            .await;

            if let Err(e) = res {
                let is_unique = e
                    .as_database_error()
                    .and_then(|d| d.code().map(|c| c == "23505"))
                    .unwrap_or(false);
                if is_unique {
                    return Err(AppError::Validation(
                        "This user is already linked to a customer in this tenant.".to_string(),
                    ));
                }
                return Err(e.into());
            }
        }

        #[cfg(feature = "sqlite")]
        {
            // SQLite uses OR IGNORE to avoid hard failure on duplicates.
            sqlx::query(
                "INSERT OR IGNORE INTO customer_users (id, tenant_id, customer_id, user_id, created_at) VALUES (?,?,?,?,?)",
            )
            .bind(&cu.id)
            .bind(&cu.tenant_id)
            .bind(&cu.customer_id)
            .bind(&cu.user_id)
            .bind(cu.created_at.to_rfc3339())
            .execute(&self.pool)
            .await?;
        }

        // Ensure customer can login: add tenant_members entry with Customer role if missing.
        let customer_role_id = self.get_system_role_id_by_name("Customer").await?;
        self.ensure_tenant_member_role(tenant_id, &cu.user_id, &customer_role_id)
            .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_USER_ADD",
                "customer_users",
                Some(&cu.id),
                Some("Added portal user to customer"),
                ip_address,
            )
            .await;

        // Return joined projection
        #[cfg(feature = "postgres")]
        let row: CustomerPortalUser = sqlx::query_as(
            r#"
            SELECT
                cu.id as customer_user_id,
                u.id as user_id,
                u.email as email,
                u.name as name,
                cu.created_at as created_at
            FROM customer_users cu
            JOIN users u ON u.id = cu.user_id
            WHERE cu.id = $1
            "#,
        )
        .bind(&cu.id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: CustomerPortalUser = sqlx::query_as(
            r#"
            SELECT
                cu.id as customer_user_id,
                u.id as user_id,
                u.email as email,
                u.name as name,
                cu.created_at as created_at
            FROM customer_users cu
            JOIN users u ON u.id = cu.user_id
            WHERE cu.id = ?
            "#,
        )
        .bind(&cu.id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn create_portal_user(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateCustomerPortalUserRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerPortalUser> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        let _ = self
            .get_customer(actor_id, tenant_id, &dto.customer_id)
            .await?;

        let user = self
            .user_service
            .create(
                crate::models::CreateUserDto {
                    email: dto.email,
                    name: dto.name,
                    password: dto.password,
                },
                Some(actor_id),
                ip_address,
            )
            .await?;

        let row = self
            .add_portal_user(
                actor_id,
                tenant_id,
                AddCustomerPortalUserRequest {
                    customer_id: dto.customer_id,
                    user_id: user.id.clone(),
                },
                ip_address,
            )
            .await?;

        Ok(row)
    }

    pub async fn remove_portal_user(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_user_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        #[cfg(feature = "postgres")]
        let res = sqlx::query("DELETE FROM customer_users WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(customer_user_id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let res = sqlx::query("DELETE FROM customer_users WHERE tenant_id = ? AND id = ?")
            .bind(tenant_id)
            .bind(customer_user_id)
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Portal user mapping not found".to_string(),
            ));
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_USER_REMOVE",
                "customer_users",
                Some(customer_user_id),
                Some("Removed portal user from customer"),
                ip_address,
            )
            .await;

        Ok(())
    }

    // =========================
    // Admin: Customer Subscriptions
    // =========================
    pub async fn list_customer_subscriptions(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: &str,
        page: u32,
        per_page: u32,
    ) -> AppResult<PaginatedResponse<CustomerSubscriptionView>> {
        if self
            .auth_service
            .check_permission(actor_id, tenant_id, "customers", "read")
            .await
            .is_err()
        {
            if self
                .auth_service
                .check_permission(actor_id, tenant_id, "work_orders", "manage")
                .await
                .is_err()
            {
                self.auth_service
                    .check_permission(actor_id, tenant_id, "work_orders", "read")
                    .await?;
            }
        }

        let offset = (page.saturating_sub(1)) * per_page;

        #[cfg(feature = "postgres")]
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM customer_subscriptions WHERE tenant_id = $1 AND customer_id = $2",
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM customer_subscriptions WHERE tenant_id = ? AND customer_id = ?",
        )
        .bind(tenant_id)
        .bind(customer_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<CustomerSubscriptionView> = sqlx::query_as(
            r#"
            SELECT
              cs.id,
              cs.tenant_id,
              cs.customer_id,
              cs.location_id,
              cs.package_id,
              cs.router_id,
              cs.billing_cycle,
              cs.price::float8 AS price,
              cs.currency_code,
              cs.status,
              cs.starts_at,
              cs.ends_at,
              cs.notes,
              cs.created_at,
              cs.updated_at,
              p.name AS package_name,
              l.label AS location_label,
              r.name AS router_name,
              (
                SELECT iwo.id
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_id,
              (
                SELECT iwo.status
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_status,
              CASE
                WHEN LOWER(cs.status) = 'cancelled' THEN true
                WHEN COALESCE((
                  SELECT LOWER(iwo.status)
                  FROM installation_work_orders iwo
                  WHERE iwo.tenant_id = cs.tenant_id
                    AND iwo.subscription_id = cs.id
                  ORDER BY iwo.created_at DESC
                  LIMIT 1
                ), '') = 'cancelled' THEN true
                ELSE false
              END AS can_request_reopen,
              (
                SELECT worr.status
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_status,
              (
                SELECT CAST(worr.requested_schedule_at AS TEXT)
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p ON p.id = cs.package_id
            LEFT JOIN customer_locations l ON l.id = cs.location_id
            LEFT JOIN mikrotik_routers r ON r.id = cs.router_id
            WHERE cs.tenant_id = $1 AND cs.customer_id = $2
            ORDER BY cs.updated_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<CustomerSubscriptionView> = sqlx::query_as(
            r#"
            SELECT
              cs.id,
              cs.tenant_id,
              cs.customer_id,
              cs.location_id,
              cs.package_id,
              cs.router_id,
              cs.billing_cycle,
              cs.price AS price,
              cs.currency_code,
              cs.status,
              cs.starts_at,
              cs.ends_at,
              cs.notes,
              cs.created_at,
              cs.updated_at,
              p.name AS package_name,
              l.label AS location_label,
              r.name AS router_name,
              (
                SELECT iwo.id
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_id,
              (
                SELECT iwo.status
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_status,
              CASE
                WHEN LOWER(cs.status) = 'cancelled' THEN 1
                WHEN COALESCE((
                  SELECT LOWER(iwo.status)
                  FROM installation_work_orders iwo
                  WHERE iwo.tenant_id = cs.tenant_id
                    AND iwo.subscription_id = cs.id
                  ORDER BY iwo.created_at DESC
                  LIMIT 1
                ), '') = 'cancelled' THEN 1
                ELSE 0
              END AS can_request_reopen,
              (
                SELECT worr.status
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_status,
              (
                SELECT CAST(worr.requested_schedule_at AS TEXT)
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p ON p.id = cs.package_id
            LEFT JOIN customer_locations l ON l.id = cs.location_id
            LEFT JOIN mikrotik_routers r ON r.id = cs.router_id
            WHERE cs.tenant_id = ? AND cs.customer_id = ?
            ORDER BY cs.updated_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(PaginatedResponse {
            data: rows,
            total,
            page,
            per_page,
        })
    }

    pub async fn create_customer_subscription(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateCustomerSubscriptionRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerSubscription> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        if dto.price <= 0.0 {
            return Err(AppError::Validation(
                "price must be greater than 0".to_string(),
            ));
        }

        let billing_cycle = Self::normalize_billing_cycle(&dto.billing_cycle)?;
        let status =
            Self::normalize_subscription_status(dto.status.as_deref().unwrap_or("active"))?;
        let starts_at = Self::parse_optional_datetime(dto.starts_at)?;
        let ends_at = Self::parse_optional_datetime(dto.ends_at)?;

        #[cfg(feature = "postgres")]
        let exists_customer: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customers WHERE id = $1 AND tenant_id = $2)",
        )
        .bind(&dto.customer_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let exists_customer: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customers WHERE id = ? AND tenant_id = ?)",
        )
        .bind(&dto.customer_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        if !exists_customer {
            return Err(AppError::NotFound("Customer not found".to_string()));
        }

        #[cfg(feature = "postgres")]
        let exists_location: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customer_locations WHERE id = $1 AND tenant_id = $2 AND customer_id = $3)",
        )
        .bind(&dto.location_id)
        .bind(tenant_id)
        .bind(&dto.customer_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let exists_location: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customer_locations WHERE id = ? AND tenant_id = ? AND customer_id = ?)",
        )
        .bind(&dto.location_id)
        .bind(tenant_id)
        .bind(&dto.customer_id)
        .fetch_one(&self.pool)
        .await?;

        if !exists_location {
            return Err(AppError::Validation(
                "Location does not belong to this customer".to_string(),
            ));
        }

        #[cfg(feature = "postgres")]
        let exists_package: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM isp_packages WHERE id = $1 AND tenant_id = $2)",
        )
        .bind(&dto.package_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let exists_package: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM isp_packages WHERE id = ? AND tenant_id = ?)",
        )
        .bind(&dto.package_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        if !exists_package {
            return Err(AppError::Validation("Package not found".to_string()));
        }

        if let Some(router_id) = dto.router_id.as_deref() {
            #[cfg(feature = "postgres")]
            let exists_router: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2)",
            )
            .bind(router_id)
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;

            #[cfg(feature = "sqlite")]
            let exists_router: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM mikrotik_routers WHERE id = ? AND tenant_id = ?)",
            )
            .bind(router_id)
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;

            if !exists_router {
                return Err(AppError::Validation("Router not found".to_string()));
            }
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let currency = dto
            .currency_code
            .unwrap_or_else(|| "IDR".to_string())
            .trim()
            .to_uppercase();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customer_subscriptions
              (id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at)
            VALUES
              ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(&dto.customer_id)
        .bind(&dto.location_id)
        .bind(&dto.package_id)
        .bind(&dto.router_id)
        .bind(&billing_cycle)
        .bind(dto.price)
        .bind(&currency)
        .bind(&status)
        .bind(starts_at)
        .bind(ends_at)
        .bind(dto.notes.as_deref().map(str::trim).filter(|s| !s.is_empty()))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO customer_subscriptions
              (id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at)
            VALUES
              (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(&dto.customer_id)
        .bind(&dto.location_id)
        .bind(&dto.package_id)
        .bind(&dto.router_id)
        .bind(&billing_cycle)
        .bind(dto.price)
        .bind(&currency)
        .bind(&status)
        .bind(starts_at)
        .bind(ends_at)
        .bind(dto.notes.as_deref().map(str::trim).filter(|s| !s.is_empty()))
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let row: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price::float8 as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE id = $1 AND tenant_id = $2",
        )
        .bind(&id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE id = ? AND tenant_id = ?",
        )
        .bind(&id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_SUBSCRIPTION_CREATE",
                "customer_subscriptions",
                Some(&id),
                Some("Created customer subscription"),
                ip_address,
            )
            .await;

        // For portal self-checkout, PPPoE provisioning is deferred until
        // installation work order is completed by technician.

        Ok(row)
    }

    pub async fn update_customer_subscription(
        &self,
        actor_id: &str,
        tenant_id: &str,
        subscription_id: &str,
        dto: UpdateCustomerSubscriptionRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerSubscription> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        #[cfg(feature = "postgres")]
        let mut row: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price::float8 as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE id = $1 AND tenant_id = $2",
        )
        .bind(subscription_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let mut row: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE id = ? AND tenant_id = ?",
        )
        .bind(subscription_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        if let Some(price) = dto.price {
            if price <= 0.0 {
                return Err(AppError::Validation(
                    "price must be greater than 0".to_string(),
                ));
            }
            row.price = price;
        }
        if let Some(v) = dto.billing_cycle {
            row.billing_cycle = Self::normalize_billing_cycle(&v)?;
        }
        if let Some(v) = dto.status {
            row.status = Self::normalize_subscription_status(&v)?;
        }
        if let Some(v) = dto.currency_code {
            let x = v.trim().to_uppercase();
            if !x.is_empty() {
                row.currency_code = x;
            }
        }
        if let Some(v) = dto.location_id {
            row.location_id = v;
        }
        if let Some(v) = dto.package_id {
            row.package_id = v;
        }
        if dto.router_id.is_some() {
            row.router_id = dto.router_id;
        }
        if dto.starts_at.is_some() {
            row.starts_at = Self::parse_optional_datetime(dto.starts_at)?;
        }
        if dto.ends_at.is_some() {
            row.ends_at = Self::parse_optional_datetime(dto.ends_at)?;
        }
        if let Some(v) = dto.notes {
            let x = v.trim().to_string();
            row.notes = if x.is_empty() { None } else { Some(x) };
        }
        row.updated_at = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE customer_subscriptions
            SET
              location_id = $1,
              package_id = $2,
              router_id = $3,
              billing_cycle = $4,
              price = $5,
              currency_code = $6,
              status = $7,
              starts_at = $8,
              ends_at = $9,
              notes = $10,
              updated_at = $11
            WHERE id = $12 AND tenant_id = $13
            "#,
        )
        .bind(&row.location_id)
        .bind(&row.package_id)
        .bind(&row.router_id)
        .bind(&row.billing_cycle)
        .bind(row.price)
        .bind(&row.currency_code)
        .bind(&row.status)
        .bind(row.starts_at)
        .bind(row.ends_at)
        .bind(&row.notes)
        .bind(row.updated_at)
        .bind(subscription_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE customer_subscriptions
            SET
              location_id = ?,
              package_id = ?,
              router_id = ?,
              billing_cycle = ?,
              price = ?,
              currency_code = ?,
              status = ?,
              starts_at = ?,
              ends_at = ?,
              notes = ?,
              updated_at = ?
            WHERE id = ? AND tenant_id = ?
            "#,
        )
        .bind(&row.location_id)
        .bind(&row.package_id)
        .bind(&row.router_id)
        .bind(&row.billing_cycle)
        .bind(row.price)
        .bind(&row.currency_code)
        .bind(&row.status)
        .bind(row.starts_at)
        .bind(row.ends_at)
        .bind(&row.notes)
        .bind(row.updated_at)
        .bind(subscription_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_SUBSCRIPTION_UPDATE",
                "customer_subscriptions",
                Some(subscription_id),
                Some("Updated customer subscription"),
                ip_address,
            )
            .await;

        self.auto_provision_pppoe_for_subscription(actor_id, tenant_id, &row, ip_address)
            .await?;

        Ok(row)
    }

    pub async fn delete_customer_subscription(
        &self,
        actor_id: &str,
        tenant_id: &str,
        subscription_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "manage")
            .await?;

        #[cfg(feature = "postgres")]
        let res =
            sqlx::query("DELETE FROM customer_subscriptions WHERE id = $1 AND tenant_id = $2")
                .bind(subscription_id)
                .bind(tenant_id)
                .execute(&self.pool)
                .await?;

        #[cfg(feature = "sqlite")]
        let res = sqlx::query("DELETE FROM customer_subscriptions WHERE id = ? AND tenant_id = ?")
            .bind(subscription_id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Subscription not found".to_string()));
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_SUBSCRIPTION_DELETE",
                "customer_subscriptions",
                Some(subscription_id),
                Some("Deleted customer subscription"),
                ip_address,
            )
            .await;

        Ok(())
    }

    // =========================
    // Portal: Self-service
    // =========================

    pub async fn get_portal_customer_id(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<String> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read_own")
            .await?;

        #[cfg(feature = "postgres")]
        let customer_id: Option<String> = sqlx::query_scalar(
            "SELECT customer_id FROM customer_users WHERE tenant_id = $1 AND user_id = $2 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let customer_id: Option<String> = sqlx::query_scalar(
            "SELECT customer_id FROM customer_users WHERE tenant_id = ? AND user_id = ? LIMIT 1",
        )
        .bind(tenant_id)
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?;

        customer_id
            .ok_or_else(|| AppError::Forbidden("You are not linked to any customer".to_string()))
    }

    pub async fn list_my_locations(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<Vec<CustomerLocation>> {
        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<CustomerLocation> = sqlx::query_as(
            r#"
            SELECT
                id,
                tenant_id,
                customer_id,
                label,
                address_line1,
                address_line2,
                city,
                state,
                postal_code,
                country,
                latitude::float8 AS latitude,
                longitude::float8 AS longitude,
                notes,
                created_at,
                updated_at
            FROM customer_locations
            WHERE tenant_id = $1 AND customer_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<CustomerLocation> = sqlx::query_as(
            "SELECT * FROM customer_locations WHERE tenant_id = ? AND customer_id = ? ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    fn validate_location_coordinates(
        latitude: Option<f64>,
        longitude: Option<f64>,
    ) -> AppResult<(f64, f64)> {
        let lat = latitude
            .ok_or_else(|| AppError::Validation("Location map point is required".to_string()))?;
        let lng = longitude
            .ok_or_else(|| AppError::Validation("Location map point is required".to_string()))?;
        if !(-90.0..=90.0).contains(&lat) {
            return Err(AppError::Validation(
                "Latitude must be between -90 and 90".to_string(),
            ));
        }
        if !(-180.0..=180.0).contains(&lng) {
            return Err(AppError::Validation(
                "Longitude must be between -180 and 180".to_string(),
            ));
        }
        Ok((lat, lng))
    }

    async fn get_my_location_or_404(
        &self,
        actor_id: &str,
        tenant_id: &str,
        location_id: &str,
    ) -> AppResult<CustomerLocation> {
        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let loc: Option<CustomerLocation> = sqlx::query_as(
            r#"
            SELECT
                id,
                tenant_id,
                customer_id,
                label,
                address_line1,
                address_line2,
                city,
                state,
                postal_code,
                country,
                latitude::float8 AS latitude,
                longitude::float8 AS longitude,
                notes,
                created_at,
                updated_at
            FROM customer_locations
            WHERE tenant_id = $1 AND customer_id = $2 AND id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let loc: Option<CustomerLocation> = sqlx::query_as(
            "SELECT * FROM customer_locations WHERE tenant_id = ? AND customer_id = ? AND id = ?",
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await?;

        loc.ok_or_else(|| AppError::NotFound("Location not found".to_string()))
    }

    pub async fn create_my_location(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreateMyCustomerLocationRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerLocation> {
        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;
        let label = dto.label.trim().to_string();
        if label.is_empty() {
            return Err(AppError::Validation("label is required".to_string()));
        }
        let (latitude, longitude) =
            Self::validate_location_coordinates(dto.latitude, dto.longitude)?;

        let loc = CustomerLocation::new(
            tenant_id.to_string(),
            customer_id,
            label,
            dto.address_line1,
            dto.address_line2,
            dto.city,
            dto.state,
            dto.postal_code,
            dto.country,
            Some(latitude),
            Some(longitude),
            dto.notes,
        );

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customer_locations
                (id, tenant_id, customer_id, label, address_line1, address_line2, city, state, postal_code, country, latitude, longitude, notes, created_at, updated_at)
            VALUES
                ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            "#,
        )
        .bind(&loc.id)
        .bind(&loc.tenant_id)
        .bind(&loc.customer_id)
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.created_at)
        .bind(loc.updated_at)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO customer_locations
                (id, tenant_id, customer_id, label, address_line1, address_line2, city, state, postal_code, country, latitude, longitude, notes, created_at, updated_at)
            VALUES
                (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
            "#,
        )
        .bind(&loc.id)
        .bind(&loc.tenant_id)
        .bind(&loc.customer_id)
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.created_at.to_rfc3339())
        .bind(loc.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_LOCATION_SELF_CREATE",
                "customer_locations",
                Some(&loc.id),
                Some("Created own customer location from portal"),
                ip_address,
            )
            .await;

        Ok(loc)
    }

    pub async fn update_my_location(
        &self,
        actor_id: &str,
        tenant_id: &str,
        location_id: &str,
        dto: UpdateCustomerLocationRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerLocation> {
        let mut loc = self
            .get_my_location_or_404(actor_id, tenant_id, location_id)
            .await?;

        if let Some(v) = dto.label {
            let vv = v.trim().to_string();
            if vv.is_empty() {
                return Err(AppError::Validation("label is required".to_string()));
            }
            loc.label = vv;
        }
        if let Some(v) = dto.address_line1 {
            let vv = v.trim().to_string();
            loc.address_line1 = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.address_line2 {
            let vv = v.trim().to_string();
            loc.address_line2 = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.city {
            let vv = v.trim().to_string();
            loc.city = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.state {
            let vv = v.trim().to_string();
            loc.state = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.postal_code {
            let vv = v.trim().to_string();
            loc.postal_code = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.country {
            let vv = v.trim().to_string();
            loc.country = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.latitude {
            loc.latitude = Some(v);
        }
        if let Some(v) = dto.longitude {
            loc.longitude = Some(v);
        }
        if let Some(v) = dto.notes {
            let vv = v.trim().to_string();
            loc.notes = if vv.is_empty() { None } else { Some(vv) };
        }

        let (latitude, longitude) =
            Self::validate_location_coordinates(loc.latitude, loc.longitude)?;
        loc.latitude = Some(latitude);
        loc.longitude = Some(longitude);
        loc.updated_at = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE customer_locations
            SET label=$1, address_line1=$2, address_line2=$3, city=$4, state=$5, postal_code=$6, country=$7,
                latitude=$8, longitude=$9, notes=$10, updated_at=$11
            WHERE tenant_id=$12 AND customer_id=$13 AND id=$14
            "#,
        )
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.updated_at)
        .bind(tenant_id)
        .bind(&loc.customer_id)
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE customer_locations
            SET label=?, address_line1=?, address_line2=?, city=?, state=?, postal_code=?, country=?,
                latitude=?, longitude=?, notes=?, updated_at=?
            WHERE tenant_id=? AND customer_id=? AND id=?
            "#,
        )
        .bind(&loc.label)
        .bind(&loc.address_line1)
        .bind(&loc.address_line2)
        .bind(&loc.city)
        .bind(&loc.state)
        .bind(&loc.postal_code)
        .bind(&loc.country)
        .bind(loc.latitude)
        .bind(loc.longitude)
        .bind(&loc.notes)
        .bind(loc.updated_at.to_rfc3339())
        .bind(tenant_id)
        .bind(&loc.customer_id)
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "PORTAL_CUSTOMER_LOCATION_UPDATE",
                "customer_locations",
                Some(location_id),
                Some("Portal user updated customer location"),
                ip_address,
            )
            .await;

        Ok(loc)
    }

    pub async fn delete_my_location(
        &self,
        actor_id: &str,
        tenant_id: &str,
        location_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        let loc = self
            .get_my_location_or_404(actor_id, tenant_id, location_id)
            .await?;

        #[cfg(feature = "postgres")]
        let res = sqlx::query(
            "DELETE FROM customer_locations WHERE tenant_id = $1 AND customer_id = $2 AND id = $3",
        )
        .bind(tenant_id)
        .bind(&loc.customer_id)
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let res = sqlx::query(
            "DELETE FROM customer_locations WHERE tenant_id = ? AND customer_id = ? AND id = ?",
        )
        .bind(tenant_id)
        .bind(&loc.customer_id)
        .bind(location_id)
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(AppError::NotFound("Location not found".to_string()));
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "PORTAL_CUSTOMER_LOCATION_DELETE",
                "customer_locations",
                Some(location_id),
                Some("Portal user deleted customer location"),
                ip_address,
            )
            .await;

        Ok(())
    }

    pub async fn list_my_packages(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<Vec<IspPackage>> {
        let _customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<IspPackage> = sqlx::query_as(
            r#"
            SELECT
              id,
              tenant_id,
              service_type,
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
              AND is_active = true
            ORDER BY price_monthly ASC, name ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<IspPackage> = sqlx::query_as(
            r#"
            SELECT
              id,
              tenant_id,
              service_type,
              name,
              description,
              features,
              is_active,
              price_monthly AS price_monthly,
              price_yearly AS price_yearly,
              created_at,
              updated_at
            FROM isp_packages
            WHERE tenant_id = ?
              AND is_active = 1
            ORDER BY price_monthly ASC, name ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn list_my_subscriptions(
        &self,
        actor_id: &str,
        tenant_id: &str,
        page: u32,
        per_page: u32,
        status: Option<String>,
        sort_by: Option<String>,
        sort_dir: Option<String>,
    ) -> AppResult<PaginatedResponse<CustomerSubscriptionView>> {
        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;
        let offset = (page.saturating_sub(1)) * per_page;
        let status_filter = status
            .map(|v| v.trim().to_lowercase())
            .filter(|v| !v.is_empty());
        let sort_column = match sort_by
            .unwrap_or_else(|| "updated_at".to_string())
            .trim()
            .to_lowercase()
            .as_str()
        {
            "price" => "cs.price",
            "status" => "LOWER(cs.status)",
            "package_name" => "LOWER(COALESCE(p.name, ''))",
            "location_label" => "LOWER(COALESCE(l.label, ''))",
            "updated_at" => "cs.updated_at",
            _ => "cs.updated_at",
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

        #[cfg(feature = "postgres")]
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = $1
              AND cs.customer_id = $2
              AND (
                    $3::text IS NULL
                    OR LOWER(cs.status) = $3
                    OR (
                      $3 = 'needs_attention'
                      AND (
                        LOWER(cs.status) IN ('suspended', 'cancelled')
                        OR COALESCE((
                          SELECT LOWER(iwo.status)
                          FROM installation_work_orders iwo
                          WHERE iwo.tenant_id = cs.tenant_id
                            AND iwo.subscription_id = cs.id
                          ORDER BY iwo.created_at DESC
                          LIMIT 1
                        ), '') = 'cancelled'
                      )
                    )
              )
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(&status_filter)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = ?
              AND cs.customer_id = ?
              AND (
                    ? IS NULL
                    OR LOWER(cs.status) = ?
                    OR (
                      ? = 'needs_attention'
                      AND (
                        LOWER(cs.status) IN ('suspended', 'cancelled')
                        OR COALESCE((
                          SELECT LOWER(iwo.status)
                          FROM installation_work_orders iwo
                          WHERE iwo.tenant_id = cs.tenant_id
                            AND iwo.subscription_id = cs.id
                          ORDER BY iwo.created_at DESC
                          LIMIT 1
                        ), '') = 'cancelled'
                      )
                    )
              )
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(status_filter.clone())
        .bind(status_filter.clone())
        .bind(status_filter.clone())
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<CustomerSubscriptionView> = sqlx::query_as(&format!(
            r#"
            SELECT
              cs.id,
              cs.tenant_id,
              cs.customer_id,
              cs.location_id,
              cs.package_id,
              cs.router_id,
              cs.billing_cycle,
              cs.price::float8 AS price,
              cs.currency_code,
              cs.status,
              cs.starts_at,
              cs.ends_at,
              cs.notes,
              cs.created_at,
              cs.updated_at,
              p.name AS package_name,
              l.label AS location_label,
              r.name AS router_name,
              (
                SELECT iwo.id
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_id,
              (
                SELECT iwo.status
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_status,
              CASE
                WHEN LOWER(cs.status) = 'cancelled' THEN true
                WHEN COALESCE((
                  SELECT LOWER(iwo.status)
                  FROM installation_work_orders iwo
                  WHERE iwo.tenant_id = cs.tenant_id
                    AND iwo.subscription_id = cs.id
                  ORDER BY iwo.created_at DESC
                  LIMIT 1
                ), '') = 'cancelled' THEN true
                ELSE false
              END AS can_request_reopen,
              (
                SELECT worr.status
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_status,
              (
                SELECT CAST(worr.requested_schedule_at AS TEXT)
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p ON p.id = cs.package_id
            LEFT JOIN customer_locations l ON l.id = cs.location_id
            LEFT JOIN mikrotik_routers r ON r.id = cs.router_id
            WHERE cs.tenant_id = $1
              AND cs.customer_id = $2
              AND (
                    $3::text IS NULL
                    OR LOWER(cs.status) = $3
                    OR (
                      $3 = 'needs_attention'
                      AND (
                        LOWER(cs.status) IN ('suspended', 'cancelled')
                        OR COALESCE((
                          SELECT LOWER(iwo.status)
                          FROM installation_work_orders iwo
                          WHERE iwo.tenant_id = cs.tenant_id
                            AND iwo.subscription_id = cs.id
                          ORDER BY iwo.created_at DESC
                          LIMIT 1
                        ), '') = 'cancelled'
                      )
                    )
              )
            ORDER BY {sort_column} {sort_direction}
            LIMIT $4 OFFSET $5
            "#,
        ))
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(&status_filter)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<CustomerSubscriptionView> = sqlx::query_as(&format!(
            r#"
            SELECT
              cs.id,
              cs.tenant_id,
              cs.customer_id,
              cs.location_id,
              cs.package_id,
              cs.router_id,
              cs.billing_cycle,
              cs.price AS price,
              cs.currency_code,
              cs.status,
              cs.starts_at,
              cs.ends_at,
              cs.notes,
              cs.created_at,
              cs.updated_at,
              p.name AS package_name,
              l.label AS location_label,
              r.name AS router_name,
              (
                SELECT iwo.id
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_id,
              (
                SELECT iwo.status
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_status,
              CASE
                WHEN LOWER(cs.status) = 'cancelled' THEN 1
                WHEN COALESCE((
                  SELECT LOWER(iwo.status)
                  FROM installation_work_orders iwo
                  WHERE iwo.tenant_id = cs.tenant_id
                    AND iwo.subscription_id = cs.id
                  ORDER BY iwo.created_at DESC
                  LIMIT 1
                ), '') = 'cancelled' THEN 1
                ELSE 0
              END AS can_request_reopen,
              (
                SELECT worr.status
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_status,
              (
                SELECT CAST(worr.requested_schedule_at AS TEXT)
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p ON p.id = cs.package_id
            LEFT JOIN customer_locations l ON l.id = cs.location_id
            LEFT JOIN mikrotik_routers r ON r.id = cs.router_id
            WHERE cs.tenant_id = ?
              AND cs.customer_id = ?
              AND (
                    ? IS NULL
                    OR LOWER(cs.status) = ?
                    OR (
                      ? = 'needs_attention'
                      AND (
                        LOWER(cs.status) IN ('suspended', 'cancelled')
                        OR COALESCE((
                          SELECT LOWER(iwo.status)
                          FROM installation_work_orders iwo
                          WHERE iwo.tenant_id = cs.tenant_id
                            AND iwo.subscription_id = cs.id
                          ORDER BY iwo.created_at DESC
                          LIMIT 1
                        ), '') = 'cancelled'
                      )
                    )
              )
            ORDER BY {sort_column} {sort_direction}
            LIMIT ? OFFSET ?
            "#,
        ))
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(status_filter.clone())
        .bind(status_filter.clone())
        .bind(status_filter.clone())
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(PaginatedResponse {
            data: rows,
            total,
            page,
            per_page,
        })
    }

    pub async fn get_my_subscription_stats(
        &self,
        actor_id: &str,
        tenant_id: &str,
    ) -> AppResult<CustomerPortalSubscriptionStats> {
        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let stats: CustomerPortalSubscriptionStats = sqlx::query_as(
            r#"
            SELECT
              COUNT(*)::bigint AS total,
              COUNT(*) FILTER (WHERE LOWER(cs.status) = 'active')::bigint AS active,
              COUNT(*) FILTER (
                WHERE LOWER(cs.status) = 'pending_installation'
                  AND COALESCE((
                    SELECT LOWER(iwo.status)
                    FROM installation_work_orders iwo
                    WHERE iwo.tenant_id = cs.tenant_id
                      AND iwo.subscription_id = cs.id
                    ORDER BY iwo.created_at DESC
                    LIMIT 1
                  ), '') <> 'cancelled'
              )::bigint AS pending_installation,
              COUNT(*) FILTER (
                WHERE LOWER(cs.status) IN ('suspended', 'cancelled')
                   OR COALESCE((
                    SELECT LOWER(iwo.status)
                    FROM installation_work_orders iwo
                    WHERE iwo.tenant_id = cs.tenant_id
                      AND iwo.subscription_id = cs.id
                    ORDER BY iwo.created_at DESC
                    LIMIT 1
                  ), '') = 'cancelled'
              )::bigint AS needs_attention
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = $1
              AND cs.customer_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let stats: CustomerPortalSubscriptionStats = sqlx::query_as(
            r#"
            SELECT
              COUNT(*) AS total,
              COALESCE(SUM(CASE WHEN LOWER(cs.status) = 'active' THEN 1 ELSE 0 END), 0) AS active,
              COALESCE(SUM(
                CASE
                  WHEN LOWER(cs.status) = 'pending_installation'
                   AND COALESCE((
                    SELECT LOWER(iwo.status)
                    FROM installation_work_orders iwo
                    WHERE iwo.tenant_id = cs.tenant_id
                      AND iwo.subscription_id = cs.id
                    ORDER BY iwo.created_at DESC
                    LIMIT 1
                   ), '') <> 'cancelled'
                  THEN 1 ELSE 0
                END
              ), 0) AS pending_installation,
              COALESCE(SUM(
                CASE
                  WHEN LOWER(cs.status) IN ('suspended', 'cancelled')
                    OR COALESCE((
                      SELECT LOWER(iwo.status)
                      FROM installation_work_orders iwo
                      WHERE iwo.tenant_id = cs.tenant_id
                        AND iwo.subscription_id = cs.id
                      ORDER BY iwo.created_at DESC
                      LIMIT 1
                    ), '') = 'cancelled'
                  THEN 1 ELSE 0
                END
              ), 0) AS needs_attention
            FROM customer_subscriptions cs
            WHERE cs.tenant_id = ?
              AND cs.customer_id = ?
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }

    pub async fn create_my_subscription(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: PortalCheckoutSubscriptionRequest,
        ip_address: Option<&str>,
    ) -> AppResult<CustomerSubscription> {
        #[cfg(feature = "postgres")]
        self.ensure_pending_installation_status_supported().await?;

        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        let location_id = dto.location_id.trim().to_string();
        if location_id.is_empty() {
            return Err(AppError::Validation("location_id is required".to_string()));
        }

        let package_id = dto.package_id.trim().to_string();
        if package_id.is_empty() {
            return Err(AppError::Validation("package_id is required".to_string()));
        }

        let billing_cycle = Self::normalize_billing_cycle(&dto.billing_cycle)?;
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let location_ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customer_locations WHERE tenant_id = $1 AND id = $2 AND customer_id = $3)",
        )
        .bind(tenant_id)
        .bind(&location_id)
        .bind(&customer_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let location_ok: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM customer_locations WHERE tenant_id = ? AND id = ? AND customer_id = ?)",
        )
        .bind(tenant_id)
        .bind(&location_id)
        .bind(&customer_id)
        .fetch_one(&self.pool)
        .await?;

        if !location_ok {
            return Err(AppError::Validation(
                "Location does not belong to your customer account".to_string(),
            ));
        }

        #[cfg(feature = "postgres")]
        let pkg_row: Option<(f64, f64)> = sqlx::query_as(
            "SELECT price_monthly::float8, price_yearly::float8 FROM isp_packages WHERE tenant_id = $1 AND id = $2 AND is_active = true LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&package_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let pkg_row: Option<(f64, f64)> = sqlx::query_as(
            "SELECT price_monthly AS price_monthly, price_yearly AS price_yearly FROM isp_packages WHERE tenant_id = ? AND id = ? AND is_active = 1 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&package_id)
        .fetch_optional(&self.pool)
        .await?;

        let (price_monthly, price_yearly) =
            pkg_row.ok_or_else(|| AppError::Validation("Package not found".to_string()))?;

        let price = if billing_cycle == "yearly" {
            if price_yearly <= 0.0 {
                return Err(AppError::Validation(
                    "Yearly billing is not available for this package".to_string(),
                ));
            }
            price_yearly
        } else {
            if price_monthly <= 0.0 {
                return Err(AppError::Validation(
                    "Package monthly price is invalid".to_string(),
                ));
            }
            price_monthly
        };

        // Portal checkout must create a new service order/subscription each time.
        // Renewal is handled by recurring invoice generation, not by overwriting
        // currently active subscription on the same location.
        let subscription_id = Uuid::new_v4().to_string();
        let currency = "IDR".to_string();
        let notes = Some("Self-service checkout".to_string());

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO customer_subscriptions
              (id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at)
            VALUES
              ($1,$2,$3,$4,$5,NULL,$6,$7,$8,'pending_installation',NULL,NULL,$9,$10,$11)
            "#,
        )
        .bind(&subscription_id)
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(&location_id)
        .bind(&package_id)
        .bind(&billing_cycle)
        .bind(price)
        .bind(&currency)
        .bind(&notes)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO customer_subscriptions
              (id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at)
            VALUES
              (?,?,?,?,?,NULL,?,?,?,'pending_installation',NULL,NULL,?,?,?)
            "#,
        )
        .bind(&subscription_id)
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(&location_id)
        .bind(&package_id)
        .bind(&billing_cycle)
        .bind(price)
        .bind(&currency)
        .bind(&notes)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let row: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price::float8 as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE id = $1 AND tenant_id = $2",
        )
        .bind(&subscription_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE id = ? AND tenant_id = ?",
        )
        .bind(&subscription_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_SUBSCRIPTION_ORDER_REQUEST",
                "customer_subscriptions",
                Some(&subscription_id),
                Some("Customer portal created a subscription order request"),
                ip_address,
            )
            .await;

        Ok(row)
    }

    pub async fn create_my_subscription_order_request(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: PortalCheckoutSubscriptionRequest,
        ip_address: Option<&str>,
    ) -> AppResult<(CustomerSubscription, InstallationWorkOrder)> {
        let subscription = self
            .create_my_subscription(actor_id, tenant_id, dto, ip_address)
            .await?;

        let (work_order, _created) = self
            .ensure_installation_work_order_for_subscription(tenant_id, &subscription)
            .await?;

        if let Err(err) = self
            .notify_new_installation_request(tenant_id, &subscription, &work_order)
            .await
        {
            warn!(
                "failed to send new installation request notification: tenant_id={}, subscription_id={}, work_order_id={}, error={}",
                tenant_id, subscription.id, work_order.id, err
            );
        }

        Ok((subscription, work_order))
    }

    pub async fn reopen_my_subscription_order_request(
        &self,
        actor_id: &str,
        tenant_id: &str,
        subscription_id: &str,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<(CustomerSubscription, InstallationWorkOrder)> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read_own")
            .await?;

        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let mut sub: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price::float8 as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2 AND customer_id = $3 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(&customer_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let mut sub: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE tenant_id = ? AND id = ? AND customer_id = ? LIMIT 1",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(&customer_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        #[cfg(feature = "postgres")]
        let latest_work_order_status: Option<String> = sqlx::query_scalar(
            r#"
            SELECT status
            FROM installation_work_orders
            WHERE tenant_id = $1
              AND subscription_id = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let latest_work_order_status: Option<String> = sqlx::query_scalar(
            r#"
            SELECT status
            FROM installation_work_orders
            WHERE tenant_id = ?
              AND subscription_id = ?
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        let has_cancelled_state =
            sub.status == "cancelled" || latest_work_order_status.as_deref() == Some("cancelled");
        if !has_cancelled_state {
            return Err(AppError::Validation(
                "Only cancelled subscription/order can be reopened".to_string(),
            ));
        }

        self.set_customer_subscription_status(tenant_id, &sub.id, "pending_installation")
            .await?;
        sub.status = "pending_installation".to_string();
        sub.updated_at = Utc::now();

        let (work_order, _created) = self
            .ensure_installation_work_order_for_subscription(tenant_id, &sub)
            .await?;

        let mut note = "Reopened by customer request from portal".to_string();
        if let Some(extra) = notes.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
            note.push_str(". ");
            note.push_str(extra);
        }
        let merged_notes =
            Self::merge_work_order_notes(work_order.notes.clone(), actor_id, Some(&note));
        let now = Utc::now();
        #[cfg(feature = "postgres")]
        let _ = sqlx::query(
            "UPDATE installation_work_orders SET notes = $1, updated_at = $2 WHERE tenant_id = $3 AND id = $4",
        )
        .bind(&merged_notes)
        .bind(now)
        .bind(tenant_id)
        .bind(&work_order.id)
        .execute(&self.pool)
        .await;
        #[cfg(feature = "sqlite")]
        let _ = sqlx::query(
            "UPDATE installation_work_orders SET notes = ?, updated_at = ? WHERE tenant_id = ? AND id = ?",
        )
        .bind(&merged_notes)
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(&work_order.id)
        .execute(&self.pool)
        .await;

        if let Err(err) = self
            .notify_new_installation_request(tenant_id, &sub, &work_order)
            .await
        {
            warn!(
                "failed to notify tenant about customer reopen request: tenant_id={}, subscription_id={}, work_order_id={}, error={}",
                tenant_id, sub.id, work_order.id, err
            );
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_ORDER_REQUEST_REOPEN",
                "customer_subscriptions",
                Some(&sub.id),
                Some("Customer requested installation order reopen"),
                ip_address,
            )
            .await;

        Ok((sub, work_order))
    }

    pub async fn get_my_subscription_installation_tracker(
        &self,
        actor_id: &str,
        tenant_id: &str,
        subscription_id: &str,
    ) -> AppResult<(
        CustomerSubscriptionView,
        Option<InstallationWorkOrderView>,
        Option<WorkOrderRescheduleRequestView>,
    )> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read_own")
            .await?;

        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let subscription: CustomerSubscriptionView = sqlx::query_as(
            r#"
            SELECT
              cs.id, cs.tenant_id, cs.customer_id, cs.location_id, cs.package_id, cs.router_id,
              cs.billing_cycle, cs.price::float8 as price, cs.currency_code, cs.status,
              cs.starts_at, cs.ends_at, cs.notes, cs.created_at, cs.updated_at,
              p.name AS package_name,
              l.label AS location_label,
              r.name AS router_name,
              (
                SELECT iwo.id
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_id,
              (
                SELECT iwo.status
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_status,
              (
                (
                  cs.status = 'cancelled'
                ) OR EXISTS (
                  SELECT 1
                  FROM installation_work_orders iwo
                  WHERE iwo.tenant_id = cs.tenant_id
                    AND iwo.subscription_id = cs.id
                    AND iwo.status = 'cancelled'
                )
              ) AS can_request_reopen,
              (
                SELECT worr.status
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_status,
              (
                SELECT CAST(worr.requested_schedule_at AS TEXT)
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p
              ON p.tenant_id = cs.tenant_id
             AND p.id = cs.package_id
            LEFT JOIN customer_locations l
              ON l.tenant_id = cs.tenant_id
             AND l.id = cs.location_id
            LEFT JOIN mikrotik_routers r
              ON r.tenant_id = cs.tenant_id
             AND r.id = cs.router_id
            WHERE cs.tenant_id = $1
              AND cs.customer_id = $2
              AND cs.id = $3
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let subscription: CustomerSubscriptionView = sqlx::query_as(
            r#"
            SELECT
              cs.id, cs.tenant_id, cs.customer_id, cs.location_id, cs.package_id, cs.router_id,
              cs.billing_cycle, cs.price as price, cs.currency_code, cs.status,
              cs.starts_at, cs.ends_at, cs.notes, cs.created_at, cs.updated_at,
              p.name AS package_name,
              l.label AS location_label,
              r.name AS router_name,
              (
                SELECT iwo.id
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_id,
              (
                SELECT iwo.status
                FROM installation_work_orders iwo
                WHERE iwo.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY iwo.created_at DESC
                LIMIT 1
              ) AS latest_work_order_status,
              (
                (
                  cs.status = 'cancelled'
                ) OR EXISTS (
                  SELECT 1
                  FROM installation_work_orders iwo
                  WHERE iwo.tenant_id = cs.tenant_id
                    AND iwo.subscription_id = cs.id
                    AND iwo.status = 'cancelled'
                )
              ) AS can_request_reopen,
              (
                SELECT worr.status
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_status,
              (
                SELECT CAST(worr.requested_schedule_at AS TEXT)
                FROM work_order_reschedule_requests worr
                JOIN installation_work_orders iwo ON iwo.id = worr.work_order_id
                WHERE worr.tenant_id = cs.tenant_id
                  AND iwo.subscription_id = cs.id
                ORDER BY worr.created_at DESC
                LIMIT 1
              ) AS latest_reschedule_requested_at
            FROM customer_subscriptions cs
            LEFT JOIN isp_packages p
              ON p.tenant_id = cs.tenant_id
             AND p.id = cs.package_id
            LEFT JOIN customer_locations l
              ON l.tenant_id = cs.tenant_id
             AND l.id = cs.location_id
            LEFT JOIN mikrotik_routers r
              ON r.tenant_id = cs.tenant_id
             AND r.id = cs.router_id
            WHERE cs.tenant_id = ?
              AND cs.customer_id = ?
              AND cs.id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        #[cfg(feature = "postgres")]
        let work_order: Option<InstallationWorkOrderView> = sqlx::query_as(
            r#"
            SELECT
              wo.id, wo.tenant_id, wo.subscription_id, wo.invoice_id, wo.customer_id, wo.location_id,
              cs.package_id AS package_id,
              COALESCE(wo.router_id, cs.router_id) AS router_id,
              wo.status, wo.assigned_to, wo.scheduled_at, wo.completed_at, wo.notes, wo.created_at, wo.updated_at,
              c.name AS customer_name,
              l.label AS location_label,
              p.name AS package_name,
              r.name AS router_name,
              u.name AS assigned_to_name,
              u.email AS assigned_to_email,
              csa.id AS assignment_id,
              csa.status AS assignment_status,
              cs.status AS subscription_status,
              cs.starts_at AS subscription_starts_at,
              EXISTS(
                SELECT 1
                FROM invoices i
                WHERE i.tenant_id = wo.tenant_id
                  AND (
                    i.external_id = 'pkgsub:' || wo.subscription_id
                    OR i.external_id LIKE 'pkgsub:' || wo.subscription_id || ':%'
                  )
              ) AS has_customer_package_invoice,
              csa.selected_zone_id AS selected_zone_id,
              sz.name AS selected_zone_name,
              csa.selected_node_id AS selected_node_id,
              nn.name AS selected_node_name,
              csa.selected_node_score::float8 AS selected_node_score,
              csa.path_node_ids AS path_node_ids,
              csa.path_link_ids AS path_link_ids
            FROM installation_work_orders wo
            LEFT JOIN customers c ON c.tenant_id = wo.tenant_id AND c.id = wo.customer_id
            LEFT JOIN customer_locations l ON l.tenant_id = wo.tenant_id AND l.id = wo.location_id
            LEFT JOIN customer_subscriptions cs ON cs.tenant_id = wo.tenant_id AND cs.id = wo.subscription_id
            LEFT JOIN isp_packages p ON p.tenant_id = wo.tenant_id AND p.id = cs.package_id
            LEFT JOIN mikrotik_routers r
              ON r.tenant_id = wo.tenant_id
             AND r.id = COALESCE(wo.router_id, cs.router_id)
            LEFT JOIN users u ON u.id = wo.assigned_to
            LEFT JOIN customer_service_assignments csa ON csa.tenant_id = wo.tenant_id AND csa.work_order_id = wo.id
            LEFT JOIN service_zones sz ON sz.tenant_id = wo.tenant_id::uuid AND sz.id::text = csa.selected_zone_id
            LEFT JOIN network_nodes nn ON nn.tenant_id = wo.tenant_id::uuid AND nn.id::text = csa.selected_node_id
            WHERE wo.tenant_id = $1
              AND wo.customer_id = $2
              AND wo.subscription_id = $3
            ORDER BY wo.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let work_order: Option<InstallationWorkOrderView> = sqlx::query_as(
            r#"
            SELECT
              wo.id, wo.tenant_id, wo.subscription_id, wo.invoice_id, wo.customer_id, wo.location_id,
              cs.package_id AS package_id,
              COALESCE(wo.router_id, cs.router_id) AS router_id,
              wo.status, wo.assigned_to, wo.scheduled_at, wo.completed_at, wo.notes, wo.created_at, wo.updated_at,
              c.name AS customer_name,
              l.label AS location_label,
              p.name AS package_name,
              r.name AS router_name,
              u.name AS assigned_to_name,
              u.email AS assigned_to_email,
              csa.id AS assignment_id,
              csa.status AS assignment_status,
              cs.status AS subscription_status,
              cs.starts_at AS subscription_starts_at,
              EXISTS(
                SELECT 1
                FROM invoices i
                WHERE i.tenant_id = wo.tenant_id
                  AND (
                    i.external_id = 'pkgsub:' || wo.subscription_id
                    OR i.external_id LIKE 'pkgsub:' || wo.subscription_id || ':%'
                  )
              ) AS has_customer_package_invoice,
              csa.selected_zone_id AS selected_zone_id,
              sz.name AS selected_zone_name,
              csa.selected_node_id AS selected_node_id,
              nn.name AS selected_node_name,
              csa.selected_node_score AS selected_node_score,
              csa.path_node_ids AS path_node_ids,
              csa.path_link_ids AS path_link_ids
            FROM installation_work_orders wo
            LEFT JOIN customers c ON c.tenant_id = wo.tenant_id AND c.id = wo.customer_id
            LEFT JOIN customer_locations l ON l.tenant_id = wo.tenant_id AND l.id = wo.location_id
            LEFT JOIN customer_subscriptions cs ON cs.tenant_id = wo.tenant_id AND cs.id = wo.subscription_id
            LEFT JOIN isp_packages p ON p.tenant_id = wo.tenant_id AND p.id = cs.package_id
            LEFT JOIN mikrotik_routers r
              ON r.tenant_id = wo.tenant_id
             AND r.id = COALESCE(wo.router_id, cs.router_id)
            LEFT JOIN users u ON u.id = wo.assigned_to
            LEFT JOIN customer_service_assignments csa ON csa.tenant_id = wo.tenant_id AND csa.work_order_id = wo.id
            LEFT JOIN service_zones sz ON sz.tenant_id = wo.tenant_id AND sz.id = csa.selected_zone_id
            LEFT JOIN network_nodes nn ON nn.tenant_id = wo.tenant_id AND nn.id = csa.selected_node_id
            WHERE wo.tenant_id = ?
              AND wo.customer_id = ?
              AND wo.subscription_id = ?
            ORDER BY wo.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let latest_reschedule_request: Option<WorkOrderRescheduleRequestView> =
            if let Some(ref wo) = work_order {
                sqlx::query_as(
                    r#"
                    SELECT
                      r.id,
                      r.work_order_id,
                      CAST(r.requested_schedule_at AS TEXT) AS requested_schedule_at,
                      r.reason,
                      r.status,
                      req.name AS requested_by_name,
                      req.email AS requested_by_email,
                      rev.name AS reviewed_by_name,
                      CAST(r.reviewed_at AS TEXT) AS reviewed_at,
                      r.review_notes,
                      CAST(r.created_at AS TEXT) AS created_at
                    FROM work_order_reschedule_requests r
                    LEFT JOIN users req ON req.id = r.requested_by
                    LEFT JOIN users rev ON rev.id = r.reviewed_by
                    WHERE r.tenant_id = $1
                      AND r.work_order_id = $2
                    ORDER BY r.created_at DESC
                    LIMIT 1
                    "#,
                )
                .bind(tenant_id)
                .bind(&wo.id)
                .fetch_optional(&self.pool)
                .await?
            } else {
                None
            };

        #[cfg(feature = "sqlite")]
        let latest_reschedule_request: Option<WorkOrderRescheduleRequestView> =
            if let Some(ref wo) = work_order {
                sqlx::query_as(
                    r#"
                    SELECT
                      r.id,
                      r.work_order_id,
                      CAST(r.requested_schedule_at AS TEXT) AS requested_schedule_at,
                      r.reason,
                      r.status,
                      req.name AS requested_by_name,
                      req.email AS requested_by_email,
                      rev.name AS reviewed_by_name,
                      CAST(r.reviewed_at AS TEXT) AS reviewed_at,
                      r.review_notes,
                      CAST(r.created_at AS TEXT) AS created_at
                    FROM work_order_reschedule_requests r
                    LEFT JOIN users req ON req.id = r.requested_by
                    LEFT JOIN users rev ON rev.id = r.reviewed_by
                    WHERE r.tenant_id = ?
                      AND r.work_order_id = ?
                    ORDER BY r.created_at DESC
                    LIMIT 1
                    "#,
                )
                .bind(tenant_id)
                .bind(&wo.id)
                .fetch_optional(&self.pool)
                .await?
            } else {
                None
            };

        Ok((subscription, work_order, latest_reschedule_request))
    }

    pub async fn request_my_subscription_reschedule(
        &self,
        actor_id: &str,
        tenant_id: &str,
        subscription_id: &str,
        requested_at: String,
        reason: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<(CustomerSubscription, InstallationWorkOrder)> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "customers", "read_own")
            .await?;

        let customer_id = self.get_portal_customer_id(actor_id, tenant_id).await?;

        #[cfg(feature = "postgres")]
        let sub: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price::float8 as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2 AND customer_id = $3 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(&customer_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let sub: CustomerSubscription = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE tenant_id = ? AND id = ? AND customer_id = ? LIMIT 1",
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(&customer_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        let requested_dt = Self::parse_optional_datetime(Some(requested_at.clone()))?
            .ok_or_else(|| AppError::Validation("Requested schedule is required".to_string()))?;
        let now = Utc::now();
        if requested_dt < (now + Duration::hours(2)) {
            return Err(AppError::Validation(
                "Reschedule must be at least 2 hours from now".to_string(),
            ));
        }

        let (current_sub, current_wo_opt, _current_reschedule) = self
            .get_my_subscription_installation_tracker(actor_id, tenant_id, &sub.id)
            .await?;
        let current_wo_view = current_wo_opt.ok_or_else(|| {
            AppError::Validation(
                "No installation work order found for this subscription".to_string(),
            )
        })?;
        if current_wo_view.status != "pending" {
            return Err(AppError::Validation(
                "Reschedule is only allowed before installation starts".to_string(),
            ));
        }
        let current_wo = self
            .get_installation_work_order_row(tenant_id, &current_wo_view.id)
            .await?;

        let reason_txt = reason
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("Customer requested schedule change");
        #[cfg(feature = "postgres")]
        let pending_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM work_order_reschedule_requests
              WHERE tenant_id = $1
                AND work_order_id = $2
                AND status = 'pending'
            )
            "#,
        )
        .bind(tenant_id)
        .bind(&current_wo_view.id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let pending_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM work_order_reschedule_requests
              WHERE tenant_id = ?
                AND work_order_id = ?
                AND status = 'pending'
            )
            "#,
        )
        .bind(tenant_id)
        .bind(&current_wo_view.id)
        .fetch_one(&self.pool)
        .await?;

        if pending_exists {
            return Err(AppError::Validation(
                "There is already a pending reschedule request for this work order".to_string(),
            ));
        }

        let request_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO work_order_reschedule_requests
              (id, tenant_id, work_order_id, subscription_id, requested_by, requested_schedule_at, reason, status, created_at, updated_at)
            VALUES
              ($1, $2, $3, $4, $5, $6, $7, 'pending', $8, $8)
            "#,
        )
        .bind(&request_id)
        .bind(tenant_id)
        .bind(&current_wo_view.id)
        .bind(&sub.id)
        .bind(actor_id)
        .bind(requested_dt)
        .bind(reason_txt)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO work_order_reschedule_requests
              (id, tenant_id, work_order_id, subscription_id, requested_by, requested_schedule_at, reason, status, created_at, updated_at)
            VALUES
              (?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?)
            "#,
        )
        .bind(&request_id)
        .bind(tenant_id)
        .bind(&current_wo_view.id)
        .bind(&sub.id)
        .bind(actor_id)
        .bind(requested_dt.to_rfc3339())
        .bind(reason_txt)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "CUSTOMER_PORTAL_RESCHEDULE_REQUEST",
                "customer_subscriptions",
                Some(&current_sub.id),
                Some("Customer requested installation reschedule"),
                ip_address,
            )
            .await;

        if let Err(err) = self
            .notify_installation_rescheduled(tenant_id, &sub, &current_wo, reason_txt)
            .await
        {
            warn!(
                "failed to send installation reschedule notification: tenant_id={}, subscription_id={}, work_order_id={}, error={}",
                tenant_id, sub.id, current_wo.id, err
            );
        }

        Ok((sub, current_wo))
    }

    pub async fn get_pending_work_order_reschedule_request(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
    ) -> AppResult<Option<WorkOrderRescheduleRequestView>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "read")
            .await?;

        #[cfg(feature = "postgres")]
        let row: Option<WorkOrderRescheduleRequestView> = sqlx::query_as(
            r#"
            SELECT
              r.id,
              r.work_order_id,
              CAST(r.requested_schedule_at AS TEXT) AS requested_schedule_at,
              r.reason,
              r.status,
              req.name AS requested_by_name,
              req.email AS requested_by_email,
              rev.name AS reviewed_by_name,
              CAST(r.reviewed_at AS TEXT) AS reviewed_at,
              r.review_notes,
              CAST(r.created_at AS TEXT) AS created_at
            FROM work_order_reschedule_requests r
            LEFT JOIN users req ON req.id = r.requested_by
            LEFT JOIN users rev ON rev.id = r.reviewed_by
            WHERE r.tenant_id = $1
              AND r.work_order_id = $2
              AND r.status = 'pending'
            ORDER BY r.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: Option<WorkOrderRescheduleRequestView> = sqlx::query_as(
            r#"
            SELECT
              r.id,
              r.work_order_id,
              CAST(r.requested_schedule_at AS TEXT) AS requested_schedule_at,
              r.reason,
              r.status,
              req.name AS requested_by_name,
              req.email AS requested_by_email,
              rev.name AS reviewed_by_name,
              CAST(r.reviewed_at AS TEXT) AS reviewed_at,
              r.review_notes,
              CAST(r.created_at AS TEXT) AS created_at
            FROM work_order_reschedule_requests r
            LEFT JOIN users req ON req.id = r.requested_by
            LEFT JOIN users rev ON rev.id = r.reviewed_by
            WHERE r.tenant_id = ?
              AND r.work_order_id = ?
              AND r.status = 'pending'
            ORDER BY r.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn approve_work_order_reschedule_request(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        dto: WorkOrderRescheduleDecisionRequest,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;
        let is_admin_owner = self.is_actor_admin_or_owner(tenant_id, actor_id).await?;
        let current = self
            .get_installation_work_order_row(tenant_id, work_order_id)
            .await?;
        let is_assigned_technician = current
            .assigned_to
            .as_deref()
            .map(str::trim)
            .map(|v| v == actor_id)
            .unwrap_or(false);
        if !is_admin_owner && !is_assigned_technician {
            return Err(AppError::Forbidden(
                "Only admin/owner or assigned technician can approve reschedule request"
                    .to_string(),
            ));
        }

        let pending = self
            .get_pending_work_order_reschedule_request(actor_id, tenant_id, work_order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("No pending reschedule request".to_string()))?;

        let target_schedule = dto
            .scheduled_at
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| pending.requested_schedule_at.clone());

        let note = format!(
            "Reschedule approved. New schedule: {}{}",
            target_schedule,
            dto.notes
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(|v| format!(". Notes: {}", v))
                .unwrap_or_default()
        );

        let row = self
            .set_installation_work_order_status_internal(
                actor_id,
                tenant_id,
                work_order_id,
                Some("pending"),
                None,
                Some(target_schedule),
                Some(note),
                false,
                ip_address,
                "WORK_ORDER_RESCHEDULE_APPROVE",
                "Approved work order reschedule request",
            )
            .await?;

        let now = Utc::now();
        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE work_order_reschedule_requests
            SET status = 'approved',
                reviewed_by = $1,
                reviewed_at = $2,
                review_notes = $3,
                updated_at = $2
            WHERE tenant_id = $4
              AND id = $5
            "#,
        )
        .bind(actor_id)
        .bind(now)
        .bind(dto.notes)
        .bind(tenant_id)
        .bind(&pending.id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE work_order_reschedule_requests
            SET status = 'approved',
                reviewed_by = ?,
                reviewed_at = ?,
                review_notes = ?,
                updated_at = ?
            WHERE tenant_id = ?
              AND id = ?
            "#,
        )
        .bind(actor_id)
        .bind(now.to_rfc3339())
        .bind(dto.notes)
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(&pending.id)
        .execute(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn reject_work_order_reschedule_request(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        dto: WorkOrderRescheduleDecisionRequest,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;
        let is_admin_owner = self.is_actor_admin_or_owner(tenant_id, actor_id).await?;
        let current = self
            .get_installation_work_order_row(tenant_id, work_order_id)
            .await?;
        let is_assigned_technician = current
            .assigned_to
            .as_deref()
            .map(str::trim)
            .map(|v| v == actor_id)
            .unwrap_or(false);
        if !is_admin_owner && !is_assigned_technician {
            return Err(AppError::Forbidden(
                "Only admin/owner or assigned technician can reject reschedule request".to_string(),
            ));
        }

        let pending = self
            .get_pending_work_order_reschedule_request(actor_id, tenant_id, work_order_id)
            .await?
            .ok_or_else(|| AppError::NotFound("No pending reschedule request".to_string()))?;

        let reason = dto
            .notes
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| AppError::Validation("Rejection reason is required".to_string()))?;

        let row = self
            .set_installation_work_order_status_internal(
                actor_id,
                tenant_id,
                work_order_id,
                None,
                None,
                None,
                Some(format!("Reschedule request rejected. Reason: {}", reason)),
                false,
                ip_address,
                "WORK_ORDER_RESCHEDULE_REJECT",
                "Rejected work order reschedule request",
            )
            .await?;

        let now = Utc::now();
        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE work_order_reschedule_requests
            SET status = 'rejected',
                reviewed_by = $1,
                reviewed_at = $2,
                review_notes = $3,
                updated_at = $2
            WHERE tenant_id = $4
              AND id = $5
            "#,
        )
        .bind(actor_id)
        .bind(now)
        .bind(Some(reason.to_string()))
        .bind(tenant_id)
        .bind(&pending.id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE work_order_reschedule_requests
            SET status = 'rejected',
                reviewed_by = ?,
                reviewed_at = ?,
                review_notes = ?,
                updated_at = ?
            WHERE tenant_id = ?
              AND id = ?
            "#,
        )
        .bind(actor_id)
        .bind(now.to_rfc3339())
        .bind(Some(reason.to_string()))
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(&pending.id)
        .execute(&self.pool)
        .await?;

        Ok(row)
    }

    async fn ensure_installation_work_order_for_subscription(
        &self,
        tenant_id: &str,
        sub: &CustomerSubscription,
    ) -> AppResult<(InstallationWorkOrder, bool)> {
        #[cfg(feature = "postgres")]
        let existing: Option<InstallationWorkOrder> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = $1
              AND subscription_id = $2
              AND status IN ('pending', 'in_progress')
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&sub.id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let existing: Option<InstallationWorkOrder> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = ?
              AND subscription_id = ?
              AND status IN ('pending', 'in_progress')
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&sub.id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = existing {
            return Ok((row, false));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let notes = Some(
            "Created from customer order request; awaiting assignment and schedule.".to_string(),
        );

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO installation_work_orders
              (id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, notes, created_at, updated_at)
            VALUES
              ($1,$2,$3,NULL,$4,$5,$6,'pending',$7,$8,$9)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(&sub.id)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(&sub.router_id)
        .bind(&notes)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            INSERT INTO installation_work_orders
              (id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, notes, created_at, updated_at)
            VALUES
              (?,?,?,NULL,?,?,?,'pending',?,?,?)
            "#,
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(&sub.id)
        .bind(&sub.customer_id)
        .bind(&sub.location_id)
        .bind(&sub.router_id)
        .bind(notes.clone())
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "postgres")]
        let row: InstallationWorkOrder = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&id)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: InstallationWorkOrder = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = ? AND id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&id)
        .fetch_one(&self.pool)
        .await?;

        Ok((row, true))
    }

    async fn has_paid_customer_package_invoice_for_subscription(
        &self,
        tenant_id: &str,
        subscription_id: &str,
    ) -> AppResult<bool> {
        #[cfg(feature = "postgres")]
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM invoices
              WHERE tenant_id = $1
                AND status = 'paid'
                AND (
                    external_id = $2
                    OR external_id LIKE $3
                )
            )
            "#,
        )
        .bind(tenant_id)
        .bind(format!(
            "{}{}",
            CUSTOMER_PACKAGE_INVOICE_PREFIX, subscription_id
        ))
        .bind(format!(
            "{}{}:%",
            CUSTOMER_PACKAGE_INVOICE_PREFIX, subscription_id
        ))
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM invoices
              WHERE tenant_id = ?
                AND status = 'paid'
                AND (
                    external_id = ?
                    OR external_id LIKE ?
                )
            )
            "#,
        )
        .bind(tenant_id)
        .bind(format!(
            "{}{}",
            CUSTOMER_PACKAGE_INVOICE_PREFIX, subscription_id
        ))
        .bind(format!(
            "{}{}:%",
            CUSTOMER_PACKAGE_INVOICE_PREFIX, subscription_id
        ))
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }

    async fn set_customer_subscription_status(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        status: &str,
    ) -> AppResult<()> {
        let now = Utc::now();
        #[cfg(feature = "postgres")]
        let rows = sqlx::query(
            r#"
            UPDATE customer_subscriptions
            SET status = $1,
                starts_at = CASE WHEN $1 = 'active' THEN COALESCE(starts_at, $2) ELSE starts_at END,
                updated_at = $2
            WHERE tenant_id = $3
              AND id = $4
            "#,
        )
        .bind(status)
        .bind(now)
        .bind(tenant_id)
        .bind(subscription_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        #[cfg(feature = "sqlite")]
        let rows = sqlx::query(
            r#"
            UPDATE customer_subscriptions
            SET status = ?,
                starts_at = CASE WHEN ? = 'active' THEN COALESCE(starts_at, ?) ELSE starts_at END,
                updated_at = ?
            WHERE tenant_id = ?
              AND id = ?
            "#,
        )
        .bind(status)
        .bind(status)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(subscription_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(AppError::NotFound(
                "Customer subscription not found".to_string(),
            ));
        }

        Ok(())
    }

    async fn set_location_pppoe_disabled_state(
        &self,
        tenant_id: &str,
        location_id: &str,
        disabled: bool,
    ) -> AppResult<u64> {
        self.pppoe_service
            .set_location_accounts_disabled_state(tenant_id, location_id, disabled)
            .await
    }

    async fn list_tenant_installation_alert_user_ids(
        &self,
        tenant_id: &str,
    ) -> AppResult<Vec<String>> {
        #[cfg(feature = "postgres")]
        let rows: Vec<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT tm.user_id, COALESCE(r.name, tm.role) AS role_name
            FROM tenant_members tm
            LEFT JOIN roles r
              ON r.id = tm.role_id
             AND (r.tenant_id = tm.tenant_id OR r.tenant_id IS NULL)
            WHERE tm.tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT tm.user_id, COALESCE(r.name, tm.role) AS role_name
            FROM tenant_members tm
            LEFT JOIN roles r
              ON r.id = tm.role_id
             AND (r.tenant_id = tm.tenant_id OR r.tenant_id IS NULL)
            WHERE tm.tenant_id = ?
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Self::filter_installation_request_user_ids(rows))
    }

    async fn notify_new_installation_request(
        &self,
        tenant_id: &str,
        sub: &CustomerSubscription,
        work_order: &InstallationWorkOrder,
    ) -> AppResult<()> {
        let recipient_ids = self
            .list_tenant_installation_alert_user_ids(tenant_id)
            .await?;
        if recipient_ids.is_empty() {
            return Ok(());
        }

        #[cfg(feature = "postgres")]
        let row: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT c.name, l.label, p.name
            FROM customer_subscriptions cs
            LEFT JOIN customers c ON c.tenant_id = cs.tenant_id AND c.id = cs.customer_id
            LEFT JOIN customer_locations l ON l.tenant_id = cs.tenant_id AND l.id = cs.location_id
            LEFT JOIN isp_packages p ON p.tenant_id = cs.tenant_id AND p.id = cs.package_id
            WHERE cs.tenant_id = $1 AND cs.id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&sub.id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let row: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            r#"
            SELECT c.name, l.label, p.name
            FROM customer_subscriptions cs
            LEFT JOIN customers c ON c.tenant_id = cs.tenant_id AND c.id = cs.customer_id
            LEFT JOIN customer_locations l ON l.tenant_id = cs.tenant_id AND l.id = cs.location_id
            LEFT JOIN isp_packages p ON p.tenant_id = cs.tenant_id AND p.id = cs.package_id
            WHERE cs.tenant_id = ? AND cs.id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&sub.id)
        .fetch_optional(&self.pool)
        .await?;

        let (customer_name, location_label, package_name) = row.unwrap_or((None, None, None));

        let title = "Installation Work Order: New Request".to_string();
        let message = format!(
            "New paid customer order is ready for assignment and scheduling. Customer: {} • Location: {} • Package: {} • Work Order: {}",
            customer_name.unwrap_or_else(|| "-".to_string()),
            location_label.unwrap_or_else(|| "-".to_string()),
            package_name.unwrap_or_else(|| "-".to_string()),
            work_order.id
        );

        for user_id in recipient_ids {
            self.notification_service
                .create_notification(
                    user_id,
                    Some(tenant_id.to_string()),
                    title.clone(),
                    message.clone(),
                    "info".to_string(),
                    "operations".to_string(),
                    Some("/admin/network/installations".to_string()),
                )
                .await?;
        }

        Ok(())
    }

    async fn notify_installation_rescheduled(
        &self,
        tenant_id: &str,
        sub: &CustomerSubscription,
        work_order: &InstallationWorkOrder,
        reason: &str,
    ) -> AppResult<()> {
        let mut recipient_ids = self
            .list_tenant_installation_alert_user_ids(tenant_id)
            .await?;
        if let Some(assigned_to) = work_order
            .assigned_to
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            if !recipient_ids.iter().any(|id| id == assigned_to) {
                recipient_ids.push(assigned_to.to_string());
            }
        }
        if recipient_ids.is_empty() {
            return Ok(());
        }

        let message = format!(
            "Customer requested installation reschedule. Work Order: {} • Requested schedule: {} • Reason: {}",
            work_order.id,
            work_order
                .scheduled_at
                .map(|v| v.to_rfc3339())
                .unwrap_or_else(|| "-".to_string()),
            reason
        );

        for user_id in recipient_ids {
            self.notification_service
                .create_notification(
                    user_id,
                    Some(tenant_id.to_string()),
                    "Installation Work Order: Reschedule Requested".to_string(),
                    message.clone(),
                    "info".to_string(),
                    "operations".to_string(),
                    Some("/admin/network/installations".to_string()),
                )
                .await?;
        }

        // Notify customer-side users too as confirmation.
        let customer_user_ids = self
            .list_customer_user_ids_for_subscription(tenant_id, &sub.id)
            .await?;
        for user_id in customer_user_ids {
            self.notification_service
                .create_notification(
                    user_id,
                    Some(tenant_id.to_string()),
                    "Reschedule Request Received".to_string(),
                    "Your reschedule request has been sent to admin/technician for review."
                        .to_string(),
                    "info".to_string(),
                    "operations".to_string(),
                    Some("/dashboard/services".to_string()),
                )
                .await?;
        }

        Ok(())
    }

    async fn run_installation_sla_reminders_for_tenant(
        &self,
        tenant_id: &str,
        overdue_minutes: i64,
        unscheduled_minutes: i64,
        cooldown_minutes: i64,
    ) -> AppResult<u64> {
        let overdue_rows = self
            .list_overdue_installation_work_orders(tenant_id, overdue_minutes, unscheduled_minutes)
            .await?;
        if overdue_rows.is_empty() {
            return Ok(0);
        }

        let recipient_ids = self
            .list_tenant_installation_alert_user_ids(tenant_id)
            .await?;
        if recipient_ids.is_empty() {
            return Ok(0);
        }

        let now = Utc::now();
        let reminder_cutoff = now - Duration::minutes(cooldown_minutes.max(1));
        let mut sent = 0_u64;

        for row in overdue_rows {
            let breach = Self::detect_installation_sla_breach(
                &row.status,
                row.scheduled_at,
                row.created_at,
                now,
                overdue_minutes,
                unscheduled_minutes,
            );
            let Some(breach_type) = breach else {
                continue;
            };

            let action_url = format!(
                "/admin/network/installations?workOrderId={}",
                row.work_order_id
            );
            let customer_label = row.customer_name.unwrap_or_else(|| "-".to_string());
            let location_label = row.location_label.unwrap_or_else(|| "-".to_string());
            let package_label = row.package_name.unwrap_or_else(|| "-".to_string());
            let title = "Installation SLA overdue".to_string();
            let message = match breach_type {
                InstallationSlaBreachType::ScheduledOverdue => {
                    let schedule_at = row.scheduled_at.unwrap_or(now);
                    let late_minutes = now.signed_duration_since(schedule_at).num_minutes().max(0);
                    format!(
                        "WO {} is overdue {} from schedule. Customer: {} • Location: {} • Package: {}",
                        row.work_order_id,
                        Self::format_elapsed_duration(late_minutes),
                        customer_label,
                        location_label,
                        package_label
                    )
                }
                InstallationSlaBreachType::PendingUnscheduled => {
                    let waiting_minutes = now
                        .signed_duration_since(row.created_at)
                        .num_minutes()
                        .max(0);
                    format!(
                        "WO {} is waiting {} without schedule/assignment. Customer: {} • Location: {} • Package: {}",
                        row.work_order_id,
                        Self::format_elapsed_duration(waiting_minutes),
                        customer_label,
                        location_label,
                        package_label
                    )
                }
            };

            for user_id in &recipient_ids {
                let recently_sent = self
                    .has_recent_installation_sla_notification(
                        user_id,
                        tenant_id,
                        &action_url,
                        reminder_cutoff,
                    )
                    .await?;
                if recently_sent {
                    continue;
                }

                self.notification_service
                    .create_notification(
                        user_id.clone(),
                        Some(tenant_id.to_string()),
                        title.clone(),
                        message.clone(),
                        "warning".to_string(),
                        "operations".to_string(),
                        Some(action_url.clone()),
                    )
                    .await?;
                sent += 1;
            }
        }

        Ok(sent)
    }

    async fn list_overdue_installation_work_orders(
        &self,
        tenant_id: &str,
        overdue_minutes: i64,
        unscheduled_minutes: i64,
    ) -> AppResult<Vec<OverdueInstallationReminderRow>> {
        let now = Utc::now();
        let scheduled_cutoff = now - Duration::minutes(overdue_minutes.max(1));
        let unscheduled_cutoff = now - Duration::minutes(unscheduled_minutes.max(1));

        #[cfg(feature = "postgres")]
        let rows: Vec<OverdueInstallationReminderRow> = sqlx::query_as(
            r#"
            SELECT
              wo.id AS work_order_id,
              wo.status,
              wo.scheduled_at,
              wo.created_at,
              c.name AS customer_name,
              l.label AS location_label,
              p.name AS package_name
            FROM installation_work_orders wo
            LEFT JOIN customers c
              ON c.tenant_id = wo.tenant_id
             AND c.id = wo.customer_id
            LEFT JOIN customer_locations l
              ON l.tenant_id = wo.tenant_id
             AND l.id = wo.location_id
            LEFT JOIN customer_subscriptions cs
              ON cs.tenant_id = wo.tenant_id
             AND cs.id = wo.subscription_id
            LEFT JOIN isp_packages p
              ON p.tenant_id = cs.tenant_id
             AND p.id = cs.package_id
            WHERE wo.tenant_id = $1
              AND wo.status IN ('pending', 'in_progress')
              AND (
                (wo.scheduled_at IS NOT NULL AND wo.scheduled_at <= $2)
                OR (wo.status = 'pending' AND wo.scheduled_at IS NULL AND wo.created_at <= $3)
              )
            ORDER BY wo.created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(scheduled_cutoff)
        .bind(unscheduled_cutoff)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<OverdueInstallationReminderRow> = sqlx::query_as(
            r#"
            SELECT
              wo.id AS work_order_id,
              wo.status,
              wo.scheduled_at,
              wo.created_at,
              c.name AS customer_name,
              l.label AS location_label,
              p.name AS package_name
            FROM installation_work_orders wo
            LEFT JOIN customers c
              ON c.tenant_id = wo.tenant_id
             AND c.id = wo.customer_id
            LEFT JOIN customer_locations l
              ON l.tenant_id = wo.tenant_id
             AND l.id = wo.location_id
            LEFT JOIN customer_subscriptions cs
              ON cs.tenant_id = wo.tenant_id
             AND cs.id = wo.subscription_id
            LEFT JOIN isp_packages p
              ON p.tenant_id = cs.tenant_id
             AND p.id = cs.package_id
            WHERE wo.tenant_id = ?
              AND wo.status IN ('pending', 'in_progress')
              AND (
                (wo.scheduled_at IS NOT NULL AND wo.scheduled_at <= ?)
                OR (wo.status = 'pending' AND wo.scheduled_at IS NULL AND wo.created_at <= ?)
              )
            ORDER BY wo.created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(scheduled_cutoff.to_rfc3339())
        .bind(unscheduled_cutoff.to_rfc3339())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn has_recent_installation_sla_notification(
        &self,
        user_id: &str,
        tenant_id: &str,
        action_url: &str,
        since: DateTime<Utc>,
    ) -> AppResult<bool> {
        #[cfg(feature = "postgres")]
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM notifications
              WHERE user_id = $1
                AND tenant_id = $2
                AND category = 'operations'
                AND title = 'Installation SLA overdue'
                AND action_url = $3
                AND created_at >= $4
            )
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .bind(action_url)
        .bind(since)
        .fetch_one(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
              SELECT 1
              FROM notifications
              WHERE user_id = ?
                AND tenant_id = ?
                AND category = 'operations'
                AND title = 'Installation SLA overdue'
                AND action_url = ?
                AND created_at >= ?
            )
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .bind(action_url)
        .bind(since.to_rfc3339())
        .fetch_one(&self.pool)
        .await?;

        Ok(exists)
    }

    pub async fn list_installation_work_orders(
        &self,
        actor_id: &str,
        tenant_id: &str,
        status: Option<String>,
        assigned_to: Option<String>,
        include_closed: bool,
        limit: u32,
    ) -> AppResult<Vec<InstallationWorkOrderView>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "read")
            .await?;

        let limit = limit.clamp(1, 500);
        let status_filter = status
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(Self::normalize_work_order_status)
            .transpose()?;
        let assigned_filter = assigned_to
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let is_admin_owner = self.is_actor_admin_or_owner(tenant_id, actor_id).await?;

        #[cfg(feature = "postgres")]
        let rows: Vec<InstallationWorkOrderView> = sqlx::query_as(
            r#"
            SELECT
              wo.id, wo.tenant_id, wo.subscription_id, wo.invoice_id, wo.customer_id, wo.location_id,
              cs.package_id AS package_id,
              COALESCE(wo.router_id, cs.router_id) AS router_id,
              wo.status, wo.assigned_to, wo.scheduled_at, wo.completed_at, wo.notes, wo.created_at, wo.updated_at,
              c.name AS customer_name,
              l.label AS location_label,
              p.name AS package_name,
              r.name AS router_name,
              u.name AS assigned_to_name,
              u.email AS assigned_to_email,
              csa.id AS assignment_id,
              csa.status AS assignment_status,
              cs.status AS subscription_status,
              cs.starts_at AS subscription_starts_at,
              EXISTS(
                SELECT 1
                FROM invoices i
                WHERE i.tenant_id = wo.tenant_id
                  AND (
                    i.external_id = 'pkgsub:' || wo.subscription_id
                    OR i.external_id LIKE 'pkgsub:' || wo.subscription_id || ':%'
                  )
              ) AS has_customer_package_invoice,
              csa.selected_zone_id AS selected_zone_id,
              sz.name AS selected_zone_name,
              csa.selected_node_id AS selected_node_id,
              nn.name AS selected_node_name,
              csa.selected_node_score::float8 AS selected_node_score,
              csa.path_node_ids AS path_node_ids,
              csa.path_link_ids AS path_link_ids
            FROM installation_work_orders wo
            LEFT JOIN customers c ON c.tenant_id = wo.tenant_id AND c.id = wo.customer_id
            LEFT JOIN customer_locations l ON l.tenant_id = wo.tenant_id AND l.id = wo.location_id
            LEFT JOIN customer_subscriptions cs ON cs.tenant_id = wo.tenant_id AND cs.id = wo.subscription_id
            LEFT JOIN isp_packages p ON p.tenant_id = wo.tenant_id AND p.id = cs.package_id
            LEFT JOIN mikrotik_routers r
              ON r.tenant_id = wo.tenant_id
             AND r.id = COALESCE(wo.router_id, cs.router_id)
            LEFT JOIN users u ON u.id = wo.assigned_to
            LEFT JOIN customer_service_assignments csa ON csa.tenant_id = wo.tenant_id AND csa.work_order_id = wo.id
            LEFT JOIN service_zones sz ON sz.tenant_id = wo.tenant_id::uuid AND sz.id::text = csa.selected_zone_id
            LEFT JOIN network_nodes nn ON nn.tenant_id = wo.tenant_id::uuid AND nn.id::text = csa.selected_node_id
            WHERE wo.tenant_id = $1
              AND ($2::text IS NULL OR wo.status = $2)
              AND ($3::text IS NULL OR wo.assigned_to = $3)
              AND (
                $4::bool
                OR wo.status NOT IN ('completed', 'cancelled')
                OR (
                  wo.status = 'completed'
                  AND LOWER(COALESCE(cs.status, '')) = 'pending_installation'
                )
              )
              AND (
                $5::bool
                OR wo.assigned_to = $6
                OR (
                  wo.status = 'pending'
                  AND (wo.assigned_to IS NULL OR btrim(wo.assigned_to) = '')
                )
              )
            ORDER BY
              CASE wo.status
                WHEN 'pending' THEN 0
                WHEN 'in_progress' THEN 1
                WHEN 'completed' THEN 2
                WHEN 'cancelled' THEN 3
                ELSE 4
              END ASC,
              wo.updated_at DESC
            LIMIT $7
            "#,
        )
        .bind(tenant_id)
        .bind(status_filter)
        .bind(assigned_filter)
        .bind(include_closed)
        .bind(is_admin_owner)
        .bind(actor_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let rows: Vec<InstallationWorkOrderView> = sqlx::query_as(
            r#"
            SELECT
              wo.id, wo.tenant_id, wo.subscription_id, wo.invoice_id, wo.customer_id, wo.location_id,
              cs.package_id AS package_id,
              COALESCE(wo.router_id, cs.router_id) AS router_id,
              wo.status, wo.assigned_to, wo.scheduled_at, wo.completed_at, wo.notes, wo.created_at, wo.updated_at,
              c.name AS customer_name,
              l.label AS location_label,
              p.name AS package_name,
              r.name AS router_name,
              u.name AS assigned_to_name,
              u.email AS assigned_to_email,
              csa.id AS assignment_id,
              csa.status AS assignment_status,
              cs.status AS subscription_status,
              cs.starts_at AS subscription_starts_at,
              EXISTS(
                SELECT 1
                FROM invoices i
                WHERE i.tenant_id = wo.tenant_id
                  AND (
                    i.external_id = 'pkgsub:' || wo.subscription_id
                    OR i.external_id LIKE 'pkgsub:' || wo.subscription_id || ':%'
                  )
              ) AS has_customer_package_invoice,
              csa.selected_zone_id AS selected_zone_id,
              sz.name AS selected_zone_name,
              csa.selected_node_id AS selected_node_id,
              nn.name AS selected_node_name,
              csa.selected_node_score AS selected_node_score,
              csa.path_node_ids AS path_node_ids,
              csa.path_link_ids AS path_link_ids
            FROM installation_work_orders wo
            LEFT JOIN customers c ON c.tenant_id = wo.tenant_id AND c.id = wo.customer_id
            LEFT JOIN customer_locations l ON l.tenant_id = wo.tenant_id AND l.id = wo.location_id
            LEFT JOIN customer_subscriptions cs ON cs.tenant_id = wo.tenant_id AND cs.id = wo.subscription_id
            LEFT JOIN isp_packages p ON p.tenant_id = wo.tenant_id AND p.id = cs.package_id
            LEFT JOIN mikrotik_routers r
              ON r.tenant_id = wo.tenant_id
             AND r.id = COALESCE(wo.router_id, cs.router_id)
            LEFT JOIN users u ON u.id = wo.assigned_to
            LEFT JOIN customer_service_assignments csa ON csa.tenant_id = wo.tenant_id AND csa.work_order_id = wo.id
            LEFT JOIN service_zones sz ON sz.tenant_id = wo.tenant_id AND sz.id = csa.selected_zone_id
            LEFT JOIN network_nodes nn ON nn.tenant_id = wo.tenant_id AND nn.id = csa.selected_node_id
            WHERE wo.tenant_id = ?
              AND (? IS NULL OR wo.status = ?)
              AND (? IS NULL OR wo.assigned_to = ?)
              AND (
                ? = 1
                OR wo.status NOT IN ('completed', 'cancelled')
                OR (
                  wo.status = 'completed'
                  AND LOWER(COALESCE(cs.status, '')) = 'pending_installation'
                )
              )
              AND (
                ? = 1
                OR wo.assigned_to = ?
                OR (
                  wo.status = 'pending'
                  AND (wo.assigned_to IS NULL OR trim(wo.assigned_to) = '')
                )
              )
            ORDER BY
              CASE wo.status
                WHEN 'pending' THEN 0
                WHEN 'in_progress' THEN 1
                WHEN 'completed' THEN 2
                WHEN 'cancelled' THEN 3
                ELSE 4
              END ASC,
              wo.updated_at DESC
            LIMIT ?
            "#,
        )
        .bind(tenant_id)
        .bind(&status_filter)
        .bind(&status_filter)
        .bind(&assigned_filter)
        .bind(&assigned_filter)
        .bind(if include_closed { 1 } else { 0 })
        .bind(if is_admin_owner { 1 } else { 0 })
        .bind(actor_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn assign_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        assigned_to: &str,
        scheduled_at: Option<String>,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        let current = self
            .get_installation_work_order_row(tenant_id, work_order_id)
            .await?;
        let is_admin_owner = self.is_actor_admin_or_owner(tenant_id, actor_id).await?;
        if !is_admin_owner {
            // Technician is allowed to save schedule/notes for own pending or in-progress work order,
            // but cannot reassign to another user.
            if current.status != "pending" && current.status != "in_progress" {
                return Err(AppError::Validation(
                    "Only pending or in-progress work order can be updated".to_string(),
                ));
            }

            let current_assigned = current.assigned_to.as_deref().map(str::trim).unwrap_or("");
            if current_assigned != actor_id {
                return Err(AppError::Forbidden(
                    "Technician can only update own assigned work order".to_string(),
                ));
            }

            if assigned_to.trim() != actor_id {
                return Err(AppError::Forbidden(
                    "Technician cannot reassign installation work order".to_string(),
                ));
            }
        }

        let assignee_eligible = self
            .is_installation_assignee_eligible(tenant_id, assigned_to)
            .await?;
        if !assignee_eligible {
            return Err(AppError::Validation(
                "Assignee must be an eligible installer (Admin/Technician or role with work_orders:manage)"
                    .to_string(),
            ));
        }

        self.set_installation_work_order_status_internal(
            actor_id,
            tenant_id,
            work_order_id,
            if current.status == "pending" {
                Some("pending")
            } else {
                None
            },
            Some(assigned_to),
            scheduled_at,
            notes,
            false,
            ip_address,
            "WORK_ORDER_ASSIGN",
            "Assigned installation work order",
        )
        .await
    }

    pub async fn claim_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        let eligible = self
            .is_installation_assignee_eligible(tenant_id, actor_id)
            .await?;
        if !eligible {
            return Err(AppError::Forbidden(
                "Only eligible installers can take installation work orders".to_string(),
            ));
        }

        let current = self
            .get_installation_work_order_row(tenant_id, work_order_id)
            .await?;
        if current.status != "pending" {
            return Err(AppError::Validation(
                "Only pending work order can be taken".to_string(),
            ));
        }
        if let Some(assigned) = current
            .assigned_to
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            if assigned != actor_id {
                return Err(AppError::Conflict(
                    "Work order already taken by another technician".to_string(),
                ));
            }
            return Ok(current);
        }

        let now = Utc::now();

        #[cfg(feature = "postgres")]
        let affected = sqlx::query(
            r#"
            UPDATE installation_work_orders
            SET assigned_to = $1, updated_at = $2
            WHERE tenant_id = $3
              AND id = $4
              AND status = 'pending'
              AND (assigned_to IS NULL OR btrim(assigned_to) = '')
            "#,
        )
        .bind(actor_id)
        .bind(now)
        .bind(tenant_id)
        .bind(work_order_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        #[cfg(feature = "sqlite")]
        let affected = sqlx::query(
            r#"
            UPDATE installation_work_orders
            SET assigned_to = ?, updated_at = ?
            WHERE tenant_id = ?
              AND id = ?
              AND status = 'pending'
              AND (assigned_to IS NULL OR trim(assigned_to) = '')
            "#,
        )
        .bind(actor_id)
        .bind(now)
        .bind(tenant_id)
        .bind(work_order_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::Conflict(
                "Work order already taken by another technician".to_string(),
            ));
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "WORK_ORDER_CLAIM",
                "installation_work_orders",
                Some(work_order_id),
                Some("Technician took installation work order"),
                ip_address,
            )
            .await;

        let mut row = self
            .get_installation_work_order_row(tenant_id, work_order_id)
            .await?;
        if notes
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .is_some()
        {
            row = self
                .set_installation_work_order_status_internal(
                    actor_id,
                    tenant_id,
                    work_order_id,
                    None,
                    None,
                    None,
                    notes,
                    false,
                    ip_address,
                    "WORK_ORDER_UPDATE_NOTE",
                    "Updated work order notes while claiming",
                )
                .await?;
        }

        Ok(row)
    }

    pub async fn release_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        if !self.is_actor_admin_or_owner(tenant_id, actor_id).await? {
            return Err(AppError::Forbidden(
                "Only admin/owner can release installation work orders".to_string(),
            ));
        }

        let current = self
            .get_installation_work_order_row(tenant_id, work_order_id)
            .await?;
        if current.status != "pending" {
            return Err(AppError::Validation(
                "Only pending work order can be released".to_string(),
            ));
        }

        // Release means making assignment empty and clearing schedule so next assignee can re-plan.
        self.set_installation_work_order_status_internal(
            actor_id,
            tenant_id,
            work_order_id,
            Some("pending"),
            Some(""),
            Some("".to_string()),
            notes,
            false,
            ip_address,
            "WORK_ORDER_RELEASE",
            "Released installation work order assignment",
        )
        .await
    }

    pub async fn start_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        let is_admin = self.is_actor_admin_or_owner(tenant_id, actor_id).await?;
        if !is_admin {
            let current = self
                .get_installation_work_order_row(tenant_id, work_order_id)
                .await?;
            let assigned = current.assigned_to.as_deref().map(str::trim).unwrap_or("");
            if assigned != actor_id {
                return Err(AppError::Forbidden(
                    "Technician can only start own assigned work order".to_string(),
                ));
            }
        }

        self.set_installation_work_order_status_internal(
            actor_id,
            tenant_id,
            work_order_id,
            Some("in_progress"),
            None,
            None,
            notes,
            false,
            ip_address,
            "WORK_ORDER_START",
            "Started installation work order",
        )
        .await
    }

    pub async fn complete_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        let is_admin = self.is_actor_admin_or_owner(tenant_id, actor_id).await?;
        if !is_admin {
            let current = self
                .get_installation_work_order_row(tenant_id, work_order_id)
                .await?;
            let assigned = current.assigned_to.as_deref().map(str::trim).unwrap_or("");
            if assigned != actor_id {
                return Err(AppError::Forbidden(
                    "Technician can only complete own assigned work order".to_string(),
                ));
            }
        }

        let row = self
            .set_installation_work_order_status_internal(
                actor_id,
                tenant_id,
                work_order_id,
                Some("completed"),
                None,
                None,
                notes,
                false,
                ip_address,
                "WORK_ORDER_COMPLETE",
                "Completed installation work order",
            )
            .await?;

        #[cfg(feature = "postgres")]
        let sub: Option<CustomerSubscription> = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price::float8 as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(&row.subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let sub: Option<CustomerSubscription> = sqlx::query_as(
            "SELECT id, tenant_id, customer_id, location_id, package_id, router_id, billing_cycle, price as price, currency_code, status, starts_at, ends_at, notes, created_at, updated_at FROM customer_subscriptions WHERE tenant_id = ? AND id = ?",
        )
        .bind(tenant_id)
        .bind(&row.subscription_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(mut s) = sub {
            if s.status != "cancelled" {
                let now = Utc::now();
                let has_paid_invoice = self
                    .has_paid_customer_package_invoice_for_subscription(tenant_id, &s.id)
                    .await?;

                if has_paid_invoice {
                    s.status = "active".to_string();
                    if s.starts_at.is_none() {
                        s.starts_at = Some(now);
                    }
                    s.updated_at = now;

                    #[cfg(feature = "postgres")]
                    sqlx::query(
                        r#"
                        UPDATE customer_subscriptions
                        SET status = 'active',
                            starts_at = COALESCE(starts_at, $1),
                            updated_at = $2
                        WHERE tenant_id = $3 AND id = $4
                        "#,
                    )
                    .bind(now)
                    .bind(s.updated_at)
                    .bind(tenant_id)
                    .bind(&s.id)
                    .execute(&self.pool)
                    .await?;

                    #[cfg(feature = "sqlite")]
                    sqlx::query(
                        r#"
                        UPDATE customer_subscriptions
                        SET status = 'active',
                            starts_at = COALESCE(starts_at, ?),
                            updated_at = ?
                        WHERE tenant_id = ? AND id = ?
                        "#,
                    )
                    .bind(now.to_rfc3339())
                    .bind(s.updated_at)
                    .bind(tenant_id)
                    .bind(&s.id)
                    .execute(&self.pool)
                    .await?;

                    let _ = self
                        .set_location_pppoe_disabled_state(tenant_id, &s.location_id, false)
                        .await;
                } else {
                    s.status = "pending_installation".to_string();
                    s.updated_at = now;
                    self.set_customer_subscription_status(tenant_id, &s.id, "pending_installation")
                        .await?;
                    let _ = self
                        .set_location_pppoe_disabled_state(tenant_id, &s.location_id, true)
                        .await;
                }

                let _ = self
                    .auto_provision_pppoe_for_subscription(actor_id, tenant_id, &s, ip_address)
                    .await;
            }
        }

        Ok(row)
    }

    pub async fn cancel_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        if !self.is_actor_admin_or_owner(tenant_id, actor_id).await? {
            return Err(AppError::Forbidden(
                "Only admin/owner can cancel installation work orders".to_string(),
            ));
        }

        let reason = notes
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                AppError::Validation(
                    "Cancellation reason is required (minimum 10 characters)".to_string(),
                )
            })?
            .to_string();

        if reason.chars().count() < 10 {
            return Err(AppError::Validation(
                "Cancellation reason is too short (minimum 10 characters)".to_string(),
            ));
        }

        let row = self
            .set_installation_work_order_status_internal(
                actor_id,
                tenant_id,
                work_order_id,
                Some("cancelled"),
                None,
                None,
                notes,
                false,
                ip_address,
                "WORK_ORDER_CANCEL",
                "Cancelled installation work order",
            )
            .await?;

        self.set_customer_subscription_status(tenant_id, &row.subscription_id, "cancelled")
            .await?;

        if let Err(err) = self
            .notify_customer_installation_cancelled(tenant_id, &row.subscription_id, &reason)
            .await
        {
            warn!(
                "failed to send installation cancellation notification: tenant_id={}, work_order_id={}, error={}",
                tenant_id, row.id, err
            );
        }

        Ok(row)
    }

    pub async fn reopen_installation_work_order(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        notes: Option<String>,
        ip_address: Option<&str>,
    ) -> AppResult<InstallationWorkOrder> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "work_orders", "manage")
            .await?;

        let row = self
            .set_installation_work_order_status_internal(
                actor_id,
                tenant_id,
                work_order_id,
                Some("pending"),
                None,
                None,
                notes,
                true,
                ip_address,
                "WORK_ORDER_REOPEN",
                "Reopened installation work order",
            )
            .await?;

        self.set_customer_subscription_status(
            tenant_id,
            &row.subscription_id,
            "pending_installation",
        )
        .await?;

        Ok(row)
    }

    async fn set_installation_work_order_status_internal(
        &self,
        actor_id: &str,
        tenant_id: &str,
        work_order_id: &str,
        new_status: Option<&str>,
        assigned_to: Option<&str>,
        scheduled_at: Option<String>,
        notes: Option<String>,
        allow_closed_update: bool,
        ip_address: Option<&str>,
        audit_action: &str,
        audit_desc: &str,
    ) -> AppResult<InstallationWorkOrder> {
        #[cfg(feature = "postgres")]
        let mut row: InstallationWorkOrder = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

        #[cfg(feature = "sqlite")]
        let mut row: InstallationWorkOrder = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, customer_id, location_id, router_id, status, assigned_to, scheduled_at, completed_at, notes, created_at, updated_at
            FROM installation_work_orders
            WHERE tenant_id = ? AND id = ?
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(work_order_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

        if allow_closed_update && row.status != "cancelled" {
            return Err(AppError::Validation(
                "Only cancelled work order can be reopened".to_string(),
            ));
        }

        if row.status == "completed" {
            return Err(AppError::Validation(
                "Closed work order cannot be changed".to_string(),
            ));
        }
        if row.status == "cancelled" {
            if !allow_closed_update {
                return Err(AppError::Validation(
                    "Cancelled work order cannot be changed. Reopen it first.".to_string(),
                ));
            }
            if new_status != Some("pending") {
                return Err(AppError::Validation(
                    "Cancelled work order can only be reopened to pending status".to_string(),
                ));
            }
        }

        let normalized_new_status = match new_status {
            Some(s) => Some(Self::normalize_work_order_status(s)?),
            None => None,
        };

        if let Some(target_status) = normalized_new_status.as_deref() {
            match target_status {
                "pending" => {
                    if row.status == "in_progress" && !allow_closed_update {
                        return Err(AppError::Validation(
                            "In-progress work order cannot be moved back to pending".to_string(),
                        ));
                    }
                }
                "in_progress" => {
                    if row.status != "pending" {
                        return Err(AppError::Validation(
                            "Only pending work order can be started".to_string(),
                        ));
                    }
                    if row
                        .assigned_to
                        .as_deref()
                        .map(str::trim)
                        .unwrap_or("")
                        .is_empty()
                    {
                        return Err(AppError::Validation(
                            "Set assignee before starting work order".to_string(),
                        ));
                    }
                    if row.scheduled_at.is_none() {
                        return Err(AppError::Validation(
                            "Set installation schedule before starting work order".to_string(),
                        ));
                    }
                }
                "completed" => {
                    if row.status != "in_progress" {
                        return Err(AppError::Validation(
                            "Only in-progress work order can be completed".to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }

        if let Some(s) = normalized_new_status {
            row.status = s;
            row.completed_at = if row.status == "completed" {
                Some(Utc::now())
            } else {
                None
            };
        }
        if let Some(uid) = assigned_to {
            let normalized_uid = uid.trim();
            row.assigned_to = if normalized_uid.is_empty() {
                None
            } else {
                Some(normalized_uid.to_string())
            };
        }
        if scheduled_at.is_some() {
            row.scheduled_at = Self::parse_optional_datetime(scheduled_at)?;
        }
        row.notes = Self::merge_work_order_notes(row.notes, actor_id, notes.as_deref());
        row.updated_at = Utc::now();

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            UPDATE installation_work_orders
            SET status = $1,
                assigned_to = $2,
                scheduled_at = $3,
                completed_at = $4,
                notes = $5,
                updated_at = $6
            WHERE tenant_id = $7 AND id = $8
            "#,
        )
        .bind(&row.status)
        .bind(&row.assigned_to)
        .bind(row.scheduled_at)
        .bind(row.completed_at)
        .bind(&row.notes)
        .bind(row.updated_at)
        .bind(tenant_id)
        .bind(work_order_id)
        .execute(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        sqlx::query(
            r#"
            UPDATE installation_work_orders
            SET status = ?,
                assigned_to = ?,
                scheduled_at = ?,
                completed_at = ?,
                notes = ?,
                updated_at = ?
            WHERE tenant_id = ? AND id = ?
            "#,
        )
        .bind(&row.status)
        .bind(&row.assigned_to)
        .bind(row.scheduled_at)
        .bind(row.completed_at)
        .bind(&row.notes)
        .bind(row.updated_at)
        .bind(tenant_id)
        .bind(work_order_id)
        .execute(&self.pool)
        .await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                audit_action,
                "installation_work_orders",
                Some(work_order_id),
                Some(audit_desc),
                ip_address,
            )
            .await;

        Ok(row)
    }

    async fn notify_customer_installation_cancelled(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        reason: &str,
    ) -> AppResult<()> {
        let user_ids = self
            .list_customer_user_ids_for_subscription(tenant_id, subscription_id)
            .await?;
        if user_ids.is_empty() {
            return Ok(());
        }

        let short_reason = reason.trim();
        let message = format!(
            "Your installation request was cancelled by admin/technician. Reason: {}. You can request reopen from Services page.",
            short_reason
        );

        for user_id in user_ids {
            self.notification_service
                .create_notification(
                    user_id,
                    Some(tenant_id.to_string()),
                    "Installation Request Cancelled".to_string(),
                    message.clone(),
                    "warning".to_string(),
                    "operations".to_string(),
                    Some("/dashboard/services".to_string()),
                )
                .await?;
        }

        Ok(())
    }

    async fn list_customer_user_ids_for_subscription(
        &self,
        tenant_id: &str,
        subscription_id: &str,
    ) -> AppResult<Vec<String>> {
        #[cfg(feature = "postgres")]
        let customer_user_ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT cu.user_id
            FROM customer_subscriptions cs
            INNER JOIN customer_users cu
              ON cu.tenant_id = cs.tenant_id
             AND cu.customer_id = cs.customer_id
            WHERE cs.tenant_id = $1
              AND cs.id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_all(&self.pool)
        .await?;

        #[cfg(feature = "sqlite")]
        let customer_user_ids: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT cu.user_id
            FROM customer_subscriptions cs
            INNER JOIN customer_users cu
              ON cu.tenant_id = cs.tenant_id
             AND cu.customer_id = cs.customer_id
            WHERE cs.tenant_id = ?
              AND cs.id = ?
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(customer_user_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::{CustomerService, InstallationSlaBreachType};
    use chrono::{Duration, Utc};

    #[test]
    fn detect_installation_sla_breach_for_scheduled_work_order() {
        let now = Utc::now();
        let created_at = now - Duration::hours(3);
        let scheduled_at = Some(now - Duration::minutes(121));

        let got = CustomerService::detect_installation_sla_breach(
            "pending",
            scheduled_at,
            created_at,
            now,
            120,
            240,
        );
        assert_eq!(got, Some(InstallationSlaBreachType::ScheduledOverdue));
    }

    #[test]
    fn detect_installation_sla_breach_for_unscheduled_pending_work_order() {
        let now = Utc::now();
        let created_at = now - Duration::minutes(241);

        let got = CustomerService::detect_installation_sla_breach(
            "pending", None, created_at, now, 120, 240,
        );
        assert_eq!(got, Some(InstallationSlaBreachType::PendingUnscheduled));
    }

    #[test]
    fn no_sla_breach_for_completed_or_fresh_work_order() {
        let now = Utc::now();
        let created_at = now - Duration::minutes(20);
        let scheduled_at = Some(now - Duration::minutes(10));

        let completed = CustomerService::detect_installation_sla_breach(
            "completed",
            scheduled_at,
            created_at,
            now,
            120,
            240,
        );
        assert_eq!(completed, None);

        let fresh_pending = CustomerService::detect_installation_sla_breach(
            "pending", None, created_at, now, 120, 240,
        );
        assert_eq!(fresh_pending, None);
    }

    #[test]
    fn elapsed_duration_formatter_is_human_readable() {
        assert_eq!(CustomerService::format_elapsed_duration(45), "45m");
        assert_eq!(CustomerService::format_elapsed_duration(120), "2h");
        assert_eq!(CustomerService::format_elapsed_duration(145), "2h 25m");
        assert_eq!(CustomerService::format_elapsed_duration(26 * 60), "1d 2h");
    }
}
