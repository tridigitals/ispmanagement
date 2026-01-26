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
    #[serde(default)]
    pub enforce_2fa: bool,
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
            enforce_2fa: false,
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
    pub role: String,            // String representation for backward compatibility
    pub role_id: Option<String>, // New RBAC role ID
    pub created_at: DateTime<Utc>,
}

impl TenantMember {
    pub fn new(tenant_id: String, user_id: String, role: String, role_id: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            user_id,
            role,
            role_id,
            created_at: Utc::now(),
        }
    }
}

/// Helper struct for team member details with user info
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TeamMemberWithUser {
    pub id: String, // tenant_member id
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub role_id: Option<String>,
    pub role_name: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}
