//! Models module

pub mod announcements;
pub mod audit_log;
pub mod email_outbox;
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
pub mod user_address;
pub mod mikrotik;

pub use announcements::*;
pub use audit_log::*;
pub use email_outbox::*;
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
pub use user_address::*;
pub use mikrotik::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}
