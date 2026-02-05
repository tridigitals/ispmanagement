use crate::db::DbPool;
use crate::services::AuthService;
use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Utc};

#[derive(Clone)]
pub struct DbFactory<'a> {
    pool: &'a DbPool,
}

impl<'a> DbFactory<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    pub async fn ensure_global_setting(
        &self,
        key: &str,
        value: &str,
        description: &str,
    ) -> Result<()> {
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        {
            sqlx::query(
                r#"
                INSERT INTO settings (id, tenant_id, key, value, description, created_at, updated_at)
                VALUES ($1, NULL, $2, $3, $4, $5, $6)
                ON CONFLICT (key) WHERE tenant_id IS NULL DO NOTHING
            "#,
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(key)
            .bind(value)
            .bind(description)
            .bind(now)
            .bind(now)
            .execute(self.pool)
            .await
            .context("ensure_global_setting insert failed")?;
        }

        #[cfg(feature = "sqlite")]
        {
            let now_str = now.to_rfc3339();
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO settings (id, tenant_id, key, value, description, created_at, updated_at)
                VALUES (?, NULL, ?, ?, ?, ?, ?)
            "#,
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(key)
            .bind(value)
            .bind(description)
            .bind(&now_str)
            .bind(&now_str)
            .execute(self.pool)
            .await
            .context("ensure_global_setting insert failed")?;
        }

        Ok(())
    }

    pub async fn ensure_user(
        &self,
        email: &str,
        name: &str,
        password: &str,
        role: &str,
        is_super_admin: bool,
    ) -> Result<String> {
        #[cfg(feature = "postgres")]
        let q = "SELECT id FROM users WHERE email = $1";
        #[cfg(feature = "sqlite")]
        let q = "SELECT id FROM users WHERE email = ?";

        if let Some(id) = sqlx::query_scalar::<_, String>(q)
            .bind(email)
            .fetch_optional(self.pool)
            .await
            .context("ensure_user select failed")?
        {
            return Ok(id);
        }

        let now = Utc::now();
        let password_hash = AuthService::hash_password(password)
            .map_err(|e| anyhow!("hash_password failed: {e}"))?;
        let id = uuid::Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        {
            sqlx::query(
                r#"
                INSERT INTO users (
                    id, email, password_hash, name, role, is_super_admin, is_active,
                    failed_login_attempts, created_at, updated_at, email_verified_at
                )
                VALUES ($1,$2,$3,$4,$5,$6,true,0,$7,$8,$9)
            "#,
            )
            .bind(&id)
            .bind(email)
            .bind(&password_hash)
            .bind(name)
            .bind(role)
            .bind(is_super_admin)
            .bind(now)
            .bind(now)
            .bind(now)
            .execute(self.pool)
            .await
            .context("ensure_user insert failed")?;
        }

        #[cfg(feature = "sqlite")]
        {
            let now_str = now.to_rfc3339();
            sqlx::query(
                r#"
                INSERT INTO users (
                    id, email, password_hash, name, role, is_super_admin, is_active,
                    failed_login_attempts, created_at, updated_at, email_verified_at
                )
                VALUES (?,?,?,?,?,?,1,0,?,?,?)
            "#,
            )
            .bind(&id)
            .bind(email)
            .bind(&password_hash)
            .bind(name)
            .bind(role)
            .bind(is_super_admin)
            .bind(&now_str)
            .bind(&now_str)
            .bind(&now_str)
            .execute(self.pool)
            .await
            .context("ensure_user insert failed")?;
        }

        Ok(id)
    }

    pub async fn ensure_tenant(&self, name: &str, slug: &str) -> Result<String> {
        #[cfg(feature = "postgres")]
        let q = "SELECT id FROM tenants WHERE slug = $1";
        #[cfg(feature = "sqlite")]
        let q = "SELECT id FROM tenants WHERE slug = ?";

        if let Some(id) = sqlx::query_scalar::<_, String>(q)
            .bind(slug)
            .fetch_optional(self.pool)
            .await
            .context("ensure_tenant select failed")?
        {
            return Ok(id);
        }

        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        #[cfg(feature = "postgres")]
        {
            sqlx::query(
                r#"
                INSERT INTO tenants (id, name, slug, custom_domain, logo_url, is_active, enforce_2fa, created_at, updated_at)
                VALUES ($1,$2,$3,NULL,NULL,true,false,$4,$5)
            "#,
            )
            .bind(&id)
            .bind(name)
            .bind(slug)
            .bind(now)
            .bind(now)
            .execute(self.pool)
            .await
            .context("ensure_tenant insert failed")?;
        }

        #[cfg(feature = "sqlite")]
        {
            let now_str = now.to_rfc3339();
            sqlx::query(
                r#"
                INSERT INTO tenants (id, name, slug, custom_domain, logo_url, is_active, enforce_2fa, created_at, updated_at)
                VALUES (?,?,?,NULL,NULL,1,0,?,?)
            "#,
            )
            .bind(&id)
            .bind(name)
            .bind(slug)
            .bind(&now_str)
            .bind(&now_str)
            .execute(self.pool)
            .await
            .context("ensure_tenant insert failed")?;
        }

        Ok(id)
    }

    pub async fn ensure_tenant_member(
        &self,
        tenant_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<()> {
        #[cfg(feature = "postgres")]
        let q = "SELECT id FROM tenant_members WHERE tenant_id = $1 AND user_id = $2";
        #[cfg(feature = "sqlite")]
        let q = "SELECT id FROM tenant_members WHERE tenant_id = ? AND user_id = ?";

        if sqlx::query_scalar::<_, String>(q)
            .bind(tenant_id)
            .bind(user_id)
            .fetch_optional(self.pool)
            .await
            .context("ensure_tenant_member select failed")?
            .is_some()
        {
            return Ok(());
        }

        let now = Utc::now();
        let id = uuid::Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        {
            sqlx::query(
                r#"
                INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at)
                VALUES ($1,$2,$3,$4,NULL,$5)
            "#,
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(user_id)
            .bind(role)
            .bind(now)
            .execute(self.pool)
            .await
            .context("ensure_tenant_member insert failed")?;
        }

        #[cfg(feature = "sqlite")]
        {
            let now_str = now.to_rfc3339();
            sqlx::query(
                r#"
                INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at)
                VALUES (?,?,?,?,NULL,?)
            "#,
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(user_id)
            .bind(role)
            .bind(&now_str)
            .execute(self.pool)
            .await
            .context("ensure_tenant_member insert failed")?;
        }

        Ok(())
    }

    pub async fn ensure_tenant_subscription_default(&self, tenant_id: &str) -> Result<()> {
        #[cfg(feature = "postgres")]
        let q_sub = "SELECT id FROM tenant_subscriptions WHERE tenant_id = $1 AND status = 'active' LIMIT 1";
        #[cfg(feature = "sqlite")]
        let q_sub =
            "SELECT id FROM tenant_subscriptions WHERE tenant_id = ? AND status = 'active' LIMIT 1";

        if sqlx::query_scalar::<_, String>(q_sub)
            .bind(tenant_id)
            .fetch_optional(self.pool)
            .await
            .context("ensure_tenant_subscription_default select sub failed")?
            .is_some()
        {
            return Ok(());
        }

        #[cfg(feature = "postgres")]
        let q_plan = "SELECT id FROM plans WHERE is_default = true ORDER BY sort_order ASC LIMIT 1";
        #[cfg(feature = "sqlite")]
        let q_plan = "SELECT id FROM plans WHERE is_default = 1 ORDER BY sort_order ASC LIMIT 1";

        let plan_id: Option<String> = sqlx::query_scalar(q_plan)
            .fetch_optional(self.pool)
            .await
            .context("ensure_tenant_subscription_default select plan failed")?;

        let Some(plan_id) = plan_id else {
            return Err(anyhow!(
                "no default plan found; seed_plans must be executed first"
            ));
        };

        let now = Utc::now();
        let end = now + Duration::days(30);
        let id = uuid::Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        {
            sqlx::query(
                r#"
                INSERT INTO tenant_subscriptions (
                    id, tenant_id, plan_id, status, current_period_start, current_period_end,
                    created_at, updated_at
                )
                VALUES ($1,$2,$3,'active',$4,$5,$6,$7)
            "#,
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(&plan_id)
            .bind(now)
            .bind(end)
            .bind(now)
            .bind(now)
            .execute(self.pool)
            .await
            .context("ensure_tenant_subscription_default insert failed")?;
        }

        #[cfg(feature = "sqlite")]
        {
            let now_str = now.to_rfc3339();
            let end_str = end.to_rfc3339();
            sqlx::query(
                r#"
                INSERT INTO tenant_subscriptions (
                    id, tenant_id, plan_id, status, current_period_start, current_period_end,
                    created_at, updated_at
                )
                VALUES (?,?,?,'active',?,?,?,?)
            "#,
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(&plan_id)
            .bind(&now_str)
            .bind(&end_str)
            .bind(&now_str)
            .bind(&now_str)
            .execute(self.pool)
            .await
            .context("ensure_tenant_subscription_default insert failed")?;
        }

        Ok(())
    }
}

pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in input.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}
