use super::*;

impl CustomerService {
    pub(super) fn build_auto_pppoe_username(
        customer_name: &str,
        customer_id: &str,
        location_id: &str,
    ) -> String {
        let mut slug = String::new();
        for ch in customer_name.trim().chars() {
            if ch.is_ascii_alphanumeric() {
                slug.push(ch.to_ascii_lowercase());
            } else if (ch.is_ascii_whitespace() || ch == '-' || ch == '_')
                && !slug.ends_with('-')
                && !slug.is_empty()
            {
                slug.push('-');
            }
            if slug.len() >= 14 {
                break;
            }
        }
        let slug = slug.trim_matches('-');
        let base = if slug.is_empty() { "cust" } else { slug };
        let c4 = customer_id.chars().rev().take(4).collect::<String>();
        let l4 = location_id.chars().rev().take(4).collect::<String>();
        format!(
            "{}-{}{}",
            base,
            c4.chars().rev().collect::<String>(),
            l4.chars().rev().collect::<String>()
        )
    }
}
