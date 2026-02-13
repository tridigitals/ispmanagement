
use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    CreatePppoeAccountRequest, PaginatedResponse, PppoeAccount, PppoeAccountPublic,
    PppoeImportAction, PppoeImportCandidate, PppoeImportError, PppoeImportFromRouterRequest,
    PppoeImportResult, UpdatePppoeAccountRequest,
};
use crate::security::secret::{decrypt_secret_opt, decrypt_secret_opt_for, encrypt_secret_for};
use crate::services::{AuditService, AuthService};
use chrono::Utc;
use mikrotik_rs::{protocol::command::CommandBuilder, protocol::CommandResponse, MikrotikDevice};
use std::time::Instant;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

const PURPOSE_PPPOE: &str = "pppoe_secrets";
const IMPORT_PLACEHOLDER_CUSTOMER_NAME: &str = "Imported (Unassigned)";
const IMPORT_PLACEHOLDER_LOCATION_LABEL: &str = "Unassigned";

#[derive(Debug, Clone)]
struct RouterSecretRow {
    username: String,
    router_secret_id: Option<String>,
    password: Option<String>,
    password_available: bool,
    profile_name: Option<String>,
    remote_address: Option<String>,
    disabled: bool,
    comment: Option<String>,
}

#[derive(Clone)]
pub struct PppoeService {
    pool: DbPool,
    auth_service: AuthService,
    audit_service: AuditService,
}

impl PppoeService {
    pub fn new(pool: DbPool, auth_service: AuthService, audit_service: AuditService) -> Self {
        Self {
            pool,
            auth_service,
            audit_service,
        }
    }

    async fn ensure_router_access(&self, tenant_id: &str, router_id: &str) -> AppResult<()> {
        let exists: Option<String> = sqlx::query_scalar(
            "SELECT id FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2",
        )
        .bind(router_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if exists.is_none() {
            return Err(AppError::Forbidden("No access to router".into()));
        }
        Ok(())
    }

    async fn ensure_location_access(
        &self,
        tenant_id: &str,
        customer_id: &str,
        location_id: &str,
    ) -> AppResult<()> {
        let exists: Option<String> = sqlx::query_scalar(
            r#"
            SELECT id FROM customer_locations
            WHERE tenant_id = $1 AND customer_id = $2 AND id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(customer_id)
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if exists.is_none() {
            return Err(AppError::Forbidden("No access to location".into()));
        }
        Ok(())
    }

    async fn connect_router(&self, tenant_id: &str, router_id: &str) -> AppResult<MikrotikDevice> {
        let row = sqlx::query_as::<_, crate::models::MikrotikRouter>(
            "SELECT * FROM mikrotik_routers WHERE id = $1 AND tenant_id = $2",
        )
        .bind(router_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Router not found".to_string()))?;

        let password = decrypt_secret_opt(row.password.as_str())?;
        let addr = format!("{}:{}", row.host, row.port);

        let dev = timeout(
            Duration::from_secs(5),
            MikrotikDevice::connect(addr, row.username.as_str(), password.as_deref()),
        )
        .await
        .map_err(|_| AppError::Internal("Connection timed out".into()))?
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(dev)
    }

    async fn router_find_secret_id_by_name(
        &self,
        dev: &MikrotikDevice,
        username: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let cmd = CommandBuilder::new().command("/ppp/secret/print").build();
        let mut rx = dev.send_command(cmd).await?;
        while let Some(res) = rx.recv().await {
            let r = res?;
            if let CommandResponse::Reply(reply) = r {
                let name = reply.attributes.get("name").and_then(|v| v.clone());
                if name.as_deref() == Some(username) {
                    let id = reply.attributes.get(".id").and_then(|v| v.clone());
                    return Ok(id);
                }
            }
        }
        Ok(None)
    }

    fn bool_from_yes_no(v: Option<&str>) -> bool {
        matches!(
            v.unwrap_or_default().trim().to_ascii_lowercase().as_str(),
            "yes" | "true" | "1" | "on"
        )
    }

    fn password_available(pw: Option<&str>) -> bool {
        let p = pw.unwrap_or_default().trim();
        if p.is_empty() {
            return false;
        }
        // Some RouterOS versions return masked passwords.
        if p.chars().all(|c| c == '*') {
            return false;
        }
        true
    }

    async fn router_list_pppoe_secrets(
        &self,
        dev: &MikrotikDevice,
        include_password: bool,
        include_disabled: bool,
    ) -> Result<Vec<RouterSecretRow>, anyhow::Error> {
        let cmd = CommandBuilder::new().command("/ppp/secret/print").build();
        let mut rx = dev.send_command(cmd).await?;

        let mut out: Vec<RouterSecretRow> = Vec::new();
        while let Some(res) = rx.recv().await {
            let r = res?;
            if let CommandResponse::Reply(reply) = r {
                let username = reply
                    .attributes
                    .get("name")
                    .and_then(|v| v.clone())
                    .unwrap_or_default();
                if username.trim().is_empty() {
                    continue;
                }

                let service = reply.attributes.get("service").and_then(|v| v.clone());
                if let Some(s) = service.as_deref() {
                    // Winbox often uses `service=any` for PPPoE secrets. Treat both `pppoe` and `any` as PPPoE-capable.
                    let sv = s.trim();
                    if !sv.is_empty() && sv != "pppoe" && sv != "any" {
                        continue;
                    }
                }

                let disabled = Self::bool_from_yes_no(
                    reply.attributes.get("disabled").and_then(|v| v.as_deref()),
                );
                if disabled && !include_disabled {
                    continue;
                }

                let pw = if include_password {
                    reply.attributes.get("password").and_then(|v| v.clone())
                } else {
                    None
                };
                let pw_avail = Self::password_available(
                    reply
                        .attributes
                        .get("password")
                        .and_then(|v| v.as_deref()),
                );

                out.push(RouterSecretRow {
                    username,
                    router_secret_id: reply.attributes.get(".id").and_then(|v| v.clone()),
                    password: pw,
                    password_available: pw_avail,
                    profile_name: reply.attributes.get("profile").and_then(|v| v.clone()),
                    remote_address: reply.attributes.get("remote-address").and_then(|v| v.clone()),
                    disabled,
                    comment: reply.attributes.get("comment").and_then(|v| v.clone()),
                });
            }
        }

        Ok(out)
    }

    async fn ensure_import_placeholder(
        &self,
        tenant_id: &str,
    ) -> AppResult<(String, String)> {
        let now = Utc::now();

        let existing_customer: Option<String> = sqlx::query_scalar(
            "SELECT id FROM customers WHERE tenant_id = $1 AND name = $2",
        )
        .bind(tenant_id)
        .bind(IMPORT_PLACEHOLDER_CUSTOMER_NAME)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let customer_id = if let Some(id) = existing_customer {
            id
        } else {
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"
                INSERT INTO customers (id, tenant_id, name, email, phone, notes, is_active, created_at, updated_at)
                VALUES ($1, $2, $3, NULL, NULL, $4, true, $5, $6)
                "#,
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(IMPORT_PLACEHOLDER_CUSTOMER_NAME)
            .bind("System placeholder for imported PPPoE accounts that are not mapped to a customer yet.")
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
            id
        };

        let existing_location: Option<String> = sqlx::query_scalar(
            r#"
            SELECT id FROM customer_locations
            WHERE tenant_id = $1 AND customer_id = $2 AND label = $3
            "#,
        )
        .bind(tenant_id)
        .bind(&customer_id)
        .bind(IMPORT_PLACEHOLDER_LOCATION_LABEL)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let location_id = if let Some(id) = existing_location {
            id
        } else {
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"
                INSERT INTO customer_locations (
                  id, tenant_id, customer_id, label,
                  address_line1, address_line2, city, state, postal_code, country,
                  latitude, longitude, notes,
                  created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, $5, $6, $7)
                "#,
            )
            .bind(&id)
            .bind(tenant_id)
            .bind(&customer_id)
            .bind(IMPORT_PLACEHOLDER_LOCATION_LABEL)
            .bind("System placeholder for imported PPPoE accounts that are not mapped to a location yet.")
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
            id
        };

        Ok((customer_id, location_id))
    }

    async fn router_add_or_set_secret(
        &self,
        dev: &MikrotikDevice,
        username: &str,
        password: &str,
        profile_name: Option<&str>,
        remote_address: Option<&str>,
        address_pool: Option<&str>,
        disabled: bool,
        comment: Option<&str>,
    ) -> Result<String, anyhow::Error> {
        let existing = self.router_find_secret_id_by_name(dev, username).await?;

        let mut args: Vec<(String, String)> = vec![
            ("name".into(), username.to_string()),
            ("password".into(), password.to_string()),
            ("service".into(), "pppoe".to_string()),
            ("disabled".into(), if disabled { "yes" } else { "no" }.to_string()),
        ];
        if let Some(p) = profile_name.filter(|s| !s.trim().is_empty()) {
            args.push(("profile".into(), p.to_string()));
        }

        // RouterOS secret supports overriding the "remote-address". In practice this can be
        // either a static IP (remote_address) or a pool name (address_pool) depending on your setup.
        let remote_addr_or_pool = remote_address
            .filter(|s| !s.trim().is_empty())
            .or_else(|| address_pool.filter(|s| !s.trim().is_empty()));
        if let Some(v) = remote_addr_or_pool {
            args.push(("remote-address".into(), v.to_string()));
        }
        if let Some(c) = comment.filter(|s| !s.trim().is_empty()) {
            args.push(("comment".into(), c.to_string()));
        }

        if let Some(id) = existing {
            // /ppp/secret/set numbers=<id> ... (omit name when editing? safe to keep)
            let mut b = CommandBuilder::new().command("/ppp/secret/set");
            b = b.attribute("numbers", Some(id.as_str()));
            for (k, v) in args.iter() {
                if k == "service" {
                    // service is not settable for existing on some versions; ignore
                    continue;
                }
                b = b.attribute(k.as_str(), Some(v.as_str()));
            }
            let cmd = b.build();
            let mut rx = dev.send_command(cmd).await?;
            while let Some(res) = rx.recv().await {
                let r = res?;
                if let CommandResponse::Done(_) = r {
                    break;
                }
            }
            Ok(id)
        } else {
            // add and then resolve id by name
            let mut b = CommandBuilder::new().command("/ppp/secret/add");
            for (k, v) in args.iter() {
                b = b.attribute(k.as_str(), Some(v.as_str()));
            }
            let cmd = b.build();
            let mut rx = dev.send_command(cmd).await?;
            while let Some(res) = rx.recv().await {
                let r = res?;
                if let CommandResponse::Done(_) = r {
                    break;
                }
            }
            Ok(self
                .router_find_secret_id_by_name(dev, username)
                .await?
                .unwrap_or_default())
        }
    }

    // ========================
    // Public API
    // ========================

    pub async fn preview_import_from_router(
        &self,
        actor_id: &str,
        tenant_id: &str,
        router_id: &str,
        include_disabled: bool,
    ) -> AppResult<Vec<PppoeImportCandidate>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "pppoe", "read")
            .await?;

        self.ensure_router_access(tenant_id, router_id).await?;

        let dev = self.connect_router(tenant_id, router_id).await?;
        let secrets = self
            .router_list_pppoe_secrets(&dev, false, include_disabled)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        #[derive(sqlx::FromRow)]
        struct ExistingRow {
            id: String,
            username: String,
            router_profile_name: Option<String>,
            remote_address: Option<String>,
            address_pool: Option<String>,
            disabled: bool,
            comment: Option<String>,
        }

        let existing: Vec<ExistingRow> = sqlx::query_as(
            r#"
            SELECT id, username, router_profile_name, remote_address, address_pool, disabled, comment
            FROM pppoe_accounts
            WHERE tenant_id = $1 AND router_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut map = std::collections::HashMap::<String, ExistingRow>::new();
        for r in existing {
            map.insert(r.username.clone(), r);
        }

        let norm = |s: Option<String>| s.unwrap_or_default().trim().to_string();

        let mut out: Vec<PppoeImportCandidate> = Vec::new();
        for s in secrets {
            let secret_remote = norm(s.remote_address.clone());
            let secret_profile = norm(s.profile_name.clone());
            let secret_comment = norm(s.comment.clone());

            if let Some(ex) = map.get(&s.username) {
                let db_remote = {
                    let a = norm(ex.remote_address.clone());
                    if !a.is_empty() {
                        a
                    } else {
                        norm(ex.address_pool.clone())
                    }
                };
                let db_profile = norm(ex.router_profile_name.clone());
                let db_comment = norm(ex.comment.clone());

                let same = secret_remote == db_remote
                    && secret_profile == db_profile
                    && s.disabled == ex.disabled
                    && secret_comment == db_comment;

                out.push(PppoeImportCandidate {
                    username: s.username,
                    router_secret_id: s.router_secret_id,
                    profile_name: s.profile_name,
                    remote_address: s.remote_address,
                    disabled: s.disabled,
                    comment: s.comment,
                    password_available: s.password_available,
                    action: if same { PppoeImportAction::Same } else { PppoeImportAction::Update },
                    existing_account_id: Some(ex.id.clone()),
                });
            } else {
                out.push(PppoeImportCandidate {
                    username: s.username,
                    router_secret_id: s.router_secret_id,
                    profile_name: s.profile_name,
                    remote_address: s.remote_address,
                    disabled: s.disabled,
                    comment: s.comment,
                    password_available: s.password_available,
                    action: PppoeImportAction::New,
                    existing_account_id: None,
                });
            }
        }

        // stable sort: New first, then Update, then Same.
        fn rank(a: &PppoeImportAction) -> i32 {
            match a {
                PppoeImportAction::New => 0,
                PppoeImportAction::Update => 1,
                PppoeImportAction::Same => 2,
            }
        }
        out.sort_by(|a, b| rank(&a.action).cmp(&rank(&b.action)).then(a.username.cmp(&b.username)));

        Ok(out)
    }

    pub async fn import_from_router(
        &self,
        actor_id: &str,
        tenant_id: &str,
        router_id: &str,
        req: PppoeImportFromRouterRequest,
        ip_address: Option<&str>,
    ) -> AppResult<PppoeImportResult> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "pppoe", "manage")
            .await?;

        self.ensure_router_access(tenant_id, router_id).await?;

        // Require both or none (otherwise we can't verify location ownership properly).
        if req.customer_id.is_some() ^ req.location_id.is_some() {
            return Err(AppError::Validation(
                "Provide both customer_id and location_id, or leave both empty".into(),
            ));
        }

        let (customer_id, location_id) = if let (Some(cid), Some(lid)) =
            (req.customer_id.clone(), req.location_id.clone())
        {
            self.ensure_location_access(tenant_id, &cid, &lid).await?;
            (cid, lid)
        } else {
            self.ensure_import_placeholder(tenant_id).await?
        };

        if req.usernames.is_empty() {
            return Ok(PppoeImportResult {
                created: 0,
                updated: 0,
                skipped: 0,
                missing_password: 0,
                errors: vec![],
                used_customer_id: customer_id,
                used_location_id: location_id,
            });
        }

        let want: std::collections::HashSet<String> =
            req.usernames.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

        let dev = self.connect_router(tenant_id, router_id).await?;
        let secrets = self
            .router_list_pppoe_secrets(&dev, true, true)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let mut secrets_by_name = std::collections::HashMap::<String, RouterSecretRow>::new();
        for s in secrets {
            secrets_by_name.insert(s.username.clone(), s);
        }

        // Existing accounts for quick upsert decisions.
        #[derive(sqlx::FromRow)]
        struct ExistingRow {
            id: String,
            username: String,
            password_enc: String,
        }
        let existing: Vec<ExistingRow> = sqlx::query_as(
            r#"
            SELECT id, username, password_enc
            FROM pppoe_accounts
            WHERE tenant_id = $1 AND router_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut existing_map = std::collections::HashMap::<String, ExistingRow>::new();
        for r in existing {
            existing_map.insert(r.username.clone(), r);
        }

        let now = Utc::now();
        let mut created = 0u32;
        let mut updated = 0u32;
        let mut skipped = 0u32;
        let mut missing_password = 0u32;
        let mut errors: Vec<PppoeImportError> = Vec::new();

        for username in want.iter() {
            let s = match secrets_by_name.get(username) {
                Some(v) => v.clone(),
                None => {
                    skipped += 1;
                    errors.push(PppoeImportError {
                        username: username.clone(),
                        message: "Not found on router".into(),
                    });
                    continue;
                }
            };

            let password_plain = s.password.clone().unwrap_or_default();
            let password_ok = Self::password_available(Some(password_plain.as_str()));

            let password_enc_new = if password_ok {
                encrypt_secret_for(PURPOSE_PPPOE, password_plain.as_str())?
            } else {
                String::new()
            };

            let router_profile_name = s.profile_name.clone().filter(|v| !v.trim().is_empty());
            let remote_address = s.remote_address.clone().filter(|v| !v.trim().is_empty());
            let comment = s.comment.clone().filter(|v| !v.trim().is_empty());
            let router_secret_id = s.router_secret_id.clone().filter(|v| !v.trim().is_empty());

            if let Some(ex) = existing_map.get(username) {
                // Update fields; keep password_enc unless we got a valid password from router.
                let password_enc = if password_ok {
                    password_enc_new.clone()
                } else {
                    ex.password_enc.clone()
                };

                sqlx::query(
                    r#"
                    UPDATE pppoe_accounts SET
                      customer_id = $1,
                      location_id = $2,
                      password_enc = $3,
                      router_profile_name = $4,
                      remote_address = $5,
                      address_pool = NULL,
                      disabled = $6,
                      comment = $7,
                      router_present = true,
                      router_secret_id = $8,
                      last_sync_at = $9,
                      last_error = $10,
                      updated_at = $11
                    WHERE tenant_id = $12 AND id = $13
                    "#,
                )
                .bind(&customer_id)
                .bind(&location_id)
                .bind(&password_enc)
                .bind(&router_profile_name)
                .bind(&remote_address)
                .bind(s.disabled)
                .bind(&comment)
                .bind(&router_secret_id)
                .bind(now)
                .bind(if password_ok { None::<String> } else { Some("Password not available from router; please set manually.".to_string()) })
                .bind(now)
                .bind(tenant_id)
                .bind(&ex.id)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;

                if !password_ok {
                    missing_password += 1;
                }
                updated += 1;
            } else {
                let id = Uuid::new_v4().to_string();
                let last_error = if password_ok {
                    None::<String>
                } else {
                    Some("Password not available from router; please set manually.".to_string())
                };

                sqlx::query(
                    r#"
                    INSERT INTO pppoe_accounts (
                      id, tenant_id, router_id, customer_id, location_id,
                      username, password_enc,
                      profile_id, router_profile_name, remote_address, address_pool,
                      disabled, comment,
                      router_present, router_secret_id, last_sync_at, last_error,
                      created_at, updated_at
                    ) VALUES (
                      $1, $2, $3, $4, $5,
                      $6, $7,
                      NULL, $8, $9, NULL,
                      $10, $11,
                      true, $12, $13, $14,
                      $15, $16
                    )
                    "#,
                )
                .bind(&id)
                .bind(tenant_id)
                .bind(router_id)
                .bind(&customer_id)
                .bind(&location_id)
                .bind(username)
                .bind(if password_ok { &password_enc_new } else { "" })
                .bind(&router_profile_name)
                .bind(&remote_address)
                .bind(s.disabled)
                .bind(&comment)
                .bind(&router_secret_id)
                .bind(now)
                .bind(&last_error)
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(AppError::Database)?;

                if !password_ok {
                    missing_password += 1;
                }
                created += 1;
            }
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "PPPOE_IMPORT_FROM_ROUTER",
                "pppoe",
                Some(router_id),
                Some("Imported PPPoE secrets from router"),
                ip_address,
            )
            .await;

        Ok(PppoeImportResult {
            created,
            updated,
            skipped,
            missing_password,
            errors,
            used_customer_id: customer_id,
            used_location_id: location_id,
        })
    }

    pub async fn list_accounts(
        &self,
        actor_id: &str,
        tenant_id: &str,
        customer_id: Option<String>,
        location_id: Option<String>,
        router_id: Option<String>,
        q: Option<String>,
        page: u32,
        per_page: u32,
    ) -> AppResult<PaginatedResponse<PppoeAccountPublic>> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "pppoe", "read")
            .await?;

        let q = q.unwrap_or_default().trim().to_string();
        let offset = (page.saturating_sub(1)) * per_page;

        // Keep query simple: optional filters via OR params.
        #[cfg(feature = "postgres")]
        let sql = r#"
          SELECT a.*, COUNT(*) OVER() AS total_count
          FROM pppoe_accounts a
          WHERE a.tenant_id = $1
            AND ($2::text IS NULL OR a.customer_id = $2)
            AND ($3::text IS NULL OR a.location_id = $3)
            AND ($4::text IS NULL OR a.router_id = $4)
            AND ($5 = '' OR a.username ILIKE '%' || $5 || '%')
          ORDER BY a.updated_at DESC
          LIMIT $6 OFFSET $7
        "#;

        #[derive(sqlx::FromRow)]
        struct Row {
            #[sqlx(flatten)]
            account: PppoeAccount,
            total_count: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(sql)
            .bind(tenant_id)
            .bind(customer_id)
            .bind(location_id)
            .bind(router_id)
            .bind(&q)
            .bind(per_page as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?;

        let total = rows.first().map(|r| r.total_count).unwrap_or(0);

        Ok(PaginatedResponse {
            data: rows.into_iter().map(|r| r.account.into()).collect(),
            total,
            page,
            per_page,
        })
    }

    pub async fn get_account(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<PppoeAccountPublic> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "pppoe", "read")
            .await?;

        let account: PppoeAccount = sqlx::query_as(
            "SELECT * FROM pppoe_accounts WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("PPPoE account not found".into()))?;

        Ok(account.into())
    }

    pub async fn create_account(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: CreatePppoeAccountRequest,
        ip_address: Option<&str>,
    ) -> AppResult<PppoeAccountPublic> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "pppoe", "manage")
            .await?;

        self.ensure_router_access(tenant_id, dto.router_id.as_str())
            .await?;
        self.ensure_location_access(
            tenant_id,
            dto.customer_id.as_str(),
            dto.location_id.as_str(),
        )
        .await?;

        if dto.username.trim().is_empty() {
            return Err(AppError::Validation("username is required".into()));
        }
        if dto.password.trim().is_empty() {
            return Err(AppError::Validation("password is required".into()));
        }

        let password_enc = encrypt_secret_for(PURPOSE_PPPOE, dto.password.as_str())?;

        let account = PppoeAccount::new(
            tenant_id.to_string(),
            dto.router_id,
            dto.customer_id,
            dto.location_id,
            dto.username.trim().to_string(),
            password_enc,
            dto.profile_id,
            dto.router_profile_name,
            dto.remote_address,
            dto.address_pool,
            dto.disabled,
            dto.comment,
        );

        #[cfg(feature = "postgres")]
        sqlx::query(
            r#"
            INSERT INTO pppoe_accounts
              (id, tenant_id, router_id, customer_id, location_id, username, password_enc, profile_id, router_profile_name,
               remote_address, address_pool, disabled, comment, router_present, router_secret_id, last_sync_at, last_error, created_at, updated_at)
            VALUES
              ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)
            "#,
        )
        .bind(&account.id)
        .bind(&account.tenant_id)
        .bind(&account.router_id)
        .bind(&account.customer_id)
        .bind(&account.location_id)
        .bind(&account.username)
        .bind(&account.password_enc)
        .bind(&account.profile_id)
        .bind(&account.router_profile_name)
        .bind(&account.remote_address)
        .bind(&account.address_pool)
        .bind(account.disabled)
        .bind(&account.comment)
        .bind(account.router_present)
        .bind(&account.router_secret_id)
        .bind(account.last_sync_at)
        .bind(&account.last_error)
        .bind(account.created_at)
        .bind(account.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if e.as_database_error()
                .and_then(|d| d.code().map(|c| c == "23505"))
                .unwrap_or(false)
            {
                AppError::Validation("PPPoE username already exists on this router".into())
            } else {
                AppError::Database(e)
            }
        })?;

        // Apply to router (best-effort; if it fails we keep record with error)
        let _ = self.apply_account_internal(tenant_id, account.id.as_str()).await;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "PPPOE_ACCOUNT_CREATE",
                "pppoe",
                Some(&account.id),
                Some(&format!("Created PPPoE account {}", account.username)),
                ip_address,
            )
            .await;

        // Reload updated row to return public view
        let updated: PppoeAccount = sqlx::query_as(
            "SELECT * FROM pppoe_accounts WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(&account.id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(updated.into())
    }

    pub async fn update_account(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        dto: UpdatePppoeAccountRequest,
        ip_address: Option<&str>,
    ) -> AppResult<PppoeAccountPublic> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "pppoe", "manage")
            .await?;

        let mut account: PppoeAccount = sqlx::query_as(
            "SELECT * FROM pppoe_accounts WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("PPPoE account not found".into()))?;

        if let Some(u) = dto.username {
            let v = u.trim().to_string();
            if !v.is_empty() {
                account.username = v;
            }
        }
        if let Some(p) = dto.password {
            if !p.trim().is_empty() {
                account.password_enc = encrypt_secret_for(PURPOSE_PPPOE, p.as_str())?;
            }
        }
        if let Some(v) = dto.profile_id {
            account.profile_id = Some(v);
        }
        if let Some(v) = dto.router_profile_name {
            let vv = v.trim().to_string();
            account.router_profile_name = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.remote_address {
            let vv = v.trim().to_string();
            account.remote_address = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.address_pool {
            let vv = v.trim().to_string();
            account.address_pool = if vv.is_empty() { None } else { Some(vv) };
        }
        if let Some(v) = dto.disabled {
            account.disabled = v;
        }
        if let Some(v) = dto.comment {
            let vv = v.trim().to_string();
            account.comment = if vv.is_empty() { None } else { Some(vv) };
        }

        account.updated_at = Utc::now();
        account.last_error = None;

        sqlx::query(
            r#"
            UPDATE pppoe_accounts SET
              username = $1,
              password_enc = $2,
              profile_id = $3,
              router_profile_name = $4,
              remote_address = $5,
              address_pool = $6,
              disabled = $7,
              comment = $8,
              updated_at = $9,
              last_error = NULL
            WHERE tenant_id = $10 AND id = $11
            "#,
        )
        .bind(&account.username)
        .bind(&account.password_enc)
        .bind(&account.profile_id)
        .bind(&account.router_profile_name)
        .bind(&account.remote_address)
        .bind(&account.address_pool)
        .bind(account.disabled)
        .bind(&account.comment)
        .bind(account.updated_at)
        .bind(tenant_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let _ = self.apply_account_internal(tenant_id, id).await;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "PPPOE_ACCOUNT_UPDATE",
                "pppoe",
                Some(id),
                Some("Updated PPPoE account"),
                ip_address,
            )
            .await;

        let updated: PppoeAccount = sqlx::query_as(
            "SELECT * FROM pppoe_accounts WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(updated.into())
    }

    pub async fn delete_account(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<()> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "pppoe", "manage")
            .await?;

        // Load row before delete for router cleanup
        let account: PppoeAccount = sqlx::query_as(
            "SELECT * FROM pppoe_accounts WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("PPPoE account not found".into()))?;

        // Best-effort remove from router
        if let Ok(dev) = self.connect_router(tenant_id, account.router_id.as_str()).await {
            if let Ok(Some(rid)) = self
                .router_find_secret_id_by_name(&dev, account.username.as_str())
                .await
            {
                let cmd = CommandBuilder::new()
                    .command("/ppp/secret/remove")
                    .attribute("numbers", Some(rid.as_str()))
                    .build();
                let _ = dev.send_command(cmd).await;
            }
        }

        sqlx::query("DELETE FROM pppoe_accounts WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "PPPOE_ACCOUNT_DELETE",
                "pppoe",
                Some(id),
                Some("Deleted PPPoE account"),
                ip_address,
            )
            .await;

        Ok(())
    }

    pub async fn apply_account(
        &self,
        actor_id: &str,
        tenant_id: &str,
        id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<PppoeAccountPublic> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "pppoe", "manage")
            .await?;

        let updated = self.apply_account_internal(tenant_id, id).await?;

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "PPPOE_ACCOUNT_APPLY",
                "pppoe",
                Some(id),
                Some("Applied PPPoE account to router"),
                ip_address,
            )
            .await;

        Ok(updated)
    }

    async fn apply_account_internal(
        &self,
        tenant_id: &str,
        id: &str,
    ) -> AppResult<PppoeAccountPublic> {
        let mut account: PppoeAccount = sqlx::query_as(
            "SELECT * FROM pppoe_accounts WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("PPPoE account not found".into()))?;

        let started = Instant::now();

        let dev = self.connect_router(tenant_id, account.router_id.as_str()).await?;

        let password = decrypt_secret_opt_for(PURPOSE_PPPOE, account.password_enc.as_str())?
            .ok_or_else(|| AppError::Internal("Missing PPPoE password".into()))?;

        // Resolve profile name (owned), then pass as &str.
        let profile_name: Option<String> = if let Some(ref override_name) =
            account.router_profile_name
        {
            Some(override_name.clone())
        } else if let Some(ref pid) = account.profile_id {
            sqlx::query_scalar("SELECT name FROM pppoe_profiles WHERE tenant_id = $1 AND id = $2")
                .bind(tenant_id)
                .bind(pid)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?
        } else {
            None
        };

        let res = self
            .router_add_or_set_secret(
                &dev,
                account.username.as_str(),
                password.as_str(),
                profile_name.as_deref(),
                account.remote_address.as_deref(),
                account.address_pool.as_deref(),
                account.disabled,
                account.comment.as_deref(),
            )
            .await;

        let now = Utc::now();
        match res {
            Ok(router_secret_id) => {
                account.router_present = true;
                account.router_secret_id = if router_secret_id.trim().is_empty() {
                    None
                } else {
                    Some(router_secret_id)
                };
                account.last_sync_at = Some(now);
                account.last_error = None;

                let _ = sqlx::query(
                    r#"
                    UPDATE pppoe_accounts SET
                      router_present = true,
                      router_secret_id = $1,
                      last_sync_at = $2,
                      last_error = NULL,
                      updated_at = $3
                    WHERE tenant_id = $4 AND id = $5
                    "#,
                )
                .bind(&account.router_secret_id)
                .bind(account.last_sync_at)
                .bind(now)
                .bind(tenant_id)
                .bind(id)
                .execute(&self.pool)
                .await;
            }
            Err(e) => {
                let msg = format!("apply failed: {}", e);
                account.last_error = Some(msg.clone());
                account.router_present = false;
                account.last_sync_at = Some(now);
                let _ = sqlx::query(
                    r#"
                    UPDATE pppoe_accounts SET
                      router_present = false,
                      last_sync_at = $1,
                      last_error = $2,
                      updated_at = $3
                    WHERE tenant_id = $4 AND id = $5
                    "#,
                )
                .bind(account.last_sync_at)
                .bind(&msg)
                .bind(now)
                .bind(tenant_id)
                .bind(id)
                .execute(&self.pool)
                .await;
            }
        }

        // Small perf log (debug) without spamming by default
        let _elapsed_ms = started.elapsed().as_millis();

        let updated: PppoeAccount = sqlx::query_as(
            "SELECT * FROM pppoe_accounts WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(updated.into())
    }

    /// Reconcile a router: mark which DB accounts exist on the router (by username).
    pub async fn reconcile_router(
        &self,
        actor_id: &str,
        tenant_id: &str,
        router_id: &str,
        ip_address: Option<&str>,
    ) -> AppResult<serde_json::Value> {
        self.auth_service
            .check_permission(actor_id, tenant_id, "pppoe", "manage")
            .await?;

        self.ensure_router_access(tenant_id, router_id).await?;

        let dev = self.connect_router(tenant_id, router_id).await?;

        let cmd = CommandBuilder::new().command("/ppp/secret/print").build();
        let mut rx = dev
            .send_command(cmd)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let mut router_usernames: std::collections::HashSet<String> = Default::default();
        while let Some(res) = rx.recv().await {
            let r = res.map_err(|e| AppError::Internal(e.to_string()))?;
            if let CommandResponse::Reply(reply) = r {
                if let Some(name) = reply.attributes.get("name").and_then(|v| v.clone()) {
                    router_usernames.insert(name);
                }
            }
        }

        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT id, username FROM pppoe_accounts WHERE tenant_id = $1 AND router_id = $2",
        )
        .bind(tenant_id)
        .bind(router_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut present = 0i64;
        let mut missing = 0i64;
        let now = Utc::now();
        for (id, username) in rows {
            let is_present = router_usernames.contains(username.as_str());
            if is_present {
                present += 1;
            } else {
                missing += 1;
            }
            let _ = sqlx::query(
                r#"
                UPDATE pppoe_accounts SET
                  router_present = $1,
                  last_sync_at = $2,
                  updated_at = $3
                WHERE tenant_id = $4 AND id = $5
                "#,
            )
            .bind(is_present)
            .bind(now)
            .bind(now)
            .bind(tenant_id)
            .bind(&id)
            .execute(&self.pool)
            .await;
        }

        self.audit_service
            .log(
                Some(actor_id),
                Some(tenant_id),
                "PPPOE_RECONCILE_ROUTER",
                "pppoe",
                Some(router_id),
                Some(&format!("Reconciled router PPPoE secrets: present={}, missing={}", present, missing)),
                ip_address,
            )
            .await;

        Ok(serde_json::json!({
            "router_id": router_id,
            "present": present,
            "missing": missing,
            "router_total": router_usernames.len() as i64
        }))
    }
}
