---
title: Performance & Monitoring
description: Real-time performance metrics, cluster-wide monitoring, health checks, and instance registry in Madhyamas Enterprise.
---

# Performance & Monitoring

Madhyamas Enterprise includes a performance monitoring system with real-time metrics, cluster-wide visibility, and detailed health checks. These features help operators understand system load, identify bottlenecks, and ensure the proxy is healthy.

## Web UI

The Metrics admin panel provides a real-time dashboard:

![Enterprise metrics panel](/screenshots/enterprise-metrics-panel.png)

### Accessing the Panel

1. Log in as an admin (or operator with metrics access)
2. Click the **Metrics** icon in the navigation rail

### Metrics Displayed

| Metric | Description |
|--------|-------------|
| **Total Requests** | Total HTTP requests processed |
| **Successful** | Requests with 2xx/3xx responses |
| **Failed** | Requests with 4xx/5xx responses |
| **Avg Latency** | Average response time (ms) |
| **Req/sec** | Current throughput |
| **Request Distribution** | Bar chart of response status codes |
| **Cluster Overview** | Aggregate metrics across all instances |
| **Instances Table** | Per-instance CPU, memory, connections |

## Health Checks

### Simple Health Check

```bash
curl http://localhost:3001/health
# Output: OK (200) or "Database not ready" (503)
```

This endpoint is unauthenticated and designed for load balancer probes. It verifies database connectivity before reporting healthy.

### Detailed Health Check

```bash
curl http://localhost:3001/api/health/detailed
```

Response:

```json
{
  "healthy": true,
  "version": "0.1.6",
  "uptime_secs": 3600,
  "memory_usage_mb": 128,
  "active_connections": 15,
  "tier": "enterprise",
  "auth_mode": "local",
  "auth_required": true,
  "license": {
    "licensed": true,
    "plan": "pro",
    "seats_used": 3,
    "seats_total": 50
  },
  "dependencies": {
    "database": "ok",
    "redis": "ok",
    "license": "ok"
  }
}
```

This endpoint is also unauthenticated so it can be used by monitoring systems without credentials.

## REST API

### Performance Metrics

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3001/api/metrics
```

Response:

```json
{
  "requests_total": 15420,
  "requests_successful": 14985,
  "requests_failed": 435,
  "avg_latency_ms": 45.2,
  "requests_per_sec": 12.5,
  "intercept_hits": {
    "block_list": 23,
    "mocks": 145,
    "rewrites": 89,
    "breakpoints": 5,
    "throttle": 12
  }
}
```

### Combined Performance Stats

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3001/api/performance
```

Returns metrics plus memory usage and connection pool stats.

### Cluster Metrics

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3001/api/metrics/cluster
```

Returns aggregate metrics across all instances in the cluster (requires Redis).

## Instance Registry

In multi-instance deployments, the Instances panel shows all active instances:

![Enterprise instances panel](/screenshots/enterprise-instances-panel.png)

### Instances API

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3001/api/instances
```

Response:

```json
[
  {
    "instance_id": "madhyamas-1",
    "address": "10.0.0.1:3001",
    "last_heartbeat": "2025-01-15T10:30:00Z",
    "status": "active",
    "metrics": {
      "cpu_percent": 15.2,
      "memory_mb": 128,
      "connections": 15
    }
  }
]
```

Each instance sends a heartbeat to Redis every 60 seconds. Instances that miss heartbeats for more than 120 seconds are automatically reaped.

## MCP Tools

AI agents can query metrics and health via MCP:

```
madhyamas_get_metrics()
madhyamas_get_health()
```

See [CLI & MCP Tools](./cli-mcp) for details.

## Performance Optimizations

Madhyamas Enterprise includes several database optimizations for high-volume traffic recording:

| Optimization | Description |
|-------------|-------------|
| **Tiered body storage** | Bodies < 4KB stored inline; larger bodies in separate table |
| **GIN indexes** | Fast full-text search on URLs and headers |
| **BRIN indexes** | Space-efficient time-based indexing |
| **Trigram indexes** | Fast LIKE/regex queries via `pg_trgm` |
| **Cursor pagination** | Stable pagination for large result sets |
| **Write batching** | Batched inserts for high-throughput recording |
| **Session counters** | O(1) entry count lookups via `session_counters` table |
| **Read replicas** | Optional read replica for query offloading |

## Monitoring Integration

### Prometheus (Future)

A Prometheus metrics endpoint is planned for future releases. Currently, use the REST API with your monitoring tool:

```bash
# Example: cron job that checks health every minute
* * * * * curl -sf http://localhost:3001/health > /dev/null || alert "Madhyamas unhealthy"
```

### Docker Health Check

The Docker Compose configuration includes a health check:

```yaml
healthcheck:
  test: ["CMD", "curl", "-sf", "http://localhost:3001/health"]
  interval: 30s
  timeout: 5s
  retries: 3
```

### Kubernetes Probes

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 3001
  initialDelaySeconds: 10
  periodSeconds: 30
readinessProbe:
  httpGet:
    path: /api/health/detailed
    port: 3001
  initialDelaySeconds: 5
  periodSeconds: 10
```

## See Also

- [Multi-Instance Deployment](./deployment) — Setting up multi-instance clusters
- [Configuration](./configuration) — Health check and metrics configuration
- [CLI & MCP Tools](./cli-mcp) — Metrics via CLI and MCP
