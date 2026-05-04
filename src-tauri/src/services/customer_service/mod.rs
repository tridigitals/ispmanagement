mod core;
mod dto;
mod helpers;
mod lifecycle;
mod mapper;
mod portal;
mod registration;
mod repository;
mod reschedule;
mod subscriptions;
mod validation;
mod work_orders;

use dto::{AgingBucketRow, InviteSummaryRow, LifecycleStageRow, OverdueInstallationReminderRow};

use crate::db::DbPool;
use crate::error::{AppError, AppResult};
use crate::models::{
    AddCustomerPortalUserRequest, CreateCustomerLocationRequest, CreateCustomerPortalUserRequest,
    CreateCustomerRegistrationInviteRequest, CreateCustomerRequest,
    CreateCustomerSubscriptionRequest, CreateCustomerWithPortalRequest,
    CreateMyCustomerLocationRequest, Customer, CustomerLifecycleAgingBucket,
    CustomerLifecycleObservability, CustomerLifecycleStageMetric, CustomerListItem, CustomerLocation,
    CustomerPortalSubscriptionStats, CustomerPortalUser, CustomerRegistrationInviteCreateResponse,
    CustomerRegistrationInvitePolicy, CustomerRegistrationInviteSummary,
    CustomerRegistrationInviteValidationView, CustomerRegistrationInviteView, CustomerSubscription,
    CustomerSubscriptionOption, CustomerSubscriptionView, CustomerSummary, CustomerUser,
    InstallationWorkOrder, InstallationWorkOrderView, IspPackage, PaginatedResponse,
    PortalCheckoutSubscriptionRequest, TeamMemberWithUser, UpdateCustomerLocationRequest,
    UpdateCustomerRegistrationInvitePolicyRequest, UpdateCustomerRequest,
    UpdateCustomerSubscriptionRequest, WorkOrderRescheduleDecisionRequest,
    WorkOrderRescheduleRequestView,
};
use crate::security::secret::encrypt_secret_for;
use crate::services::subscription_lifecycle::{
    resolve_activation_status, should_disable_pppoe_for_subscription_status, transition_status,
    SubscriptionLifecycleEvent, SubscriptionLifecycleStatus,
};
use crate::services::{
    AuditService, AuthService, NotificationService, PaymentService, PppoeService, UserService,
};
use chrono::{DateTime, Duration, Utc};
use tracing::warn;
use uuid::Uuid;

const PURPOSE_PPPOE: &str = "pppoe_secrets";
const INVITE_DEFAULT_EXPIRES_HOURS: u32 = 24;
const INVITE_DEFAULT_MAX_USES: u32 = 1;
const INVITE_DEFAULT_EXPIRES_KEY: &str = "customer_invite_default_expires_hours";
const INVITE_DEFAULT_MAX_USES_KEY: &str = "customer_invite_default_max_uses";
const CUSTOMER_PACKAGE_INVOICE_PREFIX: &str = "pkgsub:";
const INSTALLATION_SLA_REMINDER_ENABLED_KEY: &str = "installation_sla_reminder_enabled";
const INSTALLATION_SLA_OVERDUE_MINUTES_KEY: &str = "installation_sla_overdue_minutes";
const INSTALLATION_SLA_REMINDER_COOLDOWN_MINUTES_KEY: &str =
    "installation_sla_reminder_cooldown_minutes";
const INSTALLATION_SLA_SCHEDULER_INTERVAL_MINUTES_KEY: &str =
    "installation_sla_scheduler_interval_minutes";
const INSTALLATION_GRACE_HOURS_KEY: &str = "installation_grace_hours";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallationSlaBreachType {
    ScheduledOverdue,
    PendingUnscheduled,
}

#[derive(Clone)]
pub struct CustomerService {
    pool: DbPool,
    auth_service: AuthService,
    audit_service: AuditService,
    notification_service: NotificationService,
    pppoe_service: PppoeService,
    user_service: UserService,
}

impl CustomerService {
    pub fn new(
        pool: DbPool,
        auth_service: AuthService,
        audit_service: AuditService,
        notification_service: NotificationService,
        pppoe_service: PppoeService,
        user_service: UserService,
    ) -> Self {
        Self {
            pool,
            auth_service,
            audit_service,
            notification_service,
            pppoe_service,
            user_service,
        }
    }

    pub fn start_installation_sla_scheduler(&self) {
        let svc = self.clone();
        tokio::spawn(async move {
            tracing::info!("Installation SLA reminder scheduler started.");
            loop {
                if let Err(err) = svc.run_installation_sla_reminders_for_all_tenants().await {
                    tracing::warn!("installation SLA reminder scheduler failed: {}", err);
                }
                let interval_minutes = svc
                    .resolve_installation_sla_scheduler_interval_minutes()
                    .await;
                let sleep_secs = (interval_minutes.max(5) as u64) * 60;
                tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
            }
        });
    }

    // =========================
    // Admin: Customers
    // =========================
}

#[cfg(test)]
mod tests;
