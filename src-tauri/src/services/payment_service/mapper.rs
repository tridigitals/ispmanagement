use std::collections::HashSet;

use super::validation::{is_owner_admin_or_technician_role, is_owner_or_admin_role};

pub(super) fn filter_owner_admin_user_ids(rows: Vec<(String, Option<String>)>) -> Vec<String> {
    let mut set = HashSet::new();
    for (user_id, role) in rows {
        if is_owner_or_admin_role(role.as_deref()) {
            set.insert(user_id);
        }
    }
    set.into_iter().collect()
}

pub(super) fn filter_installation_request_user_ids(
    rows: Vec<(String, Option<String>)>,
) -> Vec<String> {
    let mut set = HashSet::new();
    for (user_id, role) in rows {
        if is_owner_admin_or_technician_role(role.as_deref()) {
            set.insert(user_id);
        }
    }
    set.into_iter().collect()
}
