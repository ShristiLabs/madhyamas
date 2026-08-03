# Madhyamas Docker Image
# Multi-stage build — single unified binary with embedded web UI

# Frontend build stage
FROM node:20-alpine AS frontend-builder

WORKDIR /app/web

# Copy frontend files
COPY web/package.json web/package-lock.json ./
RUN npm ci

COPY web/ ./
RUN npm run build

# Backend build stage
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl openssl-libs-static pkgconf build-base

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/madhyamas/Cargo.toml ./crates/madhyamas/
COPY crates/madhyamas-core/Cargo.toml ./crates/madhyamas-core/
COPY crates/madhyamas-api/Cargo.toml ./crates/madhyamas-api/
COPY crates/madhyamas-cli/Cargo.toml ./crates/madhyamas-cli/
COPY crates/madhyamas-mcp/Cargo.toml ./crates/madhyamas-mcp/
COPY crates/madhyamas-plugin-sdk/Cargo.toml ./crates/madhyamas-plugin-sdk/

# Create dummy files to build dependencies
RUN mkdir -p crates/madhyamas/src crates/madhyamas-core/src crates/madhyamas-api/src crates/madhyamas-cli/src crates/madhyamas-mcp/src crates/madhyamas-plugin-sdk/src crates/madhyamas-plugin-sdk/examples
RUN echo "fn main() {}" > crates/madhyamas/src/main.rs
RUN echo "fn main() {}" > crates/madhyamas-core/src/lib.rs
RUN echo "fn main() {}" > crates/madhyamas-api/src/lib.rs
RUN echo "fn main() {}" > crates/madhyamas-cli/src/main.rs
RUN echo "pub fn dummy() {}" > crates/madhyamas-cli/src/lib.rs
RUN echo "pub fn dummy() {}" > crates/madhyamas-mcp/src/lib.rs
RUN echo "fn main() {}" > crates/madhyamas-mcp/src/main.rs
RUN echo "pub fn dummy() {}" > crates/madhyamas-plugin-sdk/src/lib.rs
RUN echo "fn main() {}" > crates/madhyamas-plugin-sdk/examples/cors_helper.rs
RUN echo "fn main() {}" > crates/madhyamas-plugin-sdk/examples/domain_blocker.rs
RUN echo "fn main() {}" > crates/madhyamas-plugin-sdk/examples/request_logger.rs

# Copy web dist for rust-embed (needed at compile time)
COPY --from=frontend-builder /app/web/dist ./web/dist

# Build dependencies (only the madhyamas package, not all crates)
RUN cargo build --release -p madhyamas --locked

# Copy actual source files
COPY crates/madhyamas/src ./crates/madhyamas/src
COPY crates/madhyamas-core/src ./crates/madhyamas-core/src
COPY crates/madhyamas-api/src ./crates/madhyamas-api/src
COPY crates/madhyamas-cli/src ./crates/madhyamas-cli/src
COPY crates/madhyamas-mcp/src ./crates/madhyamas-mcp/src
COPY crates/madhyamas-plugin-sdk/src ./crates/madhyamas-plugin-sdk/src
COPY crates/madhyamas-plugin-sdk/examples ./crates/madhyamas-plugin-sdk/examples
COPY crates/madhyamas-core/tests ./crates/madhyamas-core/tests

# Touch source files to invalidate cache and force rebuild
RUN find crates -name "*.rs" -exec touch {} \;

# Build the unified binary (includes proxy + web UI + MCP + CLI)
RUN cargo build --release -p madhyamas --locked

# Runtime stage
FROM alpine:3.19

RUN apk add --no-cache ca-certificates openssl

# Create non-root user
RUN addgroup -S madhyamas && adduser -S madhyamas -G madhyamas

WORKDIR /app

# Copy the single unified binary (web UI is embedded)
COPY --from=builder /app/target/release/madhyamas /usr/local/bin/madhyamas

# Create directories for data
RUN mkdir -p /data/certs /data/sessions && chown -R madhyamas:madhyamas /data

USER madhyamas

# Expose ports
EXPOSE 3001 8888

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3001/health || exit 1

# Default configuration
ENV MADHYAMAS_PROXY_PORT=8888
ENV MADHYAMAS_API_PORT=3001
ENV MADHYAMAS_DATA_DIR=/data
ENV MADHYAMAS_LOG_LEVEL=info

ENTRYPOINT ["madhyamas"]
CMD []
