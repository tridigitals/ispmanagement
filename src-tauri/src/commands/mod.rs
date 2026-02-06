//! Commands module

pub mod audit;
pub mod auth;
pub mod backup;
pub mod announcements;
pub mod install;
pub mod notifications;
pub mod payment;
pub mod plans;
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

pub use auth::*;
pub use backup::*;
pub use announcements::*;
pub use install::*;
pub use notifications::*;
pub use payment::*;
pub use plans::*;
pub use roles::*;
pub use settings::*;
pub use storage::*;
pub use superadmin::*;
pub use support::*;
pub use system::*;
pub use team::*;
pub use tenant::*;
pub use users::*;
