//! Services module

pub mod alert_service;
// Auth service was split into `auth_service/` focused units; keep facade export stable.
pub mod auth_service;
pub mod cache;
pub mod email_outbox_service;
pub mod email_service;
pub mod managed_radius_service;
pub mod message_template_renderer;
pub mod message_template_service;
pub mod metrics_service;
pub mod mixradius_import_executor;
pub mod mixradius_import_mapper;
pub mod mixradius_import_service;
pub mod mixradius_sql_parser;
pub mod network_mapping_service;
pub mod rate_limiter;
pub mod role_service;
pub mod settings_service;
pub mod team_service;
pub mod unsubscribe_token;
pub mod user_service;
pub mod whatsapp_gateway_service;

pub use auth_service::*;
pub mod announcement_service;
pub mod audit_service;
pub mod backup;
pub mod customer_service;
pub mod isp_package_service;
pub mod mikrotik_service;
pub mod notification_service;
pub mod payment_service;
pub mod plan_service;
pub mod pppoe_service;
pub mod storage_service;
pub mod subscription_lifecycle;
pub mod system_service;

pub use alert_service::AlertService;
pub use announcement_service::AnnouncementScheduler;
pub use audit_service::AuditService;
pub use auth_service::AuthService;
pub use backup::BackupService;
pub use customer_service::CustomerService;
pub use email_outbox_service::EmailOutboxService;
pub use email_service::EmailService;
pub use isp_package_service::IspPackageService;
pub use managed_radius_service::ManagedRadiusService;
pub use message_template_service::MessageTemplateService;
pub use mikrotik_service::MikrotikService;
pub use mixradius_import_service::MixradiusImportService;
pub use network_mapping_service::NetworkMappingService;
pub use notification_service::NotificationService;
pub use payment_service::{
    BillingCollectionRunResult, BulkGenerateInvoicesResult, DuitkuPaymentMethod, PaymentService,
};
pub use plan_service::PlanService;
pub use pppoe_service::PppoeService;
pub use role_service::RoleService;
pub use settings_service::SettingsService;
pub use storage_service::StorageService;
pub use system_service::SystemService;
pub use team_service::TeamService;
pub use unsubscribe_token::*;
pub use user_service::UserService;
pub use whatsapp_gateway_service::*;

#[cfg(test)]
mod mixradius_import_executor_tests;
#[cfg(test)]
mod mixradius_import_mapper_tests;
#[cfg(test)]
mod mixradius_sql_parser_tests;
