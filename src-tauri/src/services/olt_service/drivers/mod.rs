pub mod hioso;
pub mod mock;

use crate::error::AppResult;
use crate::models::{OltGlobalStats, OltOnuDetail, OltSystemInfo};
use async_trait::async_trait;

/// Trait interface for all OLT device drivers.
/// Each vendor implements this trait to provide monitoring
/// and control capabilities specific to their hardware.
#[async_trait]
pub trait OltDriver: Send + Sync {
    /// Connect to the OLT device.
    async fn connect(
        &mut self,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> AppResult<()>;

    /// Disconnect from the OLT device.
    async fn disconnect(&mut self) -> AppResult<()>;

    /// Get basic device info (model, version, etc).
    async fn get_system_info(&self) -> AppResult<OltSystemInfo>;

    /// Get global statistics: total/online/offline ONU per PON port.
    async fn get_global_stats(&self) -> AppResult<OltGlobalStats>;

    /// Get detailed ONU information for a specific PON port.
    async fn get_pon_onu_details(&self, pon: &str) -> AppResult<Vec<OltOnuDetail>>;

    /// Get signal strength for a specific ONU by MAC address.
    async fn get_onu_signal(&self, mac: &str) -> AppResult<f64>;

    /// Get online/offline status for a specific ONU.
    async fn get_onu_status(&self, mac: &str) -> AppResult<String>;

    /// Reboot an ONU by its identifier.
    async fn reboot_onu(&self, onu_id: &str, onu_name: &str) -> AppResult<bool>;

    /// Update ONU display name (vendor-specific support varies).
    async fn update_onu_name(
        &self,
        onu_id: &str,
        pon: &str,
        new_name: &str,
    ) -> AppResult<()>;
}

use crate::error::AppError;

/// Create the appropriate OLT driver for the given device type.
pub fn create_driver(olt_type: &str) -> AppResult<Box<dyn OltDriver>> {
    match olt_type {
        "hioso_ha7302cst" => Ok(Box::new(hioso::HiosoHa7302cstDriver::new())),
        "mock" => Ok(Box::new(mock::MockOltDriver::new())),
        _ => Err(AppError::Validation(format!(
            "Unsupported OLT type: {}. Supported: hioso_ha7302cst, mock",
            olt_type
        ))),
    }
}
