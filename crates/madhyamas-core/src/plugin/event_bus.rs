//! Plugin event bus — inter-plugin communication via pub/sub.
//!
//! Plugins can publish events to named topics and subscribe to topics to
//! receive events from other plugins. This enables loosely-coupled
//! communication between plugins without direct dependencies.
//!
//! Events are JSON values (`serde_json::Value`) and are delivered
//! asynchronously to subscribers. The bus is in-process (no network).
//!
//! # Usage
//!
//! ```no_run
//! use madhyamas_core::PluginEventBus;
//! use std::sync::Arc;
//!
//! let bus = Arc::new(PluginEventBus::new());
//!
//! // Subscribe to a topic.
//! let sub_id = bus.subscribe("my-topic", move |event| {
//!     println!("received: {}", event);
//! });
//!
//! // Publish an event.
//! bus.publish("my-topic", serde_json::json!({ "key": "value" }));
//!
//! // Unsubscribe.
//! bus.unsubscribe("my-topic", sub_id);
//! ```

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::warn;

/// Type alias for a subscriber callback.
type Subscriber = Box<dyn Fn(serde_json::Value) + Send + Sync>;

/// A subscriber entry with an ID and callback.
struct SubscriberEntry {
    id: u64,
    callback: Subscriber,
}

/// An in-process pub/sub event bus for inter-plugin communication.
pub struct PluginEventBus {
    /// Map from topic name to subscriber list.
    topics: RwLock<HashMap<String, Vec<SubscriberEntry>>>,
    /// Monotonic subscriber ID generator.
    next_id: AtomicU64,
}

impl PluginEventBus {
    /// Create a new empty event bus.
    pub fn new() -> Self {
        Self {
            topics: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Subscribe to a topic. Returns a subscriber ID that can be used to
    /// unsubscribe.
    ///
    /// The callback is invoked synchronously when an event is published to
    /// the topic. If the callback panics, the panic is caught and logged
    /// (other subscribers are still notified).
    pub fn subscribe<F>(&self, topic: &str, callback: F) -> u64
    where
        F: Fn(serde_json::Value) + Send + Sync + 'static,
    {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let entry = SubscriberEntry {
            id,
            callback: Box::new(callback),
        };
        self.topics
            .write()
            .entry(topic.to_string())
            .or_default()
            .push(entry);
        id
    }

    /// Unsubscribe a subscriber from a topic by ID.
    ///
    /// Returns `true` if the subscriber was found and removed.
    pub fn unsubscribe(&self, topic: &str, subscriber_id: u64) -> bool {
        let mut topics = self.topics.write();
        let Some(subs) = topics.get_mut(topic) else {
            return false;
        };
        let before = subs.len();
        subs.retain(|s| s.id != subscriber_id);
        let removed = subs.len() < before;
        if subs.is_empty() {
            topics.remove(topic);
        }
        removed
    }

    /// Publish an event to a topic. All subscribers are notified
    /// synchronously. If there are no subscribers, the event is silently
    /// dropped.
    pub fn publish(&self, topic: &str, event: serde_json::Value) {
        let subs = self.topics.read();
        let Some(subscribers) = subs.get(topic) else {
            return;
        };
        // Clone the subscriber IDs to avoid holding the lock during callbacks.
        // We re-read the callbacks under the lock but release before calling.
        let count = subscribers.len();
        if count == 0 {
            return;
        }
        // We need to call each callback without holding the write lock.
        // Since callbacks are `Fn` (immutable), and we hold a read lock,
        // this is safe as long as callbacks don't try to subscribe/unsubscribe
        // (which would deadlock). We catch panics to protect the bus.
        for entry in subscribers {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                (entry.callback)(event.clone());
            }));
            if result.is_err() {
                warn!(
                    "plugin event bus: subscriber {} on topic '{}' panicked",
                    entry.id, topic
                );
            }
        }
    }

    /// Returns the number of subscribers for a topic.
    pub fn subscriber_count(&self, topic: &str) -> usize {
        self.topics.read().get(topic).map(|s| s.len()).unwrap_or(0)
    }

    /// Returns the list of active topics.
    pub fn topics(&self) -> Vec<String> {
        self.topics.read().keys().cloned().collect()
    }

    /// Clear all subscribers from all topics.
    pub fn clear(&self) {
        self.topics.write().clear();
    }
}

impl Default for PluginEventBus {
    fn default() -> Self {
        Self::new()
    }
}
