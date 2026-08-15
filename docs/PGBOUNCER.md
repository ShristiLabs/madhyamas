# PgBouncer Configuration Guide

PgBouncer is a lightweight connection pooler for PostgreSQL. It sits between
the Madhyamas proxy and PostgreSQL, reducing connection overhead under high
load (each Madhyamas instance opens up to 10 connections per pool; with
multiple instances, this can exhaust PostgreSQL's `max_connections`).

## When to Use PgBouncer

- **Multi-instance deployments** (enterprise tier): 3+ Madhyamas instances
  each opening 10+ connections → 30+ PostgreSQL backend connections.
- **High-traffic capture**: thousands of requests/sec with per-request DB
  writes.
- **Connection-starved environments**: managed PostgreSQL with low
  `max_connections` (e.g. AWS RDS `db.t3.micro` = 85 max connections).

## Installation

```bash
# Debian/Ubuntu
sudo apt-get install pgbouncer

# macOS (Homebrew)
brew install pgbouncer

# Docker
docker run -d --name pgbouncer \
  -p 6432:6432 \
  -v $(pwd)/pgbouncer.ini:/etc/pgbouncer/pgbouncer.ini \
  -v $(pwd)/userlist.txt:/etc/pgbouncer/userlist.txt \
  edoburu/pgbouncer
```

## Configuration

### `pgbouncer.ini`

```ini
[databases]
; Route all connections to the Madhyamas database
madhyamas = host=127.0.0.1 port=5432 dbname=madhyamas

[pgbouncer]
; Listen on all interfaces (adjust for your network)
listen_addr = 0.0.0.0
listen_port = 6432

; Pool mode: transaction-level pooling is recommended for Madhyamas.
; Each transaction borrows a server connection and returns it after COMMIT/ROLLBACK.
; This maximizes connection reuse without breaking prepared statements.
pool_mode = transaction

; Connection limits
max_client_conn = 200       ; max client connections to PgBouncer
default_pool_size = 20      ; max server connections per pool
min_pool_size = 5           ; min server connections kept ready
reserve_pool_size = 5       ; extra connections for burst traffic
reserve_pool_timeout = 3    ; seconds before using reserve pool

; Timeout
server_idle_timeout = 600   ; close idle server connections after 10 min
query_wait_timeout = 120    ; cancel clients waiting > 120s for a server conn
client_idle_timeout = 0     ; don't kill idle clients (0 = disabled)

; Logging
log_connections = 1
log_disconnections = 1
log_pooler_errors = 1
stats_period = 60           ; log stats every 60s

; Security
auth_type = md5
auth_file = /etc/pgbouncer/userlist.txt

; TLS (optional — recommended for remote connections)
; client_tls_sslmode = require
; client_tls_key_file = /etc/pgbouncer/server.key
; client_tls_cert_file = /etc/pgbouncer/server.crt
```

### `userlist.txt`

```
"madhyamas" "md5" "your_password_hash_here"
```

Generate the MD5 hash:
```bash
echo -n "your_password" | md5sum
# Or use PgBouncer's format: "md5" + md5("password" + "username")
echo -n "your_passwordmadhyamas" | md5sum | awk '{print "md5" $1}'
```

## Pointing Madhyamas at PgBouncer

Set `--database-url` to the PgBouncer port (6432) instead of the PostgreSQL
port (5432):

```bash
madhyamas --database-url "postgres://madhyamas:password@localhost:6432/madhyamas"
```

For read replicas via PgBouncer, configure a separate pool for the replica:

```ini
[databases]
madhyamas = host=127.0.0.1 port=5432 dbname=madhyamas
madhyamas_read = host=replica.example.com port=5432 dbname=madhyamas
```

Then:
```bash
madhyamas \
  --database-url "postgres://madhyamas:password@localhost:6432/madhyamas" \
  --database-read-url "postgres://madhyamas:password@localhost:6432/madhyamas_read"
```

## Pool Mode Considerations

| Mode | Reuse Level | Prepared Statements | Madhyamas Compatibility |
|------|-------------|---------------------|------------------------|
| `session` | Per client session | ✅ Safe | ✅ Full (lowest reuse) |
| `transaction` | Per transaction | ⚠️ Use `MAX_PREPARED_STATEMENTS` | ✅ Recommended |
| `statement` | Per statement | ❌ Broken | ❌ Not recommended |

**Transaction mode** is recommended for Madhyamas. sqlx uses simple query
protocol for most operations, which is compatible with transaction-level
pooling. If you use prepared statements, set `max_prepared_statements = 100`
in PgBouncer 1.21+.

## Monitoring

```bash
# Connect to the PgBouncer admin console
psql -p 6432 -U madhyamas pgbouncer

# Show active pools
SHOW POOLS;

# Show client connections
SHOW CLIENTS;

# Show server connections
SHOW SERVERS;

# Show statistics
SHOW STATS;
```

## Docker Compose Example

```yaml
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_USER: madhyamas
      POSTGRES_PASSWORD: password
      POSTGRES_DB: madhyamas
    ports:
      - "5432:5432"

  pgbouncer:
    image: edoburu/pgbouncer
    environment:
      DB_HOST: postgres
      DB_USER: madhyamas
      DB_PASSWORD: password
      DB_NAME: madhyamas
      POOL_MODE: transaction
      MAX_CLIENT_CONN: 200
      DEFAULT_POOL_SIZE: 20
    ports:
      - "6432:6432"
    depends_on:
      - postgres

  madhyamas:
    image: madhyamas:latest
    command: --database-url "postgres://madhyamas:password@pgbouncer:6432/madhyamas"
    depends_on:
      - pgbouncer
```

## See Also

- [POSTGRES_HA.md](POSTGRES_HA.md) — High availability with streaming replication
- [ENTERPRISE_PERF_SECURITY.md](ENTERPRISE_PERF_SECURITY.md) §6.7 — Performance analysis
- [ENTERPRISE_MULTI_INSTANCE.md](ENTERPRISE_MULTI_INSTANCE.md) — Multi-instance architecture
