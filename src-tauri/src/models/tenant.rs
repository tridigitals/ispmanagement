use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub custom_domain: Option<String>,
    pub logo_url: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Tenant {
    pub fn new(name: String, slug: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            slug,
            custom_domain: None,
            logo_url: None,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TenantMember {
    pub id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

impl TenantMember {
    pub fn new(tenant_id: String, user_id: String, role: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            user_id,
            role,
            created_at: Utc::now(),
        }
    }
}
