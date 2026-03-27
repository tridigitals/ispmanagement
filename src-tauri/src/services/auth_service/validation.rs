use super::dto::{AuthSettings, PasswordValidationResult};

pub fn validate_password(password: &str, settings: &AuthSettings) -> PasswordValidationResult {
    let mut errors = Vec::new();

    if password.len() < settings.password_min_length {
        errors.push(format!(
            "Password must be at least {} characters",
            settings.password_min_length
        ));
    }

    if settings.password_require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
        errors.push("Password must contain at least one uppercase letter".to_string());
    }

    if settings.password_require_number && !password.chars().any(|c| c.is_numeric()) {
        errors.push("Password must contain at least one number".to_string());
    }

    if settings.password_require_special {
        let special_chars = "!@#$%^&*()_+-=[]{}|;:',.<>?/`~";
        if !password.chars().any(|c| special_chars.contains(c)) {
            errors.push("Password must contain at least one special character".to_string());
        }
    }

    PasswordValidationResult {
        valid: errors.is_empty(),
        errors,
    }
}
