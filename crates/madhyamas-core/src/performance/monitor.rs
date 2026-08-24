//! Performance monitoring and alerting
//!
//! The [`PerformanceMonitor`] periodically inspects the running proxy's
//! metrics and memory usage, emitting [`Alert`]s when configured thresholds
//! are exceeded. Alerts are **appended** (not replaced) and rate-limited by a
//! per-kind cooldown so that sustained issues do not flood the alert log.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sysinfo::System;
use tokio::task::JoinHandle;
use tracing::debug;

use super::{MemoryManager, MemoryStats, Metrics, MetricsCollector};

/// Maximum number of alerts retained in memory.
const MAX_ALERTS: usize = 200;

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub enabled: bool,
    pub memory_threshold: f64,
    pub error_rate_threshold: f64,
    pub latency_threshold_ms: u64,
    pub throughput_threshold: f64,
    pub cooldown_period_secs: u64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            memory_threshold: 85.0,
            error_rate_threshold: 10.0,
            latency_threshold_ms: 1000,
            throughput_threshold: 10.0,
            cooldown_period_secs: 300,
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub healthy: bool,
    pub version: String,
    pub uptime_secs: u64,
    pub memory_usage_mb: u64,
    pub active_connections: u64,
    pub details: HashMap<String, String>,
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            healthy: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_secs: 0,
            memory_usage_mb: 0,
            active_connections: 0,
            details: HashMap::new(),
        }
    }
}

/// Health status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Alert level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

/// Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub level: AlertLevel,
    pub message: String,
    pub timestamp: i64,
}

/// A logical category for an alert, used to enforce per-kind cooldowns so
/// that repeated threshold violations do not flood the alert log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AlertKind {
    HighLatency,
    HighErrorRate,
    LowThroughput,
    HighMemory,
}

/// Performance monitor
#[derive(Debug)]
pub struct PerformanceMonitor {
    config: RwLock<AlertConfig>,
    alerts: RwLock<Vec<Alert>>,
    last_check: RwLock<Instant>,
    /// Last time an alert of each kind was emitted, for cooldown enforcement.
    last_alert: RwLock<HashMap<AlertKind, Instant>>,
    /// Handle for the background monitoring task (if started).
    monitor_task: RwLock<Option<JoinHandle<()>>>,
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self {
            config: RwLock::new(AlertConfig::default()),
            alerts: RwLock::new(Vec::new()),
            last_check: RwLock::new(Instant::now()),
            last_alert: RwLock::new(HashMap::new()),
            monitor_task: RwLock::new(None),
        }
    }
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: AlertConfig) -> Self {
        Self {
            config: RwLock::new(config),
            alerts: RwLock::new(Vec::new()),
            last_check: RwLock::new(Instant::now()),
            last_alert: RwLock::new(HashMap::new()),
            monitor_task: RwLock::new(None),
        }
    }

    /// Update the alert configuration at runtime.
    pub fn set_config(&self, config: AlertConfig) {
        *self.config.write() = config;
    }

    /// Returns true if a cooldown for the given alert kind has elapsed (or no
    /// prior alert of this kind exists). When true, the cooldown timer is
    /// updated so subsequent emissions within the cooldown are suppressed.
    fn try_emit(&self, kind: AlertKind) -> bool {
        let cooldown = Duration::from_secs(self.config.read().cooldown_period_secs);
        let now = Instant::now();
        let mut last = self.last_alert.write();
        if let Some(prev) = last.get(&kind) {
            if now.duration_since(*prev) < cooldown {
                return false;
            }
        }
        last.insert(kind, now);
        true
    }

    /// Append an alert (respecting the per-kind cooldown). Returns true if the
    /// alert was actually appended, false if it was suppressed by cooldown.
    fn push_alert(&self, kind: AlertKind, level: AlertLevel, message: String) -> bool {
        if !self.try_emit(kind) {
            return false;
        }
        let alert = Alert {
            level,
            message,
            timestamp: chrono::Utc::now().timestamp(),
        };
        let mut alerts = self.alerts.write();
        alerts.push(alert);
        // Bound the alert log to avoid unbounded growth.
        if alerts.len() > MAX_ALERTS {
            let drop = alerts.len() - MAX_ALERTS;
            alerts.drain(0..drop);
        }
        true
    }

    /// Inspect a metrics snapshot and emit alerts for any threshold violations.
    ///
    /// Unlike the previous implementation, this **appends** new alerts to the
    /// existing list (instead of replacing it) and rate-limits each alert kind
    /// via the configured cooldown period.
    pub fn check_metrics(&self, metrics: &Metrics) {
        if !self.config.read().enabled {
            return;
        }
        let config = self.config.read().clone();

        if metrics.avg_latency_ms >= config.latency_threshold_ms {
            self.push_alert(
                AlertKind::HighLatency,
                AlertLevel::Warning,
                format!("High latency: {}ms", metrics.avg_latency_ms),
            );
        }

        // Error rate = errors / responses * 100
        if metrics.response_count > 0 {
            let error_rate = (metrics.error_count as f64 / metrics.response_count as f64) * 100.0;
            if error_rate >= config.error_rate_threshold {
                self.push_alert(
                    AlertKind::HighErrorRate,
                    AlertLevel::Warning,
                    format!("High error rate: {:.1}%", error_rate),
                );
            }
        }

        // Throughput below threshold only meaningful once there is traffic.
        if metrics.request_count > 0 && metrics.requests_per_second < config.throughput_threshold {
            self.push_alert(
                AlertKind::LowThroughput,
                AlertLevel::Info,
                format!("Low throughput: {:.1} req/s", metrics.requests_per_second),
            );
        }

        *self.last_check.write() = Instant::now();
    }

    /// Inspect memory stats (from the [`MemoryManager`] or the OS) and emit an
    /// alert when memory usage crosses the configured threshold.
    pub fn check_memory(&self, stats: &MemoryStats) {
        if !self.config.read().enabled {
            return;
        }
        let threshold = self.config.read().memory_threshold;
        if stats.usage_percent >= threshold {
            self.push_alert(
                AlertKind::HighMemory,
                AlertLevel::Critical,
                format!(
                    "High memory usage: {:.1}% ({} / {})",
                    stats.usage_percent,
                    MemoryStats::format_bytes(stats.used_bytes),
                    MemoryStats::format_bytes(stats.max_bytes),
                ),
            );
        }
    }

    /// Query real system state (process RSS and total system memory) and build
    /// a [`HealthCheck`]. This replaces the previous stub that always returned
    /// a healthy status with zeroed-out fields.
    pub fn system_health(&self, metrics: &Metrics, memory: &MemoryStats) -> HealthCheck {
        let mut sys = System::new_all();
        sys.refresh_all();

        let pid = sysinfo::Pid::from(std::process::id() as usize);
        let process_memory_bytes = sys.process(pid).map(|p| p.memory()).unwrap_or(0);
        let memory_usage_mb = process_memory_bytes / 1024 / 1024;

        let mut details = HashMap::new();
        details.insert("process_memory_mb".to_string(), memory_usage_mb.to_string());
        details.insert(
            "traffic_memory_percent".to_string(),
            format!("{:.1}", memory.usage_percent),
        );
        details.insert(
            "requests_per_second".to_string(),
            format!("{:.1}", metrics.requests_per_second),
        );
        details.insert(
            "active_connections".to_string(),
            metrics.active_connections.to_string(),
        );
        details.insert(
            "websocket_connections".to_string(),
            metrics.websocket_connections.to_string(),
        );

        let healthy = self.get_health() != HealthStatus::Critical;

        HealthCheck {
            healthy,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_secs: metrics.uptime_secs,
            memory_usage_mb,
            active_connections: metrics.active_connections,
            details,
        }
    }

    /// Get a snapshot of the current alert log (oldest first).
    pub fn get_alerts(&self) -> Vec<Alert> {
        self.alerts.read().clone()
    }

    /// Clear all alerts.
    pub fn clear_alerts(&self) {
        self.alerts.write().clear();
        self.last_alert.write().clear();
    }

    /// Derive an overall health status from the current alert log.
    pub fn get_health(&self) -> HealthStatus {
        let alerts = self.alerts.read();
        if alerts.iter().any(|a| a.level == AlertLevel::Critical) {
            HealthStatus::Critical
        } else if alerts.iter().any(|a| a.level == AlertLevel::Warning) {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        }
    }

    /// Start a background task that periodically checks metrics and memory
    /// and emits alerts. The task runs until [`Self::stop_monitoring`] is
    /// called or the monitor is dropped.
    ///
    /// This is the main entry point used by the proxy engine: the engine
    /// owns an `Arc<PerformanceMonitor>` and calls `start_monitoring` once
    /// after construction, passing its `MetricsCollector` and
    /// `MemoryManager` handles.
    pub fn start_monitoring(
        self: &Arc<Self>,
        metrics: Arc<MetricsCollector>,
        memory: Arc<MemoryManager>,
        interval: Duration,
    ) {
        // Avoid starting a second task if one is already running.
        if self.monitor_task.read().is_some() {
            return;
        }

        let monitor = self.clone();
        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            // The first tick fires immediately; skip it so we don't alert on
            // startup before any traffic has been recorded.
            ticker.tick().await;
            loop {
                ticker.tick().await;
                let snapshot = metrics.snapshot();
                monitor.check_metrics(&snapshot);
                let mem_stats = memory.stats();
                monitor.check_memory(&mem_stats);

                let health = monitor.get_health();
                if health != HealthStatus::Healthy {
                    debug!(
                        "Performance monitor: health={:?}, alerts={}",
                        health,
                        monitor.get_alerts().len()
                    );
                }
            }
        });

        *self.monitor_task.write() = Some(handle);
    }

    /// Stop the background monitoring task, if running.
    pub fn stop_monitoring(&self) {
        if let Some(handle) = self.monitor_task.write().take() {
            handle.abort();
        }
    }
}

impl Drop for PerformanceMonitor {
    fn drop(&mut self) {
        self.stop_monitoring();
    }
}
