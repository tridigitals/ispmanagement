//! Metrics Service - Request Performance Tracking
//!
//! Tracks request counts, response times, and error rates for monitoring.

use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Maximum number of response times to track (circular buffer)
const MAX_RESPONSE_TIMES: usize = 1000;

/// Request metrics for monitoring dashboard
#[derive(Debug, Clone, Serialize)]
pub struct RequestMetrics {
    /// Total requests since startup
    pub total_requests: u64,
    /// Requests in the last minute
    pub requests_last_minute: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Minimum response time in milliseconds
    pub min_response_time_ms: f64,
    /// Maximum response time in milliseconds
    pub max_response_time_ms: f64,
    /// Total error responses (4xx, 5xx)
    pub error_count: u64,
    /// Total rate-limited requests
    pub rate_limited_count: u64,
    /// P95 response time in milliseconds
    pub p95_response_time_ms: f64,
}

impl Default for RequestMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            requests_last_minute: 0,
            avg_response_time_ms: 0.0,
            min_response_time_ms: 0.0,
            max_response_time_ms: 0.0,
            error_count: 0,
            rate_limited_count: 0,
            p95_response_time_ms: 0.0,
        }
    }
}

/// Internal struct to track request timing
struct TimedRequest {
    duration: Duration,
    timestamp: Instant,
}

/// Thread-safe metrics collector
pub struct MetricsService {
    /// Total request count
    total_requests: AtomicU64,
    /// Error count (4xx, 5xx responses)
    error_count: AtomicU64,
    /// Rate-limited request count
    rate_limited_count: AtomicU64,
    /// Response times with timestamps (circular buffer)
    response_times: RwLock<VecDeque<TimedRequest>>,
    /// Service start time
    start_time: Instant,
}

impl MetricsService {
    /// Create a new metrics service
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            rate_limited_count: AtomicU64::new(0),
            response_times: RwLock::new(VecDeque::with_capacity(MAX_RESPONSE_TIMES)),
            start_time: Instant::now(),
        }
    }

    /// Record a completed request
    pub fn record_request(&self, duration: Duration, is_error: bool) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        if is_error {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }

        // Add to response times buffer
        let mut times = self.response_times.write().unwrap();
        if times.len() >= MAX_RESPONSE_TIMES {
            times.pop_front();
        }
        times.push_back(TimedRequest {
            duration,
            timestamp: Instant::now(),
        });
    }

    /// Record a rate-limited request
    pub fn record_rate_limited(&self) {
        self.rate_limited_count.fetch_add(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub fn get_metrics(&self) -> RequestMetrics {
        let total_requests = self.total_requests.load(Ordering::Relaxed);
        let error_count = self.error_count.load(Ordering::Relaxed);
        let rate_limited_count = self.rate_limited_count.load(Ordering::Relaxed);

        let times = self.response_times.read().unwrap();
        let now = Instant::now();
        let one_minute_ago = now - Duration::from_secs(60);

        // Filter requests from last minute
        let recent_times: Vec<Duration> = times
            .iter()
            .filter(|t| t.timestamp > one_minute_ago)
            .map(|t| t.duration)
            .collect();

        let requests_last_minute = recent_times.len() as u64;

        // Calculate statistics from all tracked times
        let all_durations: Vec<f64> = times
            .iter()
            .map(|t| t.duration.as_secs_f64() * 1000.0)
            .collect();

        let (avg, min, max, p95) = if all_durations.is_empty() {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            let sum: f64 = all_durations.iter().sum();
            let avg = sum / all_durations.len() as f64;
            let min = all_durations.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = all_durations
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);

            // Calculate P95
            let mut sorted = all_durations.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let p95_idx = (sorted.len() as f64 * 0.95) as usize;
            let p95 = sorted
                .get(p95_idx.min(sorted.len() - 1))
                .copied()
                .unwrap_or(0.0);

            (avg, min, max, p95)
        };

        RequestMetrics {
            total_requests,
            requests_last_minute,
            avg_response_time_ms: (avg * 100.0).round() / 100.0,
            min_response_time_ms: (min * 100.0).round() / 100.0,
            max_response_time_ms: (max * 100.0).round() / 100.0,
            error_count,
            rate_limited_count,
            p95_response_time_ms: (p95 * 100.0).round() / 100.0,
        }
    }

    /// Get uptime in seconds
    #[allow(dead_code)]
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

impl Default for MetricsService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_request() {
        let service = MetricsService::new();

        service.record_request(Duration::from_millis(50), false);
        service.record_request(Duration::from_millis(100), false);
        service.record_request(Duration::from_millis(150), true);

        let metrics = service.get_metrics();
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(metrics.error_count, 1);
        assert!(metrics.avg_response_time_ms > 0.0);
    }

    #[test]
    fn test_rate_limited() {
        let service = MetricsService::new();

        service.record_rate_limited();
        service.record_rate_limited();

        let metrics = service.get_metrics();
        assert_eq!(metrics.rate_limited_count, 2);
        assert_eq!(metrics.total_requests, 2);
    }
}
