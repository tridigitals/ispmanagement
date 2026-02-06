//! Models module

pub mod audit_log;
pub mod file;
pub mod invoice;
pub mod notification;
pub mod plan;
pub mod role;
pub mod settings;
pub mod support;
pub mod tenant;
pub mod trusted_device;
pub mod user;

pub use audit_log::*;
pub use file::*;
pub use invoice::*;
pub use notification::*;
pub use plan::*;
pub use role::*;
pub use settings::*;
pub use support::*;
pub use tenant::*;
pub use trusted_device::*;
pub use user::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}
