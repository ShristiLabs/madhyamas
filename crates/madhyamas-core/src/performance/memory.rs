//! Memory management and optimization

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Memory manager for traffic data
#[derive(Debug)]
pub struct MemoryManager {
    /// Maximum memory to use in bytes
    max_memory_bytes: AtomicU64,
    /// Current memory usage in bytes
    current_usage_bytes: AtomicU64,
    /// Maximum entries to keep
    max_entries: AtomicU64,
    /// Current entry count
    current_entries: AtomicU64,
    /// Garbage collection config
    gc_config: RwLock<GarbageCollectionConfig>,
    /// Last GC time
    last_gc: RwLock<Instant>,
    /// Auto GC enabled
    auto_gc_enabled: AtomicBool,
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self {
            max_memory_bytes: AtomicU64::new(500 * 1024 * 1024), // 500 MB
            current_usage_bytes: AtomicU64::new(0),
            max_entries: AtomicU64::new(100_000),
            current_entries: AtomicU64::new(0),
            gc_config: RwLock::new(GarbageCollectionConfig::default()),
            last_gc: RwLock::new(Instant::now()),
            auto_gc_enabled: AtomicBool::new(true),
        }
    }
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom limits
    pub fn with_limits(max_memory_mb: u64, max_entries: u64) -> Self {
        Self {
            max_memory_bytes: AtomicU64::new(max_memory_mb * 1024 * 1024),
            max_entries: AtomicU64::new(max_entries),
            ..Self::default()
        }
    }

    /// Set maximum memory in MB
    pub fn set_max_memory_mb(&self, mb: u64) {
        self.max_memory_bytes
            .store(mb * 1024 * 1024, Ordering::SeqCst);
    }

    /// Set maximum entries
    pub fn set_max_entries(&self, count: u64) {
        self.max_entries.store(count, Ordering::SeqCst);
    }

    /// Enable or disable auto GC
    pub fn set_auto_gc(&self, enabled: bool) {
        self.auto_gc_enabled.store(enabled, Ordering::SeqCst);
    }

    /// Update GC config
    pub fn set_gc_config(&self, config: GarbageCollectionConfig) {
        *self.gc_config.write() = config;
    }

    /// Record entry added
    pub fn entry_added(&self, size_bytes: u64) {
        self.current_usage_bytes
            .fetch_add(size_bytes, Ordering::SeqCst);
        self.current_entries.fetch_add(1, Ordering::SeqCst);
    }

    /// Record entry removed
    pub fn entry_removed(&self, size_bytes: u64) {
        self.current_usage_bytes
            .fetch_sub(size_bytes, Ordering::SeqCst);
        self.current_entries.fetch_sub(1, Ordering::SeqCst);
    }

    /// Check if memory pressure is high
    pub fn is_under_pressure(&self) -> bool {
        let usage = self.current_usage_bytes.load(Ordering::SeqCst);
        let max = self.max_memory_bytes.load(Ordering::SeqCst);
        let entries = self.current_entries.load(Ordering::SeqCst);
        let max_entries = self.max_entries.load(Ordering::SeqCst);

        usage > max * 80 / 100 || entries > max_entries * 80 / 100
    }

    /// Check if GC should run
    pub fn should_run_gc(&self) -> bool {
        if !self.auto_gc_enabled.load(Ordering::SeqCst) {
            return false;
        }

        let config = self.gc_config.read();
        let last_gc = *self.last_gc.read();

        // Check time-based trigger
        if last_gc.elapsed() > config.min_interval {
            return true;
        }

        // Check memory pressure trigger
        if self.is_under_pressure() {
            return true;
        }

        false
    }

    /// Mark GC as completed
    pub fn gc_completed(&self, freed_bytes: u64, freed_entries: u64) {
        self.current_usage_bytes
            .fetch_sub(freed_bytes, Ordering::SeqCst);
        self.current_entries
            .fetch_sub(freed_entries, Ordering::SeqCst);
        *self.last_gc.write() = Instant::now();
    }

    /// Get current memory stats
    pub fn stats(&self) -> MemoryStats {
        let usage = self.current_usage_bytes.load(Ordering::SeqCst);
        let max = self.max_memory_bytes.load(Ordering::SeqCst);
        let entries = self.current_entries.load(Ordering::SeqCst);
        let max_entries = self.max_entries.load(Ordering::SeqCst);

        MemoryStats {
            used_bytes: usage,
            max_bytes: max,
            usage_percent: if max > 0 {
                (usage as f64 / max as f64) * 100.0
            } else {
                0.0
            },
            entry_count: entries,
            max_entries,
            entry_usage_percent: if max_entries > 0 {
                (entries as f64 / max_entries as f64) * 100.0
            } else {
                0.0
            },
            is_under_pressure: self.is_under_pressure(),
            auto_gc_enabled: self.auto_gc_enabled.load(Ordering::SeqCst),
        }
    }

    /// Calculate recommended cleanup threshold
    pub fn calculate_cleanup_threshold(&self) -> u64 {
        let config = self.gc_config.read();
        let usage = self.current_usage_bytes.load(Ordering::SeqCst);
        let max = self.max_memory_bytes.load(Ordering::SeqCst);

        if usage > max {
            // Over limit - need aggressive cleanup
            usage - (max * config.target_usage_percent / 100)
        } else {
            0
        }
    }
}

/// Garbage collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GarbageCollectionConfig {
    /// Minimum interval between GC runs
    pub min_interval: Duration,
    /// Target memory usage percentage (0-100)
    pub target_usage_percent: u64,
    /// Aggressiveness (1-10, higher = more aggressive)
    pub aggressiveness: u8,
    /// Preserve recent entries (in seconds)
    pub preserve_recent_secs: u64,
}

impl Default for GarbageCollectionConfig {
    fn default() -> Self {
        Self {
            min_interval: Duration::from_secs(60),
            target_usage_percent: 70,
            aggressiveness: 5,
            preserve_recent_secs: 300, // 5 minutes
        }
    }
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Used memory in bytes
    pub used_bytes: u64,
    /// Maximum memory in bytes
    pub max_bytes: u64,
    /// Memory usage percentage
    pub usage_percent: f64,
    /// Current entry count
    pub entry_count: u64,
    /// Maximum entries
    pub max_entries: u64,
    /// Entry usage percentage
    pub entry_usage_percent: f64,
    /// Whether under memory pressure
    pub is_under_pressure: bool,
    /// Auto GC enabled
    pub auto_gc_enabled: bool,
}

impl MemoryStats {
    /// Format bytes as human-readable string
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
