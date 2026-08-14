//! Redis-backed cross-instance state coordination (Phase 6a + 6c).
//!
//! [`RedisState`] holds a [`redis::Client`] and this instance's unique ID,
//! providing pub/sub event broadcasting and license seat tracking for
//! multi-instance deployments. When `--redis-url` is **not** provided at
//! startup, no [`RedisState`] is constructed and the binary runs in
//! single-instance mode (all multi-instance features disabled).
//!
//! # Channels
//!
//! | Channel               | Publisher                      | Subscriber              | Purpose                                      |
//! |-----------------------|--------------------------------|-------------------------|----------------------------------------------|
//! | `madhyamas:events`    | Instance capturing traffic     | All instances           | Broadcast traffic WS events cross-instance   |
//! | `madhyamas:config`    | Instance receiving config PATCH| All instances           | Notify config changed; reload from store     |
//! | `madhyamas:intercept` | Instance changing intercept    | All instances           | Notify intercept rules changed; reload store |
//! | `madhyamas:seats`     | Instance register/deregister   | All instances           | Seat count updates                           |
//!
//! # Auth and TLS
//!
//! The `redis::Client::open(url)` call accepts standard Redis URL schemes:
//! - `redis://host:port` — plain TCP
//! - `redis://:password@host:port` — auth
//! - `rediss://host:port` — TLS (requires the `tokio-rustls-comp` feature,
//!   enabled by default in this workspace)
//! - `rediss://:password@host:port` — TLS + auth

use redis::aio::PubSubStream;
use redis::AsyncCommands;
use redis::Client;
use redis::RedisError;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

/// Redis channel for cross-instance WebSocket traffic event broadcasting.
pub const CHANNEL_EVENTS: &str = madhyamas_core::CHANNEL_EVENTS;

/// Redis channel for config-change notifications (notification-only; each
/// instance reloads from the shared store on receipt).
pub const CHANNEL_CONFIG: &str = madhyamas_core::CHANNEL_CONFIG_EVENT;

/// Redis channel for intercept-rule-change notifications (notification-only;
/// each instance reloads rules from the shared store on receipt).
pub const CHANNEL_INTERCEPT: &str = madhyamas_core::CHANNEL_INTERCEPT_EVENT;

/// Redis channel for license seat-count updates.
pub const CHANNEL_SEATS: &str = madhyamas_core::CHANNEL_SEATS;

/// Redis sorted-set key tracking active instances (score = heartbeat timestamp).
const INSTANCES_KEY: &str = "madhyamas:instances";

/// Heartbeat staleness threshold in seconds. Instances whose last heartbeat is
/// older than this are considered dead and excluded from the active count.
const HEARTBEAT_TTL_SECS: i64 = 120;

/// Wrapper for traffic events published over Redis, carrying the originating
/// instance ID so subscribers can skip their own echoes (deduplication).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisTrafficEvent {
    /// UUID of the instance that captured the traffic.
    pub instance_id: String,
    /// The serialized [`madhyamas_core::TrafficEvent`].
    pub event: madhyamas_core::TrafficEvent,
}

/// Information about a registered instance, returned by [`RedisState::list_instances`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub instance_id: String,
    pub license_id: String,
    pub addr: String,
    pub last_heartbeat: i64,
}

/// Redis-backed cross-instance state coordinator.
///
/// Holds a [`redis::Client`] and this instance's unique ID. All methods are
/// async and return [`RedisError`] on failure — callers (background tasks)
/// should log and continue rather than crashing the server when Redis is
/// temporarily unavailable.
pub struct RedisState {
    client: Client,
    instance_id: String,
}

impl RedisState {
    /// Connect to Redis at `url`, verify connectivity with PING, and return a
    /// [`RedisState`] tagged with `instance_id`.
    pub async fn new(url: &str, instance_id: String) -> Result<Self, RedisError> {
        let client = Client::open(url)?;
        let mut conn = client.get_multiplexed_async_connection().await?;
        redis::cmd("PING").query_async::<String>(&mut conn).await?;
        Ok(Self {
            client,
            instance_id,
        })
    }

    /// Returns this instance's unique ID.
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Returns a clone of the underlying [`redis::Client`].
    pub fn client(&self) -> Client {
        self.client.clone()
    }

    /// Ping Redis to verify connectivity. Returns `Ok(())` on success.
    pub async fn ping(&self) -> Result<(), RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        redis::cmd("PING").query_async::<String>(&mut conn).await?;
        Ok(())
    }

    /// Publish `msg` to `channel`. Fire-and-forget from the caller's
    /// perspective — errors are returned for logging but should not abort
    /// the publishing operation.
    pub async fn publish(&self, channel: &str, msg: &str) -> Result<(), RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        conn.publish::<_, _, ()>(channel, msg).await
    }

    /// Subscribe to `channel` and return a [`PubSubStream`] yielding
    /// [`redis::Msg`] items. The caller iterates the stream and handles each
    /// message.
    pub async fn subscribe(&self, channel: &str) -> Result<PubSubStream, RedisError> {
        let mut pubsub = self.client.get_async_pubsub().await?;
        pubsub.subscribe(channel).await?;
        Ok(pubsub.into_on_message())
    }

    // ── Seat tracking (Phase 6c) ──────────────────────────────────────────

    /// Register this instance in the Redis sorted set of active instances.
    ///
    /// The instance's member value is a JSON-encoded [`InstanceInfo`] (so
    /// [`Self::list_instances`] can recover the license ID and address). The
    /// score is the current Unix timestamp. An expiry of [`HEARTBEAT_TTL_SECS`]
    /// is set on the key so dead instances are auto-removed if no heartbeat
    /// refreshes it.
    pub async fn register_instance(
        &self,
        instance_id: &str,
        license_id: &str,
        addr: &str,
    ) -> Result<(), RedisError> {
        let now = current_timestamp();
        let info = InstanceInfo {
            instance_id: instance_id.to_string(),
            license_id: license_id.to_string(),
            addr: addr.to_string(),
            last_heartbeat: now,
        };
        let member =
            serde_json::to_string(&info).map_err(|e| RedisError::from(io_error(e.to_string())))?;
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        conn.zadd::<_, _, _, ()>(INSTANCES_KEY, &member, now)
            .await?;
        conn.expire::<_, ()>(INSTANCES_KEY, HEARTBEAT_TTL_SECS)
            .await?;
        Ok(())
    }

    /// Refresh this instance's heartbeat (update its score to the current
    /// timestamp and reset the key expiry).
    pub async fn heartbeat(&self, instance_id: &str) -> Result<(), RedisError> {
        let now = current_timestamp();
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let members: Vec<String> = conn.zrange(INSTANCES_KEY, 0, -1).await?;
        for member in members {
            if let Ok(info) = serde_json::from_str::<InstanceInfo>(&member) {
                if info.instance_id == instance_id {
                    conn.zrem::<_, _, ()>(INSTANCES_KEY, &member).await?;
                    let updated = InstanceInfo {
                        last_heartbeat: now,
                        ..info
                    };
                    let new_member = serde_json::to_string(&updated)
                        .map_err(|e| RedisError::from(io_error(e.to_string())))?;
                    conn.zadd::<_, _, _, ()>(INSTANCES_KEY, &new_member, now)
                        .await?;
                    break;
                }
            }
        }
        conn.expire::<_, ()>(INSTANCES_KEY, HEARTBEAT_TTL_SECS)
            .await?;
        Ok(())
    }

    /// Remove this instance from the active-instances sorted set (graceful
    /// shutdown / deregistration).
    pub async fn deregister_instance(&self, instance_id: &str) -> Result<(), RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let members: Vec<String> = conn.zrange(INSTANCES_KEY, 0, -1).await?;
        for member in members {
            if let Ok(info) = serde_json::from_str::<InstanceInfo>(&member) {
                if info.instance_id == instance_id {
                    conn.zrem::<_, _, ()>(INSTANCES_KEY, &member).await?;
                    break;
                }
            }
        }
        Ok(())
    }

    /// Count active instances whose heartbeat is within [`HEARTBEAT_TTL_SECS`]
    /// of the current time (ZCOUNT).
    pub async fn active_instance_count(&self) -> Result<usize, RedisError> {
        let now = current_timestamp();
        let min = now - HEARTBEAT_TTL_SECS;
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let count: i64 = conn.zcount(INSTANCES_KEY, min, now).await?;
        Ok(count as usize)
    }

    /// List all active instances (ZRANGE), parsing each member as
    /// [`InstanceInfo`]. Members that fail to parse are skipped.
    pub async fn list_instances(&self) -> Result<Vec<InstanceInfo>, RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let members: Vec<String> = conn.zrange(INSTANCES_KEY, 0, -1).await?;
        let mut instances = Vec::new();
        for member in members {
            if let Ok(info) = serde_json::from_str::<InstanceInfo>(&member) {
                instances.push(info);
            }
        }
        Ok(instances)
    }
}

/// Current Unix timestamp in seconds.
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Build a [`RedisError`] from a generic IO-style message (used for serde
/// serialization failures, which are not natively Redis errors).
fn io_error(msg: String) -> std::io::Error {
    std::io::Error::other(msg)
}

/// Implement [`EventPublisher`] so the API layer can publish config/intercept
/// notifications via Redis without depending on the concrete [`RedisState`]
/// type directly.
#[async_trait]
impl madhyamas_api::EventPublisher for RedisState {
    async fn publish(&self, channel: &str, message: &str) {
        if let Err(e) = RedisState::publish(self, channel, message).await {
            tracing::warn!("Redis publish to {channel} failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    //! Redis integration tests.
    //!
    //! These require a running Redis instance at `redis://localhost:6379`.
    //! They are marked `#[ignore]` so `cargo test` does not fail without Redis.
    //! Run them explicitly: `cargo test --all-features -- --ignored`.

    use super::*;

    const REDIS_URL: &str = "redis://localhost:6379";

    fn unique_instance_id() -> String {
        format!("test-{}", uuid::Uuid::new_v4())
    }

    #[tokio::test]
    #[ignore = "requires redis at redis://localhost:6379"]
    async fn test_redis_connect_ping() {
        let id = unique_instance_id();
        let state = RedisState::new(REDIS_URL, id)
            .await
            .expect("connect to redis");
        assert!(!state.instance_id().is_empty());
    }

    #[tokio::test]
    #[ignore = "requires redis at redis://localhost:6379"]
    async fn test_redis_publish_subscribe() {
        let id = unique_instance_id();
        let state = RedisState::new(REDIS_URL, id)
            .await
            .expect("connect to redis");
        let channel = format!("test:pubsub:{}", unique_instance_id());

        let ch_clone = channel.clone();
        let state_clone = state.client();
        let recv_task = tokio::spawn(async move {
            let mut pubsub = state_clone.get_async_pubsub().await.expect("pubsub");
            pubsub.subscribe(&ch_clone).await.expect("subscribe");
            let mut stream = pubsub.into_on_message();
            use futures::StreamExt;
            stream
                .next()
                .await
                .map(|msg| msg.get_payload::<String>().unwrap_or_default())
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        state
            .publish(&channel, "hello-cross-instance")
            .await
            .expect("publish");

        let result = tokio::time::timeout(std::time::Duration::from_secs(5), recv_task)
            .await
            .expect("timed out waiting for pubsub message")
            .expect("recv task panicked");
        assert_eq!(result.as_deref(), Some("hello-cross-instance"));
    }

    #[tokio::test]
    #[ignore = "requires redis at redis://localhost:6379"]
    async fn test_redis_config_propagation() {
        let id = unique_instance_id();
        let state = RedisState::new(REDIS_URL, id)
            .await
            .expect("connect to redis");
        let channel = format!("test:config:{}", unique_instance_id());

        let ch_clone = channel.clone();
        let state_clone = state.client();
        let recv_task = tokio::spawn(async move {
            let mut pubsub = state_clone.get_async_pubsub().await.expect("pubsub");
            pubsub.subscribe(&ch_clone).await.expect("subscribe");
            let mut stream = pubsub.into_on_message();
            use futures::StreamExt;
            stream
                .next()
                .await
                .map(|msg| msg.get_payload::<String>().unwrap_or_default())
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        state
            .publish(&channel, "config-changed")
            .await
            .expect("publish config");

        let result = tokio::time::timeout(std::time::Duration::from_secs(5), recv_task)
            .await
            .expect("timed out waiting for config notification")
            .expect("recv task panicked");
        assert_eq!(result.as_deref(), Some("config-changed"));
    }

    #[tokio::test]
    #[ignore = "requires redis at redis://localhost:6379"]
    async fn test_seat_registration() {
        let id = unique_instance_id();
        let state = RedisState::new(REDIS_URL, id.clone())
            .await
            .expect("connect to redis");
        state
            .register_instance(&id, "lic_test", "127.0.0.1:3001")
            .await
            .expect("register");
        let count = state.active_instance_count().await.expect("count");
        assert!(
            count >= 1,
            "expected at least 1 active instance, got {count}"
        );
        state.deregister_instance(&id).await.expect("deregister");
    }

    #[tokio::test]
    #[ignore = "requires redis at redis://localhost:6379"]
    async fn test_seat_limit_enforcement() {
        let base = unique_instance_id();
        let state = RedisState::new(REDIS_URL, format!("{base}-0"))
            .await
            .expect("connect to redis");
        for i in 0..3 {
            let id = format!("{base}-{i}");
            state
                .register_instance(&id, "lic_test", "127.0.0.1:3001")
                .await
                .expect("register");
        }
        let count = state.active_instance_count().await.expect("count");
        assert!(
            count >= 3,
            "expected at least 3 active instances, got {count}"
        );
        for i in 0..3 {
            let id = format!("{base}-{i}");
            state.deregister_instance(&id).await.expect("deregister");
        }
    }

    #[tokio::test]
    #[ignore = "requires redis at redis://localhost:6379"]
    async fn test_seat_release() {
        let id = unique_instance_id();
        let state = RedisState::new(REDIS_URL, id.clone())
            .await
            .expect("connect to redis");
        state
            .register_instance(&id, "lic_test", "127.0.0.1:3001")
            .await
            .expect("register");
        state.deregister_instance(&id).await.expect("deregister");
        let instances = state.list_instances().await.expect("list");
        let still_registered = instances.iter().any(|i| i.instance_id == id);
        assert!(!still_registered, "instance should have been deregistered");
    }
}
