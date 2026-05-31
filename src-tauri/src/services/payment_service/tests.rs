use super::{
    auto_suspend_threshold_date, clamp_auto_suspend_fixed_day,
    customer_invoice_notification_action_url, customer_notification_user_ids,
    decide_midtrans_transition, filter_installation_request_user_ids, filter_owner_admin_user_ids,
    is_customer_package_invoice_external_id, is_owner_admin_or_technician_role,
    is_owner_or_admin_role, normalize_auto_suspend_isolation_pool, parse_auto_suspend_mode,
    parse_auto_suspend_pppoe_action, AutoSuspendMode, AutoSuspendPppoeAction,
    BillingCollectionSettings, MidtransTransitionDecision, PaymentService,
};
use crate::services::subscription_lifecycle::{
    resolve_activation_status, SubscriptionLifecycleStatus,
};
use chrono::{Datelike, NaiveDate, TimeZone, Utc};

#[test]
fn owner_admin_role_detection_is_case_insensitive() {
    assert!(is_owner_or_admin_role(Some("Owner")));
    assert!(is_owner_or_admin_role(Some("owner")));
    assert!(is_owner_or_admin_role(Some("ADMIN")));
    assert!(is_owner_or_admin_role(Some(" admin ")));

    assert!(!is_owner_or_admin_role(Some("customer")));
    assert!(!is_owner_or_admin_role(Some("member")));
    assert!(!is_owner_or_admin_role(None));
}

#[test]
fn installation_alert_role_detection_includes_technician() {
    assert!(is_owner_admin_or_technician_role(Some("Owner")));
    assert!(is_owner_admin_or_technician_role(Some("admin")));
    assert!(is_owner_admin_or_technician_role(Some("Technician")));
    assert!(is_owner_admin_or_technician_role(Some(" technician ")));

    assert!(!is_owner_admin_or_technician_role(Some("customer")));
    assert!(!is_owner_admin_or_technician_role(Some("member")));
    assert!(!is_owner_admin_or_technician_role(None));
}

#[test]
fn customer_package_external_id_detection() {
    assert!(is_customer_package_invoice_external_id(Some(
        "pkgsub:abc-123:monthly"
    )));
    assert!(is_customer_package_invoice_external_id(Some(
        "pkgsub:sub-id"
    )));
    assert!(!is_customer_package_invoice_external_id(Some(
        "plan:pro:monthly"
    )));
    assert!(!is_customer_package_invoice_external_id(Some("")));
    assert!(!is_customer_package_invoice_external_id(None));
}

#[test]
fn filters_recipients_to_owner_admin_only() {
    let rows = vec![
        ("u-owner".to_string(), Some("Owner".to_string())),
        ("u-admin".to_string(), Some("admin".to_string())),
        ("u-customer".to_string(), Some("customer".to_string())),
        ("u-member".to_string(), Some("member".to_string())),
        ("u-empty".to_string(), Some("".to_string())),
        ("u-null".to_string(), None),
        ("u-admin".to_string(), Some("Admin".to_string())),
    ];

    let mut got = filter_owner_admin_user_ids(rows);
    got.sort();

    assert_eq!(got, vec!["u-admin".to_string(), "u-owner".to_string()]);
}

#[test]
fn filters_installation_alert_recipients_to_owner_admin_and_technician() {
    let rows = vec![
        ("u-owner".to_string(), Some("Owner".to_string())),
        ("u-admin".to_string(), Some("admin".to_string())),
        ("u-tech".to_string(), Some("Technician".to_string())),
        ("u-customer".to_string(), Some("customer".to_string())),
        ("u-member".to_string(), Some("member".to_string())),
        ("u-tech".to_string(), Some("technician".to_string())),
    ];

    let mut got = filter_installation_request_user_ids(rows, true);
    got.sort();

    assert_eq!(
        got,
        vec![
            "u-admin".to_string(),
            "u-owner".to_string(),
            "u-tech".to_string()
        ]
    );
}

#[test]
fn filters_installation_alert_recipients_to_owner_admin_only_when_technician_hidden() {
    let rows = vec![
        ("u-owner".to_string(), Some("Owner".to_string())),
        ("u-admin".to_string(), Some("admin".to_string())),
        ("u-tech".to_string(), Some("Technician".to_string())),
        ("u-member".to_string(), Some("member".to_string())),
    ];

    let mut got = filter_installation_request_user_ids(rows, false);
    got.sort();

    assert_eq!(got, vec!["u-admin".to_string(), "u-owner".to_string()]);
}

#[test]
fn customer_notification_recipients_do_not_fallback_to_unrelated_tenant_members() {
    let got = customer_notification_user_ids(
        Vec::<String>::new(),
        vec!["tenant-owner".to_string(), "tenant-member".to_string()],
    );

    assert!(got.is_empty());
}

#[test]
fn customer_invoice_notifications_link_directly_to_the_invoice_payment_page() {
    assert_eq!(
        customer_invoice_notification_action_url("inv-123"),
        "/pay/inv-123"
    );
}

#[test]
fn activation_resolution_paid_before_install_keeps_pending_installation() {
    let status = resolve_activation_status(
        SubscriptionLifecycleStatus::PendingInstallation,
        false,
        true,
    )
    .expect("paid-before-install should remain pending_installation");
    assert_eq!(status, SubscriptionLifecycleStatus::PendingInstallation);
}

#[test]
fn activation_resolution_install_done_unpaid_waits_for_payment() {
    let status = resolve_activation_status(
        SubscriptionLifecycleStatus::PendingInstallation,
        true,
        false,
    )
    .expect("install-complete and unpaid should enter grace");
    assert_eq!(status, SubscriptionLifecycleStatus::GraceActive);
}

#[test]
fn activation_resolution_install_done_and_paid_is_active() {
    let status =
        resolve_activation_status(SubscriptionLifecycleStatus::PendingInstallation, true, true)
            .expect("install-complete and paid should become active");
    assert_eq!(status, SubscriptionLifecycleStatus::Active);
}

#[test]
fn activation_resolution_grace_paid_promotes_to_active() {
    let status = resolve_activation_status(SubscriptionLifecycleStatus::GraceActive, true, true)
        .expect("grace subscription should become active when paid");
    assert_eq!(status, SubscriptionLifecycleStatus::Active);
}

#[test]
fn midtrans_transition_decision_prevents_duplicate_or_downgrade_side_effects() {
    assert_eq!(
        decide_midtrans_transition("paid", "paid"),
        MidtransTransitionDecision::SkipDuplicate
    );
    assert_eq!(
        decide_midtrans_transition("paid", "pending"),
        MidtransTransitionDecision::SkipDowngrade
    );
    assert_eq!(
        decide_midtrans_transition("failed", "pending"),
        MidtransTransitionDecision::SkipPendingAfterFailed
    );
    assert_eq!(
        decide_midtrans_transition("pending", "paid"),
        MidtransTransitionDecision::Apply
    );
}

#[test]
fn duitku_create_signature_uses_merchant_order_amount_and_api_key() {
    let signature =
        super::duitku_create_signature("D1234", "INV-20260425-0001", 125000, "secret-key");

    assert_eq!(signature, "54e75c0848f2e7b79eb399f1d43b6f71");
}

#[test]
fn duitku_callback_signature_uses_merchant_amount_order_and_api_key() {
    let signature =
        super::duitku_callback_signature("D1234", "125000", "INV-20260425-0001", "secret-key");

    assert_eq!(signature, "270e6fe9cfa2d7ca96975f8a514df6f7");
}

#[test]
fn duitku_payment_methods_signature_uses_sha256() {
    let signature = super::duitku_payment_methods_signature(
        "D1234",
        125000,
        "2026-04-25 12:00:00",
        "secret-key",
    );

    assert_eq!(
        signature,
        "19d2ced0363cf2ad57cfeac2bbb9a019df7c903004f415addd41ca3f4e8143d0"
    );
}

#[test]
fn selected_duitku_payment_methods_parse_json_array_and_legacy_single_value() {
    assert_eq!(
        super::parse_selected_duitku_payment_methods(Some("[\"VC\",\" BC \",\"\", \"VC\"]")),
        vec!["VC".to_string(), "BC".to_string()]
    );
    assert_eq!(
        super::parse_selected_duitku_payment_methods(Some("M2")),
        vec!["M2".to_string()]
    );
    assert!(super::parse_selected_duitku_payment_methods(None).is_empty());
}

#[test]
fn duitku_transaction_status_code_maps_to_invoice_status() {
    assert_eq!(
        super::duitku_transaction_status_code_to_invoice_status("00"),
        "paid"
    );
    assert_eq!(
        super::duitku_transaction_status_code_to_invoice_status("01"),
        "pending"
    );
    assert_eq!(
        super::duitku_transaction_status_code_to_invoice_status("02"),
        "failed"
    );
    assert_eq!(
        super::duitku_transaction_status_code_to_invoice_status("99"),
        "pending"
    );
}

#[test]
fn duitku_callback_result_code_maps_to_invoice_status() {
    assert_eq!(
        super::duitku_callback_result_code_to_invoice_status("00"),
        "paid"
    );
    assert_eq!(
        super::duitku_callback_result_code_to_invoice_status("01"),
        "failed"
    );
    assert_eq!(
        super::duitku_callback_result_code_to_invoice_status("02"),
        "failed"
    );
    assert_eq!(
        super::duitku_callback_result_code_to_invoice_status("99"),
        "pending"
    );
}

#[test]
fn billing_collection_settings_default_to_grace_period_mode() {
    let defaults = BillingCollectionSettings::default();

    assert_eq!(defaults.auto_suspend_mode, AutoSuspendMode::GracePeriod);
    assert_eq!(defaults.auto_suspend_fixed_day, 1);
    assert_eq!(
        defaults.auto_suspend_pppoe_action,
        AutoSuspendPppoeAction::DisableSecret
    );
    assert_eq!(defaults.auto_suspend_isolation_pool, None);
}

#[test]
fn parse_auto_suspend_mode_accepts_known_values_and_falls_back() {
    assert_eq!(
        parse_auto_suspend_mode(Some("fixed_day".to_string()), AutoSuspendMode::GracePeriod),
        AutoSuspendMode::FixedDay
    );
    assert_eq!(
        parse_auto_suspend_mode(
            Some(" grace_period ".to_string()),
            AutoSuspendMode::FixedDay
        ),
        AutoSuspendMode::GracePeriod
    );
    assert_eq!(
        parse_auto_suspend_mode(Some("unexpected".to_string()), AutoSuspendMode::GracePeriod),
        AutoSuspendMode::GracePeriod
    );
}

#[test]
fn parse_auto_suspend_pppoe_action_accepts_known_values_and_falls_back() {
    assert_eq!(
        parse_auto_suspend_pppoe_action(
            Some("move_to_isolation_pool".to_string()),
            AutoSuspendPppoeAction::DisableSecret,
        ),
        AutoSuspendPppoeAction::MoveToIsolationPool
    );
    assert_eq!(
        parse_auto_suspend_pppoe_action(
            Some(" disable_secret ".to_string()),
            AutoSuspendPppoeAction::MoveToIsolationPool,
        ),
        AutoSuspendPppoeAction::DisableSecret
    );
    assert_eq!(
        parse_auto_suspend_pppoe_action(
            Some("unexpected".to_string()),
            AutoSuspendPppoeAction::DisableSecret,
        ),
        AutoSuspendPppoeAction::DisableSecret
    );
}

#[test]
fn normalize_auto_suspend_isolation_pool_trims_and_rejects_empty_values() {
    assert_eq!(
        normalize_auto_suspend_isolation_pool(Some(" pool-isolir ".to_string())),
        Some("pool-isolir".to_string())
    );
    assert_eq!(
        normalize_auto_suspend_isolation_pool(Some("   ".to_string())),
        None
    );
    assert_eq!(normalize_auto_suspend_isolation_pool(None), None);
}

#[test]
fn clamp_auto_suspend_fixed_day_limits_value_to_safe_month_days() {
    assert_eq!(clamp_auto_suspend_fixed_day(0), 1);
    assert_eq!(clamp_auto_suspend_fixed_day(1), 1);
    assert_eq!(clamp_auto_suspend_fixed_day(15), 15);
    assert_eq!(clamp_auto_suspend_fixed_day(31), 28);
}

#[test]
fn grace_period_threshold_is_due_date_plus_grace_days() {
    let due_date = NaiveDate::from_ymd_opt(2026, 5, 8).expect("valid due date");

    assert_eq!(
        auto_suspend_threshold_date(due_date, AutoSuspendMode::GracePeriod, 3, 10),
        NaiveDate::from_ymd_opt(2026, 5, 11).expect("valid threshold date")
    );
}

#[test]
fn fixed_day_threshold_uses_same_month_when_due_before_or_on_fixed_day() {
    let due_date = NaiveDate::from_ymd_opt(2026, 5, 8).expect("valid due date");

    assert_eq!(
        auto_suspend_threshold_date(due_date, AutoSuspendMode::FixedDay, 3, 20),
        NaiveDate::from_ymd_opt(2026, 5, 20).expect("valid threshold date")
    );
}

#[test]
fn fixed_day_threshold_rolls_to_next_month_when_due_after_fixed_day() {
    let due_date = NaiveDate::from_ymd_opt(2026, 5, 21).expect("valid due date");

    assert_eq!(
        auto_suspend_threshold_date(due_date, AutoSuspendMode::FixedDay, 3, 20),
        NaiveDate::from_ymd_opt(2026, 6, 20).expect("valid threshold date")
    );
}

// ==================== PRO-RATA BILLING TESTS ====================

#[test]
fn pro_rata_full_period_returns_full_amount() {
    let start = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let change = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
    let result = PaymentService::calculate_pro_rata_amount(100_000.0, start, end, change);
    assert_eq!(result, 100_000.0);
}

#[test]
fn pro_rata_mid_cycle_returns_proportional_amount() {
    // June has 30 days. Change on June 16 = 15 remaining days / 30 total = 50%
    let start = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let change = Utc.with_ymd_and_hms(2026, 6, 16, 0, 0, 0).unwrap();
    let result = PaymentService::calculate_pro_rata_amount(100_000.0, start, end, change);
    assert_eq!(result, 50_000.0);
}

#[test]
fn pro_rata_end_of_cycle_returns_zero() {
    let start = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let change = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let result = PaymentService::calculate_pro_rata_amount(100_000.0, start, end, change);
    assert_eq!(result, 0.0);
}

#[test]
fn pro_rata_one_day_before_end() {
    // June 30 = 1 remaining day / 30 total
    let start = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let change = Utc.with_ymd_and_hms(2026, 6, 30, 0, 0, 0).unwrap();
    let result = PaymentService::calculate_pro_rata_amount(300_000.0, start, end, change);
    // 1/30 * 300000 = 10000
    assert_eq!(result, 10_000.0);
}

#[test]
fn pro_rata_before_period_start_returns_full_amount() {
    let start = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let change = Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap();
    let result = PaymentService::calculate_pro_rata_amount(100_000.0, start, end, change);
    assert_eq!(result, 100_000.0);
}

#[test]
fn pro_rata_after_period_end_returns_zero() {
    let start = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    let change = Utc.with_ymd_and_hms(2026, 7, 15, 0, 0, 0).unwrap();
    let result = PaymentService::calculate_pro_rata_amount(100_000.0, start, end, change);
    assert_eq!(result, 0.0);
}

#[test]
fn current_billing_period_monthly() {
    let anchor = Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 6, 20, 0, 0, 0).unwrap();
    let (start, end) = PaymentService::current_billing_period("monthly", anchor, now).unwrap();
    // Should be June 15 - July 15
    assert_eq!(start.month(), 6);
    assert_eq!(start.day(), 15);
    assert_eq!(end.month(), 7);
    assert_eq!(end.day(), 15);
}

#[test]
fn current_billing_period_yearly() {
    let anchor = Utc.with_ymd_and_hms(2025, 3, 1, 0, 0, 0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 6, 20, 0, 0, 0).unwrap();
    let (start, end) = PaymentService::current_billing_period("yearly", anchor, now).unwrap();
    // Should be March 1 2026 - March 1 2027
    assert_eq!(start.year(), 2026);
    assert_eq!(start.month(), 3);
    assert_eq!(end.year(), 2027);
    assert_eq!(end.month(), 3);
}
