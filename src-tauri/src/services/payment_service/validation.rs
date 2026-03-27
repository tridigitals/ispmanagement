pub(super) fn is_owner_or_admin_role(role: Option<&str>) -> bool {
    role.map(|r| {
        let normalized = r.trim().to_ascii_lowercase();
        normalized == "owner" || normalized == "admin"
    })
    .unwrap_or(false)
}

pub(super) fn is_owner_admin_or_technician_role(role: Option<&str>) -> bool {
    role.map(|r| {
        let normalized = r.trim().to_ascii_lowercase();
        normalized == "owner" || normalized == "admin" || normalized == "technician"
    })
    .unwrap_or(false)
}
