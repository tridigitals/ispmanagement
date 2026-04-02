use super::{
    decide_midtrans_transition, filter_installation_request_user_ids, filter_owner_admin_user_ids,
    is_customer_package_invoice_external_id, is_owner_admin_or_technician_role,
    is_owner_or_admin_role, MidtransTransitionDecision,
};
use crate::services::subscription_lifecycle::{
    resolve_activation_status, SubscriptionLifecycleStatus,
};

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

    let mut got = filter_installation_request_user_ids(rows);
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
    .expect("install-complete and unpaid should wait for payment");
    assert_eq!(
        status,
        SubscriptionLifecycleStatus::InstallationDoneAwaitingPayment
    );
}

#[test]
fn activation_resolution_install_done_and_paid_is_active() {
    let status =
        resolve_activation_status(SubscriptionLifecycleStatus::PendingInstallation, true, true)
            .expect("install-complete and paid should become active");
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
