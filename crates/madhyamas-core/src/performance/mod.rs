//! Performance optimization module
//!
//! This module provides:
//! - Metrics collection and reporting
//! - Memory management and optimization
//! - Connection pooling
//! - Performance monitoring

pub mod memory;
pub mod metrics;
pub mod monitor;
pub mod pool;

pub use memory::{GarbageCollectionConfig, MemoryManager, MemoryStats};
pub use metrics::{MemoryInfo, Metrics, MetricsCollector, PerformanceStats, PoolStats};
pub use monitor::{Alert, AlertConfig, AlertLevel, HealthCheck, HealthStatus, PerformanceMonitor};
pub use pool::{ConnectionPool, PoolConfig, PoolStatistics, PooledConnection};
