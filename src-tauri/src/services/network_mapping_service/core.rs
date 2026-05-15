use super::dto::{PathLinkRow, SnappedPolylinePoint};
use super::NetworkMappingService;
use crate::error::{AppError, AppResult};

impl NetworkMappingService {
    pub(super) fn cleaned_query(q: Option<String>) -> String {
        q.unwrap_or_default().trim().to_string()
    }

    pub(super) fn point_distance_sq(a: [f64; 2], b: [f64; 2]) -> f64 {
        let dx = a[0] - b[0];
        let dy = a[1] - b[1];
        (dx * dx) + (dy * dy)
    }

    pub(super) fn coords_approx_equal(a: [f64; 2], b: [f64; 2]) -> bool {
        Self::point_distance_sq(a, b) <= 1e-16
    }

    pub(super) fn parse_line_coords(
        geometry: &serde_json::Value,
        field: &str,
    ) -> AppResult<Vec<[f64; 2]>> {
        let obj = geometry
            .as_object()
            .ok_or_else(|| AppError::Validation(format!("{field} must be a GeoJSON object")))?;
        let kind = obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Validation(format!("{field}.type is required")))?;
        let coords = obj
            .get("coordinates")
            .ok_or_else(|| AppError::Validation(format!("{field}.coordinates is required")))?;

        let mut out: Vec<[f64; 2]> = Vec::new();
        match kind {
            "LineString" => {
                let arr = coords.as_array().ok_or_else(|| {
                    AppError::Validation(format!("{field}.coordinates must be an array"))
                })?;
                for point in arr {
                    let pt = point.as_array().ok_or_else(|| {
                        AppError::Validation(format!("{field}.coordinates contains invalid point"))
                    })?;
                    if pt.len() < 2 {
                        return Err(AppError::Validation(format!(
                            "{field}.coordinates contains invalid point"
                        )));
                    }
                    let lng = pt[0].as_f64().ok_or_else(|| {
                        AppError::Validation(format!("{field}.coordinates contains invalid lng"))
                    })?;
                    let lat = pt[1].as_f64().ok_or_else(|| {
                        AppError::Validation(format!("{field}.coordinates contains invalid lat"))
                    })?;
                    out.push([lng, lat]);
                }
            }
            "MultiLineString" => {
                let lines = coords.as_array().ok_or_else(|| {
                    AppError::Validation(format!("{field}.coordinates must be an array"))
                })?;
                for line in lines {
                    let line_points = line.as_array().ok_or_else(|| {
                        AppError::Validation(format!(
                            "{field}.coordinates contains invalid line segment"
                        ))
                    })?;
                    for point in line_points {
                        let pt = point.as_array().ok_or_else(|| {
                            AppError::Validation(format!(
                                "{field}.coordinates contains invalid point"
                            ))
                        })?;
                        if pt.len() < 2 {
                            return Err(AppError::Validation(format!(
                                "{field}.coordinates contains invalid point"
                            )));
                        }
                        let lng = pt[0].as_f64().ok_or_else(|| {
                            AppError::Validation(format!(
                                "{field}.coordinates contains invalid lng"
                            ))
                        })?;
                        let lat = pt[1].as_f64().ok_or_else(|| {
                            AppError::Validation(format!(
                                "{field}.coordinates contains invalid lat"
                            ))
                        })?;
                        let candidate = [lng, lat];
                        if out
                            .last()
                            .copied()
                            .map(|prev| Self::coords_approx_equal(prev, candidate))
                            .unwrap_or(false)
                        {
                            continue;
                        }
                        out.push(candidate);
                    }
                }
            }
            _ => {
                return Err(AppError::Validation(format!(
                    "{field}.type must be LineString or MultiLineString"
                )));
            }
        }

        if out.len() < 2 {
            return Err(AppError::Validation(format!(
                "{field} must contain at least 2 coordinates"
            )));
        }
        Ok(out)
    }

    pub(super) fn build_line_geometry(coords: &[[f64; 2]]) -> serde_json::Value {
        serde_json::json!({
            "type": "LineString",
            "coordinates": coords.iter().map(|pt| vec![pt[0], pt[1]]).collect::<Vec<_>>(),
        })
    }

    pub(super) fn snap_point_to_polyline(
        coords: &[[f64; 2]],
        lng: f64,
        lat: f64,
    ) -> Option<SnappedPolylinePoint> {
        if coords.len() < 2 {
            return None;
        }

        let point = [lng, lat];
        let mut best: Option<SnappedPolylinePoint> = None;

        for segment_index in 0..(coords.len() - 1) {
            let a = coords[segment_index];
            let b = coords[segment_index + 1];
            let abx = b[0] - a[0];
            let aby = b[1] - a[1];
            let denom = (abx * abx) + (aby * aby);
            let t = if denom <= 1e-18 {
                0.0
            } else {
                (((point[0] - a[0]) * abx) + ((point[1] - a[1]) * aby)) / denom
            }
            .clamp(0.0, 1.0);
            let snapped_lng = a[0] + (abx * t);
            let snapped_lat = a[1] + (aby * t);
            let distance_sq = Self::point_distance_sq(point, [snapped_lng, snapped_lat]);
            let candidate = SnappedPolylinePoint {
                lng: snapped_lng,
                lat: snapped_lat,
                segment_index,
                t,
                distance_sq,
            };

            if best
                .as_ref()
                .map(|current| candidate.distance_sq < current.distance_sq)
                .unwrap_or(true)
            {
                best = Some(candidate);
            }
        }

        best
    }

    pub(super) fn dedupe_line_coords(coords: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
        let mut out: Vec<[f64; 2]> = Vec::with_capacity(coords.len());
        for point in coords {
            if out
                .last()
                .copied()
                .map(|prev| Self::coords_approx_equal(prev, point))
                .unwrap_or(false)
            {
                continue;
            }
            out.push(point);
        }
        out
    }

    pub(super) fn split_polyline_at_point(
        coords: &[[f64; 2]],
        snapped: &SnappedPolylinePoint,
    ) -> AppResult<(Vec<[f64; 2]>, Vec<[f64; 2]>)> {
        if coords.len() < 2 {
            return Err(AppError::Validation(
                "Target link geometry is too short".into(),
            ));
        }

        let split_point = [snapped.lng, snapped.lat];
        let start = coords[0];
        let end = coords[coords.len() - 1];

        if Self::coords_approx_equal(start, split_point)
            || Self::coords_approx_equal(end, split_point)
            || (snapped.segment_index == 0 && snapped.t <= 1e-6)
            || (snapped.segment_index == coords.len() - 2 && snapped.t >= 1.0 - 1e-6)
        {
            return Err(AppError::Validation(
                "Selected point is too close to an existing node. Click the node instead.".into(),
            ));
        }

        let mut first = coords[..=snapped.segment_index].to_vec();
        first.push(split_point);
        let mut second = vec![split_point];
        second.extend_from_slice(&coords[(snapped.segment_index + 1)..]);

        let first = Self::dedupe_line_coords(first);
        let second = Self::dedupe_line_coords(second);

        if first.len() < 2 || second.len() < 2 {
            return Err(AppError::Validation(
                "Selected point does not create two valid link segments".into(),
            ));
        }

        Ok((first, second))
    }

    pub(super) fn link_cost(link: &PathLinkRow) -> f64 {
        let distance_km = (link.distance_m.max(0.0)) / 1000.0;
        let latency_component = link.latency_ms.unwrap_or(0.0) * 0.2;
        let utilization_component = link.utilization_pct.unwrap_or(0.0) * 0.1;
        let loss_component = link.loss_db.unwrap_or(0.0).abs() * 5.0;
        let status_penalty = match link.status.as_str() {
            "degraded" => 25.0,
            "planning" => 75.0,
            _ => 0.0,
        };
        (distance_km + latency_component + utilization_component + loss_component + status_penalty)
            .max(0.0001)
    }

    pub(super) fn json_number(value: &serde_json::Value, key: &str) -> Option<f64> {
        value.get(key).and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_i64().map(|n| n as f64))
                .or_else(|| v.as_u64().map(|n| n as f64))
                .or_else(|| v.as_str().and_then(|s| s.trim().parse::<f64>().ok()))
        })
    }

    pub(super) fn clamp_0_100(v: f64) -> f64 {
        if v.is_nan() {
            0.0
        } else {
            v.clamp(0.0, 100.0)
        }
    }

    pub(super) fn compute_health_score(status: &str, health_json: &serde_json::Value) -> f64 {
        if let Some(score) = Self::json_number(health_json, "score") {
            return Self::clamp_0_100(score);
        }
        if let Some(score) = Self::json_number(health_json, "health_score") {
            return Self::clamp_0_100(score);
        }
        match status {
            "active" => 85.0,
            "maintenance" => 60.0,
            _ => 40.0,
        }
    }

    pub(super) fn compute_capacity_score(
        capacity_json: &serde_json::Value,
        avg_link_utilization_pct: Option<f64>,
    ) -> f64 {
        if let Some(free_pct) = Self::json_number(capacity_json, "free_pct") {
            return Self::clamp_0_100(free_pct);
        }
        if let Some(util_pct) = Self::json_number(capacity_json, "utilization_pct") {
            return Self::clamp_0_100(100.0 - util_pct);
        }
        let available_mbps = Self::json_number(capacity_json, "available_mbps");
        let total_mbps = Self::json_number(capacity_json, "total_mbps");
        if let (Some(avail), Some(total)) = (available_mbps, total_mbps) {
            if total > 0.0 {
                return Self::clamp_0_100((avail / total) * 100.0);
            }
        }
        if let Some(util) = avg_link_utilization_pct {
            return Self::clamp_0_100(100.0 - util);
        }
        60.0
    }

    pub(super) fn compute_distance_score(distance_m: Option<f64>) -> Option<f64> {
        distance_m.map(|distance| {
            let normalized = (distance / 50_000.0).clamp(0.0, 1.0);
            Self::clamp_0_100(100.0 - (normalized * 100.0))
        })
    }

    pub(super) fn is_system_managed_node(metadata: &serde_json::Value) -> bool {
        metadata
            .get("system_managed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub(super) fn system_managed_node_source_label(metadata: &serde_json::Value) -> Option<&str> {
        metadata
            .get("asset_source")
            .and_then(|v| v.as_str())
            .or_else(|| metadata.get("asset_type").and_then(|v| v.as_str()))
    }

    #[cfg(test)]
    pub(super) fn system_managed_node_matches_asset_source(
        metadata: &serde_json::Value,
        asset_source: &str,
    ) -> bool {
        let normalized_source = asset_source.trim();
        if normalized_source.is_empty() {
            return false;
        }

        if matches!(
            normalized_source,
            "network_asset" | "mikrotik_router" | "customer_location"
        ) {
            return metadata
                .get("asset_source")
                .and_then(|v| v.as_str())
                .map(str::trim)
                == Some(normalized_source);
        }

        metadata
            .get("asset_type")
            .and_then(|v| v.as_str())
            .map(str::trim)
            == Some(normalized_source)
    }

    #[cfg(test)]
    pub(super) fn system_managed_node_matches_asset_reference(
        metadata: &serde_json::Value,
        asset_source: &str,
        asset_id: &str,
    ) -> bool {
        let normalized_asset_id = asset_id.trim();
        if normalized_asset_id.is_empty()
            || !Self::system_managed_node_matches_asset_source(metadata, asset_source)
        {
            return false;
        }

        match asset_source.trim() {
            "mikrotik_router" => {
                for key in [
                    "router_id",
                    "routerId",
                    "mikrotik_router_id",
                    "mikrotikRouterId",
                    "asset_id",
                ] {
                    if metadata.get(key).and_then(|v| v.as_str()).map(str::trim)
                        == Some(normalized_asset_id)
                    {
                        return true;
                    }
                }
                false
            }
            "customer_location" => {
                for key in [
                    "location_id",
                    "locationId",
                    "customer_location_id",
                    "customerLocationId",
                    "asset_id",
                ] {
                    if metadata.get(key).and_then(|v| v.as_str()).map(str::trim)
                        == Some(normalized_asset_id)
                    {
                        return true;
                    }
                }
                false
            }
            _ => {
                metadata
                    .get("asset_id")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    == Some(normalized_asset_id)
            }
        }
    }

    pub(super) fn customer_subscription_to_node_status(status: &str) -> &'static str {
        match status.trim().to_lowercase().as_str() {
            "suspended" => "maintenance",
            "inactive" | "cancelled" => "inactive",
            _ => "active",
        }
    }

    pub(super) fn network_asset_to_node_status(status: &str) -> &'static str {
        match status.trim().to_lowercase().as_str() {
            "faulty" => "maintenance",
            "retired" => "inactive",
            _ => "active",
        }
    }

    pub(super) fn customer_pppoe_visual_state(
        customer_is_active: bool,
        subscription_status: &str,
        pppoe_username: Option<&str>,
        pppoe_session_active: bool,
        pppoe_disabled: bool,
    ) -> &'static str {
        let normalized_status = subscription_status.trim().to_lowercase();
        if !customer_is_active
            || pppoe_disabled
            || matches!(
                normalized_status.as_str(),
                "suspended" | "inactive" | "cancelled"
            )
        {
            return "neutral";
        }

        let has_pppoe = pppoe_username
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some();
        if !has_pppoe {
            return "neutral";
        }

        if pppoe_session_active {
            "connected"
        } else {
            "disconnected"
        }
    }
}
