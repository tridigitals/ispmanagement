use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    ManagedRadiusNas, ManagedRadiusRouterSetup, ManagedRadiusServer, MikrotikRouter, PppoeAccount,
    TenantRadiusAssignment,
};
use crate::security::secret::{decrypt_secret_opt_for, encrypt_secret_for};
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use tokio::process::Command;
use tokio::sync::Mutex;
use uuid::Uuid;

const PURPOSE_MANAGED_RADIUS_DB: &str = "managed_radius_db";
const PURPOSE_MANAGED_RADIUS_SHARED_SECRET: &str = "managed_radius_shared_secret";
const DEFAULT_RADIUS_AUTH_PORT: i32 = 1812;
const DEFAULT_RADIUS_ACCT_PORT: i32 = 1813;
const MANAGED_RADIUS_UPGRADE_PATH: &str = "/admin/subscription";
const MANAGED_RADIUS_RESTART_COMMAND_ENV: &str = "MANAGED_RADIUS_RESTART_COMMAND";
const MANAGED_RADIUS_RESTART_WORKDIR_ENV: &str = "MANAGED_RADIUS_RESTART_WORKDIR";
static MANAGED_RADIUS_SCHEMA_READY_CACHE: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static MANAGED_RADIUS_POOL_CACHE: OnceLock<Mutex<HashMap<String, PgPool>>> = OnceLock::new();

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

#[derive(Debug, Clone)]
pub struct ManagedRadiusServerUpsert {
    pub name: String,
    pub db_host: String,
    pub db_port: Option<i32>,
    pub db_name: String,
    pub db_user: String,
    pub db_password: Option<String>,
    pub is_active: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TenantRadiusAssignmentUpsert {
    pub tenant_id: String,
    pub radius_server_id: String,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct ManagedRadiusNasUpsert {
    pub tenant_id: String,
    pub radius_server_id: String,
    pub router_id: String,
    pub nas_name: String,
    pub nas_ip_or_cidr: String,
    pub shortname: Option<String>,
    pub shared_secret: Option<String>,
    pub is_active: bool,
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
            FROM radius_servers s
            INNER JOIN tenant_radius_assignments a
              ON a.radius_server_id = s.id
             AND a.tenant_id = $1
             AND a.is_active = true
            INNER JOIN managed_radius_nas n
              ON n.radius_server_id = s.id
            WHERE n.tenant_id = $1
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

    async fn get_active_assignment_for_tenant(
        &self,
        tenant_id: &str,
    ) -> AppResult<TenantRadiusAssignment> {
        sqlx::query_as::<_, TenantRadiusAssignment>(
            r#"
            SELECT *
            FROM tenant_radius_assignments
            WHERE tenant_id = $1
              AND is_active = true
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::Configuration("Managed RADIUS tenant assignment is not configured".into())
        })
    }

    pub async fn get_active_assignment_for_tenant_optional(
        &self,
        tenant_id: &str,
    ) -> AppResult<Option<TenantRadiusAssignment>> {
        sqlx::query_as::<_, TenantRadiusAssignment>(
            r#"
            SELECT *
            FROM tenant_radius_assignments
            WHERE tenant_id = $1
              AND is_active = true
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn connect_radius_db(&self, server: &ManagedRadiusServer) -> AppResult<PgPool> {
        let cache = MANAGED_RADIUS_POOL_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let cache_key = Self::schema_cache_key(server);
        {
            let guard = cache.lock().await;
            if let Some(pool) = guard.get(&cache_key) {
                return Ok(pool.clone());
            }
        }

        let password = decrypt_secret_opt_for(PURPOSE_MANAGED_RADIUS_DB, &server.db_password_enc)?
            .unwrap_or_default();
        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            server.db_user, password, server.db_host, server.db_port, server.db_name
        );
        let pool = PgPoolOptions::new()
            .max_connections(3)
            .connect(&url)
            .await
            .map_err(|e| {
                AppError::ServiceUnavailable(format!("Managed RADIUS DB unavailable: {e}"))
            })?;

        let mut guard = cache.lock().await;
        if let Some(existing) = guard.get(&cache_key) {
            return Ok(existing.clone());
        }
        guard.insert(cache_key, pool.clone());
        Ok(pool)
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

    fn schema_cache_key(server: &ManagedRadiusServer) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            server.id, server.db_host, server.db_port, server.db_name, server.db_user
        )
    }

    async fn ensure_radius_schema_cached(
        &self,
        server: &ManagedRadiusServer,
        radius_pool: &PgPool,
    ) -> AppResult<()> {
        let cache = MANAGED_RADIUS_SCHEMA_READY_CACHE.get_or_init(|| Mutex::new(HashSet::new()));
        let cache_key = Self::schema_cache_key(server);

        {
            let guard = cache.lock().await;
            if guard.contains(&cache_key) {
                return Ok(());
            }
        }

        self.ensure_radius_schema(radius_pool).await?;

        let mut guard = cache.lock().await;
        guard.insert(cache_key);
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

    async fn load_server_by_id(&self, radius_server_id: &str) -> AppResult<ManagedRadiusServer> {
        sqlx::query_as::<_, ManagedRadiusServer>("SELECT * FROM radius_servers WHERE id = $1")
            .bind(radius_server_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("Managed RADIUS server not found".into()))
    }

    async fn sync_runtime_nas(&self, nas: &ManagedRadiusNas) -> AppResult<()> {
        let server = self.load_server_by_id(&nas.radius_server_id).await?;
        let radius_pool = self.connect_radius_db(&server).await?;
        self.ensure_radius_schema_cached(&server, &radius_pool)
            .await?;
        self.upsert_nas(&radius_pool, nas).await
    }

    async fn sync_runtime_nas_by_mapping_id(&self, mapping_id: &str) -> AppResult<()> {
        let nas =
            sqlx::query_as::<_, ManagedRadiusNas>("SELECT * FROM managed_radius_nas WHERE id = $1")
                .bind(mapping_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound("Managed RADIUS mapping not found".into()))?;

        self.sync_runtime_nas(&nas).await
    }

    async fn restart_freeradius_after_mapping_change_if_configured(&self) -> AppResult<()> {
        let Some(command) = resolve_managed_radius_restart_command() else {
            return Ok(());
        };

        let mut process = Command::new("sh");
        process.arg("-lc").arg(&command);

        if let Some(workdir) = resolve_managed_radius_restart_workdir() {
            process.current_dir(workdir);
        }

        let output = process.output().await.map_err(|error| {
            AppError::ServiceUnavailable(format!(
                "Failed to execute managed RADIUS restart command: {error}"
            ))
        })?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("exit status {}", output.status)
        };

        Err(AppError::ServiceUnavailable(format!(
            "Managed RADIUS restart command failed: {detail}"
        )))
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
        self.ensure_radius_schema_cached(&server, &radius_pool)
            .await?;
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
        self.ensure_radius_schema_cached(&server, &radius_pool)
            .await?;

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
        self.ensure_radius_schema_cached(&server, &radius_pool)
            .await?;

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

    pub fn mask_shared_secret_for_display(secret: &str) -> String {
        mask_shared_secret(secret)
    }

    pub fn generate_shared_secret_for_display() -> String {
        generate_managed_radius_shared_secret()
    }

    pub async fn create_server(
        &self,
        input: ManagedRadiusServerUpsert,
    ) -> AppResult<ManagedRadiusServer> {
        let name = required_trimmed("name", &input.name)?;
        let db_host = required_trimmed("db_host", &input.db_host)?;
        let db_name = required_trimmed("db_name", &input.db_name)?;
        let db_user = required_trimmed("db_user", &input.db_user)?;
        let db_port = normalize_managed_radius_db_port(input.db_port);
        let db_password = normalize_optional_secret_input(input.db_password.as_deref())
            .ok_or_else(|| AppError::Validation("db_password is required".into()))?;
        let db_password_enc = Self::encrypt_db_password(&db_password)?;
        let notes = normalize_optional_secret_input(input.notes.as_deref());
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        let server = sqlx::query_as::<_, ManagedRadiusServer>(
            r#"
            INSERT INTO radius_servers (
              id, name, db_host, db_port, db_name, db_user, db_password_enc, is_active, is_default, notes, created_at, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,false,$9,$10,$11)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(db_host)
        .bind(db_port)
        .bind(db_name)
        .bind(db_user)
        .bind(db_password_enc)
        .bind(input.is_active)
        .bind(notes)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(server)
    }

    pub async fn update_server(
        &self,
        server_id: &str,
        input: ManagedRadiusServerUpsert,
    ) -> AppResult<ManagedRadiusServer> {
        let name = required_trimmed("name", &input.name)?;
        let db_host = required_trimmed("db_host", &input.db_host)?;
        let db_name = required_trimmed("db_name", &input.db_name)?;
        let db_user = required_trimmed("db_user", &input.db_user)?;
        let db_port = normalize_managed_radius_db_port(input.db_port);

        let existing =
            sqlx::query_as::<_, ManagedRadiusServer>("SELECT * FROM radius_servers WHERE id = $1")
                .bind(server_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound("Managed RADIUS server not found".into()))?;

        let db_password_enc = match normalize_optional_secret_input(input.db_password.as_deref()) {
            Some(password) => Self::encrypt_db_password(&password)?,
            None => existing.db_password_enc,
        };
        let notes = normalize_optional_secret_input(input.notes.as_deref());

        let now = Utc::now();

        let server = sqlx::query_as::<_, ManagedRadiusServer>(
            r#"
            UPDATE radius_servers
            SET name = $1,
                db_host = $2,
                db_port = $3,
                db_name = $4,
                db_user = $5,
                db_password_enc = $6,
                is_active = $7,
                notes = $8,
                updated_at = $9
            WHERE id = $10
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(db_host)
        .bind(db_port)
        .bind(db_name)
        .bind(db_user)
        .bind(db_password_enc)
        .bind(input.is_active)
        .bind(notes)
        .bind(now)
        .bind(server_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(server)
    }

    pub async fn set_server_active(
        &self,
        server_id: &str,
        is_active: bool,
    ) -> AppResult<ManagedRadiusServer> {
        let existing =
            sqlx::query_as::<_, ManagedRadiusServer>("SELECT * FROM radius_servers WHERE id = $1")
                .bind(server_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound("Managed RADIUS server not found".into()))?;

        if !is_active && existing.is_default {
            return Err(AppError::Validation(
                "Default Managed RADIUS server must stay active until another default is selected"
                    .into(),
            ));
        }

        let now = Utc::now();
        let server = sqlx::query_as::<_, ManagedRadiusServer>(
            "UPDATE radius_servers SET is_active = $1, updated_at = $2 WHERE id = $3 RETURNING *",
        )
        .bind(is_active)
        .bind(now)
        .bind(server_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Managed RADIUS server not found".into()))?;
        Ok(server)
    }

    pub async fn get_default_server(&self) -> AppResult<Option<ManagedRadiusServer>> {
        sqlx::query_as::<_, ManagedRadiusServer>(
            r#"
            SELECT *
            FROM radius_servers
            WHERE is_default = true
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    pub async fn set_server_default(&self, server_id: &str) -> AppResult<ManagedRadiusServer> {
        let existing =
            sqlx::query_as::<_, ManagedRadiusServer>("SELECT * FROM radius_servers WHERE id = $1")
                .bind(server_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| AppError::NotFound("Managed RADIUS server not found".into()))?;

        if !existing.is_active {
            return Err(AppError::Validation(
                "Only active Managed RADIUS servers can be set as default".into(),
            ));
        }

        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        sqlx::query(
            "UPDATE radius_servers SET is_default = false, updated_at = $1 WHERE is_default = true",
        )
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        let server = sqlx::query_as::<_, ManagedRadiusServer>(
            "UPDATE radius_servers SET is_default = true, updated_at = $1 WHERE id = $2 RETURNING *",
        )
        .bind(now)
        .bind(server_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;
        Ok(server)
    }

    pub async fn auto_assign_default_server_for_tenant(
        &self,
        tenant_id: &str,
    ) -> AppResult<Option<TenantRadiusAssignment>> {
        let Some(server) = self.get_default_server().await? else {
            return Ok(None);
        };

        let assignment = self
            .create_assignment(TenantRadiusAssignmentUpsert {
                tenant_id: tenant_id.to_string(),
                radius_server_id: server.id,
                is_active: true,
            })
            .await?;

        Ok(Some(assignment))
    }

    pub async fn create_assignment(
        &self,
        input: TenantRadiusAssignmentUpsert,
    ) -> AppResult<TenantRadiusAssignment> {
        let tenant_id = required_trimmed("tenant_id", &input.tenant_id)?;
        let radius_server_id = required_trimmed("radius_server_id", &input.radius_server_id)?;
        self.ensure_tenant_exists(tenant_id).await?;
        self.ensure_server_exists(radius_server_id).await?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        if input.is_active {
            sqlx::query(
                "UPDATE tenant_radius_assignments SET is_active = false, updated_at = $2 WHERE tenant_id = $1",
            )
            .bind(tenant_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }

        let assignment = sqlx::query_as::<_, TenantRadiusAssignment>(
            r#"
            INSERT INTO tenant_radius_assignments (
              id, tenant_id, radius_server_id, is_active, assigned_at, created_at, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7)
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(radius_server_id)
        .bind(input.is_active)
        .bind(now)
        .bind(now)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;
        Ok(assignment)
    }

    pub async fn update_assignment(
        &self,
        assignment_id: &str,
        input: TenantRadiusAssignmentUpsert,
    ) -> AppResult<TenantRadiusAssignment> {
        let tenant_id = required_trimmed("tenant_id", &input.tenant_id)?;
        let radius_server_id = required_trimmed("radius_server_id", &input.radius_server_id)?;
        self.ensure_tenant_exists(tenant_id).await?;
        self.ensure_server_exists(radius_server_id).await?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        if input.is_active {
            sqlx::query(
                "UPDATE tenant_radius_assignments SET is_active = false, updated_at = $2 WHERE tenant_id = $1 AND id <> $3",
            )
            .bind(tenant_id)
            .bind(now)
            .bind(assignment_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }

        let assignment = sqlx::query_as::<_, TenantRadiusAssignment>(
            r#"
            UPDATE tenant_radius_assignments
            SET tenant_id = $1,
                radius_server_id = $2,
                is_active = $3,
                assigned_at = $4,
                updated_at = $5
            WHERE id = $6
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(radius_server_id)
        .bind(input.is_active)
        .bind(now)
        .bind(now)
        .bind(assignment_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Managed RADIUS assignment not found".into()))?;

        tx.commit().await.map_err(AppError::Database)?;
        Ok(assignment)
    }

    pub async fn set_assignment_active(
        &self,
        tenant_id: &str,
        assignment_id: &str,
        is_active: bool,
    ) -> AppResult<TenantRadiusAssignment> {
        let tenant_id = required_trimmed("tenant_id", tenant_id)?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        if is_active {
            sqlx::query(
                "UPDATE tenant_radius_assignments SET is_active = false, updated_at = $2 WHERE tenant_id = $1 AND id <> $3",
            )
            .bind(tenant_id)
            .bind(now)
            .bind(assignment_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }

        let assignment = sqlx::query_as::<_, TenantRadiusAssignment>(
            r#"
            UPDATE tenant_radius_assignments
            SET is_active = $1,
                updated_at = $2
            WHERE id = $3 AND tenant_id = $4
            RETURNING *
            "#,
        )
        .bind(is_active)
        .bind(now)
        .bind(assignment_id)
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Managed RADIUS assignment not found".into()))?;

        tx.commit().await.map_err(AppError::Database)?;
        Ok(assignment)
    }

    pub async fn create_mapping(
        &self,
        input: ManagedRadiusNasUpsert,
    ) -> AppResult<ManagedRadiusNas> {
        let tenant_id = required_trimmed("tenant_id", &input.tenant_id)?;
        let radius_server_id = required_trimmed("radius_server_id", &input.radius_server_id)?;
        let router_id = required_trimmed("router_id", &input.router_id)?;
        let nas_name = required_trimmed("nas_name", &input.nas_name)?;
        let nas_ip_or_cidr = required_trimmed("nas_ip_or_cidr", &input.nas_ip_or_cidr)?;
        let shortname = normalize_optional_secret_input(input.shortname.as_deref());
        self.ensure_server_and_router_belong_to_tenant(tenant_id, radius_server_id, router_id)
            .await?;

        let shared_secret = normalize_optional_secret_input(input.shared_secret.as_deref())
            .unwrap_or_else(generate_managed_radius_shared_secret);
        let shared_secret_enc = Self::encrypt_shared_secret(&shared_secret)?;
        let now = Utc::now();

        let mapping = sqlx::query_as::<_, ManagedRadiusNas>(
            r#"
            INSERT INTO managed_radius_nas (
              id, tenant_id, router_id, radius_server_id, nas_name, nas_ip_or_cidr, shared_secret_enc, shortname, is_active, created_at, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id)
        .bind(router_id)
        .bind(radius_server_id)
        .bind(nas_name)
        .bind(nas_ip_or_cidr)
        .bind(shared_secret_enc)
        .bind(shortname)
        .bind(input.is_active)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.sync_runtime_nas_by_mapping_id(&mapping.id).await?;

        Ok(mapping)
    }

    pub async fn auto_create_mapping_for_router(
        &self,
        tenant_id: &str,
        router: &MikrotikRouter,
    ) -> AppResult<ManagedRadiusNas> {
        let assignment = self.get_active_assignment_for_tenant(tenant_id).await?;

        let existing = sqlx::query_as::<_, ManagedRadiusNas>(
            r#"
            SELECT *
            FROM managed_radius_nas
            WHERE tenant_id = $1
              AND router_id = $2
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(&router.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if existing.is_some() {
            return Err(AppError::Validation(
                "Managed RADIUS NAS mapping already exists for this router".into(),
            ));
        }

        let nas_name = build_default_nas_name(router);
        let shortname = build_default_nas_shortname(router);

        self.create_mapping(ManagedRadiusNasUpsert {
            tenant_id: tenant_id.to_string(),
            radius_server_id: assignment.radius_server_id,
            router_id: router.id.clone(),
            nas_name,
            nas_ip_or_cidr: router.host.clone(),
            shortname,
            shared_secret: None,
            is_active: true,
        })
        .await
    }

    pub async fn update_mapping(
        &self,
        mapping_id: &str,
        input: ManagedRadiusNasUpsert,
    ) -> AppResult<ManagedRadiusNas> {
        let tenant_id = required_trimmed("tenant_id", &input.tenant_id)?;
        let radius_server_id = required_trimmed("radius_server_id", &input.radius_server_id)?;
        let router_id = required_trimmed("router_id", &input.router_id)?;
        let nas_name = required_trimmed("nas_name", &input.nas_name)?;
        let nas_ip_or_cidr = required_trimmed("nas_ip_or_cidr", &input.nas_ip_or_cidr)?;
        let shortname = normalize_optional_secret_input(input.shortname.as_deref());
        self.ensure_server_and_router_belong_to_tenant(tenant_id, radius_server_id, router_id)
            .await?;

        let existing = sqlx::query_as::<_, ManagedRadiusNas>(
            "SELECT * FROM managed_radius_nas WHERE id = $1 AND tenant_id = $2",
        )
        .bind(mapping_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Managed RADIUS mapping not found".into()))?;

        let shared_secret_enc =
            match normalize_optional_secret_input(input.shared_secret.as_deref()) {
                Some(secret) => Self::encrypt_shared_secret(&secret)?,
                None => existing.shared_secret_enc.clone(),
            };
        let needs_freeradius_restart = mapping_change_requires_freeradius_restart(
            &existing,
            nas_name,
            nas_ip_or_cidr,
            shortname.as_deref(),
            &shared_secret_enc,
            input.is_active,
        );
        let now = Utc::now();

        let mapping = sqlx::query_as::<_, ManagedRadiusNas>(
            r#"
            UPDATE managed_radius_nas
            SET router_id = $1,
                radius_server_id = $2,
                nas_name = $3,
                nas_ip_or_cidr = $4,
                shared_secret_enc = $5,
                shortname = $6,
                is_active = $7,
                updated_at = $8
            WHERE id = $9 AND tenant_id = $10
            RETURNING *
            "#,
        )
        .bind(router_id)
        .bind(radius_server_id)
        .bind(nas_name)
        .bind(nas_ip_or_cidr)
        .bind(shared_secret_enc)
        .bind(shortname)
        .bind(input.is_active)
        .bind(now)
        .bind(mapping_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        self.sync_runtime_nas_by_mapping_id(mapping_id).await?;

        if needs_freeradius_restart {
            self.restart_freeradius_after_mapping_change_if_configured()
                .await?;
        }

        Ok(mapping)
    }

    pub async fn set_mapping_active(
        &self,
        tenant_id: &str,
        mapping_id: &str,
        is_active: bool,
    ) -> AppResult<ManagedRadiusNas> {
        let tenant_id = required_trimmed("tenant_id", tenant_id)?;
        let now = Utc::now();
        let mapping = sqlx::query_as::<_, ManagedRadiusNas>(
            "UPDATE managed_radius_nas SET is_active = $1, updated_at = $2 WHERE id = $3 AND tenant_id = $4 RETURNING *",
        )
        .bind(is_active)
        .bind(now)
        .bind(mapping_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Managed RADIUS mapping not found".into()))?;

        self.sync_runtime_nas_by_mapping_id(mapping_id).await?;

        Ok(mapping)
    }

    pub async fn rotate_mapping_secret(
        &self,
        tenant_id: &str,
        mapping_id: &str,
        shared_secret: Option<String>,
    ) -> AppResult<String> {
        let tenant_id = required_trimmed("tenant_id", tenant_id)?;
        let next_secret = normalize_optional_secret_input(shared_secret.as_deref())
            .unwrap_or_else(generate_managed_radius_shared_secret);
        let next_secret_enc = Self::encrypt_shared_secret(&next_secret)?;
        let now = Utc::now();

        let updated = sqlx::query("UPDATE managed_radius_nas SET shared_secret_enc = $1, updated_at = $2 WHERE id = $3 AND tenant_id = $4")
            .bind(next_secret_enc)
            .bind(now)
            .bind(mapping_id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        if updated.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Managed RADIUS mapping not found".into(),
            ));
        }

        self.sync_runtime_nas_by_mapping_id(mapping_id).await?;

        Ok(next_secret)
    }

    pub async fn reveal_mapping_secret(
        &self,
        tenant_id: &str,
        mapping_id: &str,
    ) -> AppResult<String> {
        let tenant_id = required_trimmed("tenant_id", tenant_id)?;
        let stored = sqlx::query_scalar::<_, String>(
            "SELECT shared_secret_enc FROM managed_radius_nas WHERE id = $1 AND tenant_id = $2",
        )
        .bind(mapping_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Managed RADIUS mapping not found".into()))?;

        decrypt_secret_opt_for(PURPOSE_MANAGED_RADIUS_SHARED_SECRET, &stored)?
            .ok_or_else(|| AppError::Configuration("Managed RADIUS secret is unavailable".into()))
    }

    async fn ensure_server_and_router_belong_to_tenant(
        &self,
        tenant_id: &str,
        radius_server_id: &str,
        router_id: &str,
    ) -> AppResult<()> {
        self.ensure_server_exists(radius_server_id).await?;
        let assignment = self.get_active_assignment_for_tenant(tenant_id).await?;
        if assignment.radius_server_id != radius_server_id {
            return Err(AppError::Validation(
                "Managed RADIUS server must match the tenant's active assignment".into(),
            ));
        }

        let router_exists = sqlx::query_scalar::<_, bool>(
            "SELECT count(*) > 0 FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2",
        )
        .bind(router_id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if !router_exists {
            return Err(AppError::Validation(
                "Router does not belong to the selected tenant".into(),
            ));
        }

        Ok(())
    }

    async fn ensure_server_exists(&self, radius_server_id: &str) -> AppResult<()> {
        let server_exists =
            sqlx::query_scalar::<_, bool>("SELECT count(*) > 0 FROM radius_servers WHERE id = $1")
                .bind(radius_server_id)
                .fetch_one(&self.pool)
                .await
                .map_err(AppError::Database)?;

        if !server_exists {
            return Err(AppError::Validation(
                "Managed RADIUS server does not exist".into(),
            ));
        }

        Ok(())
    }

    async fn ensure_tenant_exists(&self, tenant_id: &str) -> AppResult<()> {
        let tenant_exists =
            sqlx::query_scalar::<_, bool>("SELECT count(*) > 0 FROM tenants WHERE id = $1")
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await
                .map_err(AppError::Database)?;

        if !tenant_exists {
            return Err(AppError::Validation("Tenant does not exist".into()));
        }

        Ok(())
    }

    pub async fn get_router_setup(
        &self,
        tenant_id: &str,
        router: &MikrotikRouter,
        plan_allows_managed_radius: bool,
    ) -> AppResult<ManagedRadiusRouterSetup> {
        if !plan_allows_managed_radius {
            return Ok(ManagedRadiusRouterSetup {
                configured: false,
                router_id: router.id.clone(),
                plan_allows_managed_radius: false,
                plan_upgrade_required: true,
                upgrade_path: Some(MANAGED_RADIUS_UPGRADE_PATH.to_string()),
                tenant_has_active_assignment: false,
                default_server_available: false,
                can_assign_default: false,
                can_create_mapping: false,
                assignment_server_name: None,
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
        }

        let active_assignment = self
            .get_active_assignment_for_tenant_optional(tenant_id)
            .await?;
        let default_server = self.get_default_server().await?;

        let assignment_server_name = if let Some(assignment) = active_assignment.as_ref() {
            sqlx::query_scalar::<_, String>("SELECT name FROM radius_servers WHERE id = $1")
                .bind(&assignment.radius_server_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?
        } else {
            None
        };

        let config = sqlx::query_as::<_, ManagedRadiusNas>(
            r#"
            SELECT n.*
            FROM managed_radius_nas n
            INNER JOIN radius_servers s
              ON s.id = n.radius_server_id
            INNER JOIN tenant_radius_assignments a
              ON a.radius_server_id = s.id
             AND a.tenant_id = n.tenant_id
             AND a.is_active = true
            WHERE n.tenant_id = $1
              AND n.router_id = $2
              AND n.is_active = true
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
                plan_allows_managed_radius: true,
                plan_upgrade_required: false,
                upgrade_path: None,
                tenant_has_active_assignment: active_assignment.is_some(),
                default_server_available: default_server.is_some(),
                can_assign_default: active_assignment.is_none() && default_server.is_some(),
                can_create_mapping: active_assignment.is_some(),
                assignment_server_name,
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
            FROM radius_servers
            WHERE id = $1
              AND is_active = true
            LIMIT 1
            "#,
        )
        .bind(&nas.radius_server_id)
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
            plan_allows_managed_radius: true,
            plan_upgrade_required: false,
            upgrade_path: None,
            tenant_has_active_assignment: true,
            default_server_available: default_server.is_some(),
            can_assign_default: false,
            can_create_mapping: false,
            assignment_server_name: Some(server.name.clone()),
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

fn normalize_managed_radius_db_port(value: Option<i32>) -> i32 {
    value.filter(|port| *port > 0).unwrap_or(5432)
}

fn normalize_optional_secret_input(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn resolve_managed_radius_restart_command() -> Option<String> {
    std::env::var(MANAGED_RADIUS_RESTART_COMMAND_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn resolve_managed_radius_restart_workdir() -> Option<String> {
    std::env::var(MANAGED_RADIUS_RESTART_WORKDIR_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn build_default_nas_name(router: &MikrotikRouter) -> String {
    let base = if !router.identity.as_deref().unwrap_or("").trim().is_empty() {
        router.identity.as_deref().unwrap_or("router")
    } else {
        router.name.as_str()
    };

    let slug = base
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    let compact = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if compact.is_empty() {
        format!("router-{}", &router.id.chars().take(8).collect::<String>())
    } else {
        compact
    }
}

fn build_default_nas_shortname(router: &MikrotikRouter) -> Option<String> {
    let short = router
        .name
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .take(12)
        .collect::<String>();

    if short.is_empty() {
        None
    } else {
        Some(short.to_uppercase())
    }
}

fn required_trimmed<'a>(field: &str, value: &'a str) -> AppResult<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(format!("{field} is required")));
    }
    Ok(trimmed)
}

fn generate_managed_radius_shared_secret() -> String {
    Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(32)
        .collect()
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

fn mapping_change_requires_freeradius_restart(
    existing: &ManagedRadiusNas,
    next_nas_name: &str,
    next_nas_ip_or_cidr: &str,
    next_shortname: Option<&str>,
    next_shared_secret_enc: &str,
    next_is_active: bool,
) -> bool {
    existing.nas_name != next_nas_name
        || existing.nas_ip_or_cidr != next_nas_ip_or_cidr
        || existing.shortname.as_deref() != next_shortname
        || existing.shared_secret_enc != next_shared_secret_enc
        || existing.is_active != next_is_active
}

#[cfg(test)]
fn managed_radius_service_source() -> &'static str {
    include_str!("managed_radius_service.rs")
}

#[cfg(test)]
fn managed_radius_sql_template() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../deploy/freeradius/raddb/mods-available/sql.template"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PppoeAccount, PppoeAccountSource};
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

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
        let _guard = env_lock().lock().expect("env lock");
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
        let _guard = env_lock().lock().expect("env lock");
        unsafe {
            std::env::remove_var("MANAGED_RADIUS_HOST");
            std::env::remove_var("RADIUS_PUBLIC_HOST");
        }
        let (host, warning) = resolve_radius_host("radius-postgres");
        assert_eq!(host, "radius-postgres");
        assert!(warning.is_some());
    }

    #[test]
    fn db_port_defaults_to_postgres_when_missing_or_invalid() {
        assert_eq!(normalize_managed_radius_db_port(None), 5432);
        assert_eq!(normalize_managed_radius_db_port(Some(0)), 5432);
        assert_eq!(normalize_managed_radius_db_port(Some(55433)), 55433);
    }

    #[test]
    fn sql_template_scopes_auth_to_active_nas_and_disabled_flag() {
        let template = managed_radius_sql_template();
        assert!(template.contains("COALESCE(n.shortname, n.nas_name) = '%{client:shortname}'"));
        assert!(template.contains("a.disabled = false"));
        assert!(template.contains(
            "INNER JOIN managed_radius_nas n ON n.tenant_id = a.tenant_id AND n.router_id = a.router_id"
        ));
    }

    #[test]
    fn restart_command_resolution_treats_blank_values_as_missing() {
        let _guard = env_lock().lock().expect("env lock");
        unsafe {
            std::env::set_var(
                MANAGED_RADIUS_RESTART_COMMAND_ENV,
                "  docker compose restart freeradius  ",
            );
            std::env::set_var(
                MANAGED_RADIUS_RESTART_WORKDIR_ENV,
                "  /opt/isp-management  ",
            );
        }

        assert_eq!(
            resolve_managed_radius_restart_command(),
            Some("docker compose restart freeradius".into())
        );
        assert_eq!(
            resolve_managed_radius_restart_workdir(),
            Some("/opt/isp-management".into())
        );

        unsafe {
            std::env::set_var(MANAGED_RADIUS_RESTART_COMMAND_ENV, "   ");
            std::env::set_var(MANAGED_RADIUS_RESTART_WORKDIR_ENV, "   ");
        }

        assert_eq!(resolve_managed_radius_restart_command(), None);
        assert_eq!(resolve_managed_radius_restart_workdir(), None);

        unsafe {
            std::env::remove_var(MANAGED_RADIUS_RESTART_COMMAND_ENV);
            std::env::remove_var(MANAGED_RADIUS_RESTART_WORKDIR_ENV);
        }
    }

    #[test]
    fn optional_secret_input_treats_blank_values_as_none() {
        assert_eq!(normalize_optional_secret_input(None), None);
        assert_eq!(normalize_optional_secret_input(Some("   ")), None);
        assert_eq!(
            normalize_optional_secret_input(Some("  abc123  ")),
            Some("abc123".into())
        );
    }

    #[test]
    fn generated_shared_secret_is_non_empty_and_url_safe() {
        let secret = generate_managed_radius_shared_secret();
        assert_eq!(secret.len(), 32);
        assert!(secret.chars().all(|ch| ch.is_ascii_alphanumeric()));
    }

    #[test]
    fn mapping_change_detection_ignores_unchanged_values() {
        let now = Utc::now();
        let existing = ManagedRadiusNas {
            id: "mapping-1".into(),
            tenant_id: "tenant-1".into(),
            router_id: "router-1".into(),
            radius_server_id: "server-1".into(),
            nas_name: "router-pop-a".into(),
            nas_ip_or_cidr: "10.10.10.1/32".into(),
            shared_secret_enc: "enc-secret".into(),
            shortname: Some("POP-A".into()),
            is_active: true,
            created_at: now,
            updated_at: now,
        };

        assert!(!mapping_change_requires_freeradius_restart(
            &existing,
            "router-pop-a",
            "10.10.10.1/32",
            Some("POP-A"),
            "enc-secret",
            true,
        ));
    }

    #[test]
    fn mapping_change_detection_flags_runtime_client_changes() {
        let now = Utc::now();
        let existing = ManagedRadiusNas {
            id: "mapping-1".into(),
            tenant_id: "tenant-1".into(),
            router_id: "router-1".into(),
            radius_server_id: "server-1".into(),
            nas_name: "router-pop-a".into(),
            nas_ip_or_cidr: "10.10.10.1/32".into(),
            shared_secret_enc: "enc-secret".into(),
            shortname: Some("POP-A".into()),
            is_active: true,
            created_at: now,
            updated_at: now,
        };

        assert!(mapping_change_requires_freeradius_restart(
            &existing,
            "router-pop-a",
            "10.10.10.2/32",
            Some("POP-A"),
            "enc-secret",
            true,
        ));
        assert!(mapping_change_requires_freeradius_restart(
            &existing,
            "router-pop-a",
            "10.10.10.1/32",
            Some("POP-B"),
            "enc-secret",
            true,
        ));
        assert!(mapping_change_requires_freeradius_restart(
            &existing,
            "router-pop-a",
            "10.10.10.1/32",
            Some("POP-A"),
            "enc-secret-2",
            true,
        ));
        assert!(mapping_change_requires_freeradius_restart(
            &existing,
            "router-pop-a",
            "10.10.10.1/32",
            Some("POP-A"),
            "enc-secret",
            false,
        ));
    }

    #[test]
    fn mapping_mutations_sync_runtime_nas_state() {
        let source = super::managed_radius_service_source();

        assert!(
            source.contains("self.sync_runtime_nas_by_mapping_id(&mapping.id).await?;"),
            "expected create_mapping to sync runtime NAS state"
        );
        let mapping_id_sync_calls = source
            .matches("self.sync_runtime_nas_by_mapping_id(mapping_id).await?;")
            .count();
        assert!(
            mapping_id_sync_calls >= 3,
            "expected update, active-state, and secret-rotation mapping mutations to sync runtime NAS state"
        );

        assert!(
            source.contains("mapping_change_requires_freeradius_restart("),
            "expected update_mapping to evaluate whether a runtime client change needs a freeradius restart"
        );
        assert!(
            source.contains("self.restart_freeradius_after_mapping_change_if_configured()"),
            "expected update_mapping to trigger the configured freeradius restart hook after runtime client changes"
        );
    }
}
