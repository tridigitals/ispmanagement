use super::{
    customer_invoice_notification_action_url, customer_notification_user_ids,
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
