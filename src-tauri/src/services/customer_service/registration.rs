use super::*;

#[derive(sqlx::FromRow)]
struct CustomerRegistrationInviteListRow {
    id: String,
    tenant_id: String,
    created_by: Option<String>,
    max_uses: i64,
    used_count: i64,
    expires_at: DateTime<Utc>,
    is_revoked: bool,
    revoked_at: Option<DateTime<Utc>>,
    last_used_at: Option<DateTime<Utc>>,
    note: Option<String>,
    created_at: DateTime<Utc>,
    token_enc: Option<String>,
    custom_domain: Option<String>,
}

impl CustomerService {
    fn map_registration_invite_row(
        row: CustomerRegistrationInviteListRow,
    ) -> AppResult<CustomerRegistrationInviteView> {
        let invite_url = match (row.custom_domain.as_deref(), row.token_enc.as_deref()) {
            (Some(domain), Some(token_enc)) => Self::decrypt_registration_invite_token(token_enc)?
                .map(|token| Self::build_registration_invite_url(domain, &token)),
            _ => None,
        };

        Ok(CustomerRegistrationInviteView {
            id: row.id,
            tenant_id: row.tenant_id,
            created_by: row.created_by,
            max_uses: row.max_uses,
            used_count: row.used_count,
            expires_at: row.expires_at,
            is_revoked: row.is_revoked,
            revoked_at: row.revoked_at,
            last_used_at: row.last_used_at,
            note: row.note,
            created_at: row.created_at,
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
        let rows: Vec<CustomerRegistrationInviteListRow> = sqlx::query_as(
            r#"
            SELECT
                cri.id,
                cri.tenant_id,
                cri.created_by,
                cri.max_uses,
                cri.used_count,
                cri.expires_at,
                cri.is_revoked,
                cri.revoked_at,
                cri.last_used_at,
                cri.note,
                cri.created_at,
                cri.token_enc,
                t.custom_domain
            FROM customer_registration_invites cri
            JOIN tenants t ON t.id = cri.tenant_id
            WHERE cri.tenant_id = $1
              AND (
                    $2::bool = true
                 OR (cri.is_revoked = false AND cri.expires_at > $3 AND cri.used_count < cri.max_uses)
              )
            ORDER BY cri.created_at DESC
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
        let rows: Vec<CustomerRegistrationInviteListRow> = sqlx::query_as(
            r#"
            SELECT
                cri.id,
                cri.tenant_id,
                cri.created_by,
                cri.max_uses,
                cri.used_count,
                cri.expires_at,
                cri.is_revoked,
                cri.revoked_at,
                cri.last_used_at,
                cri.note,
                cri.created_at,
                cri.token_enc,
                t.custom_domain
            FROM customer_registration_invites cri
            JOIN tenants t ON t.id = cri.tenant_id
            WHERE cri.tenant_id = ?
              AND (
                    ? = 1
                 OR (cri.is_revoked = 0 AND cri.expires_at > ? AND cri.used_count < cri.max_uses)
              )
            ORDER BY cri.created_at DESC
            LIMIT ?
            "#,
        )
        .bind(tenant_id)
        .bind(if include_inactive { 1 } else { 0 })
        .bind(now.to_rfc3339())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::map_registration_invite_row).collect()
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
}
