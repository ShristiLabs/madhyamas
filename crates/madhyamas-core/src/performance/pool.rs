//! Connection pooling for upstream connections

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Maximum connections per host
    pub max_connections_per_host: usize,
    /// Maximum idle time before connection is closed
    pub idle_timeout_secs: u64,
    /// Maximum connection lifetime
    pub max_lifetime_secs: u64,
    /// Connection timeout
    pub connect_timeout_secs: u64,
    /// Enable connection pooling
    pub enabled: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_host: 10,
            idle_timeout_secs: 60,
            max_lifetime_secs: 300,
            connect_timeout_secs: 10,
            enabled: true,
        }
    }
}

/// Pooled connection wrapper
#[derive(Debug)]
pub struct PooledConnection {
    /// Connection ID
    pub id: u64,
    /// Host this connection is for
    pub host: String,
    /// When the connection was created
    pub created_at: Instant,
    /// Last activity time
    pub last_used: Instant,
    /// Number of times used
    pub use_count: u64,
}

impl PooledConnection {
    /// Check if connection is expired
    pub fn is_expired(&self, config: &PoolConfig) -> bool {
        let now = Instant::now();

        // Check idle timeout
        if now.duration_since(self.last_used) > Duration::from_secs(config.idle_timeout_secs) {
            return true;
        }

        // Check max lifetime
        if now.duration_since(self.created_at) > Duration::from_secs(config.max_lifetime_secs) {
            return true;
        }

        false
    }
}

/// Connection pool for a single host
#[derive(Debug)]
struct HostPool {
    /// Idle connections ready for reuse
    idle: VecDeque<PooledConnection>,
    /// Total connections (idle + in-use)
    total: usize,
}

impl HostPool {
    fn new() -> Self {
        Self {
            idle: VecDeque::new(),
            total: 0,
        }
    }
}

/// Connection pool manager
#[derive(Debug)]
pub struct ConnectionPool {
    /// Per-host pools
    pools: Mutex<VecDeque<(String, HostPool)>>,
    /// Configuration
    config: PoolConfig,
    /// Total connections created
    total_created: AtomicU64,
    /// Total connections reused
    total_reused: AtomicU64,
    /// Total connections expired
    total_expired: AtomicU64,
    /// Next connection ID
    next_id: AtomicU64,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: PoolConfig) -> Self {
        Self {
            pools: Mutex::new(VecDeque::new()),
            config,
            total_created: AtomicU64::new(0),
            total_reused: AtomicU64::new(0),
            total_expired: AtomicU64::new(0),
            next_id: AtomicU64::new(1),
        }
    }

    /// Get a connection for a host
    pub fn get(&self, host: &str) -> Option<PooledConnection> {
        if !self.config.enabled {
            return None;
        }

        let mut pools = self.pools.lock();

        // Find or create host pool
        let pool_idx = pools.iter().position(|(h, _)| h == host);
        let host_pool = if let Some(idx) = pool_idx {
            &mut pools[idx].1
        } else {
            pools.push_back((host.to_string(), HostPool::new()));
            &mut pools.back_mut().unwrap().1
        };

        // Try to get an idle connection
        while let Some(mut conn) = host_pool.idle.pop_front() {
            if conn.is_expired(&self.config) {
                host_pool.total -= 1;
                self.total_expired.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            conn.last_used = Instant::now();
            conn.use_count += 1;
            self.total_reused.fetch_add(1, Ordering::Relaxed);
            return Some(conn);
        }

        None
    }

    /// Create a new connection
    pub fn create(&self, host: &str) -> PooledConnection {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Instant::now();

        let conn = PooledConnection {
            id,
            host: host.to_string(),
            created_at: now,
            last_used: now,
            use_count: 1,
        };

        {
            let mut pools = self.pools.lock();
            let pool_idx = pools.iter().position(|(h, _)| h == host);
            let host_pool = if let Some(idx) = pool_idx {
                &mut pools[idx].1
            } else {
                pools.push_back((host.to_string(), HostPool::new()));
                &mut pools.back_mut().unwrap().1
            };

            host_pool.total += 1;
        }

        self.total_created.fetch_add(1, Ordering::Relaxed);
        conn
    }

    /// Return a connection to the pool
    pub fn release(&self, conn: PooledConnection) {
        if !self.config.enabled {
            return;
        }

        let mut pools = self.pools.lock();

        if let Some(pool_idx) = pools.iter().position(|(h, _)| *h == conn.host) {
            let host_pool = &mut pools[pool_idx].1;

            if host_pool.idle.len() < self.config.max_connections_per_host {
                host_pool.idle.push_back(conn);
            }
            // Otherwise, let it drop (connection closed)
        }
    }

    /// Cleanup expired connections
    pub fn cleanup(&self) {
        let mut pools = self.pools.lock();
        let mut expired_count = 0;

        for (_, host_pool) in pools.iter_mut() {
            let before = host_pool.idle.len();
            host_pool.idle.retain(|conn| !conn.is_expired(&self.config));
            let removed = before - host_pool.idle.len();
            host_pool.total -= removed;
            expired_count += removed;
        }

        // Remove empty pools
        pools.retain(|(_, p)| p.total > 0);

        self.total_expired
            .fetch_add(expired_count as u64, Ordering::Relaxed);
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStatistics {
        let pools = self.pools.lock();

        let mut total_idle = 0;
        let mut total_active = 0;
        let mut host_count = 0;

        for (_, host_pool) in pools.iter() {
            total_idle += host_pool.idle.len();
            total_active += host_pool.total - host_pool.idle.len();
            host_count += 1;
        }

        PoolStatistics {
            host_count,
            total_connections: total_idle + total_active,
            idle_connections: total_idle,
            active_connections: total_active,
            total_created: self.total_created.load(Ordering::Relaxed),
            total_reused: self.total_reused.load(Ordering::Relaxed),
            total_expired: self.total_expired.load(Ordering::Relaxed),
            reuse_rate: self.calculate_reuse_rate(),
        }
    }

    fn calculate_reuse_rate(&self) -> f64 {
        let created = self.total_created.load(Ordering::Relaxed);
        let reused = self.total_reused.load(Ordering::Relaxed);

        if created + reused == 0 {
            return 0.0;
        }

        (reused as f64 / (created + reused) as f64) * 100.0
    }
}

/// Pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatistics {
    /// Number of hosts with connections
    pub host_count: usize,
    /// Total connections in pool
    pub total_connections: usize,
    /// Idle connections
    pub idle_connections: usize,
    /// Active connections
    pub active_connections: usize,
    /// Total connections created
    pub total_created: u64,
    /// Total connections reused
    pub total_reused: u64,
    /// Total connections expired
    pub total_expired: u64,
    /// Reuse rate percentage
    pub reuse_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
