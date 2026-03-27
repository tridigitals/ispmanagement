use super::*;

impl CustomerService {
    pub(super) async fn set_customer_subscription_status(
        &self,
        tenant_id: &str,
        subscription_id: &str,
        expected_status: &str,
        status: &str,
    ) -> AppResult<bool> {
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
              AND status = $5
            "#,
        )
        .bind(status)
        .bind(now)
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(expected_status)
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
              AND status = ?
            "#,
        )
        .bind(status)
        .bind(status)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(expected_status)
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(rows > 0)
    }

    pub(super) async fn set_location_pppoe_disabled_state(
        &self,
        tenant_id: &str,
        location_id: &str,
        disabled: bool,
    ) -> AppResult<u64> {
        self.pppoe_service
            .set_location_accounts_disabled_state(tenant_id, location_id, disabled)
            .await
    }

    pub(super) async fn list_tenant_installation_alert_user_ids(
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

    pub(super) async fn notify_new_installation_request(
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

    pub(super) async fn notify_installation_rescheduled(
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

    pub(super) async fn run_installation_sla_reminders_for_tenant(
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

    pub(super) async fn list_overdue_installation_work_orders(
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

    pub(super) async fn has_recent_installation_sla_notification(
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


    pub(super) async fn list_customer_user_ids_for_subscription(
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
