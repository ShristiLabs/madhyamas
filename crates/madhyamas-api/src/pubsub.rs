//! Pluggable event publisher trait for cross-instance pub/sub.
//!
//! The API layer defines [`EventPublisher`] as a trait object so handlers can
//! publish notifications (config changes, intercept rule changes) to a pub/sub
//! bus without depending on the enterprise crate's concrete [`RedisState`].
//! In single-instance mode (no Redis), no publisher is wired and publish calls
//! are no-ops.

use async_trait::async_trait;
use std::sync::Arc;

/// Trait for publishing notification messages to a pub/sub channel.
///
/// The enterprise crate's [`RedisState`](madhyamas_enterprise::RedisState)
/// implements this trait. When `None` (OSS / single-instance mode), handlers
/// skip publishing — the change is local-only.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish `message` to `channel`. Implementations should be fire-and-forget:
    /// errors are logged internally and do not propagate to the caller (a failed
    /// pub/sub notification must not fail the API request).
    async fn publish(&self, channel: &str, message: &str);
}

/// A no-op publisher used as a placeholder when Redis is not configured.
/// Never actually stored in [`AppState`](crate::AppState) (the field is
/// `Option`), but documented here for completeness.
#[allow(dead_code)]
pub struct NoopPublisher;

#[async_trait]
impl EventPublisher for NoopPublisher {
    async fn publish(&self, _channel: &str, _message: &str) {}
}

/// Convenience helper: if `publisher` is `Some`, publish `message` to
/// `channel`; otherwise do nothing. Spawns the publish as a best-effort
/// background task so the handler is not blocked on Redis latency.
pub fn notify(
    publisher: &Option<Arc<dyn EventPublisher + Send + Sync>>,
    channel: &str,
    message: &str,
) {
    if let Some(pub_) = publisher {
        let pub_ = pub_.clone();
        let channel = channel.to_string();
        let message = message.to_string();
        tokio::spawn(async move {
            pub_.publish(&channel, &message).await;
        });
    }
}
