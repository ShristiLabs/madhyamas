//! Performance optimization module
//!
//! This module provides:
//! - Metrics collection and reporting
//! - Memory management and optimization
//! - Connection pooling
//! - Performance monitoring

pub mod metrics;
pub mod memory;
pub mod pool;
pub mod monitor;

pub use metrics::{Metrics, MetricsCollector, PerformanceStats, MemoryInfo, PoolStats};
pub use memory::{MemoryManager, MemoryStats, GarbageCollectionConfig};
pub use pool::{ConnectionPool, PoolConfig, PooledConnection, PoolStatistics};
pub use monitor::{PerformanceMonitor, HealthCheck, AlertConfig, HealthStatus, Alert, AlertLevel};
