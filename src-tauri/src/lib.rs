//! SaaS Boilerplate - Main Library Entry Point
//! 
//! This is the core library for the Tauri application.
//! It wires together all modules: database, services, and commands.

mod commands;
mod db;
mod error;
mod models;
mod services;

use commands::*;
use db::connection::{init_db, seed_defaults};
use services::{AuthService, EmailService, SettingsService, UserService};
use tauri::Manager;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// ... (imports remain the same)

// ...

                // Create services
                let settings_service = SettingsService::new(pool.clone());
                let email_service = EmailService::new(settings_service.clone());
                let auth_service = AuthService::new(pool.clone(), jwt_secret, email_service.clone());
                let user_service = UserService::new(pool.clone());

                // Manage state
                app_handle.manage(auth_service);
                app_handle.manage(user_service);
                app_handle.manage(settings_service);
                app_handle.manage(email_service); // Also manage email service for direct use

                info!("Services initialized successfully");
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth commands
            register,
            login,
            logout,
            change_password,
            get_current_user,
            validate_token,
            verify_email,
            forgot_password,
            reset_password,
            // User commands
            list_users,
            get_user,
            create_user,
            update_user,
            delete_user,
            // Settings commands
            get_all_settings,
            get_auth_settings,
            get_setting,
            get_setting_value,
            upsert_setting,
            delete_setting,
            upload_logo,
            get_logo,
            // Install commands
            is_installed,
            install_app,
            // Email commands
            send_test_email,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
