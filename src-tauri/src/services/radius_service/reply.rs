use crate::models::PppoeAccount;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusReplyAttribute {
    pub name: &'static str,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusReplyAttributes {
    pub attributes: Vec<RadiusReplyAttribute>,
}

impl RadiusReplyAttributes {
    pub fn from_account(account: &PppoeAccount) -> Self {
        let mut attributes = Vec::new();

        if let Some(profile_name) = account
            .router_profile_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            attributes.push(RadiusReplyAttribute {
                name: "Mikrotik-Group",
                value: profile_name.to_string(),
            });
        }

        if let Some(remote_address) = account
            .remote_address
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            attributes.push(RadiusReplyAttribute {
                name: "Framed-IP-Address",
                value: remote_address.to_string(),
            });
        }

        if let Some(address_pool) = account
            .address_pool
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            attributes.push(RadiusReplyAttribute {
                name: "Framed-Pool",
                value: address_pool.to_string(),
            });
        }

        Self { attributes }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name == name)
            .map(|attribute| attribute.value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::RadiusReplyAttributes;
    use crate::models::{PppoeAccount, PppoeAccountSource};
    use chrono::Utc;

    fn sample_account() -> PppoeAccount {
        let now = Utc::now();
        PppoeAccount {
            id: "acct-1".into(),
            tenant_id: "tenant-1".into(),
            router_id: "router-1".into(),
            customer_id: "cust-1".into(),
            location_id: "loc-1".into(),
            username: "alice".into(),
            password_enc: "enc".into(),
            package_id: None,
            profile_id: None,
            router_profile_name: Some("basic".into()),
            remote_address: Some("10.10.10.2".into()),
            address_pool: Some("pool-a".into()),
            disabled: false,
            comment: None,
            account_source: PppoeAccountSource::ManagedRadius,
            router_present: false,
            router_secret_id: None,
            last_sync_at: None,
            last_error: None,
            is_provisioned: false,
            radius_identity: None,
            provisioned_at: None,
            provisioning_error: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn reply_builder_emits_mikrotik_profile_and_ip_attributes() {
        let reply = RadiusReplyAttributes::from_account(&sample_account());

        assert_eq!(reply.get("Mikrotik-Group"), Some("basic"));
        assert_eq!(reply.get("Framed-IP-Address"), Some("10.10.10.2"));
        assert_eq!(reply.get("Framed-Pool"), Some("pool-a"));
    }

    #[test]
    fn reply_builder_omits_blank_optional_values() {
        let mut account = sample_account();
        account.router_profile_name = Some("   ".into());
        account.remote_address = None;
        account.address_pool = Some("".into());

        let reply = RadiusReplyAttributes::from_account(&account);

        assert_eq!(reply.get("Mikrotik-Group"), None);
        assert_eq!(reply.get("Framed-IP-Address"), None);
        assert_eq!(reply.get("Framed-Pool"), None);
        assert!(reply.attributes.is_empty());
    }
}
