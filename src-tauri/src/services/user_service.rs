//! User Service for CRUD operations

use crate::error::{AppError, AppResult};
use crate::models::{CreateUserDto, UpdateUserDto, User, UserResponse};
use crate::services::auth_service::AuthService;
use sqlx::{Pool, Sqlite};
use chrono::Utc;

/// User service for managing users
pub struct UserService {
    pool: Pool<Sqlite>,
}

impl UserService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// List all users with pagination
    pub async fn list(&self, page: u32, per_page: u32) -> AppResult<(Vec<UserResponse>, i64)> {
        let offset = (page.saturating_sub(1)) * per_page;

        let users: Vec<User> = sqlx::query_as(
            "SELECT * FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(per_page as i32)
        .bind(offset as i32)
        .fetch_all(&self.pool)
        .await?;

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        Ok((users.into_iter().map(UserResponse::from).collect(), count.0))
    }

    /// Get user by ID
    pub async fn get_by_id(&self, id: &str) -> AppResult<UserResponse> {
        let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::UserNotFound)?;

        Ok(user.into())
    }

    /// Create new user
    pub async fn create(&self, dto: CreateUserDto) -> AppResult<UserResponse> {
        // Check if email already exists
        let existing: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = ?")
            .bind(&dto.email)
            .fetch_optional(&self.pool)
            .await?;

        if existing.is_some() {
            return Err(AppError::UserAlreadyExists);
        }

        let password_hash = AuthService::hash_password(&dto.password)?;
        let user = User::new(dto.email, password_hash, dto.name);

        sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, name, role, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.name)
        .bind(&user.role)
        .bind(user.is_active)
        .bind(user.created_at.to_rfc3339())
        .bind(user.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(user.into())
    }

    /// Update user
    pub async fn update(&self, id: &str, dto: UpdateUserDto) -> AppResult<UserResponse> {
        let mut user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::UserNotFound)?;

        // Update fields if provided
        if let Some(email) = dto.email {
            // Check if email is taken by another user
            let existing: Option<User> = sqlx::query_as(
                "SELECT * FROM users WHERE email = ? AND id != ?"
            )
            .bind(&email)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

            if existing.is_some() {
                return Err(AppError::UserAlreadyExists);
            }
            user.email = email;
        }
        if let Some(name) = dto.name {
            user.name = name;
        }
        if let Some(role) = dto.role {
            user.role = role;
        }
        if let Some(is_active) = dto.is_active {
            user.is_active = is_active;
        }

        let updated_at = Utc::now();

        sqlx::query(
            r#"
            UPDATE users SET email = ?, name = ?, role = ?, is_active = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&user.email)
        .bind(&user.name)
        .bind(&user.role)
        .bind(user.is_active)
        .bind(updated_at.to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await?;

        user.updated_at = updated_at;
        Ok(user.into())
    }

    /// Delete user
    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::UserNotFound);
        }

        Ok(())
    }
    /// Count all users
    pub async fn count(&self) -> AppResult<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.0)
    }
}
