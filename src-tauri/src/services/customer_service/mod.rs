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
    CustomerLifecycleObservability, CustomerLifecycleStageMetric, CustomerLocation,
    CustomerPortalSubscriptionStats, CustomerPortalUser, CustomerRegistrationInviteCreateResponse,
    CustomerRegistrationInvitePolicy, CustomerRegistrationInviteSummary,
    CustomerRegistrationInviteValidationView, CustomerRegistrationInviteView,
    CustomerSubscription, CustomerSubscriptionView, CustomerUser, InstallationWorkOrder,
    InstallationWorkOrderView, IspPackage, PaginatedResponse, PortalCheckoutSubscriptionRequest,
    TeamMemberWithUser, UpdateCustomerLocationRequest,
    UpdateCustomerRegistrationInvitePolicyRequest, UpdateCustomerRequest,
    UpdateCustomerSubscriptionRequest, WorkOrderRescheduleDecisionRequest,
    WorkOrderRescheduleRequestView,
};
use crate::security::secret::encrypt_secret_for;
use crate::services::subscription_lifecycle::{
    resolve_activation_status, transition_status, SubscriptionLifecycleEvent,
    SubscriptionLifecycleStatus,
};
use crate::services::{AuditService, AuthService, NotificationService, PppoeService, UserService};
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
mod tests {
    use super::{CustomerService, InstallationSlaBreachType};
    use crate::error::AppError;
    use crate::models::CustomerLifecycleObservability;
    use chrono::{Duration, Utc};

    fn lifecycle_count(metrics: &CustomerLifecycleObservability, stage: &str) -> i64 {
        metrics
            .lifecycle_funnel
            .iter()
            .find(|item| item.stage == stage)
            .map(|item| item.count)
            .unwrap_or_default()
    }

    fn work_order_count(metrics: &CustomerLifecycleObservability, stage: &str) -> i64 {
        metrics
            .work_order_funnel
            .iter()
            .find(|item| item.stage == stage)
            .map(|item| item.count)
            .unwrap_or_default()
    }

    fn aging_bucket_count(metrics: &CustomerLifecycleObservability, bucket: &str) -> i64 {
        metrics
            .aging_buckets
            .iter()
            .find(|item| item.bucket == bucket)
            .map(|item| item.count)
            .unwrap_or_default()
    }

    #[test]
    fn detect_installation_sla_breach_for_scheduled_work_order() {
        let now = Utc::now();
        let created_at = now - Duration::hours(3);
        let scheduled_at = Some(now - Duration::minutes(121));

        let got = CustomerService::detect_installation_sla_breach(
            "pending",
            scheduled_at,
            created_at,
            now,
            120,
            240,
        );
        assert_eq!(got, Some(InstallationSlaBreachType::ScheduledOverdue));
    }

    #[test]
    fn detect_installation_sla_breach_for_unscheduled_pending_work_order() {
        let now = Utc::now();
        let created_at = now - Duration::minutes(241);

        let got = CustomerService::detect_installation_sla_breach(
            "pending", None, created_at, now, 120, 240,
        );
        assert_eq!(got, Some(InstallationSlaBreachType::PendingUnscheduled));
    }

    #[test]
    fn no_sla_breach_for_completed_or_fresh_work_order() {
        let now = Utc::now();
        let created_at = now - Duration::minutes(20);
        let scheduled_at = Some(now - Duration::minutes(10));

        let completed = CustomerService::detect_installation_sla_breach(
            "completed",
            scheduled_at,
            created_at,
            now,
            120,
            240,
        );
        assert_eq!(completed, None);

        let fresh_pending = CustomerService::detect_installation_sla_breach(
            "pending", None, created_at, now, 120, 240,
        );
        assert_eq!(fresh_pending, None);
    }

    #[test]
    fn elapsed_duration_formatter_is_human_readable() {
        assert_eq!(CustomerService::format_elapsed_duration(45), "45m");
        assert_eq!(CustomerService::format_elapsed_duration(120), "2h");
        assert_eq!(CustomerService::format_elapsed_duration(145), "2h 25m");
        assert_eq!(CustomerService::format_elapsed_duration(26 * 60), "1d 2h");
    }

    #[test]
    fn lifecycle_observability_helpers_extract_counts() {
        let metrics = CustomerLifecycleObservability {
            generated_at: Utc::now(),
            lifecycle_funnel: vec![
                crate::models::CustomerLifecycleStageMetric {
                    stage: "pending_installation".to_string(),
                    count: 3,
                },
                crate::models::CustomerLifecycleStageMetric {
                    stage: "installation_done_awaiting_payment".to_string(),
                    count: 2,
                },
            ],
            work_order_funnel: vec![crate::models::CustomerLifecycleStageMetric {
                stage: "in_progress".to_string(),
                count: 4,
            }],
            aging_buckets: vec![crate::models::CustomerLifecycleAgingBucket {
                bucket: ">7d".to_string(),
                count: 1,
            }],
        };

        assert_eq!(lifecycle_count(&metrics, "pending_installation"), 3);
        assert_eq!(
            lifecycle_count(&metrics, "installation_done_awaiting_payment"),
            2
        );
        assert_eq!(work_order_count(&metrics, "in_progress"), 4);
        assert_eq!(aging_bucket_count(&metrics, ">7d"), 1);
        assert_eq!(lifecycle_count(&metrics, "cancelled"), 0);
    }

    #[test]
    fn normalizers_lock_subscription_and_work_order_status_semantics() {
        assert_eq!(
            CustomerService::normalize_subscription_status(" ACTIVE ").unwrap(),
            "active"
        );
        assert_eq!(
            CustomerService::normalize_subscription_status("pending_installation").unwrap(),
            "pending_installation"
        );

        let sub_err = CustomerService::normalize_subscription_status("draft").unwrap_err();
        assert!(matches!(sub_err, AppError::Validation(_)));

        assert_eq!(
            CustomerService::normalize_work_order_status(" In_Progress ").unwrap(),
            "in_progress"
        );
        assert_eq!(
            CustomerService::normalize_work_order_status("cancelled").unwrap(),
            "cancelled"
        );

        let wo_err = CustomerService::normalize_work_order_status("queued").unwrap_err();
        assert!(matches!(wo_err, AppError::Validation(_)));
    }

    #[test]
    fn invite_policy_and_token_hash_helpers_are_stable() {
        assert_eq!(
            CustomerService::parse_invite_policy_u32(Some(" 48 ".to_string()), 24, 1, 720),
            48
        );
        assert_eq!(
            CustomerService::parse_invite_policy_u32(Some("9999".to_string()), 24, 1, 720),
            720
        );
        assert_eq!(
            CustomerService::parse_invite_policy_u32(Some("not-a-number".to_string()), 24, 1, 720),
            24
        );

        let token = CustomerService::build_registration_invite_token();
        assert_eq!(token.len(), 64);

        let hash = CustomerService::hash_registration_invite_token("invite-token");
        assert_eq!(hash.len(), 64);
        assert_eq!(
            hash,
            CustomerService::hash_registration_invite_token("invite-token")
        );
    }

    #[test]
    fn portal_and_reschedule_helpers_preserve_validation_contracts() {
        let parsed =
            CustomerService::parse_optional_datetime(Some("2026-03-27".to_string())).unwrap();
        assert_eq!(
            parsed.unwrap().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "2026-03-27T00:00:00Z"
        );

        let invalid =
            CustomerService::parse_optional_datetime(Some("27/03/2026 10:00".to_string()))
                .unwrap_err();
        assert!(matches!(invalid, AppError::Validation(_)));

        assert_eq!(
            CustomerService::validate_location_coordinates(Some(-6.2), Some(106.8)).unwrap(),
            (-6.2, 106.8)
        );

        let missing = CustomerService::validate_location_coordinates(None, Some(106.8)).unwrap_err();
        assert!(matches!(missing, AppError::Validation(_)));
    }
}
