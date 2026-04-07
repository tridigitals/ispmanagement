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

    let got =
        CustomerService::detect_installation_sla_breach("pending", None, created_at, now, 120, 240);
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

    let fresh_pending =
        CustomerService::detect_installation_sla_breach("pending", None, created_at, now, 120, 240);
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
            crate::models::CustomerLifecycleStageMetric {
                stage: "grace_active".to_string(),
                count: 1,
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
    assert_eq!(lifecycle_count(&metrics, "grace_active"), 1);
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
    assert_eq!(
        CustomerService::normalize_subscription_status("grace_active").unwrap(),
        "grace_active"
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
    let parsed = CustomerService::parse_optional_datetime(Some("2026-03-27".to_string())).unwrap();
    assert_eq!(
        parsed.unwrap().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "2026-03-27T00:00:00Z"
    );

    let invalid =
        CustomerService::parse_optional_datetime(Some("27/03/2026 10:00".to_string())).unwrap_err();
    assert!(matches!(invalid, AppError::Validation(_)));

    assert_eq!(
        CustomerService::validate_location_coordinates(Some(-6.2), Some(106.8)).unwrap(),
        (-6.2, 106.8)
    );

    let missing = CustomerService::validate_location_coordinates(None, Some(106.8)).unwrap_err();
    assert!(matches!(missing, AppError::Validation(_)));
}

#[test]
fn portal_subscription_select_fragments_include_grace_columns() {
    let view_select = CustomerService::portal_subscription_view_select_columns("cs", "cs.price");
    assert!(view_select.contains("cs.grace_started_at"));
    assert!(view_select.contains("cs.grace_until"));

    let entity_select = CustomerService::portal_subscription_select_columns("cs", "cs.price");
    assert!(entity_select.contains("cs.grace_started_at"));
    assert!(entity_select.contains("cs.grace_until"));
}

#[test]
fn installation_completion_auto_invoice_only_runs_for_unpaid_grace_activation() {
    assert!(CustomerService::should_auto_create_first_invoice_on_completion(
        crate::services::subscription_lifecycle::SubscriptionLifecycleStatus::GraceActive,
        false,
    ));

    assert!(!CustomerService::should_auto_create_first_invoice_on_completion(
        crate::services::subscription_lifecycle::SubscriptionLifecycleStatus::Active,
        false,
    ));

    assert!(!CustomerService::should_auto_create_first_invoice_on_completion(
        crate::services::subscription_lifecycle::SubscriptionLifecycleStatus::GraceActive,
        true,
    ));
}
