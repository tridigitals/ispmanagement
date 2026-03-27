use crate::models::User;

use super::dto::Claims;
use chrono::{DateTime, Utc};

pub fn build_login_claims(
    user: &User,
    tenant_id: Option<String>,
    now: DateTime<Utc>,
    jwt_expires_at: DateTime<Utc>,
) -> Claims {
    Claims {
        sub: user.id.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
        tenant_id,
        is_super_admin: user.is_super_admin,
        exp: jwt_expires_at.timestamp() as usize,
        iat: now.timestamp() as usize,
    }
}

pub fn generate_device_fingerprint(user_agent: Option<&str>, ip_address: Option<&str>) -> String {
    use sha2::{Digest, Sha256};
    let combined = format!(
        "{}:{}",
        user_agent.unwrap_or("unknown"),
        ip_address.unwrap_or("unknown")
    );
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    format!("{:x}", hasher.finalize())
}
