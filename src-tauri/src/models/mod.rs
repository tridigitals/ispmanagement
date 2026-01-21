//! Models module

pub mod role;
pub mod tenant;
pub mod user;
pub mod settings;
pub mod audit_log;
pub mod plan;
pub mod file;
pub mod invoice;
pub mod notification;

pub use role::*;
pub use tenant::*;
pub use user::*;
pub use settings::*;
pub use audit_log::*;
pub use plan::*;
pub use file::*;
pub use invoice::*;
pub use notification::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}
