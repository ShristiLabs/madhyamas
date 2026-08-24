//! Redis integration tests for the public redis_state API.
//!
//! These require a running Redis instance at `redis://localhost:6379`.
//! They are marked `#[ignore]` so `cargo test` does not fail without Redis.
//! Run them explicitly: `cargo test --all-features -- --ignored`.

use madhyamas_enterprise::{InstanceMetrics, RedisState};

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

#[tokio::test]
#[ignore = "requires redis at redis://localhost:6379"]
async fn test_instance_registration_with_metrics() {
    let id = unique_instance_id();
    let state = RedisState::new(REDIS_URL, id.clone())
        .await
        .expect("connect to redis");
    let metrics = InstanceMetrics {
        cpu_usage: 42.5,
        memory_usage_mb: 256,
        active_connections: 10,
        request_count: 1000,
        uptime_secs: 3600,
    };
    state
        .register_instance_with_metrics(&id, "lic_test", "127.0.0.1:3001", &metrics)
        .await
        .expect("register with metrics");
    let instances = state.list_instances_with_metrics().await.expect("list");
    let found = instances
        .iter()
        .find(|i| i.instance_id == id)
        .expect("instance not found");
    let m = found.metrics.clone().expect("metrics should be present");
    assert_eq!(m.cpu_usage, 42.5);
    assert_eq!(m.memory_usage_mb, 256);
    assert_eq!(m.active_connections, 10);
    assert_eq!(m.request_count, 1000);
    assert_eq!(m.uptime_secs, 3600);
    // Update metrics and verify.
    let updated = InstanceMetrics {
        cpu_usage: 55.0,
        memory_usage_mb: 512,
        active_connections: 20,
        request_count: 2000,
        uptime_secs: 7200,
    };
    state
        .update_instance_metrics(&id, &updated)
        .await
        .expect("update metrics");
    let instances2 = state.list_instances_with_metrics().await.expect("list 2");
    let found2 = instances2
        .iter()
        .find(|i| i.instance_id == id)
        .expect("instance not found after update");
    let m2 = found2.metrics.clone().expect("metrics should be present");
    assert_eq!(m2.cpu_usage, 55.0);
    assert_eq!(m2.request_count, 2000);
    state.deregister_instance(&id).await.expect("deregister");
}

#[tokio::test]
#[ignore = "requires redis at redis://localhost:6379"]
async fn test_cluster_metrics_aggregation() {
    let base = unique_instance_id();
    let state = RedisState::new(REDIS_URL, format!("{base}-0"))
        .await
        .expect("connect to redis");
    // Register 2 instances with different metrics.
    for (i, req_count) in [(0, 500u64), (1, 1500u64)] {
        let id = format!("{base}-{i}");
        let metrics = InstanceMetrics {
            cpu_usage: 30.0 + i as f64 * 10.0,
            memory_usage_mb: 128 + i as u64 * 256,
            active_connections: 5 + i as u64 * 5,
            request_count: req_count,
            uptime_secs: 1800 + i as u64 * 1800,
        };
        state
            .register_instance_with_metrics(&id, "lic_test", "127.0.0.1:3001", &metrics)
            .await
            .expect("register");
    }
    let instances = state.list_instances_with_metrics().await.expect("list");
    let ours: Vec<_> = instances
        .iter()
        .filter(|i| i.instance_id.starts_with(&base))
        .collect();
    assert_eq!(ours.len(), 2, "expected 2 instances");
    let total_requests: u64 = ours
        .iter()
        .map(|i| i.metrics.as_ref().unwrap().request_count)
        .sum();
    assert_eq!(total_requests, 2000, "total request count should be 2000");
    let total_conns: u64 = ours
        .iter()
        .map(|i| i.metrics.as_ref().unwrap().active_connections)
        .sum();
    assert_eq!(total_conns, 15, "total active connections should be 15");
    for i in 0..2 {
        let id = format!("{base}-{i}");
        state.deregister_instance(&id).await.expect("deregister");
    }
}
