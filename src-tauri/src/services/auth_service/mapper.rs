use crate::models::tenant::CUSTOM_DOMAIN_STATUS_ACTIVE;
use crate::models::UserResponse;

pub fn apply_tenant_info(
    user_response: &mut UserResponse,
    tenant_info: Option<(String, Option<String>, Option<String>)>,
) {
    if let Some((slug, domain, domain_status)) = tenant_info {
        user_response.tenant_slug = Some(slug);
        user_response.tenant_custom_domain = if domain_status.as_deref() == Some(CUSTOM_DOMAIN_STATUS_ACTIVE) {
            domain
        } else {
            None
        };
    }
}
