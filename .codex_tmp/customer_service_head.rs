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
use sha2::{Digest, Sha256};
use std::collections::HashSet;
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
