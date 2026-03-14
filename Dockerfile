# ProxyForge Docker Image
# Multi-stage build for optimized image size

# Build stage
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl pkgconf build-base

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/proxyforge-core/Cargo.toml ./crates/proxyforge-core/
COPY crates/proxyforge-api/Cargo.toml ./crates/proxyforge-api/
COPY crates/proxyforge-cli/Cargo.toml ./crates/proxyforge-cli/
COPY crates/proxyforge-mcp/Cargo.toml ./crates/proxyforge-mcp/

# Create dummy files to build dependencies
RUN mkdir -p crates/proxyforge-core/src crates/proxyforge-api/src crates/proxyforge-cli/src crates/proxyforge-mcp/src
RUN echo "fn main() {}" > crates/proxyforge-core/src/lib.rs
RUN echo "fn main() {}" > crates/proxyforge-api/src/lib.rs
RUN echo "fn main() {}" > crates/proxyforge-cli/src/main.rs
RUN echo "pub fn dummy() {}" > crates/proxyforge-mcp/src/lib.rs
RUN echo "fn main() {}" > crates/proxyforge-mcp/src/main.rs

# Build dependencies
RUN cargo build --release

# Copy actual source files
COPY crates/proxyforge-core/src ./crates/proxyforge-core/src
COPY crates/proxyforge-api/src ./crates/proxyforge-api/src
COPY crates/proxyforge-cli/src ./crates/proxyforge-cli/src
COPY crates/proxyforge-mcp/src ./crates/proxyforge-mcp/src

# Touch source files to invalidate cache and force rebuild
RUN find crates -name "*.rs" -exec touch {} \;

# Build the application
RUN cargo build --release -p proxyforge-cli

# Runtime stage
FROM alpine:3.19

RUN apk add --no-cache ca-certificates openssl

# Create non-root user
RUN addgroup -S proxyforge && adduser -S proxyforge -G proxyforge

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/proxyforge /usr/local/bin/proxyforge

# Copy web assets (built separately)
COPY web/dist ./web/dist

# Create directories for data
RUN mkdir -p /data/certs /data/sessions && chown -R proxyforge:proxyforge /data

USER proxyforge

# Expose ports
EXPOSE 3001 8888 8443

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3001/api/health || exit 1

# Default configuration
ENV PROXYFORGE_PROXY_PORT=8888
ENV PROXYFORGE_PROXY_TLS_PORT=8443
ENV PROXYFORGE_API_PORT=3001
ENV PROXYFORGE_DATA_DIR=/data
ENV PROXYFORGE_LOG_LEVEL=info

ENTRYPOINT ["proxyforge"]
CMD []
