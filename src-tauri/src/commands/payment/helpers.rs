use crate::models::Invoice;
use chrono::{DateTime, Utc};

pub(super) fn is_customer_package_invoice(invoice: &Invoice) -> bool {
    invoice
        .external_id
        .as_deref()
        .map(|v| v.starts_with("pkgsub:"))
        .unwrap_or(false)
}

pub(super) fn parse_datetime_opt(
    input: Option<String>,
    field: &str,
) -> Result<Option<DateTime<Utc>>, String> {
    let Some(raw) = input
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    else {
        return Ok(None);
    };

    chrono::DateTime::parse_from_rfc3339(&raw)
        .map(|dt| Some(dt.with_timezone(&Utc)))
        .map_err(|_| format!("{field} must be ISO-8601 datetime (RFC3339)"))
}
