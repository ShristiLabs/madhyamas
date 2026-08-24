//! Integration tests for the public performance API: alerting (append and
//! cooldown), memory manager limits and pressure, the connection pool, and
//! the metrics collector.

use std::collections::HashMap;
use std::time::Duration;

use madhyamas_core::performance::{
    AlertConfig, ConnectionPool, MemoryManager, MemoryStats, Metrics, MetricsCollector,
    PerformanceMonitor, PoolConfig,
};

fn hot_metrics() -> Metrics {
    Metrics {
        uptime_secs: 0,
        request_count: 10,
        response_count: 10,
        error_count: 0,
        bytes_received: 0,
        bytes_sent: 0,
        avg_latency_ms: 2000, // above default latency threshold
        active_connections: 0,
        websocket_connections: 0,
        grpc_streams: 0,
        breakpoint_hits: 0,
        mock_hits: 0,
        rewrite_applications: 0,
        script_executions: 0,
        plugin_invocations: 0,
        latency_histogram: HashMap::new(),
        requests_per_second: 100.0,
    }
}

#[test]
fn test_append_alerts_not_replace() {
    let monitor = PerformanceMonitor::new();
    let config = AlertConfig {
        enabled: true,
        cooldown_period_secs: 0, // no cooldown for the test
        ..AlertConfig::default()
    };
    monitor.set_config(config);

    let metrics = hot_metrics();

    monitor.check_metrics(&metrics);
    assert_eq!(monitor.get_alerts().len(), 1);

    // A second check should APPEND another alert, not replace the first.
    monitor.check_metrics(&metrics);
    assert_eq!(
        monitor.get_alerts().len(),
        2,
        "alerts should be appended, not replaced"
    );
}

#[test]
fn test_cooldown_suppresses_repeated_alerts() {
    let monitor = PerformanceMonitor::new();
    monitor.set_config(AlertConfig {
        enabled: true,
        cooldown_period_secs: 3600, // 1 hour cooldown
        ..AlertConfig::default()
    });

    let metrics = hot_metrics();

    monitor.check_metrics(&metrics);
    assert_eq!(monitor.get_alerts().len(), 1);

    // Within cooldown: should be suppressed.
    monitor.check_metrics(&metrics);
    assert_eq!(monitor.get_alerts().len(), 1);
}

#[test]
fn test_memory_alert() {
    let monitor = PerformanceMonitor::new();
    monitor.set_config(AlertConfig {
        enabled: true,
        cooldown_period_secs: 0,
        memory_threshold: 50.0,
        ..AlertConfig::default()
    });

    let stats = MemoryStats {
        used_bytes: 600 * 1024 * 1024,
        max_bytes: 1024 * 1024 * 1024,
        usage_percent: 58.6,
        entry_count: 0,
        max_entries: 0,
        entry_usage_percent: 0.0,
        is_under_pressure: true,
        auto_gc_enabled: true,
    };

    monitor.check_memory(&stats);
    assert_eq!(monitor.get_alerts().len(), 1);
    assert_eq!(
        monitor.get_health(),
        madhyamas_core::performance::HealthStatus::Critical
    );
}

#[test]
fn test_memory_manager() {
    let manager = MemoryManager::with_limits(100, 1000);

    manager.entry_added(1024);
    manager.entry_added(2048);

    let stats = manager.stats();
    assert_eq!(stats.used_bytes, 3072);
    assert_eq!(stats.entry_count, 2);
    assert!(!stats.is_under_pressure);
}

#[test]
fn test_memory_pressure() {
    let manager = MemoryManager::with_limits(1, 100); // 1 MB limit

    // Add entries to trigger pressure (> 80%)
    for _ in 0..90 {
        manager.entry_added(10_000); // 10 KB each
    }

    assert!(manager.is_under_pressure());
}

#[test]
fn test_connection_pool() {
    let pool = ConnectionPool::new(PoolConfig::default());

    // Create a connection
    let conn = pool.create("example.com");
    assert_eq!(conn.host, "example.com");
    assert_eq!(conn.use_count, 1);

    // Release it back
    pool.release(conn);

    // Get it back (should reuse)
    let reused = pool.get("example.com").unwrap();
    assert_eq!(reused.use_count, 2);

    let stats = pool.stats();
    assert_eq!(stats.total_created, 1);
    assert_eq!(stats.total_reused, 1);
}

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
