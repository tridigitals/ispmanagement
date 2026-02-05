use crate::db::{slugify, DbFactory, DbPool};
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedMode {
    Dev,
    Prod,
}

#[derive(Debug, Clone)]
pub struct SeedOptions {
    pub mode: SeedMode,
    pub admin_email: String,
    pub admin_password: String,
    pub admin_name: String,
    pub tenant_name: String,
    pub tenant_slug: String,
    pub app_timezone: String,
}

impl Default for SeedOptions {
    fn default() -> Self {
        let tenant_name = "Default Tenant".to_string();
        Self {
            mode: SeedMode::Dev,
            admin_email: "superadmin@local".to_string(),
            admin_password: "password123".to_string(),
            admin_name: "Super Admin".to_string(),
            tenant_slug: slugify(&tenant_name),
            tenant_name,
            app_timezone: "Asia/Jakarta".to_string(),
        }
    }
}

pub async fn run_seed(pool: &DbPool, opts: SeedOptions) -> Result<()> {
    let f = DbFactory::new(pool);

    // Make sure the app is consistent for multi-project boilerplate use.
    // Defaults are inserted in init_db(), but seeding can be executed standalone too.
    f.ensure_global_setting(
        "base_currency_code",
        "IDR",
        "Base currency for pricing (keep stable)",
    )
    .await?;
    f.ensure_global_setting(
        "currency_code",
        "IDR",
        "Default display currency code (ISO 4217)",
    )
    .await?;
    f.ensure_global_setting(
        "app_timezone",
        &opts.app_timezone,
        "Application timezone for schedules (IANA, e.g. Asia/Jakarta)",
    )
    .await?;

    // For prod, only seed safe defaults unless explicitly asked via the CLI.
    if opts.mode == SeedMode::Prod {
        return Ok(());
    }

    // DEV: create a usable environment without the install wizard.
    let user_id = f
        .ensure_user(
            &opts.admin_email,
            &opts.admin_name,
            &opts.admin_password,
            "admin",
            true,
        )
        .await?;

    let tenant_id = f
        .ensure_tenant(&opts.tenant_name, &opts.tenant_slug)
        .await?;
    f.ensure_tenant_member(&tenant_id, &user_id, "owner")
        .await?;
    f.ensure_tenant_subscription_default(&tenant_id).await?;

    Ok(())
}
