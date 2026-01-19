//! Commands module

pub mod auth;
pub mod users;
pub mod settings;
pub mod install;
pub mod superadmin;
pub mod roles;
pub mod team;
pub mod audit;
pub mod system;
pub mod plans;
pub mod storage;
pub mod payment;
pub mod tenant;

#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub use auth::*;
pub use users::*;
pub use settings::*;
pub use install::*;
pub use superadmin::*;
pub use roles::*;
pub use team::*;
pub use system::*;
pub use plans::*;
pub use storage::*;
pub use payment::*;
pub use tenant::*;
