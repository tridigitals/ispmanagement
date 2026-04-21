use super::Invoice;

pub(super) const CUSTOMER_PACKAGE_INVOICE_PREFIX: &str = "pkgsub:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MidtransTransitionDecision {
    Apply,
    SkipDuplicate,
    SkipDowngrade,
    SkipPendingAfterFailed,
}

pub(super) fn decide_midtrans_transition(
    current_status: &str,
    incoming_status: &str,
) -> MidtransTransitionDecision {
    if current_status == incoming_status {
        return MidtransTransitionDecision::SkipDuplicate;
    }
    if current_status == "paid" && incoming_status != "paid" {
        return MidtransTransitionDecision::SkipDowngrade;
    }
    if current_status == "failed" && incoming_status == "pending" {
        return MidtransTransitionDecision::SkipPendingAfterFailed;
    }
    MidtransTransitionDecision::Apply
}

pub(super) fn is_customer_package_invoice_external_id(external_id: Option<&str>) -> bool {
    external_id
        .map(|v| v.starts_with(CUSTOMER_PACKAGE_INVOICE_PREFIX))
        .unwrap_or(false)
}

pub(super) fn parse_customer_subscription_id(external_id: Option<&str>) -> Option<String> {
    let rest = external_id?.strip_prefix(CUSTOMER_PACKAGE_INVOICE_PREFIX)?;
    let id = rest.split(':').next()?.trim();
    if id.is_empty() {
        return None;
    }
    Some(id.to_string())
}

pub(super) fn reminder_code_for_day_offset(day_offset: i64) -> String {
    if day_offset >= 0 {
        format!("H+{}", day_offset)
    } else {
        format!("H{}", day_offset)
    }
}

pub(super) fn customer_notification_user_ids(
    customer_user_ids: Vec<String>,
    _tenant_member_user_ids: Vec<String>,
) -> Vec<String> {
    customer_user_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn customer_invoice_notification_action_url(invoice_id: &str) -> String {
    format!("/pay/{}", invoice_id)
}

fn json_number(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_i64().map(|n| n as f64))
            .or_else(|| v.as_u64().map(|n| n as f64))
            .or_else(|| v.as_str().and_then(|s| s.trim().parse::<f64>().ok()))
    })
}

pub(super) fn assignment_health_score(status: &str, health_json: &serde_json::Value) -> f64 {
    if let Some(score) = json_number(health_json, "score") {
        return score.clamp(0.0, 100.0);
    }
    if let Some(score) = json_number(health_json, "health_score") {
        return score.clamp(0.0, 100.0);
    }
    match status {
        "active" => 85.0,
        "maintenance" => 60.0,
        _ => 40.0,
    }
}

pub(super) fn assignment_capacity_score(
    capacity_json: &serde_json::Value,
    avg_link_utilization_pct: Option<f64>,
) -> f64 {
    if let Some(free_pct) = json_number(capacity_json, "free_pct") {
        return free_pct.clamp(0.0, 100.0);
    }
    if let Some(util_pct) = json_number(capacity_json, "utilization_pct") {
        return (100.0 - util_pct).clamp(0.0, 100.0);
    }
    if let (Some(avail), Some(total)) = (
        json_number(capacity_json, "available_mbps"),
        json_number(capacity_json, "total_mbps"),
    ) {
        if total > 0.0 {
            return ((avail / total) * 100.0).clamp(0.0, 100.0);
        }
    }
    if let Some(util) = avg_link_utilization_pct {
        return (100.0 - util).clamp(0.0, 100.0);
    }
    60.0
}

pub(super) fn assignment_distance_score(distance_m: Option<f64>) -> Option<f64> {
    distance_m.map(|distance| {
        let normalized = (distance / 50_000.0).clamp(0.0, 1.0);
        (100.0 - (normalized * 100.0)).clamp(0.0, 100.0)
    })
}

pub(super) fn is_manual_payment_invoice(invoice: &Invoice) -> bool {
    let method = invoice
        .payment_method
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    invoice.status == "verification_pending"
        || invoice.proof_attachment.is_some()
        || method.contains("bank")
        || method.contains("manual")
}
