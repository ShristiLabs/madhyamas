//! Bandwidth throttling and network simulation

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Pre-defined network profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrottleProfile {
    /// Profile name
    pub name: String,
    /// Download bandwidth in bytes per second (0 = unlimited)
    pub download_bps: u64,
    /// Upload bandwidth in bytes per second (0 = unlimited)
    pub upload_bps: u64,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Jitter in milliseconds (random variation)
    pub jitter_ms: u64,
    /// Packet loss percentage (0-100)
    pub packet_loss_percent: u8,
}

impl ThrottleProfile {
    /// No throttling
    pub fn none() -> Self {
        Self {
            name: "None".to_string(),
            download_bps: 0,
            upload_bps: 0,
            latency_ms: 0,
            jitter_ms: 0,
            packet_loss_percent: 0,
        }
    }

    /// GPRS (2G) - Very slow
    pub fn gprs() -> Self {
        Self {
            name: "GPRS (2G)".to_string(),
            download_bps: 50_000, // ~50 KB/s
            upload_bps: 20_000,   // ~20 KB/s
            latency_ms: 500,
            jitter_ms: 100,
            packet_loss_percent: 2,
        }
    }

    /// EDGE (2G) - Slow
    pub fn edge() -> Self {
        Self {
            name: "EDGE (2G)".to_string(),
            download_bps: 200_000, // ~200 KB/s
            upload_bps: 100_000,   // ~100 KB/s
            latency_ms: 300,
            jitter_ms: 50,
            packet_loss_percent: 1,
        }
    }

    /// 3G
    pub fn three_g() -> Self {
        Self {
            name: "3G".to_string(),
            download_bps: 1_000_000, // ~1 MB/s
            upload_bps: 500_000,     // ~500 KB/s
            latency_ms: 100,
            jitter_ms: 20,
            packet_loss_percent: 0,
        }
    }

    /// 4G LTE
    pub fn four_g() -> Self {
        Self {
            name: "4G LTE".to_string(),
            download_bps: 10_000_000, // ~10 MB/s
            upload_bps: 5_000_000,    // ~5 MB/s
            latency_ms: 30,
            jitter_ms: 10,
            packet_loss_percent: 0,
        }
    }

    /// Slow 3G (good for testing)
    pub fn slow_3g() -> Self {
        Self {
            name: "Slow 3G".to_string(),
            download_bps: 400_000, // ~400 KB/s
            upload_bps: 200_000,   // ~200 KB/s
            latency_ms: 200,
            jitter_ms: 50,
            packet_loss_percent: 0,
        }
    }

    /// High latency satellite
    pub fn satellite() -> Self {
        Self {
            name: "Satellite".to_string(),
            download_bps: 5_000_000, // ~5 MB/s
            upload_bps: 2_000_000,   // ~2 MB/s
            latency_ms: 600,
            jitter_ms: 100,
            packet_loss_percent: 1,
        }
    }

    /// DSL
    pub fn dsl() -> Self {
        Self {
            name: "DSL".to_string(),
            download_bps: 2_000_000, // ~2 MB/s
            upload_bps: 500_000,     // ~500 KB/s
            latency_ms: 20,
            jitter_ms: 5,
            packet_loss_percent: 0,
        }
    }

    /// Custom profile
    pub fn custom(name: &str, download_bps: u64, upload_bps: u64, latency_ms: u64) -> Self {
        Self {
            name: name.to_string(),
            download_bps,
            upload_bps,
            latency_ms,
            jitter_ms: 0,
            packet_loss_percent: 0,
        }
    }

    /// Get all predefined profiles
    pub fn all() -> Vec<Self> {
        vec![
            Self::none(),
            Self::gprs(),
            Self::edge(),
            Self::three_g(),
            Self::slow_3g(),
            Self::four_g(),
            Self::dsl(),
            Self::satellite(),
        ]
    }

    /// Calculate actual latency with jitter
    pub fn effective_latency(&self) -> Duration {
        use rand::Rng;
        let jitter = if self.jitter_ms > 0 {
            rand::thread_rng().gen_range(0..=self.jitter_ms)
        } else {
            0
        };
        Duration::from_millis(self.latency_ms + jitter)
    }

    /// Check if we should drop a packet
    pub fn should_drop_packet(&self) -> bool {
        if self.packet_loss_percent == 0 {
            return false;
        }
        use rand::Rng;
        rand::thread_rng().gen_ratio(self.packet_loss_percent as u32, 100)
    }
}

/// Manages throttling state
pub struct ThrottleManager {
    /// Current active profile
    profile: RwLock<ThrottleProfile>,
    /// Whether throttling is enabled
    enabled: RwLock<bool>,
}

impl ThrottleManager {
    pub fn new() -> Self {
        Self {
            profile: RwLock::new(ThrottleProfile::none()),
            enabled: RwLock::new(false),
        }
    }

    /// Set the throttle profile
    pub fn set_profile(&self, profile: ThrottleProfile) {
        *self.profile.write() = profile;
    }

    /// Get the current profile
    pub fn get_profile(&self) -> ThrottleProfile {
        self.profile.read().clone()
    }

    /// Enable/disable throttling
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.write() = enabled;
    }

    /// Check if throttling is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    /// Apply latency delay
    pub async fn apply_latency(&self) {
        if !self.is_enabled() {
            return;
        }
        let delay = self.profile.read().effective_latency();
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
    }

    /// Apply packet loss check - returns true if packet should be dropped
    pub fn check_packet_loss(&self) -> bool {
        if !self.is_enabled() {
            return false;
        }
        self.profile.read().should_drop_packet()
    }

    /// Calculate time to transfer data (for bandwidth limiting)
    pub fn transfer_time(&self, bytes: usize, is_upload: bool) -> Duration {
        if !self.is_enabled() {
            return Duration::ZERO;
        }

        let profile = self.profile.read();
        let bps = if is_upload {
            profile.upload_bps
        } else {
            profile.download_bps
        };

        if bps == 0 {
            return Duration::ZERO;
        }

        Duration::from_millis((bytes as u64 * 1000) / bps)
    }

    /// Throttle data transfer (call during read/write operations)
    pub async fn throttle_transfer(&self, bytes: usize, is_upload: bool) {
        let transfer_time = self.transfer_time(bytes, is_upload);
        if !transfer_time.is_zero() {
            tokio::time::sleep(transfer_time).await;
        }
    }
}

impl Default for ThrottleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for throttled I/O operations
pub trait ThrottledIO {
    /// Read with throttling applied
    async fn throttled_read(
        &self,
        buf: &mut [u8],
        throttle: &ThrottleManager,
    ) -> std::io::Result<usize>;

    /// Write with throttling applied
    async fn throttled_write(
        &self,
        buf: &[u8],
        throttle: &ThrottleManager,
    ) -> std::io::Result<usize>;
}

#[cfg(test)]
mod tests {
    use super::*;

    mod throttle_profile_tests {
        use super::*;

        #[test]
        fn test_none_profile() {
            let profile = ThrottleProfile::none();

            assert_eq!(profile.name, "None");
            assert_eq!(profile.download_bps, 0);
            assert_eq!(profile.upload_bps, 0);
            assert_eq!(profile.latency_ms, 0);
            assert_eq!(profile.jitter_ms, 0);
            assert_eq!(profile.packet_loss_percent, 0);
        }

        #[test]
        fn test_gprs_profile() {
            let profile = ThrottleProfile::gprs();

            assert_eq!(profile.name, "GPRS (2G)");
            assert_eq!(profile.download_bps, 50_000);
            assert_eq!(profile.upload_bps, 20_000);
            assert_eq!(profile.latency_ms, 500);
            assert_eq!(profile.jitter_ms, 100);
            assert_eq!(profile.packet_loss_percent, 2);
        }

        #[test]
        fn test_edge_profile() {
            let profile = ThrottleProfile::edge();

            assert_eq!(profile.name, "EDGE (2G)");
            assert_eq!(profile.download_bps, 200_000);
            assert_eq!(profile.upload_bps, 100_000);
            assert_eq!(profile.latency_ms, 300);
            assert_eq!(profile.jitter_ms, 50);
            assert_eq!(profile.packet_loss_percent, 1);
        }

        #[test]
        fn test_3g_profile() {
            let profile = ThrottleProfile::three_g();

            assert_eq!(profile.name, "3G");
            assert_eq!(profile.download_bps, 1_000_000);
            assert_eq!(profile.upload_bps, 500_000);
            assert_eq!(profile.latency_ms, 100);
            assert_eq!(profile.jitter_ms, 20);
            assert_eq!(profile.packet_loss_percent, 0);
        }

        #[test]
        fn test_slow_3g_profile() {
            let profile = ThrottleProfile::slow_3g();

            assert_eq!(profile.name, "Slow 3G");
            assert_eq!(profile.download_bps, 400_000);
            assert_eq!(profile.upload_bps, 200_000);
            assert_eq!(profile.latency_ms, 200);
        }

        #[test]
        fn test_4g_profile() {
            let profile = ThrottleProfile::four_g();

            assert_eq!(profile.name, "4G LTE");
            assert_eq!(profile.download_bps, 10_000_000);
            assert_eq!(profile.upload_bps, 5_000_000);
            assert_eq!(profile.latency_ms, 30);
            assert_eq!(profile.jitter_ms, 10);
        }

        #[test]
        fn test_dsl_profile() {
            let profile = ThrottleProfile::dsl();

            assert_eq!(profile.name, "DSL");
            assert_eq!(profile.download_bps, 2_000_000);
            assert_eq!(profile.upload_bps, 500_000);
            assert_eq!(profile.latency_ms, 20);
        }

        #[test]
        fn test_satellite_profile() {
            let profile = ThrottleProfile::satellite();

            assert_eq!(profile.name, "Satellite");
            assert_eq!(profile.download_bps, 5_000_000);
            assert_eq!(profile.upload_bps, 2_000_000);
            assert_eq!(profile.latency_ms, 600);
            assert_eq!(profile.packet_loss_percent, 1);
        }

        #[test]
        fn test_custom_profile() {
            let profile = ThrottleProfile::custom("Custom Test", 500_000, 250_000, 150);

            assert_eq!(profile.name, "Custom Test");
            assert_eq!(profile.download_bps, 500_000);
            assert_eq!(profile.upload_bps, 250_000);
            assert_eq!(profile.latency_ms, 150);
            assert_eq!(profile.jitter_ms, 0);
            assert_eq!(profile.packet_loss_percent, 0);
        }

        #[test]
        fn test_all_profiles() {
            let profiles = ThrottleProfile::all();

            assert_eq!(profiles.len(), 8);
            assert!(profiles.iter().any(|p| p.name == "None"));
            assert!(profiles.iter().any(|p| p.name == "GPRS (2G)"));
            assert!(profiles.iter().any(|p| p.name == "EDGE (2G)"));
            assert!(profiles.iter().any(|p| p.name == "3G"));
            assert!(profiles.iter().any(|p| p.name == "Slow 3G"));
            assert!(profiles.iter().any(|p| p.name == "4G LTE"));
            assert!(profiles.iter().any(|p| p.name == "DSL"));
            assert!(profiles.iter().any(|p| p.name == "Satellite"));
        }

        #[test]
        fn test_effective_latency_no_jitter() {
            let profile = ThrottleProfile::custom("No Jitter", 0, 0, 100);

            for _ in 0..10 {
                let latency = profile.effective_latency();
                assert_eq!(latency, Duration::from_millis(100));
            }
        }

        #[test]
        fn test_effective_latency_with_jitter() {
            let profile = ThrottleProfile {
                name: "Jitter Test".to_string(),
                download_bps: 0,
                upload_bps: 0,
                latency_ms: 100,
                jitter_ms: 50,
                packet_loss_percent: 0,
            };

            // Check that latency is within expected range (100-150ms)
            for _ in 0..20 {
                let latency = profile.effective_latency();
                let ms = latency.as_millis();
                assert!(
                    ms >= 100 && ms <= 150,
                    "Latency {}ms not in range 100-150",
                    ms
                );
            }
        }

        #[test]
        fn test_should_drop_packet_no_loss() {
            let profile = ThrottleProfile::none();

            for _ in 0..100 {
                assert!(!profile.should_drop_packet());
            }
        }

        #[test]
        fn test_serialization() {
            let profile = ThrottleProfile::three_g();
            let json = serde_json::to_string(&profile).unwrap();

            assert!(json.contains("\"name\":\"3G\""));
            assert!(json.contains("\"download_bps\":1000000"));
            assert!(json.contains("\"latency_ms\":100"));
        }

        #[test]
        fn test_deserialization() {
            let json = r#"{
                "name": "Test Profile",
                "download_bps": 500000,
                "upload_bps": 250000,
                "latency_ms": 75,
                "jitter_ms": 25,
                "packet_loss_percent": 5
            }"#;

            let profile: ThrottleProfile = serde_json::from_str(json).unwrap();
            assert_eq!(profile.name, "Test Profile");
            assert_eq!(profile.download_bps, 500_000);
            assert_eq!(profile.upload_bps, 250_000);
            assert_eq!(profile.latency_ms, 75);
            assert_eq!(profile.jitter_ms, 25);
            assert_eq!(profile.packet_loss_percent, 5);
        }
    }

    mod throttle_manager_tests {
        use super::*;

        #[test]
        fn test_new() {
            let manager = ThrottleManager::new();

            assert!(!manager.is_enabled());
            assert_eq!(manager.get_profile().name, "None");
        }

        #[test]
        fn test_default() {
            let manager = ThrottleManager::default();

            assert!(!manager.is_enabled());
        }

        #[test]
        fn test_set_enabled() {
            let manager = ThrottleManager::new();

            manager.set_enabled(true);
            assert!(manager.is_enabled());

            manager.set_enabled(false);
            assert!(!manager.is_enabled());
        }

        #[test]
        fn test_set_profile() {
            let manager = ThrottleManager::new();

            let profile = ThrottleProfile::three_g();
            manager.set_profile(profile);

            let current = manager.get_profile();
            assert_eq!(current.name, "3G");
        }

        #[test]
        fn test_transfer_time_disabled() {
            let manager = ThrottleManager::new();
            // Disabled by default

            let time = manager.transfer_time(1_000_000, true);
            assert_eq!(time, Duration::ZERO);

            let time = manager.transfer_time(1_000_000, false);
            assert_eq!(time, Duration::ZERO);
        }

        #[test]
        fn test_transfer_time_enabled() {
            let manager = ThrottleManager::new();
            manager.set_enabled(true);
            manager.set_profile(ThrottleProfile::three_g());

            // 3G: 1 MB/s download
            let time = manager.transfer_time(1_000_000, false);
            assert_eq!(time, Duration::from_millis(1000));

            // 3G: 500 KB/s upload
            let time = manager.transfer_time(500_000, true);
            assert_eq!(time, Duration::from_millis(1000));
        }

        #[test]
        fn test_transfer_time_unlimited() {
            let manager = ThrottleManager::new();
            manager.set_enabled(true);
            manager.set_profile(ThrottleProfile::none());

            let time = manager.transfer_time(1_000_000, false);
            assert_eq!(time, Duration::ZERO);
        }

        #[test]
        fn test_check_packet_loss_disabled() {
            let manager = ThrottleManager::new();
            manager.set_profile(ThrottleProfile::gprs()); // Has 2% packet loss

            // Disabled, should never drop
            for _ in 0..100 {
                assert!(!manager.check_packet_loss());
            }
        }

        #[tokio::test]
        async fn test_apply_latency_disabled() {
            let manager = ThrottleManager::new();
            manager.set_profile(ThrottleProfile::satellite()); // 600ms latency

            let start = std::time::Instant::now();
            manager.apply_latency().await;
            let elapsed = start.elapsed();

            // Should be near instant when disabled
            assert!(elapsed.as_millis() < 50);
        }
    }
}
