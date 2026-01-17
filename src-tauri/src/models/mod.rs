//! Models module

pub mod settings;
pub mod user;
pub mod tenant;
pub mod role;

pub use settings::*;
pub use user::*;
pub use tenant::*;
pub use role::*;
pub mod audit_log;
pub use audit_log::*;
pub use audit_log::{AuditLog, AuditLogResponse};
pub mod plan;
pub use plan::*;
pub mod file;
pub use file::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}
