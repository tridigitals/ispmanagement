//! Models module

pub mod announcements;
pub mod audit_log;
pub mod customer;
pub mod customer_communication;
pub mod dhcp_static;
pub mod email_outbox;
pub mod file;
pub mod invoice;
pub mod isp_packages;
pub mod message_template;
pub mod mikrotik;
pub mod mixradius_import;
pub mod network_asset;
pub mod network_mapping;
pub mod notification;
pub mod plan;
pub mod pppoe;
pub mod radius;
pub mod role;
pub mod settings;
pub mod support;
pub mod tenant;
pub mod trusted_device;
pub mod user;
pub mod user_address;
pub mod user_device;
pub mod whatsapp;

pub use announcements::*;
pub use audit_log::*;
pub use customer::*;
pub use customer_communication::*;
pub use dhcp_static::*;
pub use email_outbox::*;
pub use file::*;
pub use invoice::*;
pub use isp_packages::*;
pub use message_template::*;
pub use mikrotik::*;
pub use mixradius_import::*;
pub use network_asset::*;
pub use network_mapping::*;
pub use notification::*;
pub use plan::*;
pub use pppoe::*;
pub use radius::*;
pub use role::*;
pub use settings::*;
pub use support::*;
pub use tenant::*;
pub use trusted_device::*;
pub use user::*;
pub use user_address::*;
pub use user_device::*;
pub use whatsapp::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}
