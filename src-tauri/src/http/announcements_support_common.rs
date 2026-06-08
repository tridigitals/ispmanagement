use crate::models::Announcement;

pub(crate) fn strip_html_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn ann_snapshot_json(ann: &Announcement) -> serde_json::Value {
    serde_json::json!({
        "id": ann.id,
        "tenant_id": ann.tenant_id,
        "created_by": ann.created_by,
        "cover_file_id": ann.cover_file_id,
        "title": ann.title,
        "severity": ann.severity,
        "audience": ann.audience,
        "mode": ann.mode,
        "format": ann.format,
        "deliver_in_app": ann.deliver_in_app,
        "deliver_email": ann.deliver_email,
        "deliver_email_force": ann.deliver_email_force,
        "starts_at": ann.starts_at.to_rfc3339(),
        "ends_at": ann.ends_at.map(|d| d.to_rfc3339()),
        "notified_at": ann.notified_at.map(|d| d.to_rfc3339()),
        "created_at": ann.created_at.to_rfc3339(),
        "updated_at": ann.updated_at.to_rfc3339(),
    })
}

pub(crate) fn ann_changed_fields(before: &Announcement, after: &Announcement) -> Vec<&'static str> {
    let mut out = Vec::new();
    if before.cover_file_id != after.cover_file_id {
        out.push("cover_file_id");
    }
    if before.title != after.title {
        out.push("title");
    }
    if before.body != after.body {
        out.push("body");
    }
    if before.severity != after.severity {
        out.push("severity");
    }
    if before.audience != after.audience {
        out.push("audience");
    }
    if before.mode != after.mode {
        out.push("mode");
    }
    if before.format != after.format {
        out.push("format");
    }
    if before.deliver_in_app != after.deliver_in_app {
        out.push("deliver_in_app");
    }
    if before.deliver_email != after.deliver_email {
        out.push("deliver_email");
    }
    if before.deliver_email_force != after.deliver_email_force {
        out.push("deliver_email_force");
    }
    if before.starts_at != after.starts_at {
        out.push("starts_at");
    }
    if before.ends_at != after.ends_at {
        out.push("ends_at");
    }
    out
}

pub(crate) fn norm_severity(s: Option<String>) -> String {
    match s.as_deref() {
        Some("info") | Some("success") | Some("warning") | Some("error") => s.unwrap(),
        _ => "info".to_string(),
    }
}

pub(crate) fn norm_audience(a: Option<String>) -> String {
    match a.as_deref() {
        Some("all") | Some("admins") | Some("customers") | Some("active_subscribers")
        | Some("suspended_subscribers") => a.unwrap(),
        _ => "all".to_string(),
    }
}

pub(crate) fn norm_mode(m: Option<String>) -> String {
    match m.as_deref() {
        Some("post") | Some("banner") => m.unwrap(),
        _ => "post".to_string(),
    }
}

pub(crate) fn norm_format(f: Option<String>) -> String {
    match f.as_deref() {
        Some("plain") | Some("markdown") | Some("html") => f.unwrap(),
        _ => "plain".to_string(),
    }
}

#[cfg(feature = "postgres")]
pub(crate) async fn tenant_admin_user_ids(
    pool: &sqlx::Pool<sqlx::Postgres>,
    tenant_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT DISTINCT user_id FROM tenant_members WHERE tenant_id = $1")
        .bind(tenant_id)
        .fetch_all(pool)
        .await
}

#[cfg(feature = "postgres")]
pub(crate) async fn is_internal_tenant_member(
    pool: &sqlx::Pool<sqlx::Postgres>,
    tenant_id: &str,
    user_id: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tenant_members WHERE tenant_id = $1 AND user_id = $2)",
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

#[cfg(feature = "postgres")]
pub(crate) async fn tenant_user_ids(
    pool: &sqlx::Pool<sqlx::Postgres>,
    tenant_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT DISTINCT user_id FROM tenant_members WHERE tenant_id = $1")
        .bind(tenant_id)
        .fetch_all(pool)
        .await
}

#[cfg(feature = "postgres")]
pub(crate) async fn support_admin_user_ids(
    pool: &sqlx::Pool<sqlx::Postgres>,
    tenant_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT DISTINCT tm.user_id
        FROM tenant_members tm
        JOIN role_permissions rp ON rp.role_id = tm.role_id
        WHERE tm.tenant_id = $1
          AND tm.role_id IS NOT NULL
          AND rp.permission_id = ANY($2)
    "#,
    )
    .bind(tenant_id)
    .bind(["support:read_all", "support:reply"])
    .fetch_all(pool)
    .await
}

/// Portal users linked to any customer in the tenant.
#[cfg(feature = "postgres")]
pub(crate) async fn customer_portal_user_ids(
    pool: &sqlx::Pool<sqlx::Postgres>,
    tenant_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT DISTINCT cu.user_id
        FROM customer_users cu
        JOIN customers c ON c.id = cu.customer_id
        WHERE c.tenant_id = $1 AND c.is_active = true
    "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
}

/// Portal users linked to customers with at least one active subscription.
#[cfg(feature = "postgres")]
pub(crate) async fn active_subscriber_portal_user_ids(
    pool: &sqlx::Pool<sqlx::Postgres>,
    tenant_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT DISTINCT cu.user_id
        FROM customer_users cu
        JOIN customer_subscriptions cs ON cs.customer_id = cu.customer_id AND cs.tenant_id = cu.tenant_id
        WHERE cu.tenant_id = $1 AND cs.status = 'active'
    "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
}

/// Portal users linked to customers with at least one suspended subscription.
#[cfg(feature = "postgres")]
pub(crate) async fn suspended_subscriber_portal_user_ids(
    pool: &sqlx::Pool<sqlx::Postgres>,
    tenant_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT DISTINCT cu.user_id
        FROM customer_users cu
        JOIN customer_subscriptions cs ON cs.customer_id = cu.customer_id AND cs.tenant_id = cu.tenant_id
        WHERE cu.tenant_id = $1 AND cs.status = 'suspended'
    "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
}

/// Portal users linked to customers subscribed to a specific package.
#[cfg(feature = "postgres")]
pub(crate) async fn package_subscriber_portal_user_ids(
    pool: &sqlx::Pool<sqlx::Postgres>,
    tenant_id: &str,
    package_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT DISTINCT cu.user_id
        FROM customer_users cu
        JOIN customer_subscriptions cs ON cs.customer_id = cu.customer_id AND cs.tenant_id = cu.tenant_id
        WHERE cu.tenant_id = $1 AND cs.package_id = $2 AND cs.status IN ('active', 'suspended')
    "#,
    )
    .bind(tenant_id)
    .bind(package_id)
    .fetch_all(pool)
    .await
}

pub(crate) fn normalize_status(s: Option<String>) -> Option<String> {
    match s.as_deref() {
        Some("open") | Some("pending") | Some("closed") => s,
        _ => None,
    }
}

pub(crate) fn normalize_priority(p: Option<String>) -> String {
    match p.as_deref() {
        Some("low") | Some("normal") | Some("high") | Some("urgent") => p.unwrap(),
        _ => "normal".to_string(),
    }
}

pub(crate) fn normalize_priority_optional_lowercase(p: Option<String>) -> Option<String> {
    p.and_then(|p| {
        let p = p.to_lowercase();
        match p.as_str() {
            "low" | "normal" | "high" | "urgent" => Some(p),
            _ => None,
        }
    })
}

pub(crate) fn can_access_admin_audience(
    is_internal_tenant_member: bool,
    is_super_admin: bool,
) -> bool {
    is_internal_tenant_member || is_super_admin
}

#[cfg(test)]
mod tests {
    use super::can_access_admin_audience;

    #[test]
    fn admin_audience_is_visible_to_internal_members() {
        assert!(can_access_admin_audience(true, false));
    }

    #[test]
    fn admin_audience_is_visible_to_superadmins() {
        assert!(can_access_admin_audience(false, true));
    }

    #[test]
    fn admin_audience_is_hidden_from_non_internal_users() {
        assert!(!can_access_admin_audience(false, false));
    }
}
