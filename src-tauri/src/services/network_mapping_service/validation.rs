use super::NetworkMappingService;
use crate::error::{AppError, AppResult};

impl NetworkMappingService {

    pub(super) fn validate_lat_lng(lat: f64, lng: f64, field: &str) -> AppResult<()> {
        if !(-90.0..=90.0).contains(&lat) {
            return Err(AppError::Validation(format!(
                "{field}.lat must be between -90 and 90"
            )));
        }
        if !(-180.0..=180.0).contains(&lng) {
            return Err(AppError::Validation(format!(
                "{field}.lng must be between -180 and 180"
            )));
        }
        Ok(())
    }


    pub(super) fn validate_geojson_geometry(
        geometry: &serde_json::Value,
        expected_types: &[&str],
        field: &str,
    ) -> AppResult<()> {
        let obj = geometry
            .as_object()
            .ok_or_else(|| AppError::Validation(format!("{field} must be a GeoJSON object")))?;
        let kind = obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Validation(format!("{field}.type is required")))?;
        if !expected_types.iter().any(|t| *t == kind) {
            return Err(AppError::Validation(format!(
                "{field}.type must be one of: {}",
                expected_types.join(", ")
            )));
        }
        if !obj.contains_key("coordinates") {
            return Err(AppError::Validation(format!(
                "{field}.coordinates is required"
            )));
        }
        Ok(())
    }


    pub(super) fn map_geometry_db_error(err: sqlx::Error, field: &str) -> AppError {
        let msg = err.to_string().to_lowercase();
        if msg.contains("st_geomfromgeojson")
            || msg.contains("parse error")
            || msg.contains("invalid geojson")
            || msg.contains("geometry")
            || msg.contains("lwgeom")
        {
            return AppError::Validation(format!("{field} is invalid GeoJSON geometry"));
        }
        AppError::Database(err)
    }


    pub(super) fn normalize_link_status(input: &str) -> String {
        match input.trim().to_lowercase().as_str() {
            "active" => "up".to_string(),
            "inactive" => "down".to_string(),
            other => other.to_string(),
        }
    }


    pub(super) fn validate_link_status(status: &str) -> AppResult<()> {
        match status {
            "up" | "down" | "degraded" | "maintenance" | "planning" | "retired" => Ok(()),
            _ => Err(AppError::Validation(
                "link status must be one of: up, down, degraded, maintenance, planning, retired"
                    .into(),
            )),
        }
    }


}
