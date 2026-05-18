use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

const TERMINAL_TYPES: &[&str] = &["ont", "onu"];
const DIRECT_ATTACHMENT_TYPES: &[&str] = &["media_converter", "ont", "onu"];
const EXCLUDED_STATUSES: &[&str] = &["faulty", "retired"];

const CACHE_TOTAL_KEY: &str = "port_usage_total";
const CACHE_USED_KEY: &str = "port_usage_used";
const CACHE_AVAILABLE_KEY: &str = "port_usage_available";
const CACHE_STATE_KEY: &str = "port_usage_state";
const CACHE_UPDATED_AT_KEY: &str = "port_usage_updated_at";

const ASSET_PORT_RANK: &[(&str, i32)] = &[
    ("olt", 0),
    ("odf", 1),
    ("switch", 2),
    ("odc", 3),
    ("splitter", 4),
    ("fat", 5),
    ("nap", 6),
    ("odp", 7),
    ("ont", 8),
    ("onu", 8),
    ("media_converter", 8),
];

#[derive(Clone, sqlx::FromRow)]
struct AssetRow {
    id: String,
    asset_type: String,
    status: String,
    parent_asset_id: Option<String>,
    customer_id: Option<String>,
    location_id: Option<String>,
    metadata: Value,
}

#[derive(Clone, sqlx::FromRow)]
struct NodeRow {
    id: String,
    node_type: String,
    metadata: Value,
}

#[derive(Clone, sqlx::FromRow)]
struct LinkRow {
    from_node_id: String,
    to_node_id: String,
}

pub async fn refresh_port_usage_cache_for_tenant(pool: &DbPool, tenant_id: &str) -> AppResult<()> {
    let assets: Vec<AssetRow> = sqlx::query_as(
        r#"
        SELECT id, asset_type, status, parent_asset_id, customer_id, location_id, metadata
        FROM network_assets
        WHERE tenant_id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let node_rows: Vec<NodeRow> = sqlx::query_as(
        r#"
        SELECT id::text AS id, node_type, metadata
        FROM network_nodes
        WHERE tenant_id = $1::uuid
        "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let link_rows: Vec<LinkRow> = sqlx::query_as(
        r#"
        SELECT from_node_id::text AS from_node_id, to_node_id::text AS to_node_id
        FROM network_links
        WHERE tenant_id = $1::uuid
        "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let node_by_id: HashMap<String, NodeRow> =
        node_rows.iter().cloned().map(|node| (node.id.clone(), node)).collect();
    let asset_node_id_by_asset_id: HashMap<String, String> = node_rows
        .iter()
        .filter_map(|node| {
            let source = metadata_text(&node.metadata, "asset_source");
            let asset_id = metadata_text(&node.metadata, "asset_id");
            if source == "network_asset" && !asset_id.is_empty() {
                Some((asset_id, node.id.clone()))
            } else {
                None
            }
        })
        .collect();

    for asset in assets.iter().filter(|asset| asset.asset_type == "odp") {
        let Some(total) = parse_positive_integer(asset.metadata.get("total_port_capacity")) else {
            continue;
        };

        let mut endpoint_keys = HashSet::<String>::new();

        for child in assets.iter().filter(|candidate| candidate.parent_asset_id.as_deref() == Some(asset.id.as_str())) {
            if EXCLUDED_STATUSES.contains(&child.status.as_str()) {
                continue;
            }
            if !TERMINAL_TYPES.contains(&child.asset_type.as_str())
                && !DIRECT_ATTACHMENT_TYPES.contains(&child.asset_type.as_str())
            {
                continue;
            }
            endpoint_keys.insert(asset_endpoint_key(
                child.location_id.as_deref(),
                child.customer_id.as_deref(),
                &child.id,
            ));
        }

        if let Some(source_node_id) = asset_node_id_by_asset_id.get(&asset.id) {
            for link in link_rows.iter() {
                let other_node_id = if link.from_node_id == *source_node_id {
                    Some(link.to_node_id.as_str())
                } else if link.to_node_id == *source_node_id {
                    Some(link.from_node_id.as_str())
                } else {
                    None
                };
                let Some(other_node_id) = other_node_id else {
                    continue;
                };
                let Some(endpoint_key) = topology_endpoint_key(
                    &asset.asset_type,
                    other_node_id,
                    &node_by_id,
                ) else {
                    continue;
                };
                endpoint_keys.insert(endpoint_key);
            }
        }

        let used = endpoint_keys.len() as i64;
        let available = (total - used).max(0);
        let state = if used <= 0 {
            "empty"
        } else if available <= 0 {
            "full"
        } else {
            "partial"
        };

        let mut metadata = asset.metadata.as_object().cloned().unwrap_or_default();
        metadata.insert(CACHE_TOTAL_KEY.into(), Value::Number(total.into()));
        metadata.insert(CACHE_USED_KEY.into(), Value::Number(used.into()));
        metadata.insert(CACHE_AVAILABLE_KEY.into(), Value::Number(available.into()));
        metadata.insert(CACHE_STATE_KEY.into(), Value::String(state.to_string()));
        metadata.insert(
            CACHE_UPDATED_AT_KEY.into(),
            Value::String(chrono::Utc::now().to_rfc3339()),
        );

        sqlx::query(
            r#"
            UPDATE network_assets
            SET metadata = $3
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(&asset.id)
        .bind(Value::Object(metadata))
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    }

    Ok(())
}

fn parse_positive_integer(value: Option<&Value>) -> Option<i64> {
    match value {
        Some(Value::Number(number)) => number.as_i64().filter(|value| *value > 0),
        Some(Value::String(text)) => text.trim().parse::<i64>().ok().filter(|value| *value > 0),
        _ => None,
    }
}

fn asset_endpoint_key(location_id: Option<&str>, customer_id: Option<&str>, asset_id: &str) -> String {
    if let Some(location_id) = location_id.filter(|value| !value.trim().is_empty()) {
        return format!("location:{}", location_id.trim());
    }
    if let Some(customer_id) = customer_id.filter(|value| !value.trim().is_empty()) {
        return format!("customer:{}", customer_id.trim());
    }
    format!("asset:{asset_id}")
}

fn topology_endpoint_key(
    source_asset_type: &str,
    other_node_id: &str,
    node_by_id: &HashMap<String, NodeRow>,
) -> Option<String> {
    let other_node = node_by_id.get(other_node_id)?;
    let source = metadata_text(&other_node.metadata, "asset_source");

    if source == "mikrotik_router" || other_node.node_type == "router" {
        return Some(format!("router:{}", other_node.id));
    }

    if source == "network_asset" {
        let asset_id = metadata_text(&other_node.metadata, "asset_id");
        let target_asset_type = metadata_text(&other_node.metadata, "asset_type");
        if asset_id.is_empty() || !should_count_topology_asset_port_usage(source_asset_type, &target_asset_type) {
            return None;
        }
        return Some(format!("asset:{asset_id}"));
    }

    if source != "customer_location" || other_node.node_type != "customer_premise" {
        return None;
    }

    let location_id = metadata_text(&other_node.metadata, "location_id");
    if !location_id.is_empty() {
        return Some(format!("location:{location_id}"));
    }

    let customer_id = metadata_text(&other_node.metadata, "customer_id");
    if !customer_id.is_empty() {
        return Some(format!("customer:{customer_id}"));
    }

    Some(format!("node:{}", other_node.id))
}

fn should_count_topology_asset_port_usage(source_asset_type: &str, target_asset_type: &str) -> bool {
    let source_rank = asset_rank(source_asset_type);
    let target_rank = asset_rank(target_asset_type);
    matches!((source_rank, target_rank), (Some(source_rank), Some(target_rank)) if target_rank >= source_rank)
}

fn asset_rank(asset_type: &str) -> Option<i32> {
    let normalized = asset_type.trim().to_lowercase();
    ASSET_PORT_RANK
        .iter()
        .find(|(candidate, _)| *candidate == normalized)
        .map(|(_, rank)| *rank)
}

fn metadata_text(metadata: &Value, key: &str) -> String {
    metadata
        .as_object()
        .and_then(|map: &Map<String, Value>| map.get(key))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}
