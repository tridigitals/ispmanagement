use super::{
    CustomerService, InstallationSlaBreachType, INSTALLATION_GRACE_HOURS_KEY,
    INSTALLATION_SLA_REMINDER_ENABLED_KEY,
};
use crate::error::{AppError, AppResult};
use crate::services::subscription_lifecycle::SubscriptionLifecycleStatus;
use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use uuid::Uuid;

impl CustomerService {
    pub(super) fn normalize_installation_work_order_visibility_mode(
        raw: Option<String>,
    ) -> &'static str {
        match raw
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("all_staff") => "all_staff",
            _ => "admin_only",
        }
    }

    pub(super) fn should_non_admin_see_unassigned_installation_work_orders(mode: &str) -> bool {
        mode.trim().eq_ignore_ascii_case("all_staff")
    }

    #[cfg(test)]
    pub(super) fn should_actor_have_full_installation_visibility(
        is_admin_owner: bool,
        can_manage_work_orders: bool,
    ) -> bool {
        is_admin_owner || can_manage_work_orders
    }

    #[cfg(test)]
    pub(super) fn should_actor_see_installation_work_order(
        can_view_unassigned: bool,
        actor_id: &str,
        assigned_to: Option<&str>,
        status: &str,
    ) -> bool {
        let normalized_assigned_to = assigned_to.map(str::trim).filter(|value| !value.is_empty());
        if normalized_assigned_to == Some(actor_id.trim()) {
            return true;
        }

        can_view_unassigned
            && status.trim().eq_ignore_ascii_case("pending")
            && normalized_assigned_to.is_none()
    }

    pub(super) fn normalize_billing_cycle(v: &str) -> AppResult<String> {
        let x = v.trim().to_lowercase();
        match x.as_str() {
            "monthly" | "yearly" => Ok(x),
            _ => Err(AppError::Validation(
                "billing_cycle must be monthly or yearly".to_string(),
            )),
        }
    }

    pub(super) fn normalize_subscription_status(v: &str) -> AppResult<String> {
        let x = v.trim().to_lowercase();
        SubscriptionLifecycleStatus::parse(&x)
            .map(|status| status.as_str().to_string())
            .map_err(|_| {
                AppError::Validation(
                    "status must be active, grace_active, pending_installation, installation_done_awaiting_payment, suspended, or cancelled"
                        .to_string(),
                )
            })
    }

    pub(super) fn normalize_work_order_status(v: &str) -> AppResult<String> {
        let x = v.trim().to_lowercase();
        match x.as_str() {
            "pending" | "in_progress" | "completed" | "cancelled" => Ok(x),
            _ => Err(AppError::Validation(
                "status must be pending, in_progress, completed, or cancelled".to_string(),
            )),
        }
    }

    pub(super) fn parse_setting_bool(raw: Option<String>, default_value: bool) -> bool {
        let Some(value) = raw else {
            return default_value;
        };
        match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default_value,
        }
    }

    pub(super) fn parse_setting_i64(
        raw: Option<String>,
        default_value: i64,
        min: i64,
        max: i64,
    ) -> i64 {
        raw.and_then(|v| v.trim().parse::<i64>().ok())
            .map(|v| v.clamp(min, max))
            .unwrap_or(default_value)
    }

    pub(super) fn detect_installation_sla_breach(
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
        if let Some(scheduled_at) = scheduled_at {
            if scheduled_at <= scheduled_cutoff {
                return Some(InstallationSlaBreachType::ScheduledOverdue);
            }
        } else {
            let unscheduled_cutoff = now - Duration::minutes(unscheduled_minutes.max(1));
            if created_at <= unscheduled_cutoff {
                return Some(InstallationSlaBreachType::PendingUnscheduled);
            }
        }

        None
    }

    pub(super) fn format_elapsed_duration(minutes: i64) -> String {
        let total_minutes = minutes.max(0);
        if total_minutes < 60 {
            return format!("{}m", total_minutes);
        }

        let hours = total_minutes / 60;
        let minutes = total_minutes % 60;
        if hours < 24 {
            if minutes == 0 {
                return format!("{}h", hours);
            }
            return format!("{}h {}m", hours, minutes);
        }

        let days = hours / 24;
        let rem_hours = hours % 24;
        if rem_hours == 0 {
            format!("{}d", days)
        } else {
            format!("{}d {}h", days, rem_hours)
        }
    }

    pub(super) fn is_owner_admin_or_technician_role(role: Option<&str>) -> bool {
        matches!(
            role.map(|value| value.trim().to_ascii_lowercase()),
            Some(role) if matches!(role.as_str(), "owner" | "admin" | "technician")
        )
    }

    pub(super) fn is_technician_role(role: Option<&str>) -> bool {
        matches!(
            role.map(|value| value.trim().to_ascii_lowercase()),
            Some(role) if matches!(role.as_str(), "technician" | "teknisi")
        )
    }

    pub(super) fn filter_installation_request_user_ids(
        rows: Vec<(String, Option<String>)>,
        include_technician: bool,
    ) -> Vec<String> {
        let mut set = HashSet::new();
        for (user_id, role) in rows {
            let allowed = if include_technician {
                Self::is_owner_admin_or_technician_role(role.as_deref())
            } else {
                matches!(
                    role.map(|value| value.trim().to_ascii_lowercase()),
                    Some(role) if matches!(role.as_str(), "owner" | "admin")
                )
            };
            if allowed {
                set.insert(user_id);
            }
        }
        set.into_iter().collect()
    }

    pub(super) fn merge_work_order_notes(
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

    pub(super) fn parse_optional_datetime(
        input: Option<String>,
    ) -> AppResult<Option<DateTime<Utc>>> {
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
            "invalid datetime format; expected RFC3339 or YYYY-MM-DD".to_string(),
        ))
    }

    pub(super) fn hash_registration_invite_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub(super) fn build_registration_invite_token() -> String {
        format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
    }

    pub(super) fn parse_invite_policy_u32(
        raw: Option<String>,
        default_value: u32,
        min: u32,
        max: u32,
    ) -> u32 {
        raw.and_then(|v| v.trim().parse::<u32>().ok())
            .map(|v| v.clamp(min, max))
            .unwrap_or(default_value)
    }

    pub(super) async fn read_tenant_setting_value(
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

    pub(super) async fn read_global_setting_value(&self, key: &str) -> AppResult<Option<String>> {
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

    pub(super) async fn resolve_installation_sla_reminder_enabled(&self) -> bool {
        let raw = self
            .read_global_setting_value(INSTALLATION_SLA_REMINDER_ENABLED_KEY)
            .await
            .ok()
            .flatten();
        Self::parse_setting_bool(raw, true)
    }

    pub(super) async fn resolve_installation_grace_hours(&self, tenant_id: &str) -> i64 {
        let tenant_raw = self
            .read_tenant_setting_value(tenant_id, INSTALLATION_GRACE_HOURS_KEY)
            .await
            .ok()
            .flatten();
        let global_raw = self
            .read_global_setting_value(INSTALLATION_GRACE_HOURS_KEY)
            .await
            .ok()
            .flatten();
        Self::parse_setting_i64(tenant_raw.or(global_raw), 72, 1, 24 * 30)
    }

    pub(super) async fn resolve_installation_work_order_visibility_mode(
        &self,
        tenant_id: &str,
    ) -> &'static str {
        let tenant_raw = self
            .read_tenant_setting_value(tenant_id, "installation_work_order_visibility_mode")
            .await
            .ok()
            .flatten();
        Self::normalize_installation_work_order_visibility_mode(tenant_raw)
    }
}
