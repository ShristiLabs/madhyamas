//! Connection pooling for upstream connections
//!
//! # Status: intentionally unused
//!
//! This module implements a [`ConnectionPool`] that tracks reusable upstream
//! connections on a per-host basis. It is **not currently wired into the proxy
//! engine**, by design:
//!
//! * **HTTP/HTTPS forwarding** is performed via `reqwest::Client` (see the C1
//!   fix in `proxy::pipeline`), which maintains its own internal connection
//!   pool with keep-alive support. Layering a second pool on top would be
//!   redundant and could interfere with reqwest's own lifecycle management.
//! * **WebSocket upstream connections** (see `ProxyEngine::handle_websocket_upgrade_*`)
//!   use raw `TcpStream`/`TlsStream` sockets, but these are long-lived,
//!   bidirectional streams that stay open until either side disconnects. They
//!   are not short-lived request/response connections, so pooling them would
//!   not improve throughput.
//! * The [`PooledConnection`] type only carries connection **metadata**
//!   (id, host, timestamps, use count) — it does not hold an actual socket.
//!   As a result the pool is a bookkeeping/tracking structure rather than a
//!   true socket pool, and wiring it in would add complexity without a
//!   concrete benefit.
//!
//! The implementation is retained for future use (e.g. if the engine moves
//! away from `reqwest` for HTTP forwarding, or if short-lived raw-TCP upstream
//! connections are introduced). All public items are annotated with
//! `#[allow(dead_code)]` so the unused state is explicit and intentional.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Connection pool configuration
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    #[allow(dead_code)]
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
#[allow(dead_code)]
#[derive(Debug)]
struct HostPool {
    /// Idle connections ready for reuse
    idle: VecDeque<PooledConnection>,
    /// Total connections (idle + in-use)
    total: usize,
}

impl HostPool {
    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            idle: VecDeque::new(),
            total: 0,
        }
    }
}

/// Connection pool manager
#[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    #[allow(dead_code)]
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
#[allow(dead_code)]
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
