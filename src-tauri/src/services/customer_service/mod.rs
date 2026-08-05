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
    AddCustomerPortalUserRequest, BackofficeInstallationOrderResponse,
    CreateBackofficeInstallationOrderRequest, CreateCustomerLocationRequest,
    CreateCustomerPortalUserRequest, CreateCustomerRegistrationInviteRequest,
    CreateCustomerRequest, CreateCustomerSubscriptionRequest, CreateCustomerWithPortalRequest,
    CreateMyCustomerLocationRequest, Customer, CustomerLifecycleAgingBucket,
    CustomerLifecycleObservability, CustomerLifecycleStageMetric, CustomerListItem,
    CustomerLocation, CustomerPortalSubscriptionStats, CustomerPortalUser,
    CustomerRegistrationInviteCreateResponse, CustomerRegistrationInvitePolicy,
    CustomerRegistrationInviteSummary, CustomerRegistrationInviteValidationView,
    CustomerRegistrationInviteView, CustomerServiceLifecycleIssue,
    CustomerServiceLifecycleRepairResult, CustomerServiceLifecycleReport, CustomerSubscription,
    CustomerSubscriptionOption, CustomerSubscriptionView, CustomerSummary, CustomerUser,
    InstallationWorkOrder, InstallationWorkOrderView, IspPackage, NetworkAsset, PaginatedResponse,
    PortalCheckoutSubscriptionRequest, RepairCustomerServiceLifecycleRequest,
    ResetCustomerPortalPasswordResponse, TeamMemberWithUser,
    UpdateCustomerLocationRequest, UpdateCustomerRegistrationInvitePolicyRequest,
    UpdateCustomerRequest, UpdateCustomerSubscriptionRequest, WorkOrderRescheduleDecisionRequest,
    WorkOrderRescheduleRequestView,
};
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
const PURPOSE_CUSTOMER_INVITE: &str = "customer_invite_tokens";
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
    ws_hub: Option<std::sync::Arc<crate::http::WsHub>>,
}

impl CustomerService {
    pub fn new(
        pool: DbPool,
        auth_service: AuthService,
        audit_service: AuditService,
        notification_service: NotificationService,
        pppoe_service: PppoeService,
        user_service: UserService,
        ws_hub: Option<std::sync::Arc<crate::http::WsHub>>,
    ) -> Self {
        Self {
            pool,
            auth_service,
            audit_service,
            notification_service,
            pppoe_service,
            user_service,
            ws_hub,
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

fn validate_installation_asset_selection(
    terminal_asset_type: &str,
    terminal_asset_status: &str,
    terminal_customer_id: Option<&str>,
    work_order_customer_id: &str,
    parent_asset_type: Option<&str>,
    parent_asset_status: Option<&str>,
) -> AppResult<()> {
    if !matches!(terminal_asset_type, "ont" | "onu") {
        return Err(AppError::Validation(
            "Terminal asset must be ONT or ONU".into(),
        ));
    }
    if matches!(terminal_asset_status, "faulty" | "retired") {
        return Err(AppError::Validation(
            "Selected terminal asset is not usable".into(),
        ));
    }
    if terminal_customer_id.is_some() && terminal_customer_id != Some(work_order_customer_id) {
        return Err(AppError::Conflict(
            "Selected terminal asset is already assigned to another customer".into(),
        ));
    }

    if let Some(parent_asset_type) = parent_asset_type {
        if !matches!(
            parent_asset_type,
            "olt" | "odc" | "odp" | "splitter" | "fat" | "nap" | "odf"
        ) {
            return Err(AppError::Validation(
                "Parent asset must be an FTTH upstream asset".into(),
            ));
        }
        if matches!(parent_asset_status, Some("faulty" | "retired")) {
            return Err(AppError::Validation(
                "Selected parent asset is not usable".into(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
