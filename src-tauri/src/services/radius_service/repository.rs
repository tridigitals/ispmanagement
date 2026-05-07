use super::models::RadiusRequestContext;
use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{ManagedRadiusNas, PppoeAccount, RadiusAccountingSession, RadiusAuthLog};
use crate::security::secret::decrypt_secret_opt_for;

const PURPOSE_MANAGED_RADIUS_SHARED_SECRET: &str = "managed_radius_shared_secret";

const RESOLVE_ACTIVE_NAS_BY_SOURCE_IP_SQL: &str = r#"
SELECT *
FROM managed_radius_nas
WHERE is_active = true
  AND $1::inet <<= nas_ip_or_cidr::cidr
ORDER BY masklen(nas_ip_or_cidr::cidr) DESC, updated_at DESC
LIMIT 1
"#;

const LOAD_MANAGED_RADIUS_ACCOUNT_SQL: &str = r#"
SELECT *
FROM pppoe_accounts
WHERE tenant_id = $1
  AND router_id = $2
  AND username = $3
  AND account_source = 'managed_radius'
LIMIT 1
"#;

#[derive(Clone)]
pub struct RadiusRepository {
    pool: DbPool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusNasClient {
    pub id: String,
    pub tenant_id: String,
    pub router_id: String,
    pub nas_name: String,
    pub nas_ip_or_cidr: String,
    pub shortname: Option<String>,
    pub shared_secret: String,
}

#[derive(Debug, Clone)]
pub struct ManagedRadiusRuntimeAccount {
    pub account: PppoeAccount,
}

impl RadiusRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn resolve_nas_client(
        &self,
        context: &RadiusRequestContext,
    ) -> AppResult<Option<RadiusNasClient>> {
        let nas = sqlx::query_as::<_, ManagedRadiusNas>(RESOLVE_ACTIVE_NAS_BY_SOURCE_IP_SQL)
            .bind(&context.source_ip)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::Database)?;

        nas.map(Self::map_nas_client).transpose()
    }

    pub async fn load_managed_account(
        &self,
        tenant_id: &str,
        router_id: &str,
        username: &str,
    ) -> AppResult<Option<ManagedRadiusRuntimeAccount>> {
        let account = sqlx::query_as::<_, PppoeAccount>(LOAD_MANAGED_RADIUS_ACCOUNT_SQL)
            .bind(tenant_id)
            .bind(router_id)
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::Database)?;

        Ok(account.map(|account| ManagedRadiusRuntimeAccount { account }))
    }

    pub async fn insert_auth_log(&self, auth_log: &RadiusAuthLog) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO radius_auth_log (
              id, tenant_id, router_id, source_ip, username, radius_identity,
              outcome, reason, auth_type, latency_ms, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(&auth_log.id)
        .bind(&auth_log.tenant_id)
        .bind(&auth_log.router_id)
        .bind(&auth_log.source_ip)
        .bind(&auth_log.username)
        .bind(&auth_log.radius_identity)
        .bind(&auth_log.outcome)
        .bind(&auth_log.reason)
        .bind(&auth_log.auth_type)
        .bind(auth_log.latency_ms)
        .bind(auth_log.created_at)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    pub async fn upsert_accounting_session(
        &self,
        session: &RadiusAccountingSession,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO radius_accounting_sessions (
              id, tenant_id, router_id, nas_ip_address, nas_ip_or_cidr, username,
              radius_identity, acct_session_id, status_type, framed_ip_address,
              calling_station_id, session_time_seconds, input_octets, output_octets,
              terminate_cause, started_at, last_update_at, ended_at,
              raw_attributes_json, created_at, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21)
            ON CONFLICT (tenant_id, router_id, acct_session_id) DO UPDATE SET
              nas_ip_address = EXCLUDED.nas_ip_address,
              nas_ip_or_cidr = EXCLUDED.nas_ip_or_cidr,
              username = EXCLUDED.username,
              radius_identity = EXCLUDED.radius_identity,
              status_type = EXCLUDED.status_type,
              framed_ip_address = EXCLUDED.framed_ip_address,
              calling_station_id = EXCLUDED.calling_station_id,
              session_time_seconds = EXCLUDED.session_time_seconds,
              input_octets = EXCLUDED.input_octets,
              output_octets = EXCLUDED.output_octets,
              terminate_cause = EXCLUDED.terminate_cause,
              started_at = EXCLUDED.started_at,
              last_update_at = EXCLUDED.last_update_at,
              ended_at = EXCLUDED.ended_at,
              raw_attributes_json = EXCLUDED.raw_attributes_json,
              updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(&session.id)
        .bind(&session.tenant_id)
        .bind(&session.router_id)
        .bind(&session.nas_ip_address)
        .bind(&session.nas_ip_or_cidr)
        .bind(&session.username)
        .bind(&session.radius_identity)
        .bind(&session.acct_session_id)
        .bind(&session.status_type)
        .bind(&session.framed_ip_address)
        .bind(&session.calling_station_id)
        .bind(session.session_time_seconds)
        .bind(session.input_octets)
        .bind(session.output_octets)
        .bind(&session.terminate_cause)
        .bind(session.started_at)
        .bind(session.last_update_at)
        .bind(session.ended_at)
        .bind(&session.raw_attributes_json)
        .bind(session.created_at)
        .bind(session.updated_at)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    fn map_nas_client(nas: ManagedRadiusNas) -> AppResult<RadiusNasClient> {
        let shared_secret =
            decrypt_secret_opt_for(PURPOSE_MANAGED_RADIUS_SHARED_SECRET, &nas.shared_secret_enc)?
                .unwrap_or_default();

        Ok(RadiusNasClient {
            id: nas.id,
            tenant_id: nas.tenant_id,
            router_id: nas.router_id,
            nas_name: nas.nas_name,
            nas_ip_or_cidr: nas.nas_ip_or_cidr,
            shortname: nas.shortname,
            shared_secret,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{LOAD_MANAGED_RADIUS_ACCOUNT_SQL, RESOLVE_ACTIVE_NAS_BY_SOURCE_IP_SQL};

    #[test]
    fn nas_resolution_query_prefers_longest_prefix_and_active_rows() {
        assert!(RESOLVE_ACTIVE_NAS_BY_SOURCE_IP_SQL.contains("is_active = true"));
        assert!(RESOLVE_ACTIVE_NAS_BY_SOURCE_IP_SQL.contains("$1::inet <<= nas_ip_or_cidr::cidr"));
        assert!(RESOLVE_ACTIVE_NAS_BY_SOURCE_IP_SQL.contains("masklen(nas_ip_or_cidr::cidr) DESC"));
    }

    #[test]
    fn managed_account_query_scopes_to_tenant_router_and_managed_radius_source() {
        assert!(LOAD_MANAGED_RADIUS_ACCOUNT_SQL.contains("tenant_id = $1"));
        assert!(LOAD_MANAGED_RADIUS_ACCOUNT_SQL.contains("router_id = $2"));
        assert!(LOAD_MANAGED_RADIUS_ACCOUNT_SQL.contains("username = $3"));
        assert!(LOAD_MANAGED_RADIUS_ACCOUNT_SQL.contains("account_source = 'managed_radius'"));
    }
}
