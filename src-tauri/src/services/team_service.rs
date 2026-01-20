//! Team Service for managing tenant members

use crate::db::DbPool;
use crate::models::{TeamMemberWithUser, User};
use crate::services::{AuthService, AuditService, PlanService};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
#[allow(dead_code)]
pub struct TeamService {
    pool: DbPool,
    auth_service: AuthService,
    audit_service: AuditService,
    plan_service: PlanService,
}

impl TeamService {
    pub fn new(pool: DbPool, auth_service: AuthService, audit_service: AuditService, plan_service: PlanService) -> Self {
        Self { pool, auth_service, audit_service, plan_service }
    }

    /// List all members of a team
    pub async fn list_members(&self, tenant_id: &str) -> Result<Vec<TeamMemberWithUser>, sqlx::Error> {
        #[cfg(feature = "postgres")]
        let query = r#"
            SELECT 
                tm.id, 
                tm.user_id, 
                u.name, 
                u.email, 
                tm.role,
                tm.role_id,
                r.name as role_name,
                u.is_active, 
                tm.created_at
            FROM tenant_members tm
            JOIN users u ON tm.user_id = u.id
            LEFT JOIN roles r ON tm.role_id = r.id
            WHERE tm.tenant_id = $1
            ORDER BY u.name
        "#;
        
        #[cfg(feature = "sqlite")]
        let query = r#"
            SELECT 
                tm.id, 
                tm.user_id, 
                u.name, 
                u.email, 
                tm.role,
                tm.role_id,
                r.name as role_name,
                u.is_active, 
                tm.created_at
            FROM tenant_members tm
            JOIN users u ON tm.user_id = u.id
            LEFT JOIN roles r ON tm.role_id = r.id
            WHERE tm.tenant_id = ?
            ORDER BY u.name
        "#;

        sqlx::query_as::<_, TeamMemberWithUser>(query)
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
    }

    /// Get user role level
    pub async fn get_user_role_level(&self, user_id: &str, tenant_id: &str) -> Result<i32, String> {
        #[cfg(feature = "postgres")]
        let level: Option<i32> = sqlx::query_scalar("SELECT r.level FROM tenant_members tm JOIN roles r ON tm.role_id = r.id WHERE tm.user_id = $1 AND tm.tenant_id = $2")
            .bind(user_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        #[cfg(feature = "sqlite")]
        let level: Option<i32> = sqlx::query_scalar("SELECT r.level FROM tenant_members tm JOIN roles r ON tm.role_id = r.id WHERE tm.user_id = ? AND tm.tenant_id = ?")
            .bind(user_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(level.unwrap_or(0))
    }

    /// Get member role level
    pub async fn get_member_role_level(&self, member_id: &str) -> Result<i32, String> {
        #[cfg(feature = "postgres")]
        let level: Option<i32> = sqlx::query_scalar("SELECT r.level FROM tenant_members tm JOIN roles r ON tm.role_id = r.id WHERE tm.id = $1")
            .bind(member_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        #[cfg(feature = "sqlite")]
        let level: Option<i32> = sqlx::query_scalar("SELECT r.level FROM tenant_members tm JOIN roles r ON tm.role_id = r.id WHERE tm.id = ?")
            .bind(member_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(level.unwrap_or(0))
    }

    /// Get role level by ID
    pub async fn get_role_level_by_id(&self, role_id: &str) -> Result<i32, String> {
         #[cfg(feature = "postgres")]
        let level: Option<i32> = sqlx::query_scalar("SELECT level FROM roles WHERE id = $1")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        #[cfg(feature = "sqlite")]
        let level: Option<i32> = sqlx::query_scalar("SELECT level FROM roles WHERE id = ?")
            .bind(role_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(level.unwrap_or(0))
    }

    /// Add a new member (create user if needed, or link existing)
    pub async fn add_member(
        &self, 
        tenant_id: &str, 
        email: &str, 
        name: &str, 
        role_id: &str,
        password: Option<String>,
        actor_id: Option<&str>,
        ip_address: Option<&str>
    ) -> Result<TeamMemberWithUser, String> {
        // 0. Check Plan Limits (max_users)
        let limit = self.plan_service.get_feature_limit(tenant_id, "max_users")
            .await
            .map_err(|e| e.to_string())?;

        if let Some(max_users) = limit {
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenant_members WHERE tenant_id = $1")
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

            if count >= max_users {
                return Err(format!("Plan limit reached: Maximum {} users allowed.", max_users));
            }
        }

        // 1. Check if user exists
        let existing_user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        let user_id = if let Some(user) = existing_user {
            // User exists, just link them
            user.id
        } else {
            // User doesn't exist, create them
            let password_str = password.unwrap_or_else(|| Uuid::new_v4().to_string());
            let hash = AuthService::hash_password(&password_str).map_err(|e| e.to_string())?;
            let now = Utc::now();
            let new_id = Uuid::new_v4().to_string();
            
            let query = "INSERT INTO users (id, email, name, password_hash, role, is_active, failed_login_attempts, created_at, updated_at) VALUES ($1, $2, $3, $4, 'user', true, 0, $5, $6)";
            
            #[cfg(feature = "postgres")]
            sqlx::query(query)
                .bind(&new_id)
                .bind(email)
                .bind(name)
                .bind(hash)
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
                
            #[cfg(feature = "sqlite")]
            sqlx::query(query)
                .bind(&new_id)
                .bind(email)
                .bind(name)
                .bind(hash)
                .bind(now.to_rfc3339())
                .bind(now.to_rfc3339())
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

            // TODO: Send welcome email with password reset link
            
            new_id
        };

        // 2. Get role details
        let role_name: String = sqlx::query_scalar("SELECT name FROM roles WHERE id = $1")
            .bind(role_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| "Role not found".to_string())?;

        // 3. Add to tenant_members
        // Check if already a member
        let is_member: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tenant_members WHERE tenant_id = $1 AND user_id = $2)")
            .bind(tenant_id)
            .bind(&user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
            
        if is_member {
            return Err("User is already a member of this team".to_string());
        }

        let member_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let query = "INSERT INTO tenant_members (id, tenant_id, user_id, role, role_id, created_at) VALUES ($1, $2, $3, $4, $5, $6)";
        
        #[cfg(feature = "postgres")]
        sqlx::query(query)
            .bind(&member_id)
            .bind(tenant_id)
            .bind(&user_id)
            .bind(&role_name) // Fallback string role
            .bind(role_id)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
            
        #[cfg(feature = "sqlite")]
        sqlx::query(query)
            .bind(&member_id)
            .bind(tenant_id)
            .bind(&user_id)
            .bind(&role_name)
            .bind(role_id)
            .bind(now.to_rfc3339())
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        // Audit Log
        self.audit_service.log(
            actor_id,
            Some(tenant_id),
            "TEAM_MEMBER_ADD",
            "team",
            Some(&user_id),
            Some(&format!("Added user {} to team as {}", email, role_name)),
            ip_address,
        ).await;

        // Return the created member
        Ok(TeamMemberWithUser {
            id: member_id,
            user_id,
            name: name.to_string(),
            email: email.to_string(),
            role: role_name.clone(),
            role_id: Some(role_id.to_string()),
            role_name: Some(role_name),
            is_active: true,
            created_at: now,
        })
    }

    /// Update member role
    pub async fn update_member(&self, tenant_id: &str, member_id: &str, role_id: &str, actor_id: Option<&str>, ip_address: Option<&str>) -> Result<(), String> {
        let role_name: String = sqlx::query_scalar("SELECT name FROM roles WHERE id = $1")
            .bind(role_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| "Role not found".to_string())?;

        let query = "UPDATE tenant_members SET role = $1, role_id = $2 WHERE id = $3";
        
        sqlx::query(query)
            .bind(&role_name)
            .bind(role_id)
            .bind(member_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        
        // Audit Log
        self.audit_service.log(
            actor_id,
            Some(tenant_id),
            "TEAM_MEMBER_UPDATE",
            "team",
            Some(member_id),
            Some(&format!("Updated member role to {}", role_name)),
            ip_address,
        ).await;

        Ok(())
    }

    /// Remove member
    pub async fn remove_member(&self, tenant_id: &str, member_id: &str, actor_id: Option<&str>, ip_address: Option<&str>) -> Result<(), String> {
        let query = "DELETE FROM tenant_members WHERE id = $1";
        
        sqlx::query(query)
            .bind(member_id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        
        // Audit Log
        self.audit_service.log(
            actor_id,
            Some(tenant_id),
            "TEAM_MEMBER_REMOVE",
            "team",
            Some(member_id),
            Some("Removed member from team"),
            ip_address,
        ).await;

        Ok(())
    }
}
