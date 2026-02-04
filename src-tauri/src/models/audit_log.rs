#![allow(dead_code)]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: String,
    pub user_id: Option<String>,
    pub tenant_id: Option<String>,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl AuditLog {
    pub fn new(
        action: String,
        resource: String,
        resource_id: Option<String>,
        details: Option<String>,
        ip_address: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: None,
            tenant_id: None,
            action,
            resource,
            resource_id,
            details,
            ip_address,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLogResponse {
    pub id: String,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub tenant_id: Option<String>,
    pub tenant_name: Option<String>,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub resource_name: Option<String>, // Human-readable name for the resource
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AuditLogFilter {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub user_id: Option<String>,
    pub tenant_id: Option<String>,
    pub action: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub search: Option<String>, // Generic search for resource, details, user name
}
