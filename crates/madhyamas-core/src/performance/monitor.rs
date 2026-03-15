//! Performance monitoring and alerting

use std::collections::HashMap;
use std::time::Instant;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::Metrics;

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

/// Performance monitor
#[derive(Debug)]
#[allow(dead_code)]
pub struct PerformanceMonitor {
    config: AlertConfig,
    alerts: RwLock<Vec<Alert>>,
    last_check: RwLock<Instant>,
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self {
            config: AlertConfig::default(),
            alerts: RwLock::new(Vec::new()),
            last_check: RwLock::new(Instant::now()),
        }
    }
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: AlertConfig) -> Self {
        Self {
            config,
            alerts: RwLock::new(Vec::new()),
            last_check: RwLock::new(Instant::now()),
        }
    }

    pub fn check_metrics(&self, metrics: &Metrics) {
        if !self.config.enabled {
            return;
        }

        let mut new_alerts = Vec::new();

        if metrics.avg_latency_ms >= self.config.latency_threshold_ms {
            new_alerts.push(Alert {
                level: AlertLevel::Warning,
                message: format!("High latency: {}ms", metrics.avg_latency_ms),
                timestamp: chrono::Utc::now().timestamp(),
            });
        }

        *self.alerts.write() = new_alerts;
    }

    pub fn get_alerts(&self) -> Vec<Alert> {
        self.alerts.read().clone()
    }

    pub fn clear_alerts(&self) {
        self.alerts.write().clear();
    }

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

    pub fn set_config(&mut self, config: AlertConfig) {
        self.config = config;
    }
}
