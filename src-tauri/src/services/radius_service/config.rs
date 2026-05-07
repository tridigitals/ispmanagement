use std::collections::HashMap;

const DEFAULT_RADIUS_BIND_ADDR: &str = "0.0.0.0";
const DEFAULT_RADIUS_AUTH_PORT: u16 = 1812;
const DEFAULT_RADIUS_ACCT_PORT: u16 = 1813;
const DEFAULT_RADIUS_WORKER_CONCURRENCY: usize = 4;
const DEFAULT_RADIUS_REQUEST_TIMEOUT_MS: u64 = 3_000;
const DEFAULT_RADIUS_MAX_PACKET_SIZE: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiusRuntimeConfig {
    pub enabled: bool,
    pub bind_addr: String,
    pub auth_port: u16,
    pub acct_port: u16,
    pub worker_concurrency: usize,
    pub request_timeout_ms: u64,
    pub max_packet_size: usize,
    pub require_message_authenticator: bool,
}

impl RadiusRuntimeConfig {
    pub fn from_env() -> Self {
        let env = std::env::vars().collect::<HashMap<_, _>>();
        Self::from_map(&env)
    }

    pub fn from_map(env: &HashMap<String, String>) -> Self {
        Self {
            enabled: env_bool(env, "RADIUS_ENABLED", false),
            bind_addr: env
                .get("RADIUS_BIND_ADDR")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| DEFAULT_RADIUS_BIND_ADDR.to_string()),
            auth_port: env_u16(env, "RADIUS_AUTH_PORT", DEFAULT_RADIUS_AUTH_PORT),
            acct_port: env_u16(env, "RADIUS_ACCT_PORT", DEFAULT_RADIUS_ACCT_PORT),
            worker_concurrency: env_usize(
                env,
                "RADIUS_WORKER_CONCURRENCY",
                DEFAULT_RADIUS_WORKER_CONCURRENCY,
            ),
            request_timeout_ms: env_u64(
                env,
                "RADIUS_REQUEST_TIMEOUT_MS",
                DEFAULT_RADIUS_REQUEST_TIMEOUT_MS,
            ),
            max_packet_size: env_usize(
                env,
                "RADIUS_MAX_PACKET_SIZE",
                DEFAULT_RADIUS_MAX_PACKET_SIZE,
            ),
            require_message_authenticator: env_bool(
                env,
                "RADIUS_REQUIRE_MESSAGE_AUTHENTICATOR",
                true,
            ),
        }
    }
}

fn env_bool(env: &HashMap<String, String>, key: &str, default: bool) -> bool {
    env.get(key)
        .map(|value| value.trim().to_ascii_lowercase())
        .and_then(|value| match value.as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

fn env_u16(env: &HashMap<String, String>, key: &str, default: u16) -> u16 {
    env.get(key)
        .and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn env_u64(env: &HashMap<String, String>, key: &str, default: u64) -> u64 {
    env.get(key)
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn env_usize(env: &HashMap<String, String>, key: &str, default: usize) -> usize {
    env.get(key)
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::RadiusRuntimeConfig;
    use std::collections::HashMap;

    #[test]
    fn radius_runtime_config_uses_expected_defaults() {
        let env = HashMap::new();

        let config = RadiusRuntimeConfig::from_map(&env);

        assert!(!config.enabled);
        assert_eq!(config.bind_addr, "0.0.0.0");
        assert_eq!(config.auth_port, 1812);
        assert_eq!(config.acct_port, 1813);
        assert_eq!(config.worker_concurrency, 4);
        assert_eq!(config.request_timeout_ms, 3_000);
        assert_eq!(config.max_packet_size, 4096);
        assert!(config.require_message_authenticator);
    }

    #[test]
    fn radius_runtime_config_normalizes_invalid_numeric_values() {
        let env = HashMap::from([
            ("RADIUS_AUTH_PORT".to_string(), "0".to_string()),
            ("RADIUS_ACCT_PORT".to_string(), "-1".to_string()),
            ("RADIUS_WORKER_CONCURRENCY".to_string(), "0".to_string()),
            ("RADIUS_REQUEST_TIMEOUT_MS".to_string(), "abc".to_string()),
            ("RADIUS_MAX_PACKET_SIZE".to_string(), "".to_string()),
        ]);

        let config = RadiusRuntimeConfig::from_map(&env);

        assert_eq!(config.auth_port, 1812);
        assert_eq!(config.acct_port, 1813);
        assert_eq!(config.worker_concurrency, 4);
        assert_eq!(config.request_timeout_ms, 3_000);
        assert_eq!(config.max_packet_size, 4096);
    }

    #[test]
    fn radius_runtime_config_parses_enable_flag_and_ports_from_env() {
        let env = HashMap::from([
            ("RADIUS_ENABLED".to_string(), "true".to_string()),
            ("RADIUS_BIND_ADDR".to_string(), "127.0.0.1".to_string()),
            ("RADIUS_AUTH_PORT".to_string(), "1912".to_string()),
            ("RADIUS_ACCT_PORT".to_string(), "1913".to_string()),
            ("RADIUS_WORKER_CONCURRENCY".to_string(), "8".to_string()),
            ("RADIUS_REQUEST_TIMEOUT_MS".to_string(), "1500".to_string()),
            ("RADIUS_MAX_PACKET_SIZE".to_string(), "8192".to_string()),
            (
                "RADIUS_REQUIRE_MESSAGE_AUTHENTICATOR".to_string(),
                "false".to_string(),
            ),
        ]);

        let config = RadiusRuntimeConfig::from_map(&env);

        assert!(config.enabled);
        assert_eq!(config.bind_addr, "127.0.0.1");
        assert_eq!(config.auth_port, 1912);
        assert_eq!(config.acct_port, 1913);
        assert_eq!(config.worker_concurrency, 8);
        assert_eq!(config.request_timeout_ms, 1500);
        assert_eq!(config.max_packet_size, 8192);
        assert!(!config.require_message_authenticator);
    }
}
