//! Metrics collection and reporting

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Performance metrics collector
#[derive(Debug)]
pub struct MetricsCollector {
    /// Request count
    request_count: AtomicU64,
    /// Response count
    response_count: AtomicU64,
    /// Error count
    error_count: AtomicU64,
    /// Total bytes received
    bytes_received: AtomicU64,
    /// Total bytes sent
    bytes_sent: AtomicU64,
    /// Total latency (for averaging)
    total_latency_ns: AtomicU64,
    /// Active connections
    active_connections: AtomicU64,
    /// WebSocket connections
    websocket_connections: AtomicU64,
    /// gRPC streams
    grpc_streams: AtomicU64,
    /// Breakpoint hits
    breakpoint_hits: AtomicU64,
    /// Mock hits
    mock_hits: AtomicU64,
    /// Rewrite applications
    rewrite_applications: AtomicU64,
    /// Script executions
    script_executions: AtomicU64,
    /// Plugin invocations
    plugin_invocations: AtomicU64,
    /// Is collecting enabled
    enabled: AtomicBool,
    /// Start time
    start_time: Instant,
    /// Latency histogram buckets (in milliseconds)
    latency_buckets: Arc<RwLock<HashMap<u64, u64>>>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self {
            request_count: AtomicU64::new(0),
            response_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            websocket_connections: AtomicU64::new(0),
            grpc_streams: AtomicU64::new(0),
            breakpoint_hits: AtomicU64::new(0),
            mock_hits: AtomicU64::new(0),
            rewrite_applications: AtomicU64::new(0),
            script_executions: AtomicU64::new(0),
            plugin_invocations: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
            start_time: Instant::now(),
            latency_buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable collection
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// Record a request
    pub fn record_request(&self, bytes: u64) {
        if !self.enabled.load(Ordering::SeqCst) {
            return;
        }
        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a response
    pub fn record_response(&self, bytes: u64, latency: Duration) {
        if !self.enabled.load(Ordering::SeqCst) {
            return;
        }
        self.response_count.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
        self.total_latency_ns.fetch_add(latency.as_nanos() as u64, Ordering::Relaxed);

        // Update latency histogram
        let latency_ms = latency.as_millis() as u64;
        let bucket = Self::get_latency_bucket(latency_ms);
        let mut buckets = self.latency_buckets.write();
        *buckets.entry(bucket).or_insert(0) += 1;
    }

    /// Record an error
    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment active connections
    pub fn connection_opened(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections
    pub fn connection_closed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Increment WebSocket connections
    pub fn websocket_opened(&self) {
        self.websocket_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement WebSocket connections
    pub fn websocket_closed(&self) {
        self.websocket_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Increment gRPC streams
    pub fn grpc_stream_opened(&self) {
        self.grpc_streams.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement gRPC streams
    pub fn grpc_stream_closed(&self) {
        self.grpc_streams.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record breakpoint hit
    pub fn record_breakpoint_hit(&self) {
        self.breakpoint_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record mock hit
    pub fn record_mock_hit(&self) {
        self.mock_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record rewrite application
    pub fn record_rewrite(&self) {
        self.rewrite_applications.fetch_add(1, Ordering::Relaxed);
    }

    /// Record script execution
    pub fn record_script_execution(&self) {
        self.script_executions.fetch_add(1, Ordering::Relaxed);
    }

    /// Record plugin invocation
    pub fn record_plugin_invocation(&self) {
        self.plugin_invocations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get latency bucket (exponential buckets)
    fn get_latency_bucket(latency_ms: u64) -> u64 {
        match latency_ms {
            0..=1 => 1,
            2..=5 => 5,
            6..=10 => 10,
            11..=25 => 25,
            26..=50 => 50,
            51..=100 => 100,
            101..=250 => 250,
            251..=500 => 500,
            501..=1000 => 1000,
            1001..=2500 => 2500,
            2501..=5000 => 5000,
            _ => 10000,
        }
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> Metrics {
        let request_count = self.request_count.load(Ordering::Relaxed);
        let response_count = self.response_count.load(Ordering::Relaxed);
        let total_latency_ns = self.total_latency_ns.load(Ordering::Relaxed);

        let avg_latency_ns = if response_count > 0 {
            total_latency_ns / response_count
        } else {
            0
        };

        Metrics {
            uptime_secs: self.start_time.elapsed().as_secs(),
            request_count,
            response_count,
            error_count: self.error_count.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            avg_latency_ms: Duration::from_nanos(avg_latency_ns).as_millis() as u64,
            active_connections: self.active_connections.load(Ordering::Relaxed),
            websocket_connections: self.websocket_connections.load(Ordering::Relaxed),
            grpc_streams: self.grpc_streams.load(Ordering::Relaxed),
            breakpoint_hits: self.breakpoint_hits.load(Ordering::Relaxed),
            mock_hits: self.mock_hits.load(Ordering::Relaxed),
            rewrite_applications: self.rewrite_applications.load(Ordering::Relaxed),
            script_executions: self.script_executions.load(Ordering::Relaxed),
            plugin_invocations: self.plugin_invocations.load(Ordering::Relaxed),
            latency_histogram: self.latency_buckets.read().clone(),
            requests_per_second: self.calculate_rps(),
        }
    }

    /// Calculate requests per second
    fn calculate_rps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs();
        if elapsed == 0 {
            return 0.0;
        }
        let requests = self.request_count.load(Ordering::Relaxed);
        requests as f64 / elapsed as f64
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.request_count.store(0, Ordering::SeqCst);
        self.response_count.store(0, Ordering::SeqCst);
        self.error_count.store(0, Ordering::SeqCst);
        self.bytes_received.store(0, Ordering::SeqCst);
        self.bytes_sent.store(0, Ordering::SeqCst);
        self.total_latency_ns.store(0, Ordering::SeqCst);
        self.breakpoint_hits.store(0, Ordering::SeqCst);
        self.mock_hits.store(0, Ordering::SeqCst);
        self.rewrite_applications.store(0, Ordering::SeqCst);
        self.script_executions.store(0, Ordering::SeqCst);
        self.plugin_invocations.store(0, Ordering::SeqCst);
        self.latency_buckets.write().clear();
    }
}

/// Metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Total requests processed
    pub request_count: u64,
    /// Total responses processed
    pub response_count: u64,
    /// Total errors
    pub error_count: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: u64,
    /// Active HTTP connections
    pub active_connections: u64,
    /// Active WebSocket connections
    pub websocket_connections: u64,
    /// Active gRPC streams
    pub grpc_streams: u64,
    /// Breakpoint hits
    pub breakpoint_hits: u64,
    /// Mock hits
    pub mock_hits: u64,
    /// Rewrite applications
    pub rewrite_applications: u64,
    /// Script executions
    pub script_executions: u64,
    /// Plugin invocations
    pub plugin_invocations: u64,
    /// Latency histogram (bucket -> count)
    pub latency_histogram: HashMap<u64, u64>,
    /// Requests per second
    pub requests_per_second: f64,
}

/// Performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    /// Current metrics
    pub metrics: Metrics,
    /// Memory stats
    pub memory: MemoryInfo,
    /// Connection pool stats
    pub pool: PoolStats,
}

/// Memory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Used memory in bytes
    pub used_bytes: u64,
    /// Total system memory in bytes
    pub total_bytes: u64,
    /// Memory usage percentage
    pub usage_percent: f64,
}

/// Connection pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    /// Total connections in pool
    pub total_connections: u64,
    /// Idle connections
    pub idle_connections: u64,
    /// Active connections
    pub active_connections: u64,
    /// Pending connection requests
    pub pending_requests: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        collector.record_request(100);
        collector.record_response(200, Duration::from_millis(50));
        collector.record_error();

        let metrics = collector.snapshot();
        assert_eq!(metrics.request_count, 1);
        assert_eq!(metrics.response_count, 1);
        assert_eq!(metrics.error_count, 1);
        assert_eq!(metrics.bytes_sent, 100);
        assert_eq!(metrics.bytes_received, 200);
    }
}
