use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    ManagedRadiusNas, ManagedRadiusRouterSetup, ManagedRadiusServer, MikrotikRouter, PppoeAccount,
};
use crate::security::secret::{decrypt_secret_opt_for, encrypt_secret_for};
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

const PURPOSE_MANAGED_RADIUS_DB: &str = "managed_radius_db";
const PURPOSE_MANAGED_RADIUS_SHARED_SECRET: &str = "managed_radius_shared_secret";
const DEFAULT_RADIUS_AUTH_PORT: i32 = 1812;
const DEFAULT_RADIUS_ACCT_PORT: i32 = 1813;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedRadiusAccountPayload {
    pub radius_identity: String,
    pub cleartext_password: String,
    pub profile_name: Option<String>,
    pub remote_address: Option<String>,
    pub address_pool: Option<String>,
    pub disabled: bool,
    pub comment: Option<String>,
}

impl ManagedRadiusAccountPayload {
    fn from_account(account: &PppoeAccount, cleartext_password: &str) -> Self {
        let radius_identity = account
            .radius_identity
            .clone()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| account.username.clone());

        Self {
            radius_identity,
            cleartext_password: cleartext_password.to_string(),
            profile_name: account
                .router_profile_name
                .clone()
                .filter(|v| !v.trim().is_empty()),
            remote_address: account
                .remote_address
                .clone()
                .filter(|v| !v.trim().is_empty()),
            address_pool: account
                .address_pool
                .clone()
                .filter(|v| !v.trim().is_empty()),
            disabled: account.disabled,
            comment: account.comment.clone().filter(|v| !v.trim().is_empty()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ManagedRadiusApplyResult {
    pub radius_identity: String,
}

#[derive(Clone)]
pub struct ManagedRadiusService {
    pool: DbPool,
}

impl ManagedRadiusService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    async fn load_router_config(
        &self,
        tenant_id: &str,
        router_id: &str,
    ) -> AppResult<(ManagedRadiusServer, ManagedRadiusNas)> {
        let server = sqlx::query_as::<_, ManagedRadiusServer>(
            r#"
            SELECT s.*
            FROM managed_radius_servers s
            INNER JOIN managed_radius_nas n
              ON n.radius_server_id = s.id
            WHERE s.tenant_id = $1
              AND n.tenant_id = $1
              AND n.router_id = $2
              AND s.is_active = true
              AND n.is_active = true
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::Configuration(
                "Managed RADIUS is not configured for the selected router".into(),
            )
        })?;

        let nas = sqlx::query_as::<_, ManagedRadiusNas>(
            r#"
            SELECT *
            FROM managed_radius_nas
            WHERE tenant_id = $1
              AND router_id = $2
              AND radius_server_id = $3
              AND is_active = true
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .bind(&server.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::Configuration("Managed RADIUS NAS entry is missing for router".into())
        })?;

        Ok((server, nas))
    }

    async fn connect_radius_db(&self, server: &ManagedRadiusServer) -> AppResult<PgPool> {
        let password = decrypt_secret_opt_for(PURPOSE_MANAGED_RADIUS_DB, &server.db_password_enc)?
            .unwrap_or_default();
        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            server.db_user, password, server.db_host, server.db_port, server.db_name
        );
        PgPoolOptions::new()
            .max_connections(3)
            .connect(&url)
            .await
            .map_err(|e| {
                AppError::ServiceUnavailable(format!("Managed RADIUS DB unavailable: {e}"))
            })
    }

    async fn ensure_radius_schema(&self, radius_pool: &PgPool) -> AppResult<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS managed_radius_nas (
              id text PRIMARY KEY,
              tenant_id text NOT NULL,
              router_id text NOT NULL,
              nas_name text NOT NULL,
              nas_ip_or_cidr text NOT NULL,
              shared_secret text NOT NULL,
              shortname text,
              is_active boolean NOT NULL DEFAULT true,
              created_at timestamp with time zone NOT NULL,
              updated_at timestamp with time zone NOT NULL,
              UNIQUE (tenant_id, router_id),
              UNIQUE (nas_ip_or_cidr)
            )
            "#,
        )
        .execute(radius_pool)
        .await
        .map_err(AppError::Database)?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS managed_radius_accounts (
              id text PRIMARY KEY,
              tenant_id text NOT NULL,
              router_id text NOT NULL,
              username text NOT NULL,
              radius_identity text NOT NULL,
              cleartext_password text NOT NULL,
              profile_name text,
              remote_address text,
              address_pool text,
              disabled boolean NOT NULL DEFAULT false,
              comment text,
              created_at timestamp with time zone NOT NULL,
              updated_at timestamp with time zone NOT NULL,
              UNIQUE (tenant_id, username),
              UNIQUE (tenant_id, radius_identity)
            )
            "#,
        )
        .execute(radius_pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    async fn upsert_nas(&self, radius_pool: &PgPool, nas: &ManagedRadiusNas) -> AppResult<()> {
        let secret =
            decrypt_secret_opt_for(PURPOSE_MANAGED_RADIUS_SHARED_SECRET, &nas.shared_secret_enc)?
                .unwrap_or_default();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO managed_radius_nas (
              id, tenant_id, router_id, nas_name, nas_ip_or_cidr, shared_secret,
              shortname, is_active, created_at, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            ON CONFLICT (tenant_id, router_id) DO UPDATE SET
              nas_name = EXCLUDED.nas_name,
              nas_ip_or_cidr = EXCLUDED.nas_ip_or_cidr,
              shared_secret = EXCLUDED.shared_secret,
              shortname = EXCLUDED.shortname,
              is_active = EXCLUDED.is_active,
              updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(&nas.id)
        .bind(&nas.tenant_id)
        .bind(&nas.router_id)
        .bind(&nas.nas_name)
        .bind(&nas.nas_ip_or_cidr)
        .bind(&secret)
        .bind(&nas.shortname)
        .bind(nas.is_active)
        .bind(now)
        .bind(now)
        .execute(radius_pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    pub async fn apply_account(
        &self,
        tenant_id: &str,
        account: &PppoeAccount,
        cleartext_password: &str,
    ) -> AppResult<ManagedRadiusApplyResult> {
        let (server, nas) = self
            .load_router_config(tenant_id, &account.router_id)
            .await?;
        let radius_pool = self.connect_radius_db(&server).await?;
        self.ensure_radius_schema(&radius_pool).await?;
        self.upsert_nas(&radius_pool, &nas).await?;

        let payload = ManagedRadiusAccountPayload::from_account(account, cleartext_password);
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO managed_radius_accounts (
              id, tenant_id, router_id, username, radius_identity, cleartext_password,
              profile_name, remote_address, address_pool, disabled, comment, created_at, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            ON CONFLICT (tenant_id, username) DO UPDATE SET
              router_id = EXCLUDED.router_id,
              radius_identity = EXCLUDED.radius_identity,
              cleartext_password = EXCLUDED.cleartext_password,
              profile_name = EXCLUDED.profile_name,
              remote_address = EXCLUDED.remote_address,
              address_pool = EXCLUDED.address_pool,
              disabled = EXCLUDED.disabled,
              comment = EXCLUDED.comment,
              updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(&account.router_id)
        .bind(&account.username)
        .bind(&payload.radius_identity)
        .bind(&payload.cleartext_password)
        .bind(&payload.profile_name)
        .bind(&payload.remote_address)
        .bind(&payload.address_pool)
        .bind(payload.disabled)
        .bind(&payload.comment)
        .bind(now)
        .bind(now)
        .execute(&radius_pool)
        .await
        .map_err(AppError::Database)?;

        Ok(ManagedRadiusApplyResult {
            radius_identity: payload.radius_identity,
        })
    }

    pub async fn delete_account(&self, tenant_id: &str, account: &PppoeAccount) -> AppResult<()> {
        let (server, _) = self
            .load_router_config(tenant_id, &account.router_id)
            .await?;
        let radius_pool = self.connect_radius_db(&server).await?;
        self.ensure_radius_schema(&radius_pool).await?;

        sqlx::query("DELETE FROM managed_radius_accounts WHERE tenant_id = $1 AND username = $2")
            .bind(tenant_id)
            .bind(&account.username)
            .execute(&radius_pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    pub async fn reconcile_router(
        &self,
        tenant_id: &str,
        router_id: &str,
    ) -> AppResult<HashSet<String>> {
        let (server, _) = self.load_router_config(tenant_id, router_id).await?;
        let radius_pool = self.connect_radius_db(&server).await?;
        self.ensure_radius_schema(&radius_pool).await?;

        let usernames = sqlx::query_scalar::<_, String>(
            r#"
            SELECT username
            FROM managed_radius_accounts
            WHERE tenant_id = $1 AND router_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_all(&radius_pool)
        .await
        .map_err(AppError::Database)?;

        Ok(usernames.into_iter().collect())
    }

    pub fn encrypt_db_password(plaintext: &str) -> AppResult<String> {
        encrypt_secret_for(PURPOSE_MANAGED_RADIUS_DB, plaintext)
    }

    pub fn encrypt_shared_secret(plaintext: &str) -> AppResult<String> {
        encrypt_secret_for(PURPOSE_MANAGED_RADIUS_SHARED_SECRET, plaintext)
    }

    pub async fn get_router_setup(
        &self,
        tenant_id: &str,
        router: &MikrotikRouter,
    ) -> AppResult<ManagedRadiusRouterSetup> {
        let config = sqlx::query_as::<_, ManagedRadiusNas>(
            r#"
            SELECT n.*
            FROM managed_radius_nas n
            INNER JOIN managed_radius_servers s
              ON s.id = n.radius_server_id
            WHERE n.tenant_id = $1
              AND n.router_id = $2
              AND n.is_active = true
              AND s.tenant_id = $1
              AND s.is_active = true
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&router.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let Some(nas) = config else {
            return Ok(ManagedRadiusRouterSetup {
                configured: false,
                router_id: router.id.clone(),
                server_name: None,
                radius_host: None,
                auth_port: resolve_radius_port(
                    "MANAGED_RADIUS_AUTH_PORT",
                    "RADIUS_AUTH_PORT",
                    DEFAULT_RADIUS_AUTH_PORT,
                ),
                acct_port: resolve_radius_port(
                    "MANAGED_RADIUS_ACCT_PORT",
                    "RADIUS_ACCT_PORT",
                    DEFAULT_RADIUS_ACCT_PORT,
                ),
                nas_ip_or_cidr: None,
                shared_secret: None,
                shared_secret_masked: None,
                cli_script: None,
                warnings: vec![],
            });
        };

        let server = sqlx::query_as::<_, ManagedRadiusServer>(
            r#"
            SELECT *
            FROM managed_radius_servers
            WHERE id = $1
              AND tenant_id = $2
              AND is_active = true
            LIMIT 1
            "#,
        )
        .bind(&nas.radius_server_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let shared_secret =
            decrypt_secret_opt_for(PURPOSE_MANAGED_RADIUS_SHARED_SECRET, &nas.shared_secret_enc)?
                .unwrap_or_default();
        let auth_port = resolve_radius_port(
            "MANAGED_RADIUS_AUTH_PORT",
            "RADIUS_AUTH_PORT",
            DEFAULT_RADIUS_AUTH_PORT,
        );
        let acct_port = resolve_radius_port(
            "MANAGED_RADIUS_ACCT_PORT",
            "RADIUS_ACCT_PORT",
            DEFAULT_RADIUS_ACCT_PORT,
        );
        let (radius_host, host_warning) = resolve_radius_host(&server.db_host);
        let warnings = host_warning.into_iter().collect::<Vec<_>>();
        let cli_script = build_routeros_cli(&radius_host, &shared_secret, auth_port, acct_port);

        Ok(ManagedRadiusRouterSetup {
            configured: true,
            router_id: router.id.clone(),
            server_name: Some(server.name),
            radius_host: Some(radius_host),
            auth_port,
            acct_port,
            nas_ip_or_cidr: Some(nas.nas_ip_or_cidr),
            shared_secret_masked: Some(mask_shared_secret(&shared_secret)),
            shared_secret: Some(shared_secret),
            cli_script: Some(cli_script),
            warnings,
        })
    }
}

fn resolve_radius_port(primary_env: &str, fallback_env: &str, default_port: i32) -> i32 {
    std::env::var(primary_env)
        .ok()
        .or_else(|| std::env::var(fallback_env).ok())
        .and_then(|raw| raw.trim().parse::<i32>().ok())
        .filter(|port| *port > 0)
        .unwrap_or(default_port)
}

fn resolve_radius_host(default_host: &str) -> (String, Option<String>) {
    let env_host = std::env::var("MANAGED_RADIUS_HOST")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("RADIUS_PUBLIC_HOST")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });

    match env_host {
        Some(host) => (host, None),
        None => (
            default_host.to_string(),
            Some(
                "MANAGED_RADIUS_HOST is not set, so the CLI uses the managed RADIUS database host as a fallback.".into(),
            ),
        ),
    }
}

fn build_routeros_cli(
    radius_host: &str,
    shared_secret: &str,
    auth_port: i32,
    acct_port: i32,
) -> String {
    [
        format!(
            "/radius add service=ppp address={} secret={} authentication-port={} accounting-port={} protocol=udp",
            routeros_quote(radius_host),
            routeros_quote(shared_secret),
            auth_port,
            acct_port
        ),
        "/ppp aaa set use-radius=yes accounting=yes".to_string(),
    ]
    .join("\n")
}

fn routeros_quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn mask_shared_secret(secret: &str) -> String {
    let chars = secret.chars().count();
    if chars <= 8 {
        return "••••••••".into();
    }

    let prefix = secret.chars().take(4).collect::<String>();
    let suffix = secret
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{prefix}••••••••{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PppoeAccount, PppoeAccountSource};

    fn sample_account() -> PppoeAccount {
        let now = Utc::now();
        PppoeAccount {
            id: "acct-1".into(),
            tenant_id: "tenant-1".into(),
            router_id: "router-1".into(),
            customer_id: "cust-1".into(),
            location_id: "loc-1".into(),
            username: "alice".into(),
            password_enc: "enc".into(),
            package_id: None,
            profile_id: None,
            router_profile_name: Some("basic".into()),
            remote_address: None,
            address_pool: Some("pool-a".into()),
            disabled: false,
            comment: Some("hello".into()),
            account_source: PppoeAccountSource::ManagedRadius,
            router_present: false,
            router_secret_id: None,
            last_sync_at: None,
            last_error: None,
            radius_present: false,
            radius_identity: None,
            radius_last_sync_at: None,
            radius_last_error: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn payload_defaults_radius_identity_to_username() {
        let payload = ManagedRadiusAccountPayload::from_account(&sample_account(), "secret");
        assert_eq!(payload.radius_identity, "alice");
        assert_eq!(payload.address_pool.as_deref(), Some("pool-a"));
        assert_eq!(payload.profile_name.as_deref(), Some("basic"));
    }

    #[test]
    fn payload_prefers_explicit_radius_identity() {
        let mut account = sample_account();
        account.radius_identity = Some("tenant-1/alice".into());

        let payload = ManagedRadiusAccountPayload::from_account(&account, "secret");
        assert_eq!(payload.radius_identity, "tenant-1/alice");
    }

    #[test]
    fn routeros_cli_quotes_host_and_secret() {
        let script = build_routeros_cli("radius.example.com", "s3cr\"et", 1812, 1813);
        assert!(script.contains("address=\"radius.example.com\""));
        assert!(script.contains("secret=\"s3cr\\\"et\""));
        assert!(script.contains("/ppp aaa set use-radius=yes accounting=yes"));
    }

    #[test]
    fn secret_masking_keeps_edges() {
        assert_eq!(
            mask_shared_secret("JNkLuybWiKmHoIW4RbkjBc4pUO2dDPKQ"),
            "JNkL••••••••DPKQ"
        );
        assert_eq!(mask_shared_secret("short"), "••••••••");
    }

    #[test]
    fn host_resolution_prefers_public_env() {
        unsafe {
            std::env::set_var("MANAGED_RADIUS_HOST", "radius-public.example.com");
            std::env::remove_var("RADIUS_PUBLIC_HOST");
        }
        let (host, warning) = resolve_radius_host("radius-postgres");
        assert_eq!(host, "radius-public.example.com");
        assert_eq!(warning, None);
        unsafe {
            std::env::remove_var("MANAGED_RADIUS_HOST");
        }
    }

    #[test]
    fn host_resolution_falls_back_to_server_host_with_warning() {
        unsafe {
            std::env::remove_var("MANAGED_RADIUS_HOST");
            std::env::remove_var("RADIUS_PUBLIC_HOST");
        }
        let (host, warning) = resolve_radius_host("radius-postgres");
        assert_eq!(host, "radius-postgres");
        assert!(warning.is_some());
    }
}
