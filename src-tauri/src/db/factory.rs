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

    fn canonical_role_name(role: &str) -> String {
        match role.trim().to_ascii_lowercase().as_str() {
            "owner" => "Owner".to_string(),
            "admin" => "Admin".to_string(),
            "noc" => "NOC".to_string(),
            "planner" => "Planner".to_string(),
            "customer service" => "Customer Service".to_string(),
            "technician" => "Technician".to_string(),
            "viewer" => "Viewer".to_string(),
            "customer" => "Customer".to_string(),
            _ => role.trim().to_string(),
        }
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

        if let Some(member_id) = sqlx::query_scalar::<_, String>(q)
            .bind(tenant_id)
            .bind(user_id)
            .fetch_optional(self.pool)
            .await
            .context("ensure_tenant_member select failed")?
        {
            #[cfg(feature = "postgres")]
            sqlx::query(
                "UPDATE tenant_members SET role = $1, role_id = (SELECT id FROM roles WHERE name = $2 AND tenant_id IS NULL LIMIT 1) WHERE id = $3 AND role_id IS NULL",
            )
            .bind(role)
            .bind(Self::canonical_role_name(role))
            .bind(&member_id)
            .execute(self.pool)
            .await
            .context("ensure_tenant_member role repair failed")?;

            #[cfg(feature = "sqlite")]
            sqlx::query(
                "UPDATE tenant_members SET role = ?, role_id = (SELECT id FROM roles WHERE name = ? AND tenant_id IS NULL LIMIT 1) WHERE id = ? AND role_id IS NULL",
            )
            .bind(role)
            .bind(Self::canonical_role_name(role))
            .bind(&member_id)
            .execute(self.pool)
            .await
            .context("ensure_tenant_member role repair failed")?;

            return Ok(());
        }

        let now = Utc::now();
        let canonical_role = Self::canonical_role_name(role);
        let id = uuid::Uuid::new_v4().to_string();

        #[cfg(feature = "postgres")]
        {
            sqlx::query(
                r#"
                INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at)
                VALUES ($1,$2,$3,$4,(SELECT id FROM roles WHERE name = $5 AND tenant_id IS NULL LIMIT 1),$6)
            "#,
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(user_id)
            .bind(role)
            .bind(&canonical_role)
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
                VALUES (?,?,?,?,(SELECT id FROM roles WHERE name = ? AND tenant_id IS NULL LIMIT 1),?)
            "#,
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(user_id)
            .bind(role)
            .bind(&canonical_role)
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

    pub async fn ensure_default_message_templates(&self, tenant_id: &str) -> Result<()> {
        #[cfg(feature = "postgres")]
        {
            sqlx::query(
                r#"
                WITH default_templates (
                    key, name, description, use_case, trigger_mode, event_key, channel, status,
                    whatsapp_body, email_subject, email_body, variables
                ) AS (
                    VALUES
                    (
                        'billing_payment_reminder',
                        'Billing - Friendly Payment Reminder',
                        'A polite manual reminder for customers who need a payment follow-up.',
                        'billing',
                        'manual',
                        'billing.payment_reminder',
                        'both',
                        'active',
                        $$Halo {{customer.name}}, kami dari {{tenant.name}} ingin mengingatkan tagihan layanan internet Anda.

Jika sudah melakukan pembayaran, abaikan pesan ini. Jika membutuhkan bantuan, balas pesan ini agar tim kami bisa membantu.$$,
                        'Pengingat pembayaran layanan {{tenant.name}}',
                        $$Halo {{customer.name}},

Kami dari {{tenant.name}} ingin mengingatkan tagihan layanan internet Anda.

Jika pembayaran sudah dilakukan, email ini dapat diabaikan. Jika ada kendala pembayaran atau membutuhkan bantuan, silakan hubungi tim kami melalui channel resmi.

Terima kasih,
{{tenant.name}}$$,
                        '["tenant.name","customer.name"]'
                    ),
                    (
                        'billing_overdue_followup',
                        'Billing - Overdue Follow-up',
                        'A firmer follow-up for overdue billing without sounding aggressive.',
                        'billing',
                        'manual',
                        'billing.overdue_followup',
                        'both',
                        'active',
                        $$Halo {{customer.name}}, kami dari {{tenant.name}} mencatat pembayaran layanan Anda masih perlu ditindaklanjuti.

Mohon lakukan pembayaran atau hubungi kami jika ada kendala, agar layanan tetap berjalan dengan baik.$$,
                        'Tindak lanjut pembayaran layanan {{tenant.name}}',
                        $$Halo {{customer.name}},

Kami mencatat pembayaran layanan Anda masih perlu ditindaklanjuti.

Mohon lakukan pembayaran melalui metode yang tersedia. Jika ada kendala, balas email ini atau hubungi tim kami agar kami dapat membantu pengecekan.

Terima kasih,
{{tenant.name}}$$,
                        '["tenant.name","customer.name"]'
                    ),
                    (
                        'installation_schedule_confirmation',
                        'Installation - Schedule Confirmation',
                        'Confirm installation readiness and keep the customer informed before field work.',
                        'installation',
                        'manual',
                        'installation.schedule_confirmation',
                        'both',
                        'active',
                        $$Halo {{customer.name}}, tim {{tenant.name}} akan menindaklanjuti jadwal instalasi layanan Anda.

Mohon pastikan lokasi dapat diakses dan nomor ini aktif untuk koordinasi teknisi. Jika ada perubahan jadwal, balas pesan ini.$$,
                        'Konfirmasi jadwal instalasi {{tenant.name}}',
                        $$Halo {{customer.name}},

Tim {{tenant.name}} akan menindaklanjuti jadwal instalasi layanan Anda.

Mohon pastikan lokasi dapat diakses, ada PIC yang dapat ditemui, dan nomor kontak tetap aktif untuk koordinasi teknisi. Jika ada perubahan jadwal atau akses lokasi, silakan balas email ini agar tim kami dapat menyesuaikan kunjungan.

Terima kasih,
{{tenant.name}}$$,
                        '["tenant.name","customer.name"]'
                    ),
                    (
                        'installation_completed',
                        'Installation - Completed Handoff',
                        'Send after installation is completed to guide the customer on the next step.',
                        'installation',
                        'manual',
                        'installation.completed',
                        'both',
                        'active',
                        $$Halo {{customer.name}}, instalasi layanan {{tenant.name}} sudah selesai.

Silakan coba koneksi internet Anda. Jika ada kendala, balas pesan ini agar tim kami dapat membantu pengecekan.$$,
                        'Instalasi layanan {{tenant.name}} selesai',
                        $$Halo {{customer.name}},

Instalasi layanan {{tenant.name}} sudah selesai.

Silakan coba koneksi internet Anda. Jika ada kendala setelah instalasi, hubungi tim kami dengan menjelaskan gejala yang dialami agar pengecekan dapat dilakukan lebih cepat.

Terima kasih,
{{tenant.name}}$$,
                        '["tenant.name","customer.name"]'
                    ),
                    (
                        'outage_customer_notice',
                        'Outage - Customer Notice',
                        'Notify customers about an incident while keeping the message calm and concise.',
                        'outage',
                        'manual',
                        'network.outage_notice',
                        'both',
                        'active',
                        $$Halo {{customer.name}}, saat ini tim {{tenant.name}} sedang menangani gangguan layanan di beberapa area.

Kami akan menginformasikan pembaruan berikutnya setelah pengecekan selesai. Terima kasih atas kesabarannya.$$,
                        'Informasi gangguan layanan {{tenant.name}}',
                        $$Halo {{customer.name}},

Saat ini tim {{tenant.name}} sedang menangani gangguan layanan di beberapa area.

Tim teknis sedang melakukan pengecekan dan kami akan menginformasikan pembaruan berikutnya setelah ada perkembangan. Terima kasih atas pengertian dan kesabarannya.

Hormat kami,
{{tenant.name}}$$,
                        '["tenant.name","customer.name"]'
                    ),
                    (
                        'support_followup',
                        'Support - Follow-up Check',
                        'A clean follow-up message after a support case is handled.',
                        'support',
                        'manual',
                        'support.followup',
                        'both',
                        'active',
                        $$Halo {{customer.name}}, kami dari {{tenant.name}} ingin memastikan kendala Anda sudah terbantu.

Jika masih ada masalah, balas pesan ini agar tim support dapat melanjutkan pengecekan.$$,
                        'Follow-up bantuan dari {{tenant.name}}',
                        $$Halo {{customer.name}},

Kami ingin memastikan kendala Anda sudah terbantu oleh tim {{tenant.name}}.

Jika masih ada masalah atau membutuhkan bantuan lanjutan, silakan balas email ini dengan detail kendala yang dialami.

Terima kasih,
{{tenant.name}}$$,
                        '["tenant.name","customer.name"]'
                    )
                )
                INSERT INTO message_templates (
                    id, tenant_id, key, name, description, use_case, target, trigger_mode, event_key,
                    channel, locale, status, whatsapp_body, email_subject, email_body, variables,
                    version, created_at, updated_at
                )
                SELECT
                    'seed_tpl_' || md5($1 || ':' || d.key),
                    $1,
                    d.key,
                    d.name,
                    d.description,
                    d.use_case,
                    'customer',
                    d.trigger_mode,
                    d.event_key,
                    d.channel,
                    'id-ID',
                    d.status,
                    d.whatsapp_body,
                    d.email_subject,
                    d.email_body,
                    d.variables,
                    1,
                    now(),
                    now()
                FROM default_templates d
                ON CONFLICT (tenant_id, key) DO NOTHING
                "#,
            )
            .bind(tenant_id)
            .execute(self.pool)
            .await
            .context("ensure_default_message_templates insert failed")?;
        }

        #[cfg(feature = "sqlite")]
        {
            let now = Utc::now().to_rfc3339();
            let templates = default_message_templates();
            for template in templates {
                sqlx::query(
                    r#"
                    INSERT OR IGNORE INTO message_templates (
                        id, tenant_id, key, name, description, use_case, target, trigger_mode, event_key,
                        channel, locale, status, whatsapp_body, email_subject, email_body, variables,
                        version, created_at, updated_at
                    )
                    VALUES (?, ?, ?, ?, ?, ?, 'customer', ?, ?, ?, 'id-ID', ?, ?, ?, ?, ?, 1, ?, ?)
                    "#,
                )
                .bind(format!("seed_tpl_{}_{}", tenant_id.replace('-', ""), template.key))
                .bind(tenant_id)
                .bind(template.key)
                .bind(template.name)
                .bind(template.description)
                .bind(template.use_case)
                .bind(template.trigger_mode)
                .bind(template.event_key)
                .bind(template.channel)
                .bind(template.status)
                .bind(template.whatsapp_body)
                .bind(template.email_subject)
                .bind(template.email_body)
                .bind(template.variables)
                .bind(&now)
                .bind(&now)
                .execute(self.pool)
                .await
                .context("ensure_default_message_templates insert failed")?;
            }
        }

        Ok(())
    }
}

#[cfg(feature = "sqlite")]
struct DefaultMessageTemplate {
    key: &'static str,
    name: &'static str,
    description: &'static str,
    use_case: &'static str,
    trigger_mode: &'static str,
    event_key: &'static str,
    channel: &'static str,
    status: &'static str,
    whatsapp_body: &'static str,
    email_subject: Option<&'static str>,
    email_body: Option<&'static str>,
    variables: &'static str,
}

#[cfg(feature = "sqlite")]
fn default_message_templates() -> Vec<DefaultMessageTemplate> {
    vec![
        DefaultMessageTemplate {
            key: "billing_payment_reminder",
            name: "Billing - Friendly Payment Reminder",
            description: "A polite manual reminder for customers who need a payment follow-up.",
            use_case: "billing",
            trigger_mode: "manual",
            event_key: "billing.payment_reminder",
            channel: "both",
            status: "active",
            whatsapp_body: "Halo {{customer.name}}, kami dari {{tenant.name}} ingin mengingatkan tagihan layanan internet Anda.\n\nJika sudah melakukan pembayaran, abaikan pesan ini. Jika membutuhkan bantuan, balas pesan ini agar tim kami bisa membantu.",
            email_subject: Some("Pengingat pembayaran layanan {{tenant.name}}"),
            email_body: Some("Halo {{customer.name}},\n\nKami dari {{tenant.name}} ingin mengingatkan tagihan layanan internet Anda.\n\nJika pembayaran sudah dilakukan, email ini dapat diabaikan. Jika ada kendala pembayaran atau membutuhkan bantuan, silakan hubungi tim kami melalui channel resmi.\n\nTerima kasih,\n{{tenant.name}}"),
            variables: "[\"tenant.name\",\"customer.name\"]",
        },
        DefaultMessageTemplate {
            key: "billing_overdue_followup",
            name: "Billing - Overdue Follow-up",
            description: "A firmer follow-up for overdue billing without sounding aggressive.",
            use_case: "billing",
            trigger_mode: "manual",
            event_key: "billing.overdue_followup",
            channel: "both",
            status: "active",
            whatsapp_body: "Halo {{customer.name}}, kami dari {{tenant.name}} mencatat pembayaran layanan Anda masih perlu ditindaklanjuti.\n\nMohon lakukan pembayaran atau hubungi kami jika ada kendala, agar layanan tetap berjalan dengan baik.",
            email_subject: Some("Tindak lanjut pembayaran layanan {{tenant.name}}"),
            email_body: Some("Halo {{customer.name}},\n\nKami mencatat pembayaran layanan Anda masih perlu ditindaklanjuti.\n\nMohon lakukan pembayaran melalui metode yang tersedia. Jika ada kendala, balas email ini atau hubungi tim kami agar kami dapat membantu pengecekan.\n\nTerima kasih,\n{{tenant.name}}"),
            variables: "[\"tenant.name\",\"customer.name\"]",
        },
        DefaultMessageTemplate {
            key: "installation_schedule_confirmation",
            name: "Installation - Schedule Confirmation",
            description: "Confirm installation readiness and keep the customer informed before field work.",
            use_case: "installation",
            trigger_mode: "manual",
            event_key: "installation.schedule_confirmation",
            channel: "both",
            status: "active",
            whatsapp_body: "Halo {{customer.name}}, tim {{tenant.name}} akan menindaklanjuti jadwal instalasi layanan Anda.\n\nMohon pastikan lokasi dapat diakses dan nomor ini aktif untuk koordinasi teknisi. Jika ada perubahan jadwal, balas pesan ini.",
            email_subject: Some("Konfirmasi jadwal instalasi {{tenant.name}}"),
            email_body: Some("Halo {{customer.name}},\n\nTim {{tenant.name}} akan menindaklanjuti jadwal instalasi layanan Anda.\n\nMohon pastikan lokasi dapat diakses, ada PIC yang dapat ditemui, dan nomor kontak tetap aktif untuk koordinasi teknisi. Jika ada perubahan jadwal atau akses lokasi, silakan balas email ini agar tim kami dapat menyesuaikan kunjungan.\n\nTerima kasih,\n{{tenant.name}}"),
            variables: "[\"tenant.name\",\"customer.name\"]",
        },
        DefaultMessageTemplate {
            key: "installation_completed",
            name: "Installation - Completed Handoff",
            description: "Send after installation is completed to guide the customer on the next step.",
            use_case: "installation",
            trigger_mode: "manual",
            event_key: "installation.completed",
            channel: "both",
            status: "active",
            whatsapp_body: "Halo {{customer.name}}, instalasi layanan {{tenant.name}} sudah selesai.\n\nSilakan coba koneksi internet Anda. Jika ada kendala, balas pesan ini agar tim kami dapat membantu pengecekan.",
            email_subject: Some("Instalasi layanan {{tenant.name}} selesai"),
            email_body: Some("Halo {{customer.name}},\n\nInstalasi layanan {{tenant.name}} sudah selesai.\n\nSilakan coba koneksi internet Anda. Jika ada kendala setelah instalasi, hubungi tim kami dengan menjelaskan gejala yang dialami agar pengecekan dapat dilakukan lebih cepat.\n\nTerima kasih,\n{{tenant.name}}"),
            variables: "[\"tenant.name\",\"customer.name\"]",
        },
        DefaultMessageTemplate {
            key: "outage_customer_notice",
            name: "Outage - Customer Notice",
            description: "Notify customers about an incident while keeping the message calm and concise.",
            use_case: "outage",
            trigger_mode: "manual",
            event_key: "network.outage_notice",
            channel: "both",
            status: "active",
            whatsapp_body: "Halo {{customer.name}}, saat ini tim {{tenant.name}} sedang menangani gangguan layanan di beberapa area.\n\nKami akan menginformasikan pembaruan berikutnya setelah pengecekan selesai. Terima kasih atas kesabarannya.",
            email_subject: Some("Informasi gangguan layanan {{tenant.name}}"),
            email_body: Some("Halo {{customer.name}},\n\nSaat ini tim {{tenant.name}} sedang menangani gangguan layanan di beberapa area.\n\nTim teknis sedang melakukan pengecekan dan kami akan menginformasikan pembaruan berikutnya setelah ada perkembangan. Terima kasih atas pengertian dan kesabarannya.\n\nHormat kami,\n{{tenant.name}}"),
            variables: "[\"tenant.name\",\"customer.name\"]",
        },
        DefaultMessageTemplate {
            key: "support_followup",
            name: "Support - Follow-up Check",
            description: "A clean follow-up message after a support case is handled.",
            use_case: "support",
            trigger_mode: "manual",
            event_key: "support.followup",
            channel: "both",
            status: "active",
            whatsapp_body: "Halo {{customer.name}}, kami dari {{tenant.name}} ingin memastikan kendala Anda sudah terbantu.\n\nJika masih ada masalah, balas pesan ini agar tim support dapat melanjutkan pengecekan.",
            email_subject: Some("Follow-up bantuan dari {{tenant.name}}"),
            email_body: Some("Halo {{customer.name}},\n\nKami ingin memastikan kendala Anda sudah terbantu oleh tim {{tenant.name}}.\n\nJika masih ada masalah atau membutuhkan bantuan lanjutan, silakan balas email ini dengan detail kendala yang dialami.\n\nTerima kasih,\n{{tenant.name}}"),
            variables: "[\"tenant.name\",\"customer.name\"]",
        },
    ]
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
