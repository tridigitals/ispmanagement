//! Role and Permission service for RBAC

use crate::db::DbPool;
use crate::models::{Role, Permission, RoleWithPermissions, CreateRoleDto, UpdateRoleDto};
use chrono::Utc;
use uuid::Uuid;

/// Default permissions to seed
pub fn get_default_permissions() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        // Team permissions
        ("team", "create", "Create team members"),
        ("team", "read", "View team members"),
        ("team", "update", "Update team members"),
        ("team", "delete", "Remove team members"),
        // Settings permissions
        ("settings", "read", "View settings"),
        ("settings", "update", "Modify settings"),
        // Roles permissions
        ("roles", "create", "Create roles"),
        ("roles", "read", "View roles"),
        ("roles", "update", "Modify roles"),
        ("roles", "delete", "Delete roles"),
        // Dashboard permissions
        ("dashboard", "read", "View dashboard"),
    ]
}

/// Default roles with their permissions
pub fn get_default_roles() -> Vec<(&'static str, &'static str, bool, Vec<&'static str>)> {
    vec![
        // (name, description, is_system, permissions)
        (
            "Owner",
            "Full access to all features",
            true,
            vec![
                "team:create", "team:read", "team:update", "team:delete",
                "settings:read", "settings:update",
                "roles:create", "roles:read", "roles:update", "roles:delete",
                "dashboard:read",
            ],
        ),
        (
            "Admin",
            "Manage team and settings",
            true,
            vec![
                "team:create", "team:read", "team:update", "team:delete",
                "settings:read", "settings:update",
                "roles:read",
                "dashboard:read",
            ],
        ),
        (
            "Member",
            "Standard team member",
            true,
            vec!["team:read", "dashboard:read"],
        ),
        (
            "Viewer",
            "Read-only access",
            true,
            vec!["dashboard:read"],
        ),
    ]
}

/// Seed default permissions into database
pub async fn seed_permissions(pool: &DbPool) -> Result<(), sqlx::Error> {
    let permissions = get_default_permissions();
    
    for (resource, action, description) in permissions {
        let id = Uuid::new_v4().to_string();
        
        #[cfg(feature = "postgres")]
        {
            sqlx::query(r#"
                INSERT INTO permissions (id, resource, action, description)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (resource, action) DO NOTHING
            "#)
            .bind(&id)
            .bind(resource)
            .bind(action)
            .bind(description)
            .execute(pool)
            .await?;
        }
        
        #[cfg(feature = "sqlite")]
        {
            sqlx::query(r#"
                INSERT OR IGNORE INTO permissions (id, resource, action, description)
                VALUES (?, ?, ?, ?)
            "#)
            .bind(&id)
            .bind(resource)
            .bind(action)
            .bind(description)
            .execute(pool)
            .await?;
        }
    }
    
    tracing::info!("Default permissions seeded");
    Ok(())
}

/// Seed default roles into database (global roles, tenant_id = NULL)
pub async fn seed_roles(pool: &DbPool) -> Result<(), sqlx::Error> {
    let now = Utc::now();
    let roles = get_default_roles();
    
    for (name, description, is_system, permission_keys) in roles {
        // Check if role already exists
        #[cfg(feature = "postgres")]
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM roles WHERE name = $1 AND tenant_id IS NULL"
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;
        
        #[cfg(feature = "sqlite")]
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM roles WHERE name = ? AND tenant_id IS NULL"
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;
        
        if existing.is_some() {
            continue; // Role already exists
        }
        
        let role_id = Uuid::new_v4().to_string();
        
        // Insert role
        #[cfg(feature = "postgres")]
        {
            sqlx::query(r#"
                INSERT INTO roles (id, tenant_id, name, description, is_system, created_at, updated_at)
                VALUES ($1, NULL, $2, $3, $4, $5, $6)
            "#)
            .bind(&role_id)
            .bind(name)
            .bind(description)
            .bind(is_system)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }
        
        #[cfg(feature = "sqlite")]
        {
            let now_str = now.to_rfc3339();
            sqlx::query(r#"
                INSERT INTO roles (id, tenant_id, name, description, is_system, created_at, updated_at)
                VALUES (?, NULL, ?, ?, ?, ?, ?)
            "#)
            .bind(&role_id)
            .bind(name)
            .bind(description)
            .bind(is_system as i32)
            .bind(&now_str)
            .bind(&now_str)
            .execute(pool)
            .await?;
        }
        
        // Link permissions to role
        for perm_key in permission_keys {
            let parts: Vec<&str> = perm_key.split(':').collect();
            if parts.len() != 2 {
                continue;
            }
            let (resource, action) = (parts[0], parts[1]);
            
            #[cfg(feature = "postgres")]
            {
                sqlx::query(r#"
                    INSERT INTO role_permissions (role_id, permission_id)
                    SELECT $1, id FROM permissions WHERE resource = $2 AND action = $3
                    ON CONFLICT DO NOTHING
                "#)
                .bind(&role_id)
                .bind(resource)
                .bind(action)
                .execute(pool)
                .await?;
            }
            
            #[cfg(feature = "sqlite")]
            {
                sqlx::query(r#"
                    INSERT OR IGNORE INTO role_permissions (role_id, permission_id)
                    SELECT ?, id FROM permissions WHERE resource = ? AND action = ?
                "#)
                .bind(&role_id)
                .bind(resource)
                .bind(action)
                .execute(pool)
                .await?;
            }
        }
    }
    
    tracing::info!("Default roles seeded");
    Ok(())
}

/// Get all roles for a tenant (includes global roles where tenant_id IS NULL)
pub async fn list_roles(pool: &DbPool, tenant_id: Option<&str>) -> Result<Vec<RoleWithPermissions>, sqlx::Error> {
    #[cfg(feature = "postgres")]
    let roles: Vec<Role> = if let Some(tid) = tenant_id {
        sqlx::query_as(
            "SELECT * FROM roles WHERE tenant_id IS NULL OR tenant_id = $1 ORDER BY is_system DESC, name"
        )
        .bind(tid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as("SELECT * FROM roles WHERE tenant_id IS NULL ORDER BY is_system DESC, name")
            .fetch_all(pool)
            .await?
    };
    
    #[cfg(feature = "sqlite")]
    let roles: Vec<Role> = if let Some(tid) = tenant_id {
        sqlx::query_as(
            "SELECT * FROM roles WHERE tenant_id IS NULL OR tenant_id = ? ORDER BY is_system DESC, name"
        )
        .bind(tid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as("SELECT * FROM roles WHERE tenant_id IS NULL ORDER BY is_system DESC, name")
            .fetch_all(pool)
            .await?
    };
    
    let mut result = Vec::new();
    for role in roles {
        let permissions = get_role_permissions(pool, &role.id).await?;
        result.push(RoleWithPermissions::from_role(role, permissions));
    }
    
    Ok(result)
}

/// Get permissions for a role
pub async fn get_role_permissions(pool: &DbPool, role_id: &str) -> Result<Vec<String>, sqlx::Error> {
    #[cfg(feature = "postgres")]
    let perms: Vec<(String, String)> = sqlx::query_as(r#"
        SELECT p.resource, p.action 
        FROM permissions p
        JOIN role_permissions rp ON p.id = rp.permission_id
        WHERE rp.role_id = $1
    "#)
    .bind(role_id)
    .fetch_all(pool)
    .await?;
    
    #[cfg(feature = "sqlite")]
    let perms: Vec<(String, String)> = sqlx::query_as(r#"
        SELECT p.resource, p.action 
        FROM permissions p
        JOIN role_permissions rp ON p.id = rp.permission_id
        WHERE rp.role_id = ?
    "#)
    .bind(role_id)
    .fetch_all(pool)
    .await?;
    
    Ok(perms.into_iter().map(|(r, a)| format!("{}:{}", r, a)).collect())
}

/// Get all available permissions
pub async fn list_permissions(pool: &DbPool) -> Result<Vec<Permission>, sqlx::Error> {
    let perms: Vec<Permission> = sqlx::query_as("SELECT * FROM permissions ORDER BY resource, action")
        .fetch_all(pool)
        .await?;
    Ok(perms)
}

/// Create a new role
pub async fn create_role(
    pool: &DbPool,
    tenant_id: Option<&str>,
    dto: CreateRoleDto,
) -> Result<RoleWithPermissions, sqlx::Error> {
    let now = Utc::now();
    let role_id = Uuid::new_v4().to_string();
    
    #[cfg(feature = "postgres")]
    {
        sqlx::query(r#"
            INSERT INTO roles (id, tenant_id, name, description, is_system, level, created_at, updated_at)
            VALUES ($1, $2, $3, $4, false, $5, $6, $7)
        "#)
        .bind(&role_id)
        .bind(tenant_id)
        .bind(&dto.name)
        .bind(&dto.description)
        .bind(dto.level.unwrap_or(0))
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
    }
    
    #[cfg(feature = "sqlite")]
    {
        let now_str = now.to_rfc3339();
        sqlx::query(r#"
            INSERT INTO roles (id, tenant_id, name, description, is_system, level, created_at, updated_at)
            VALUES (?, ?, ?, ?, 0, ?, ?, ?)
        "#)
        .bind(&role_id)
        .bind(tenant_id)
        .bind(&dto.name)
        .bind(&dto.description)
        .bind(dto.level.unwrap_or(0))
        .bind(&now_str)
        .bind(&now_str)
        .execute(pool)
        .await?;
    }
    
    // Assign permissions
    for perm_key in &dto.permissions {
        let parts: Vec<&str> = perm_key.split(':').collect();
        if parts.len() != 2 {
            continue;
        }
        let (resource, action) = (parts[0], parts[1]);
        
        #[cfg(feature = "postgres")]
        {
            sqlx::query(r#"
                INSERT INTO role_permissions (role_id, permission_id)
                SELECT $1, id FROM permissions WHERE resource = $2 AND action = $3
            "#)
            .bind(&role_id)
            .bind(resource)
            .bind(action)
            .execute(pool)
            .await?;
        }
        
        #[cfg(feature = "sqlite")]
        {
            sqlx::query(r#"
                INSERT INTO role_permissions (role_id, permission_id)
                SELECT ?, id FROM permissions WHERE resource = ? AND action = ?
            "#)
            .bind(&role_id)
            .bind(resource)
            .bind(action)
            .execute(pool)
            .await?;
        }
    }
    
    // Fetch the created role
    let role = get_role_by_id(pool, &role_id).await?
        .ok_or_else(|| sqlx::Error::RowNotFound)?;
    
    Ok(role)
}

/// Get role by ID
pub async fn get_role_by_id(pool: &DbPool, role_id: &str) -> Result<Option<RoleWithPermissions>, sqlx::Error> {
    #[cfg(feature = "postgres")]
    let role: Option<Role> = sqlx::query_as("SELECT * FROM roles WHERE id = $1")
        .bind(role_id)
        .fetch_optional(pool)
        .await?;
    
    #[cfg(feature = "sqlite")]
    let role: Option<Role> = sqlx::query_as("SELECT * FROM roles WHERE id = ?")
        .bind(role_id)
        .fetch_optional(pool)
        .await?;
    
    match role {
        Some(r) => {
            let permissions = get_role_permissions(pool, &r.id).await?;
            Ok(Some(RoleWithPermissions::from_role(r, permissions)))
        }
        None => Ok(None),
    }
}

/// Update a role
pub async fn update_role(
    pool: &DbPool,
    role_id: &str,
    dto: UpdateRoleDto,
) -> Result<RoleWithPermissions, sqlx::Error> {
    let now = Utc::now();
    
    // Check if role is system role
    #[cfg(feature = "postgres")]
    let role: Option<Role> = sqlx::query_as("SELECT * FROM roles WHERE id = $1")
        .bind(role_id)
        .fetch_optional(pool)
        .await?;
    
    #[cfg(feature = "sqlite")]
    let role: Option<Role> = sqlx::query_as("SELECT * FROM roles WHERE id = ?")
        .bind(role_id)
        .fetch_optional(pool)
        .await?;
    
    let role = role.ok_or_else(|| sqlx::Error::RowNotFound)?;
    
    if role.is_system {
        // Cannot modify system role name/description, only permissions
    } else {
        if let Some(name) = &dto.name {
            #[cfg(feature = "postgres")]
            sqlx::query("UPDATE roles SET name = $1, updated_at = $2 WHERE id = $3")
                .bind(name)
                .bind(now)
                .bind(role_id)
                .execute(pool)
                .await?;
            
            #[cfg(feature = "sqlite")]
            sqlx::query("UPDATE roles SET name = ?, updated_at = ? WHERE id = ?")
                .bind(name)
                .bind(now.to_rfc3339())
                .bind(role_id)
                .execute(pool)
                .await?;
        }

        if let Some(level) = dto.level {
            #[cfg(feature = "postgres")]
            sqlx::query("UPDATE roles SET level = $1, updated_at = $2 WHERE id = $3")
                .bind(level)
                .bind(now)
                .bind(role_id)
                .execute(pool)
                .await?;
            
            #[cfg(feature = "sqlite")]
            sqlx::query("UPDATE roles SET level = ?, updated_at = ? WHERE id = ?")
                .bind(level)
                .bind(now.to_rfc3339())
                .bind(role_id)
                .execute(pool)
                .await?;
        }

        if let Some(description) = &dto.description {
             #[cfg(feature = "postgres")]
            sqlx::query("UPDATE roles SET description = $1, updated_at = $2 WHERE id = $3")
                .bind(description)
                .bind(now)
                .bind(role_id)
                .execute(pool)
                .await?;
            
            #[cfg(feature = "sqlite")]
            sqlx::query("UPDATE roles SET description = ?, updated_at = ? WHERE id = ?")
                .bind(description)
                .bind(now.to_rfc3339())
                .bind(role_id)
                .execute(pool)
                .await?;
        }
    }
    
    // Update permissions if provided
    if let Some(permissions) = &dto.permissions {
        // Clear existing permissions
        #[cfg(feature = "postgres")]
        sqlx::query("DELETE FROM role_permissions WHERE role_id = $1")
            .bind(role_id)
            .execute(pool)
            .await?;
        
        #[cfg(feature = "sqlite")]
        sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
            .bind(role_id)
            .execute(pool)
            .await?;
        
        // Add new permissions
        for perm_key in permissions {
            let parts: Vec<&str> = perm_key.split(':').collect();
            if parts.len() != 2 {
                continue;
            }
            let (resource, action) = (parts[0], parts[1]);
            
            #[cfg(feature = "postgres")]
            {
                sqlx::query(r#"
                    INSERT INTO role_permissions (role_id, permission_id)
                    SELECT $1, id FROM permissions WHERE resource = $2 AND action = $3
                "#)
                .bind(role_id)
                .bind(resource)
                .bind(action)
                .execute(pool)
                .await?;
            }
            
            #[cfg(feature = "sqlite")]
            {
                sqlx::query(r#"
                    INSERT INTO role_permissions (role_id, permission_id)
                    SELECT ?, id FROM permissions WHERE resource = ? AND action = ?
                "#)
                .bind(role_id)
                .bind(resource)
                .bind(action)
                .execute(pool)
                .await?;
            }
        }
    }
    
    get_role_by_id(pool, role_id).await?
        .ok_or_else(|| sqlx::Error::RowNotFound)
}

/// Delete a role (cannot delete system roles)
pub async fn delete_role(pool: &DbPool, role_id: &str) -> Result<bool, sqlx::Error> {
    // Check if system role
    #[cfg(feature = "postgres")]
    let is_system: Option<(bool,)> = sqlx::query_as("SELECT is_system FROM roles WHERE id = $1")
        .bind(role_id)
        .fetch_optional(pool)
        .await?;
    
    #[cfg(feature = "sqlite")]
    let is_system: Option<(bool,)> = sqlx::query_as("SELECT is_system FROM roles WHERE id = ?")
        .bind(role_id)
        .fetch_optional(pool)
        .await?;
    
    if let Some((true,)) = is_system {
        return Ok(false); // Cannot delete system role
    }
    
    #[cfg(feature = "postgres")]
    sqlx::query("DELETE FROM roles WHERE id = $1")
        .bind(role_id)
        .execute(pool)
        .await?;
    
    #[cfg(feature = "sqlite")]
    sqlx::query("DELETE FROM roles WHERE id = ?")
        .bind(role_id)
        .execute(pool)
        .await?;
    
    Ok(true)
}
