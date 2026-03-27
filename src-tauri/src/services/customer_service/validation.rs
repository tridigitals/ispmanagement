use super::*;

impl CustomerService {
    pub(super) fn validate_location_coordinates(
        latitude: Option<f64>,
        longitude: Option<f64>,
    ) -> AppResult<(f64, f64)> {
        let lat = latitude
            .ok_or_else(|| AppError::Validation("Location map point is required".to_string()))?;
        let lng = longitude
            .ok_or_else(|| AppError::Validation("Location map point is required".to_string()))?;
        if !(-90.0..=90.0).contains(&lat) {
            return Err(AppError::Validation(
                "Latitude must be between -90 and 90".to_string(),
            ));
        }
        if !(-180.0..=180.0).contains(&lng) {
            return Err(AppError::Validation(
                "Longitude must be between -180 and 180".to_string(),
            ));
        }
        Ok((lat, lng))
    }
}
