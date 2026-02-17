pub fn can_access_global_user_management(is_super_admin: bool) -> bool {
    is_super_admin
}

pub fn can_update_user(
    is_super_admin: bool,
    actor_user_id: &str,
    target_user_id: &str,
    attempts_privileged_change: bool,
) -> bool {
    if is_super_admin {
        return true;
    }
    if actor_user_id != target_user_id {
        return false;
    }
    !attempts_privileged_change
}

pub fn can_reset_user_2fa(
    is_super_admin: bool,
    has_team_update_permission: bool,
    target_in_same_tenant: bool,
    target_is_super_admin: bool,
) -> bool {
    if target_is_super_admin && !is_super_admin {
        return false;
    }
    is_super_admin || (has_team_update_permission && target_in_same_tenant)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_user_management_requires_superadmin() {
        assert!(can_access_global_user_management(true));
        assert!(!can_access_global_user_management(false));
    }

    #[test]
    fn update_user_rule_allows_superadmin_anything() {
        assert!(can_update_user(true, "actor", "target", false));
        assert!(can_update_user(true, "actor", "target", true));
    }

    #[test]
    fn update_user_rule_allows_self_non_privileged_only() {
        assert!(can_update_user(false, "u1", "u1", false));
        assert!(!can_update_user(false, "u1", "u1", true));
    }

    #[test]
    fn update_user_rule_denies_non_superadmin_other_user() {
        assert!(!can_update_user(false, "u1", "u2", false));
        assert!(!can_update_user(false, "u1", "u2", true));
    }

    #[test]
    fn reset_2fa_rule_requires_superadmin_or_tenant_admin_capability() {
        assert!(can_reset_user_2fa(true, false, false, false));
        assert!(can_reset_user_2fa(false, true, true, false));
        assert!(!can_reset_user_2fa(false, true, false, false));
        assert!(!can_reset_user_2fa(false, false, true, false));
        assert!(!can_reset_user_2fa(false, false, false, false));
    }

    #[test]
    fn reset_2fa_rule_blocks_non_superadmin_against_superadmin_target() {
        assert!(!can_reset_user_2fa(false, true, true, true));
        assert!(can_reset_user_2fa(true, false, false, true));
    }
}
