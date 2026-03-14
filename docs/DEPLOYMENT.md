# Deployment Guide

## Overview

This guide covers deploying ProxyForge in various environments, from local development to production infrastructure.

## Deployment Options

### 1. Binary Distribution
### 2. Docker Container
### 3. Kubernetes
### 4. Cloud Platforms (AWS, GCP, Azure)
### 5. Package Managers (Homebrew, Snap, AUR)

---

## Binary Distribution

### Building Release Binary

```bash
# Clone repository
git clone https://github.com/proxyforge/proxyforge.git
cd proxyforge

# Build optimized release binary
cargo build --release

# Binary location
ls -lh target/release/proxyforge

# Optional: Strip symbols for smaller size
strip target/release/proxyforge
```

### Installation

```bash
# Copy binary to system path
sudo cp target/release/proxyforge /usr/local/bin/

# Verify installation
proxyforge --version

# Create data directory
mkdir -p ~/.proxyforge/{certs,logs}
```

### Running as Service

#### systemd (Linux)

Create `/etc/systemd/system/proxyforge.service`:

```ini
[Unit]
Description=ProxyForge HTTP/HTTPS Debugging Proxy
After=network.target

[Service]
Type=simple
User=proxyforge
Group=proxyforge
WorkingDirectory=/opt/proxyforge
ExecStart=/usr/local/bin/proxyforge \
    --proxy-port 8888 \
    --api-port 3001 \
    --host 0.0.0.0 \
    --db-path /var/lib/proxyforge/traffic.db \
    --cert-path /var/lib/proxyforge/certs
Restart=on-failure
RestartSec=5s

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/proxyforge

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
# Create user and directories
sudo useradd -r -s /bin/false proxyforge
sudo mkdir -p /var/lib/proxyforge/{certs,logs}
sudo chown -R proxyforge:proxyforge /var/lib/proxyforge

# Enable service
sudo systemctl daemon-reload
sudo systemctl enable proxyforge
sudo systemctl start proxyforge

# Check status
sudo systemctl status proxyforge

# View logs
sudo journalctl -u proxyforge -f
```

#### launchd (macOS)

Create `~/Library/LaunchAgents/com.proxyforge.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.proxyforge</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/proxyforge</string>
        <string>--proxy-port</string>
        <string>8888</string>
        <string>--api-port</string>
        <string>3001</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/proxyforge.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/proxyforge.err</string>
</dict>
</plist>
```

Load service:

```bash
launchctl load ~/Library/LaunchAgents/com.proxyforge.plist
launchctl start com.proxyforge
```

---

## Docker Deployment

### Using Pre-built Image

```bash
# Pull latest image
docker pull proxyforge/proxyforge:latest

# Run container
docker run -d \
  --name proxyforge \
  -p 8888:8888 \
  -p 3001:3001 \
  -v proxyforge-data:/data \
  proxyforge/proxyforge:latest

# View logs
docker logs -f proxyforge

# Stop container
docker stop proxyforge
```

### Building Custom Image

Create `Dockerfile`:

```dockerfile
# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -u 1000 -s /bin/false proxyforge

# Copy binary from builder
COPY --from=builder /app/target/release/proxyforge /usr/local/bin/

# Copy web UI assets
COPY web/dist /usr/share/proxyforge/web

# Create data directory
RUN mkdir -p /data/certs /data/logs && \
    chown -R proxyforge:proxyforge /data

USER proxyforge
WORKDIR /data

EXPOSE 8888 3001

ENTRYPOINT ["/usr/local/bin/proxyforge"]
CMD ["--host", "0.0.0.0", "--db-path", "/data/traffic.db", "--cert-path", "/data/certs"]
```

Build and run:

```bash
# Build image
docker build -t proxyforge:local .

# Run container
docker run -d \
  --name proxyforge \
  -p 8888:8888 \
  -p 3001:3001 \
  -v $(pwd)/data:/data \
  proxyforge:local
```

### Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  proxyforge:
    image: proxyforge/proxyforge:latest
    container_name: proxyforge
    restart: unless-stopped
    ports:
      - "8888:8888"
      - "3001:3001"
    volumes:
      - proxyforge-data:/data
      - ./config.toml:/etc/proxyforge/config.toml:ro
    environment:
      - RUST_LOG=info
      - PROXYFORGE_CONFIG=/etc/proxyforge/config.toml
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3001/api/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

volumes:
  proxyforge-data:
    driver: local
```

Deploy:

```bash
docker-compose up -d
docker-compose logs -f
docker-compose down
```

---

## Kubernetes Deployment

### Basic Deployment

Create `k8s/deployment.yaml`:

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: proxyforge

---
apiVersion: v1
kind: ConfigMap
metadata:
  name: proxyforge-config
  namespace: proxyforge
data:
  config.toml: |
    [general]
    api_port = 3001
    proxy_port = 8888
    log_level = "info"
    
    [storage]
    data_dir = "/data"
    max_entries = 100000

---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: proxyforge-data
  namespace: proxyforge
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi

---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: proxyforge
  namespace: proxyforge
spec:
  replicas: 1
  selector:
    matchLabels:
      app: proxyforge
  template:
    metadata:
      labels:
        app: proxyforge
    spec:
      containers:
      - name: proxyforge
        image: proxyforge/proxyforge:latest
        ports:
        - containerPort: 8888
          name: proxy
        - containerPort: 3001
          name: api
        env:
        - name: RUST_LOG
          value: "info"
        volumeMounts:
        - name: data
          mountPath: /data
        - name: config
          mountPath: /etc/proxyforge
          readOnly: true
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /api/health
            port: 3001
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /api/health
            port: 3001
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: proxyforge-data
      - name: config
        configMap:
          name: proxyforge-config

---
apiVersion: v1
kind: Service
metadata:
  name: proxyforge
  namespace: proxyforge
spec:
  type: LoadBalancer
  selector:
    app: proxyforge
  ports:
  - name: proxy
    port: 8888
    targetPort: 8888
  - name: api
    port: 3001
    targetPort: 3001
```

Deploy to Kubernetes:

```bash
# Apply configuration
kubectl apply -f k8s/deployment.yaml

# Check deployment status
kubectl get pods -n proxyforge
kubectl get svc -n proxyforge

# View logs
kubectl logs -f -n proxyforge deployment/proxyforge

# Port forward for local access
kubectl port-forward -n proxyforge svc/proxyforge 8888:8888 3001:3001
```

### Helm Chart

Create `helm/proxyforge/values.yaml`:

```yaml
replicaCount: 1

image:
  repository: proxyforge/proxyforge
  tag: latest
  pullPolicy: IfNotPresent

service:
  type: LoadBalancer
  proxyPort: 8888
  apiPort: 3001

persistence:
  enabled: true
  size: 10Gi
  storageClass: standard

resources:
  requests:
    memory: 256Mi
    cpu: 250m
  limits:
    memory: 512Mi
    cpu: 500m

config:
  logLevel: info
  maxEntries: 100000
```

Install with Helm:

```bash
helm install proxyforge ./helm/proxyforge
helm upgrade proxyforge ./helm/proxyforge
helm uninstall proxyforge
```

---

## Cloud Platform Deployments

### AWS (EC2 + ECS)

#### EC2 Instance

```bash
# Launch EC2 instance (Amazon Linux 2)
# Install Docker
sudo yum update -y
sudo yum install -y docker
sudo service docker start
sudo usermod -a -G docker ec2-user

# Run ProxyForge
docker run -d \
  --name proxyforge \
  --restart unless-stopped \
  -p 8888:8888 \
  -p 3001:3001 \
  -v /data/proxyforge:/data \
  proxyforge/proxyforge:latest
```

#### ECS Fargate

Create task definition:

```json
{
  "family": "proxyforge",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "512",
  "memory": "1024",
  "containerDefinitions": [
    {
      "name": "proxyforge",
      "image": "proxyforge/proxyforge:latest",
      "portMappings": [
        {
          "containerPort": 8888,
          "protocol": "tcp"
        },
        {
          "containerPort": 3001,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {
          "name": "RUST_LOG",
          "value": "info"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/proxyforge",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      }
    }
  ]
}
```

### Google Cloud Platform (Cloud Run)

```bash
# Build and push image
gcloud builds submit --tag gcr.io/PROJECT_ID/proxyforge

# Deploy to Cloud Run
gcloud run deploy proxyforge \
  --image gcr.io/PROJECT_ID/proxyforge \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated \
  --port 3001 \
  --memory 512Mi \
  --cpu 1
```

### Azure (Container Instances)

```bash
# Create resource group
az group create --name proxyforge-rg --location eastus

# Deploy container
az container create \
  --resource-group proxyforge-rg \
  --name proxyforge \
  --image proxyforge/proxyforge:latest \
  --dns-name-label proxyforge \
  --ports 8888 3001 \
  --cpu 1 \
  --memory 1
```

---

## Package Manager Distribution

### Homebrew (macOS/Linux)

Create formula `proxyforge.rb`:

```ruby
class Proxyforge < Formula
  desc "Open source HTTP/HTTPS debugging proxy"
  homepage "https://github.com/proxyforge/proxyforge"
  url "https://github.com/proxyforge/proxyforge/archive/v0.1.0.tar.gz"
  sha256 "..."
  license "MIT OR Apache-2.0"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  service do
    run [opt_bin/"proxyforge"]
    keep_alive true
    log_path var/"log/proxyforge.log"
    error_log_path var/"log/proxyforge.err"
  end

  test do
    system "#{bin}/proxyforge", "--version"
  end
end
```

Install:

```bash
brew tap proxyforge/tap
brew install proxyforge
brew services start proxyforge
```

### Snap (Linux)

Create `snapcraft.yaml`:

```yaml
name: proxyforge
version: '0.1.0'
summary: Open source HTTP/HTTPS debugging proxy
description: |
  ProxyForge is a high-performance debugging proxy built in Rust
  with a modern web-based UI.

grade: stable
confinement: strict
base: core22

apps:
  proxyforge:
    command: bin/proxyforge
    daemon: simple
    plugs:
      - network
      - network-bind

parts:
  proxyforge:
    plugin: rust
    source: .
    build-packages:
      - pkg-config
      - libssl-dev
```

Build and publish:

```bash
snapcraft
snapcraft upload --release=stable proxyforge_0.1.0_amd64.snap
```

### AUR (Arch Linux)

Create `PKGBUILD`:

```bash
pkgname=proxyforge
pkgver=0.1.0
pkgrel=1
pkgdesc="Open source HTTP/HTTPS debugging proxy"
arch=('x86_64')
url="https://github.com/proxyforge/proxyforge"
license=('MIT' 'Apache')
depends=('gcc-libs')
makedepends=('rust' 'cargo')
source=("$pkgname-$pkgver.tar.gz::https://github.com/proxyforge/proxyforge/archive/v$pkgver.tar.gz")
sha256sums=('...')

build() {
    cd "$pkgname-$pkgver"
    cargo build --release --locked
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 target/release/proxyforge "$pkgdir/usr/bin/proxyforge"
    install -Dm644 LICENSE-MIT "$pkgdir/usr/share/licenses/$pkgname/LICENSE-MIT"
}
```

---

## Production Configuration

### Environment Variables

```bash
# Logging
export RUST_LOG=info
export RUST_BACKTRACE=1

# Custom config path
export PROXYFORGE_CONFIG=/etc/proxyforge/config.toml

# Performance tuning
export TOKIO_WORKER_THREADS=4
```

### Configuration File

Create `/etc/proxyforge/config.toml`:

```toml
[general]
api_port = 3001
proxy_port = 8888
host = "0.0.0.0"
log_level = "info"

[tls]
cert_dir = "/var/lib/proxyforge/certs"
auto_generate = true

[storage]
data_dir = "/var/lib/proxyforge"
db_path = "/var/lib/proxyforge/traffic.db"
max_entries = 100000

[performance]
max_connections = 10000
request_timeout_secs = 30
buffer_size = 8192

[security]
enable_auth = false
api_key = ""
allowed_origins = ["*"]
```

### Reverse Proxy (nginx)

```nginx
upstream proxyforge_api {
    server localhost:3001;
}

server {
    listen 80;
    server_name proxyforge.example.com;
    
    location / {
        proxy_pass http://proxyforge_api;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
    
    location /api/ws {
        proxy_pass http://proxyforge_api;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

---

## Monitoring and Observability

### Health Checks

```bash
# API health check
curl http://localhost:3001/api/health

# Metrics endpoint
curl http://localhost:3001/api/metrics
```

### Prometheus Integration

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'proxyforge'
    static_configs:
      - targets: ['localhost:3001']
    metrics_path: '/api/metrics'
```

### Logging

```bash
# View logs
tail -f /var/log/proxyforge/proxyforge.log

# With systemd
journalctl -u proxyforge -f

# Docker
docker logs -f proxyforge
```

---

## Security Considerations

### TLS/SSL
- Generate strong CA certificates
- Rotate certificates regularly
- Use TLS 1.3 for upstream connections

### Network Security
- Use firewall rules to restrict access
- Enable authentication for production
- Use HTTPS for API access

### Data Protection
- Encrypt sensitive data at rest
- Implement access controls
- Regular security audits

---

## Backup and Recovery

### Backup Data

```bash
# Backup database
cp ~/.proxyforge/traffic.db ~/backups/traffic-$(date +%Y%m%d).db

# Backup certificates
tar -czf ~/backups/certs-$(date +%Y%m%d).tar.gz ~/.proxyforge/certs/
```

### Restore Data

```bash
# Restore database
cp ~/backups/traffic-20260314.db ~/.proxyforge/traffic.db

# Restore certificates
tar -xzf ~/backups/certs-20260314.tar.gz -C ~/.proxyforge/
```

---

## Troubleshooting

### Common Issues

**Port conflicts**
```bash
# Check port usage
lsof -i :8888
lsof -i :3001

# Use different ports
proxyforge --proxy-port 9999 --api-port 4001
```

**Permission errors**
```bash
# Fix permissions
sudo chown -R $USER:$USER ~/.proxyforge
chmod 755 ~/.proxyforge
```

**High memory usage**
```bash
# Reduce max entries
proxyforge --max-requests 5000

# Clear old data
curl -X POST http://localhost:3001/api/traffic/clear
```

---

## Resources

- [README](../README.md)
- [Architecture Guide](ARCHITECTURE.md)
- [Development Guide](DEVELOPMENT.md)
- [API Documentation](API.md)
