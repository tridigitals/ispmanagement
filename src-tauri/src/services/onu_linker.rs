//! OnuLinker — auto-link discovered ONU (from OLT poll) ke network_assets & customer
//!
//! Flow:
//! 1. OLT poller detects new ONU with MAC `00:11:22:33:44:55`
//! 2. OltService::save_onu_history() calls OnuLinker::link_unlinked_onus()
//! 3. OnuLinker queries `network_assets` WHERE lower(serial_number) = mac
//!    AND asset_type IN ('ont','onu')
//! 4. If match: update asset.olt_id + asset.pon_port
//! 5. ONU history record gets customer_id propagated
//!
//! Convention: MAC address stored in `network_assets.serial_number`.
//! This is a per-tenant unique column with existing unique index.

use crate::db::DbPool;
use crate::error::AppResult;
use serde::Serialize;
use sqlx::Row;

/// A discovered ONU + its linked asset/customer (if any).
#[derive(Debug, Clone, Serialize)]
pub struct OnuLink {
    pub asset_id: String,
    pub customer_id: Option<String>,
    pub serial_number: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
}

#[derive(Clone)]
pub struct OnuLinker {
    pool: DbPool,
}

impl OnuLinker {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Normalize a MAC address to lowercase, colon-separated form.
    /// Handles "00:11:22:33:44:55", "001122334455", "00-11-22-33-44-55".
    pub fn normalize_mac(mac: &str) -> String {
        mac.chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect::<String>()
            .to_lowercase()
            .chars()
            .enumerate()
            .fold(String::new(), |mut acc, (i, c)| {
                if i > 0 && i % 2 == 0 {
                    acc.push(':');
                }
                acc.push(c);
                acc
            })
    }

    /// Look up a network_asset by its MAC address.
    /// MAC is matched against the `serial_number` column.
    /// Returns `None` if no asset exists for this MAC.
    pub async fn lookup_by_mac(
        &self,
        tenant_id: &str,
        mac: &str,
    ) -> AppResult<Option<OnuLink>> {
        let mac_normalized = Self::normalize_mac(mac);
        let row = sqlx::query(
            "SELECT id, customer_id, serial_number, vendor, model
             FROM public.network_assets
             WHERE tenant_id = $1
               AND asset_type IN ('ont', 'onu')
               AND lower(serial_number) = $2
             LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&mac_normalized)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| OnuLink {
            asset_id: r.get("id"),
            customer_id: r.get("customer_id"),
            serial_number: r.get("serial_number"),
            vendor: r.get("vendor"),
            model: r.get("model"),
        }))
    }

    /// Update a network_asset with its OLT + PON port assignment.
    /// Idempotent — safe to call multiple times.
    pub async fn link_to_olt(
        &self,
        tenant_id: &str,
        asset_id: &str,
        olt_id: &str,
        pon_port: &str,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE public.network_assets
             SET olt_id = $3, pon_port = $4, updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(asset_id)
        .bind(olt_id)
        .bind(pon_port)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Batch link a list of (mac, pon_port) pairs discovered on an OLT.
    /// For each pair, looks up the network_asset by MAC and updates
    /// olt_id + pon_port. Assets that don't exist are skipped (and
    /// logged) — auto-create of new assets is a separate flow.
    ///
    /// Returns stats: { linked, skipped_no_asset, errors }.
    pub async fn link_batch(
        &self,
        tenant_id: &str,
        olt_id: &str,
        onus: &[(String, String)], // (mac, pon_port)
    ) -> AppResult<LinkBatchResult> {
        let mut linked = 0u32;
        let mut skipped = 0u32;
        let mut errors = 0u32;

        for (mac, pon_port) in onus {
            if mac.is_empty() {
                skipped += 1;
                continue;
            }
            match self.lookup_by_mac(tenant_id, mac).await {
                Ok(Some(link)) => {
                    if let Err(e) = self
                        .link_to_olt(tenant_id, &link.asset_id, olt_id, pon_port)
                        .await
                    {
                        tracing::warn!(
                            "OnuLinker: failed to link asset {} to OLT {}: {}",
                            link.asset_id,
                            olt_id,
                            e
                        );
                        errors += 1;
                    } else {
                        linked += 1;
                        tracing::debug!(
                            "OnuLinker: linked asset {} (customer={:?}) → OLT {} PON {}",
                            link.asset_id,
                            link.customer_id,
                            olt_id,
                            pon_port
                        );
                    }
                }
                Ok(None) => {
                    skipped += 1;
                }
                Err(e) => {
                    tracing::warn!("OnuLinker: lookup_by_mac failed for {}: {}", mac, e);
                    errors += 1;
                }
            }
        }

        Ok(LinkBatchResult {
            linked,
            skipped,
            errors,
        })
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct LinkBatchResult {
    pub linked: u32,
    pub skipped: u32,
    pub errors: u32,
}
