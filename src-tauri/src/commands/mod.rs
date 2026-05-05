//! Commands module

pub mod announcements;
pub mod announcements_support_common;
pub mod audit;
pub mod auth;
pub mod backup;
pub mod customer_communication;
pub mod customers;
pub mod dhcp_static;
pub mod email_outbox;
pub mod install;
pub mod isp_packages;
pub mod message_templates;
pub mod mikrotik;
pub mod mixradius_import;
pub mod network_mapping;
pub mod notifications;
pub mod payment;
pub mod plans;
pub mod pppoe;
pub mod public;
pub mod roles;
pub mod settings;
pub mod storage;
pub mod superadmin;
pub mod support;
pub mod system;
pub mod team;
pub mod tenant;
pub mod users;
pub mod whatsapp;
pub mod work_orders;

#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub use announcements::*;
pub use auth::*;
pub use backup::*;
pub use customer_communication::*;
pub use customers::*;
pub use dhcp_static::*;
pub use email_outbox::*;
pub use install::*;
pub use isp_packages::*;
pub use message_templates::*;
pub use mikrotik::*;
pub use mixradius_import::*;
pub use network_mapping::*;
pub use notifications::*;
pub use payment::*;
pub use plans::*;
pub use pppoe::*;
pub use public::*;
pub use roles::*;
pub use settings::*;
pub use storage::*;
pub use superadmin::*;
pub use support::*;
pub use system::*;
pub use team::*;
pub use tenant::*;
pub use users::*;
pub use whatsapp::*;
pub use work_orders::*;
