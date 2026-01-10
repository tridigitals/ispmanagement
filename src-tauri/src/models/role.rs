//! Role and Permission models for RBAC

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Role - defines a set of permissions
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Role {
    pub id: String,
    pub tenant_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Role {
    pub fn new(tenant_id: Option<String>, name: String, description: Option<String>, is_system: bool) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id,
            name,
            description,
            is_system,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Permission - represents a single action on a resource
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Permission {
    pub id: String,
    pub resource: String,
    pub action: String,
    pub description: Option<String>,
}

impl Permission {
    pub fn new(resource: String, action: String, description: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            resource,
            action,
            description,
        }
    }

    /// Create permission key like "team:create"
    pub fn key(&self) -> String {
        format!("{}:{}", self.resource, self.action)
    }
}

/// RolePermission - pivot table linking roles to permissions
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RolePermission {
    pub role_id: String,
    pub permission_id: String,
}

/// Role with permissions (for API responses)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleWithPermissions {
    pub id: String,
    pub tenant_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub permissions: Vec<String>, // List of permission keys like ["team:create", "team:read"]
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RoleWithPermissions {
    pub fn from_role(role: Role, permissions: Vec<String>) -> Self {
        Self {
            id: role.id,
            tenant_id: role.tenant_id,
            name: role.name,
            description: role.description,
            is_system: role.is_system,
            permissions,
            created_at: role.created_at,
            updated_at: role.updated_at,
        }
    }
}

/// DTO for creating a role
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRoleDto {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>, // List of permission keys
}

/// DTO for updating a role
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRoleDto {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}
