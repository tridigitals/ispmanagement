use anyhow::{anyhow, Result};
use chrono::Utc;
use saas_tauri_lib::db::init_db;
use saas_tauri_lib::security::secret::encrypt_secret_for;
use saas_tauri_lib::services::{
    AuditService, AuthService, EmailService, PppoeService, SettingsService,
};
use std::env;
use uuid::Uuid;

const PURPOSE_PPPOE: &str = "pppoe_secrets";

#[derive(Debug)]
struct Options {
    tenant_id: String,
    router_id: String,
    username: String,
    password: String,
    disabled: bool,
}

fn parse_args() -> Result<Options> {
    let mut args = env::args().skip(1);
    let mut tenant_id = None;
    let mut router_id = None;
    let mut username = Some("pilot-managed-radius-smoke".to_string());
    let mut password = Some("pilot-radius-pass".to_string());
    let mut disabled = true;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--tenant-id" => tenant_id = args.next(),
            "--router-id" => router_id = args.next(),
            "--username" => username = args.next(),
            "--password" => password = args.next(),
            "--enabled" => disabled = false,
            "--disabled" => disabled = true,
            other => return Err(anyhow!("Unknown argument: {other}")),
        }
    }

    Ok(Options {
        tenant_id: tenant_id.ok_or_else(|| anyhow!("Missing --tenant-id"))?,
        router_id: router_id.ok_or_else(|| anyhow!("Missing --router-id"))?,
        username: username.unwrap_or_else(|| "pilot-managed-radius-smoke".to_string()),
        password: password.unwrap_or_else(|| "pilot-radius-pass".to_string()),
        disabled,
    })
}

#[derive(Debug, sqlx::FromRow)]
struct ExistingScope {
    customer_id: String,
    location_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let opts = parse_args()?;
    let pool = init_db(std::env::temp_dir()).await?;

    let scope = sqlx::query_as::<_, ExistingScope>(
        r#"
        SELECT customer_id, location_id
        FROM pppoe_accounts
        WHERE tenant_id = $1 AND router_id = $2
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(&opts.tenant_id)
    .bind(&opts.router_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| anyhow!("No existing PPPoE account found to borrow customer/location scope"))?;

    let password_enc = encrypt_secret_for(PURPOSE_PPPOE, &opts.password)?;
    let now = Utc::now();

    let account_id: String = sqlx::query_scalar(
        r#"
        INSERT INTO pppoe_accounts (
          id, tenant_id, router_id, customer_id, location_id, username, password_enc,
          package_id, profile_id, router_profile_name, remote_address, address_pool,
          disabled, comment, account_source, router_present, router_secret_id,
          last_sync_at, last_error, is_provisioned, radius_identity,
          provisioned_at, provisioning_error, created_at, updated_at
        ) VALUES (
          $1,$2,$3,$4,$5,$6,$7,
          NULL,NULL,NULL,NULL,NULL,
          $8,'Pilot managed radius smoke test','managed_radius',false,NULL,
          NULL,NULL,false,$6,
          NULL,NULL,$9,$10
        )
        ON CONFLICT (tenant_id, router_id, username) DO UPDATE SET
          password_enc = EXCLUDED.password_enc,
          disabled = EXCLUDED.disabled,
          comment = EXCLUDED.comment,
          account_source = 'managed_radius',
          radius_identity = EXCLUDED.radius_identity,
          updated_at = EXCLUDED.updated_at
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&opts.tenant_id)
    .bind(&opts.router_id)
    .bind(&scope.customer_id)
    .bind(&scope.location_id)
    .bind(&opts.username)
    .bind(&password_enc)
    .bind(opts.disabled)
    .bind(now)
    .bind(now)
    .fetch_one(&pool)
    .await?;

    let audit_service = AuditService::new(pool.clone(), None);
    let settings_service = SettingsService::new(pool.clone(), audit_service.clone());
    let email_service = EmailService::new(settings_service.clone());
    let auth_service = AuthService::new(
        pool.clone(),
        "pilot-managed-radius-cli".to_string(),
        email_service,
        audit_service.clone(),
        settings_service.clone(),
    );
    let pppoe_service =
        PppoeService::new(pool.clone(), auth_service, audit_service, settings_service);

    let applied = pppoe_service
        .apply_account_direct(&opts.tenant_id, &account_id)
        .await?;

    println!("pppoe_account_id={}", applied.id);
    println!("username={}", applied.username);
    println!("account_source={:?}", applied.account_source);
    println!("is_provisioned={}", applied.is_provisioned);
    println!(
        "radius_identity={}",
        applied.radius_identity.unwrap_or_else(|| "-".to_string())
    );

    Ok(())
}
