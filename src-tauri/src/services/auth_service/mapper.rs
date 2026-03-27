use crate::models::UserResponse;

pub fn apply_tenant_info(
    user_response: &mut UserResponse,
    tenant_info: Option<(String, Option<String>)>,
) {
    if let Some((slug, domain)) = tenant_info {
        user_response.tenant_slug = Some(slug);
        user_response.tenant_custom_domain = domain;
    }
}
