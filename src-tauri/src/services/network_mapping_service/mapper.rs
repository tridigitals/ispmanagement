use super::NetworkMappingService;
use super::dto::{CandidateNodeRow, NodeStatusRow, PathEdge, PathLinkRow};
use crate::error::{AppError, AppResult};
use crate::models::{
    ComputePathRequest, ComputePathResponse, ComputedPathHop, RankCandidateNodesRequest,
    RankCandidateNodesResponse, RankedCandidateNode,
};
use std::collections::HashMap;

impl NetworkMappingService {
    pub(super) fn candidate_row_to_ranked(row: CandidateNodeRow) -> RankedCandidateNode {
        let health_score = Self::compute_health_score(&row.status, &row.health_json);
        let capacity_score =
            Self::compute_capacity_score(&row.capacity_json, row.avg_link_utilization_pct);
        let distance_score = Self::compute_distance_score(row.distance_m);
        let distance_component = distance_score.unwrap_or(60.0);

        let stability_penalty = (row.down_links as f64 * 7.5) + if row.link_count == 0 { 12.0 } else { 0.0 };
        let base_score =
            (health_score * 0.45) + (capacity_score * 0.35) + (distance_component * 0.20);
        let score = (base_score - stability_penalty).clamp(0.0, 100.0);
        let reason = format!(
            "health {:.0}, capacity {:.0}{}{}",
            health_score,
            capacity_score,
            match distance_score {
                Some(s) => format!(", distance {:.0}", s),
                None => String::new(),
            },
            if row.down_links > 0 {
                format!(", penalty: {} down link(s)", row.down_links)
            } else {
                String::new()
            }
        );

        RankedCandidateNode {
            node_id: row.node_id,
            name: row.name,
            node_type: row.node_type,
            status: row.status,
            score,
            health_score,
            capacity_score,
            distance_score,
            distance_m: row.distance_m,
            avg_link_utilization_pct: row.avg_link_utilization_pct,
            down_links: row.down_links,
            link_count: row.link_count,
            reason,
        }
    }

    pub(super) fn build_path_not_found_response(
        source_node_id: String,
        target_node_id: String,
    ) -> ComputePathResponse {
        ComputePathResponse {
            found: false,
            source_node_id,
            target_node_id,
            node_ids: vec![],
            link_ids: vec![],
            hops: vec![],
            total_cost: None,
            total_distance_m: None,
        }
    }

    fn build_path_found_response(
        source_id: String,
        target_id: String,
        total_cost: f64,
        edges: Vec<PathEdge>,
    ) -> ComputePathResponse {
        let mut node_ids = vec![source_id.clone()];
        let mut link_ids = Vec::with_capacity(edges.len());
        let mut hops = Vec::with_capacity(edges.len());
        let mut total_distance = 0.0;

        for (idx, step) in edges.into_iter().enumerate() {
            total_distance += step.distance_m;
            link_ids.push(step.link_id.clone());
            node_ids.push(step.to_node_id.clone());
            hops.push(ComputedPathHop {
                seq_no: idx as i32 + 1,
                link_id: step.link_id,
                from_node_id: step.from_node_id,
                to_node_id: step.to_node_id,
                name: step.name,
                link_type: step.link_type,
                status: step.status,
                distance_m: step.distance_m,
                cost: step.cost,
            });
        }

        ComputePathResponse {
            found: true,
            source_node_id: source_id,
            target_node_id: target_id,
            node_ids,
            link_ids,
            hops,
            total_cost: Some(total_cost),
            total_distance_m: Some(total_distance),
        }
    }

    pub(super) async fn rank_candidate_nodes_flow(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: RankCandidateNodesRequest,
    ) -> AppResult<RankCandidateNodesResponse> {
        self.require_read(actor_id, tenant_id).await?;

        if dto.lat.is_some() ^ dto.lng.is_some() {
            return Err(AppError::Validation(
                "lat and lng must be provided together".into(),
            ));
        }
        if let (Some(lat), Some(lng)) = (dto.lat, dto.lng) {
            Self::validate_lat_lng(lat, lng, "candidate")?;
        }

        let limit = dto.limit.unwrap_or(10).clamp(1, 100) as i64;
        let node_types = dto.node_types.filter(|v| !v.is_empty());
        let require_active_nodes = dto.require_active_nodes.unwrap_or(true);
        let zone_id = dto.zone_id.clone();
        let has_point = dto.lat.is_some() && dto.lng.is_some();

        let rows: Vec<CandidateNodeRow> = sqlx::query_as(
            r#"
            SELECT
              n.id::text AS node_id,
              n.name,
              n.node_type,
              n.status,
              n.capacity_json,
              n.health_json,
              CASE
                WHEN $2::bool = true
                THEN ST_Distance(
                  geography(n.geom),
                  geography(ST_SetSRID(ST_MakePoint($3::float8, $4::float8), 4326))
                )::float8
                ELSE NULL
              END AS distance_m,
              AVG(l.utilization_pct::float8) FILTER (WHERE l.utilization_pct IS NOT NULL) AS avg_link_utilization_pct,
              COALESCE(COUNT(l.id) FILTER (WHERE l.status = 'down'), 0)::bigint AS down_links,
              COALESCE(COUNT(l.id), 0)::bigint AS link_count
            FROM network_nodes n
            LEFT JOIN network_links l
              ON l.tenant_id = n.tenant_id
             AND (l.from_node_id = n.id OR l.to_node_id = n.id)
            WHERE n.tenant_id = $1::uuid
              AND ($5::bool = false OR n.status = 'active')
              AND ($6::text[] IS NULL OR n.node_type = ANY($6::text[]))
              AND (
                $7::uuid IS NULL
                OR EXISTS (
                  SELECT 1
                  FROM zone_node_bindings znb
                  WHERE znb.tenant_id = n.tenant_id
                    AND znb.zone_id = $7::uuid
                    AND znb.node_id = n.id
                )
              )
            GROUP BY n.id
            LIMIT 400
            "#,
        )
        .bind(tenant_id)
        .bind(has_point)
        .bind(dto.lng)
        .bind(dto.lat)
        .bind(require_active_nodes)
        .bind(node_types)
        .bind(zone_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let mut items: Vec<RankedCandidateNode> =
            rows.into_iter().map(Self::candidate_row_to_ranked).collect();

        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.distance_m
                        .unwrap_or(f64::INFINITY)
                        .partial_cmp(&b.distance_m.unwrap_or(f64::INFINITY))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        items.truncate(limit as usize);

        Ok(RankCandidateNodesResponse { items })
    }

    pub(super) async fn compute_path_flow(
        &self,
        actor_id: &str,
        tenant_id: &str,
        dto: ComputePathRequest,
    ) -> AppResult<ComputePathResponse> {
        self.require_read(actor_id, tenant_id).await?;
        let source_id = dto.source_node_id.clone();
        let target_id = dto.target_node_id.clone();

        if source_id == target_id {
            return Err(AppError::Validation(
                "source_node_id and target_node_id must be different".into(),
            ));
        }

        let node_rows: Vec<NodeStatusRow> = sqlx::query_as(
            r#"
            SELECT id::text AS id, status
            FROM network_nodes
            WHERE tenant_id = $1::uuid
              AND id::text IN ($2, $3)
            "#,
        )
        .bind(tenant_id)
        .bind(&source_id)
        .bind(&target_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if node_rows.len() < 2 {
            return Err(AppError::Validation(
                "source_node_id or target_node_id not found".into(),
            ));
        }

        let source_status = node_rows
            .iter()
            .find(|n| n.id == source_id)
            .map(|n| n.status.clone())
            .unwrap_or_else(|| "inactive".to_string());
        let target_status = node_rows
            .iter()
            .find(|n| n.id == target_id)
            .map(|n| n.status.clone())
            .unwrap_or_else(|| "inactive".to_string());

        let require_active_nodes = dto.require_active_nodes.unwrap_or(true);
        if require_active_nodes && (source_status != "active" || target_status != "active") {
            return Ok(Self::build_path_not_found_response(
                source_id.clone(),
                target_id.clone(),
            ));
        }

        let allowed_statuses = if let Some(v) = dto.allowed_statuses {
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        } else {
            Some(vec!["up".to_string(), "degraded".to_string()])
        };
        let allowed_link_types = dto.allowed_link_types.filter(|v| !v.is_empty());
        let exclude_link_ids = dto.exclude_link_ids.filter(|v| !v.is_empty());

        let links: Vec<PathLinkRow> = sqlx::query_as(
            r#"
            SELECT
              id::text AS id,
              from_node_id::text AS from_node_id,
              to_node_id::text AS to_node_id,
              name,
              link_type,
              status,
              COALESCE(ST_Length(geography(geom)), 0)::float8 AS distance_m,
              utilization_pct::float8 AS utilization_pct,
              loss_db::float8 AS loss_db,
              latency_ms::float8 AS latency_ms
            FROM network_links
            WHERE tenant_id = $1::uuid
              AND ($2::text[] IS NULL OR link_type = ANY($2::text[]))
              AND ($3::text[] IS NULL OR status = ANY($3::text[]))
              AND ($4::text[] IS NULL OR NOT (id::text = ANY($4::text[])))
              AND ($5::float8 IS NULL OR utilization_pct IS NULL OR utilization_pct::float8 <= $5::float8)
            "#,
        )
        .bind(tenant_id)
        .bind(allowed_link_types)
        .bind(allowed_statuses)
        .bind(exclude_link_ids)
        .bind(dto.max_utilization_pct)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if links.is_empty() {
            return Ok(Self::build_path_not_found_response(
                source_id.clone(),
                target_id.clone(),
            ));
        }

        let node_status_rows: Vec<NodeStatusRow> = sqlx::query_as(
            r#"
            SELECT id::text AS id, status
            FROM network_nodes
            WHERE tenant_id = $1::uuid
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;
        let node_statuses: HashMap<String, String> =
            node_status_rows.into_iter().map(|r| (r.id, r.status)).collect();

        let mut adjacency: HashMap<String, Vec<PathEdge>> = HashMap::new();
        for link in links {
            if require_active_nodes {
                let from_active = node_statuses
                    .get(&link.from_node_id)
                    .map(|s| s == "active")
                    .unwrap_or(false);
                let to_active = node_statuses
                    .get(&link.to_node_id)
                    .map(|s| s == "active")
                    .unwrap_or(false);
                if !from_active || !to_active {
                    continue;
                }
            }

            let cost = Self::link_cost(&link);
            let forward = PathEdge {
                link_id: link.id.clone(),
                from_node_id: link.from_node_id.clone(),
                to_node_id: link.to_node_id.clone(),
                name: link.name.clone(),
                link_type: link.link_type.clone(),
                status: link.status.clone(),
                distance_m: link.distance_m,
                cost,
            };
            let backward = PathEdge {
                link_id: link.id.clone(),
                from_node_id: link.to_node_id.clone(),
                to_node_id: link.from_node_id.clone(),
                name: link.name,
                link_type: link.link_type,
                status: link.status,
                distance_m: link.distance_m,
                cost,
            };
            adjacency
                .entry(forward.from_node_id.clone())
                .or_default()
                .push(forward);
            adjacency
                .entry(backward.from_node_id.clone())
                .or_default()
                .push(backward);
        }

        if !adjacency.contains_key(&source_id) {
            return Ok(Self::build_path_not_found_response(
                source_id.clone(),
                target_id.clone(),
            ));
        }

        let max_hops = dto.max_hops.unwrap_or(64).max(1) as usize;
        let mut dist: HashMap<String, f64> = HashMap::new();
        let mut hop_count: HashMap<String, usize> = HashMap::new();
        let mut prev: HashMap<String, PathEdge> = HashMap::new();
        let mut frontier: Vec<(String, f64)> = vec![(source_id.clone(), 0.0)];
        dist.insert(source_id.clone(), 0.0);
        hop_count.insert(source_id.clone(), 0);

        while !frontier.is_empty() {
            let mut min_idx = 0usize;
            for i in 1..frontier.len() {
                if frontier[i].1 < frontier[min_idx].1 {
                    min_idx = i;
                }
            }
            let (node, cost_here) = frontier.swap_remove(min_idx);
            let best = *dist.get(&node).unwrap_or(&f64::INFINITY);
            if cost_here > best {
                continue;
            }
            if node == target_id {
                break;
            }

            let current_hops = *hop_count.get(&node).unwrap_or(&0);
            if current_hops >= max_hops {
                continue;
            }

            for edge in adjacency.get(&node).cloned().unwrap_or_default() {
                let next = edge.to_node_id.clone();
                let next_hops = current_hops + 1;
                if next_hops > max_hops {
                    continue;
                }
                let candidate = cost_here + edge.cost;
                let current_best = *dist.get(&next).unwrap_or(&f64::INFINITY);
                if candidate + 1e-9 < current_best {
                    dist.insert(next.clone(), candidate);
                    hop_count.insert(next.clone(), next_hops);
                    prev.insert(next.clone(), edge);
                    frontier.push((next, candidate));
                }
            }
        }

        let Some(total_cost) = dist.get(&target_id).copied() else {
            return Ok(Self::build_path_not_found_response(
                source_id.clone(),
                target_id.clone(),
            ));
        };

        let mut reversed: Vec<PathEdge> = Vec::new();
        let mut cursor = target_id.clone();
        while cursor != source_id {
            let Some(step) = prev.get(&cursor).cloned() else {
                return Ok(Self::build_path_not_found_response(
                    source_id.clone(),
                    target_id.clone(),
                ));
            };
            cursor = step.from_node_id.clone();
            reversed.push(step);
        }
        reversed.reverse();

        Ok(Self::build_path_found_response(
            source_id,
            target_id,
            total_cost,
            reversed,
        ))
    }
}
