//! Rate Limiter Service - IP-based request rate limiting
//!
//! Implements a sliding window rate limiter to protect against
//! brute force and DDoS attacks.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Rate limit error when limit is exceeded
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u32,
    pub remaining: u32,
    pub reset_in_secs: u64,
}

/// Thread-safe rate limiter using sliding window algorithm
pub struct RateLimiter {
    /// Map of key -> list of request timestamps
    requests: RwLock<HashMap<String, Vec<Instant>>>,
    /// Cleanup interval - entries older than this are removed
    cleanup_threshold: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `cleanup_threshold` - How long to keep old entries before cleanup (default: 5 minutes)
    pub fn new(cleanup_threshold_secs: u64) -> Self {
        Self {
            requests: RwLock::new(HashMap::new()),
            cleanup_threshold: Duration::from_secs(cleanup_threshold_secs),
        }
    }

    /// Check if a request is allowed under the rate limit
    ///
    /// # Arguments
    /// * `key` - Unique identifier (e.g., IP address)
    /// * `limit` - Maximum number of requests allowed
    /// * `window_secs` - Time window in seconds
    ///
    /// # Returns
    /// * `Ok(RateLimitInfo)` - Request allowed, with remaining quota
    /// * `Err(RateLimitInfo)` - Rate limit exceeded
    pub fn check(
        &self,
        key: &str,
        limit: u32,
        window_secs: u64,
    ) -> Result<RateLimitInfo, RateLimitInfo> {
        let now = Instant::now();
        let window = Duration::from_secs(window_secs);
        let cutoff = now - window;

        let mut requests = self.requests.write().unwrap();

        // Get or create entry for this key
        let timestamps = requests.entry(key.to_string()).or_insert_with(Vec::new);

        // Remove expired timestamps (sliding window)
        timestamps.retain(|&ts| ts > cutoff);

        let current_count = timestamps.len() as u32;

        // Calculate reset time (when the oldest request in window expires)
        let reset_in_secs = if let Some(&oldest) = timestamps.first() {
            let elapsed = now.duration_since(oldest);
            if elapsed < window {
                (window - elapsed).as_secs()
            } else {
                window_secs
            }
        } else {
            window_secs
        };

        if current_count >= limit {
            // Rate limit exceeded
            Err(RateLimitInfo {
                limit,
                remaining: 0,
                reset_in_secs,
            })
        } else {
            // Allow request and record it
            timestamps.push(now);

            Ok(RateLimitInfo {
                limit,
                remaining: limit - current_count - 1,
                reset_in_secs,
            })
        }
    }

    /// Check rate limit without recording the request
    /// Useful for checking status without incrementing counter
    #[allow(dead_code)]
    pub fn peek(&self, key: &str, limit: u32, window_secs: u64) -> RateLimitInfo {
        let now = Instant::now();
        let window = Duration::from_secs(window_secs);
        let cutoff = now - window;

        let requests = self.requests.read().unwrap();

        let current_count = requests
            .get(key)
            .map(|ts| ts.iter().filter(|&&t| t > cutoff).count() as u32)
            .unwrap_or(0);

        RateLimitInfo {
            limit,
            remaining: limit.saturating_sub(current_count),
            reset_in_secs: window_secs,
        }
    }

    /// Cleanup old entries to prevent memory bloat
    /// Should be called periodically (e.g., every minute)
    pub fn cleanup(&self) {
        let now = Instant::now();
        let cutoff = now - self.cleanup_threshold;

        let mut requests = self.requests.write().unwrap();

        // Remove entries where all timestamps are expired
        requests.retain(|_, timestamps| {
            timestamps.retain(|&ts| ts > cutoff);
            !timestamps.is_empty()
        });
    }

    /// Get the number of tracked keys (for monitoring)
    #[allow(dead_code)]
    pub fn key_count(&self) -> usize {
        self.requests.read().unwrap().len()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(300) // 5 minutes default cleanup threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_allows_under_limit() {
        let limiter = RateLimiter::default();

        // Should allow 5 requests when limit is 5
        for i in 0..5 {
            let result = limiter.check("test_ip", 5, 60);
            assert!(result.is_ok(), "Request {} should be allowed", i);
        }
    }

    #[test]
    fn test_rate_limit_blocks_over_limit() {
        let limiter = RateLimiter::default();

        // Use up the limit
        for _ in 0..5 {
            let _ = limiter.check("test_ip", 5, 60);
        }

        // 6th request should be blocked
        let result = limiter.check("test_ip", 5, 60);
        assert!(result.is_err(), "Request over limit should be blocked");
    }

    #[test]
    fn test_different_keys_independent() {
        let limiter = RateLimiter::default();

        // Use up limit for ip1
        for _ in 0..5 {
            let _ = limiter.check("ip1", 5, 60);
        }

        // ip2 should still be allowed
        let result = limiter.check("ip2", 5, 60);
        assert!(result.is_ok(), "Different IP should have independent limit");
    }
}
