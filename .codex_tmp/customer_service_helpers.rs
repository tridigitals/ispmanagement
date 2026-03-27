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
        SubscriptionLifecycleStatus::parse(&x)
            .map(|status| status.as_str().to_string())
            .map_err(|_| {
                AppError::Validation(
                    "status must be active, pending_installation, installation_done_awaiting_payment, suspended, or cancelled"
                        .to_string(),
                )
            })
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
