use super::{AuthService, AuthSettings, Claims};
use crate::error::AppError;
use crate::services::{AuditService, EmailService, SettingsService};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};

fn build_test_service(jwt_secret: &str) -> AuthService {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@127.0.0.1/test_db")
        .expect("lazy postgres pool should be constructible");

    let audit_service = AuditService::new(pool.clone(), None);
    let settings_service = SettingsService::new(pool.clone(), audit_service.clone());
    let email_service = EmailService::new(settings_service.clone());

    AuthService::new(
        pool,
        jwt_secret.to_string(),
        email_service,
        audit_service,
        settings_service,
    )
}

fn build_claims(exp: usize, role: &str) -> Claims {
    Claims {
        sub: "user-1".to_string(),
        email: "user@example.com".to_string(),
        role: role.to_string(),
        tenant_id: Some("tenant-1".to_string()),
        is_super_admin: false,
        exp,
        iat: Utc::now().timestamp() as usize,
    }
}

#[tokio::test]
async fn password_validation_characterization_respects_policy_flags() {
    let service = build_test_service("secret");
    let settings = AuthSettings {
        password_min_length: 10,
        password_require_uppercase: true,
        password_require_number: true,
        password_require_special: true,
        ..AuthSettings::default()
    };

    let invalid = service.validate_password("short", &settings);
    assert!(!invalid.valid);
    assert!(invalid
        .errors
        .iter()
        .any(|e| e.contains("at least 10 characters")));
    assert!(invalid
        .errors
        .iter()
        .any(|e| e.contains("uppercase letter")));
    assert!(invalid.errors.iter().any(|e| e.contains("one number")));
    assert!(invalid
        .errors
        .iter()
        .any(|e| e.contains("special character")));

    let valid = service.validate_password("ValidPass1!", &settings);
    assert!(valid.valid);
    assert!(valid.errors.is_empty());
}

#[tokio::test]
async fn validate_2fa_token_characterization_distinguishes_expired_and_invalid_signature() {
    let service = build_test_service("phase8-secret");

    let expired_claims = build_claims(
        (Utc::now() - Duration::minutes(10)).timestamp() as usize,
        "2fa_pending",
    );
    let expired_token = encode(
        &Header::default(),
        &expired_claims,
        &EncodingKey::from_secret("phase8-secret".as_bytes()),
    )
    .expect("expired token should be encodable");

    let wrong_secret_token = encode(
        &Header::default(),
        &build_claims(
            (Utc::now() + Duration::minutes(5)).timestamp() as usize,
            "2fa_pending",
        ),
        &EncodingKey::from_secret("different-secret".as_bytes()),
    )
    .expect("wrong-secret token should be encodable");

    let expired_err = service
        .validate_2fa_token(&expired_token)
        .await
        .expect_err("expired token should fail");
    assert!(matches!(expired_err, AppError::TokenExpired));

    let invalid_err = service
        .validate_2fa_token(&wrong_secret_token)
        .await
        .expect_err("wrong-signature token should fail");
    assert!(matches!(invalid_err, AppError::InvalidToken));
}

#[tokio::test]
async fn validate_2fa_token_characterization_accepts_valid_token_and_claims_shape() {
    let service = build_test_service("phase8-secret");

    let expected = build_claims(
        (Utc::now() + Duration::minutes(5)).timestamp() as usize,
        "2fa_pending",
    );
    let token = encode(
        &Header::default(),
        &expected,
        &EncodingKey::from_secret("phase8-secret".as_bytes()),
    )
    .expect("valid token should be encodable");

    let claims = service
        .validate_2fa_token(&token)
        .await
        .expect("valid token should decode");

    assert_eq!(claims.sub, expected.sub);
    assert_eq!(claims.email, expected.email);
    assert_eq!(claims.role, "2fa_pending");
    assert_eq!(claims.tenant_id, expected.tenant_id);
    assert!(!claims.is_super_admin);
}

#[test]
fn device_fingerprint_characterization_is_stable_per_input_pair() {
    let fp_one = AuthService::generate_device_fingerprint(Some("agent-a"), Some("10.0.0.1"));
    let fp_two = AuthService::generate_device_fingerprint(Some("agent-a"), Some("10.0.0.1"));
    let fp_other = AuthService::generate_device_fingerprint(Some("agent-b"), Some("10.0.0.1"));

    assert_eq!(fp_one.len(), 64);
    assert_eq!(fp_one, fp_two);
    assert_ne!(fp_one, fp_other);
}
