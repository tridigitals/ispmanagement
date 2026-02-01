//! In-memory cache with TTL support
//!
//! This module provides a simple thread-safe cache implementation
//! for reducing database queries on frequently accessed data.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// A cached entry with expiration time
#[derive(Clone)]
pub struct CacheEntry<T: Clone> {
    pub value: T,
    pub expires_at: Instant,
}

impl<T: Clone> CacheEntry<T> {
    /// Check if this entry has expired
    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Thread-safe in-memory cache with TTL
pub struct MemoryCache<T: Clone> {
    entries: RwLock<HashMap<String, CacheEntry<T>>>,
    ttl: Duration,
}

impl<T: Clone> MemoryCache<T> {
    /// Create a new cache with the specified TTL
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Get a value from the cache if it exists and hasn't expired
    pub fn get(&self, key: &str) -> Option<T> {
        let entries = self.entries.read().ok()?;

        if let Some(entry) = entries.get(key) {
            if !entry.is_expired() {
                return Some(entry.value.clone());
            }
        }

        None
    }

    /// Set a value in the cache
    pub fn set(&self, key: String, value: T) {
        if let Ok(mut entries) = self.entries.write() {
            entries.insert(
                key,
                CacheEntry {
                    value,
                    expires_at: Instant::now() + self.ttl,
                },
            );
        }
    }

    /// Remove a specific key from the cache
    pub fn invalidate(&self, key: &str) {
        if let Ok(mut entries) = self.entries.write() {
            entries.remove(key);
        }
    }

    /// Remove all entries from the cache
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.clear();
        }
    }

    /// Remove expired entries (call periodically for cleanup)
    pub fn cleanup_expired(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.retain(|_, entry| !entry.is_expired());
        }
    }
}

/// Simple single-value cache (for global settings like AuthSettings)
pub struct SingleValueCache<T: Clone> {
    entry: RwLock<Option<CacheEntry<T>>>,
    ttl: Duration,
}

impl<T: Clone> SingleValueCache<T> {
    /// Create a new single-value cache with the specified TTL
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            entry: RwLock::new(None),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Get the cached value if it exists and hasn't expired
    pub fn get(&self) -> Option<T> {
        let entry = self.entry.read().ok()?;

        if let Some(ref cached) = *entry {
            if !cached.is_expired() {
                return Some(cached.value.clone());
            }
        }

        None
    }

    /// Set the cached value
    pub fn set(&self, value: T) {
        if let Ok(mut entry) = self.entry.write() {
            *entry = Some(CacheEntry {
                value,
                expires_at: Instant::now() + self.ttl,
            });
        }
    }

    /// Clear the cached value
    pub fn invalidate(&self) {
        if let Ok(mut entry) = self.entry.write() {
            *entry = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_memory_cache_basic() {
        let cache: MemoryCache<String> = MemoryCache::new(60);

        cache.set("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert_eq!(cache.get("nonexistent"), None);
    }

    #[test]
    fn test_memory_cache_expiration() {
        let cache: MemoryCache<String> = MemoryCache::new(1); // 1 second TTL

        cache.set("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get("key1"), Some("value1".to_string()));

        sleep(Duration::from_secs(2));
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_single_value_cache() {
        let cache: SingleValueCache<i32> = SingleValueCache::new(60);

        assert_eq!(cache.get(), None);
        cache.set(42);
        assert_eq!(cache.get(), Some(42));

        cache.invalidate();
        assert_eq!(cache.get(), None);
    }
}
