//! HTTP layer for technician GPS tracking endpoints.
//! Mirrors src-tauri/src/commands/technician_location.rs (which is for the
//! Tauri desktop invoke handler; this file is for the Axum REST API).

use super::AppState;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::auth::auth_claims;

#[derive(Debug, Deserialize)]
pub struct RecordLocationRequest {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: Option<f64>,
    pub altitude: Option<f64>,
    pub bearing: Option<f64>,
    pub speed: Option<f64>,
    pub captured_at: DateTime<Utc>,
    pub battery_level: Option<i16>,
}

#[derive(Debug, Serialize)]
pub struct RecordedLocation {
    pub id: String,
    pub technician_id: String,
    pub captured_at: DateTime<Utc>,
}

/// POST /api/technician/locations
pub async fn record_technician_location(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RecordLocationRequest>,
) -> Result<Json<RecordedLocation>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    // GPS endpoint open to any authenticated technician/staff/admin.
    // (No specific permission required — role check is enough.)
    if !matches!(
        claims.role.as_str(),
        "technician" | "staff" | "admin" | "super_admin" | "owner"
    ) {
        return Err(crate::error::AppError::Forbidden(
            "Only field workers and admins can submit GPS pings".to_string(),
        ));
    }

    if !(-90.0..=90.0).contains(&req.latitude) {
        return Err(crate::error::AppError::Validation(format!(
            "Invalid latitude: {}",
            req.latitude
        )));
    }
    if !(-180.0..=180.0).contains(&req.longitude) {
        return Err(crate::error::AppError::Validation(format!(
            "Invalid longitude: {}",
            req.longitude
        )));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let battery = req.battery_level.map(|b| b.clamp(0, 100) as i16);

    sqlx::query(
        r#"
        INSERT INTO technician_locations (
            id, tenant_id, technician_id, latitude, longitude,
            accuracy, altitude, bearing, speed, captured_at, battery_level
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(&id)
    .bind(&tenant_id)
    .bind(&claims.sub)
    .bind(req.latitude)
    .bind(req.longitude)
    .bind(req.accuracy)
    .bind(req.altitude)
    .bind(req.bearing)
    .bind(req.speed)
    .bind(req.captured_at)
    .bind(battery)
    .execute(&state.auth_service.pool)
    .await?;

    Ok(Json(RecordedLocation {
        id,
        technician_id: claims.sub,
        captured_at: req.captured_at,
    }))
}

#[derive(Debug, Serialize)]
pub struct LatestLocation {
    pub technician_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: Option<f64>,
    pub captured_at: DateTime<Utc>,
}

/// GET /api/technician/locations/:technician_id/latest
pub async fn get_latest_technician_location(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(technician_id): Path<String>,
) -> Result<Json<Option<LatestLocation>>, crate::error::AppError> {
    let claims = auth_claims(&state, &headers).await?;
    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or(crate::error::AppError::Validation(
            "Tenant context required".to_string(),
        ))?;

    if !matches!(
        claims.role.as_str(),
        "admin" | "super_admin" | "owner" | "staff"
    ) {
        return Err(crate::error::AppError::Forbidden(
            "Only admins can view technician locations".to_string(),
        ));
    }

    let row: Option<(String, f64, f64, Option<f64>, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT technician_id, latitude, longitude, accuracy, captured_at
        FROM technician_locations
        WHERE tenant_id = $1 AND technician_id = $2
        ORDER BY captured_at DESC
        LIMIT 1
        "#,
    )
    .bind(&tenant_id)
    .bind(&technician_id)
    .fetch_optional(&state.auth_service.pool)
    .await?;

    Ok(Json(row.map(
        |(tid, lat, lng, acc, cap)| LatestLocation {
            technician_id: tid,
            latitude: lat,
            longitude: lng,
            accuracy: acc,
            captured_at: cap,
        },
    )))
}