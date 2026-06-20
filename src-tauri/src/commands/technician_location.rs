//! Technician GPS location tracking (mobile-technician app pings every ~2 min).
//! Admin reads latest positions for live map view.

use crate::services::AuthService;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

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

/// Record a single GPS ping from the technician's phone.
/// Permission `technician:track:self` is required (added in seed).
/// Silent on success — the client doesn't need the response back.
#[tauri::command]
pub async fn record_technician_location(
    token: String,
    req: RecordLocationRequest,
    auth_service: State<'_, AuthService>,
) -> Result<RecordedLocation, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "technician", "track:self")
        .await
        .map_err(|e| e.to_string())?;

    // Validate lat/lng ranges — reject obvious garbage
    if !(-90.0..=90.0).contains(&req.latitude) {
        return Err(format!("Invalid latitude: {}", req.latitude));
    }
    if !(-180.0..=180.0).contains(&req.longitude) {
        return Err(format!("Invalid longitude: {}", req.longitude));
    }

    let id = Uuid::new_v4().to_string();
    let battery = req.battery_level.map(|b| b.clamp(0, 100) as i16);

    sqlx::query(
        r#"
        INSERT INTO technician_locations (
            id, tenant_id, technician_id, latitude, longitude,
            accuracy, altitude, bearing, speed, captured_at, battery_level
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
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
    .execute(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(RecordedLocation {
        id,
        technician_id: claims.sub,
        captured_at: req.captured_at,
    })
}

/// Latest GPS position for a single technician.
/// Requires `technician:track:read_all` (admin/staff).
#[derive(Debug, Serialize)]
pub struct LatestLocation {
    pub technician_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: Option<f64>,
    pub captured_at: DateTime<Utc>,
}

#[tauri::command]
pub async fn get_latest_technician_location(
    token: String,
    technician_id: String,
    auth_service: State<'_, AuthService>,
) -> Result<Option<LatestLocation>, String> {
    let claims = auth_service
        .validate_token(&token)
        .await
        .map_err(|e| e.to_string())?;

    let tenant_id = claims
        .tenant_id
        .clone()
        .ok_or_else(|| "Tenant context required".to_string())?;

    auth_service
        .check_permission(&claims.sub, &tenant_id, "technician", "track:read_all")
        .await
        .map_err(|e| e.to_string())?;

    let row = sqlx::query_as::<_, (String, f64, f64, Option<f64>, DateTime<Utc>)>(
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
    .fetch_optional(&auth_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(|(tid, lat, lng, acc, cap)| LatestLocation {
        technician_id: tid,
        latitude: lat,
        longitude: lng,
        accuracy: acc,
        captured_at: cap,
    }))
}