//! Commands module

pub mod announcements;
pub mod audit;
pub mod auth;
pub mod backup;
pub mod customers;
pub mod email_outbox;
pub mod install;
pub mod isp_packages;
pub mod mikrotik;
pub mod notifications;
pub mod payment;
pub mod plans;
pub mod pppoe;
pub mod roles;
pub mod settings;
pub mod storage;
pub mod superadmin;
pub mod support;
pub mod system;
pub mod team;
pub mod tenant;
pub mod users;

#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub use announcements::*;
pub use auth::*;
pub use backup::*;
pub use customers::*;
pub use email_outbox::*;
pub use install::*;
pub use isp_packages::*;
pub use mikrotik::*;
pub use notifications::*;
pub use payment::*;
pub use plans::*;
pub use pppoe::*;
pub use roles::*;
pub use settings::*;
pub use storage::*;
pub use superadmin::*;
pub use support::*;
pub use system::*;
pub use team::*;
pub use tenant::*;
pub use users::*;
