# Madhyamas Docker Image
# Multi-stage build for optimized image size

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

# Create dummy files to build dependencies
RUN mkdir -p crates/madhyamas/src crates/madhyamas-core/src crates/madhyamas-api/src crates/madhyamas-cli/src crates/madhyamas-mcp/src
RUN echo "fn main() {}" > crates/madhyamas/src/main.rs
RUN echo "fn main() {}" > crates/madhyamas-core/src/lib.rs
RUN echo "fn main() {}" > crates/madhyamas-api/src/lib.rs
RUN echo "fn main() {}" > crates/madhyamas-cli/src/main.rs
RUN echo "pub fn dummy() {}" > crates/madhyamas-mcp/src/lib.rs
RUN echo "fn main() {}" > crates/madhyamas-mcp/src/main.rs

# Build dependencies
RUN cargo build --release

# Copy actual source files
COPY crates/madhyamas/src ./crates/madhyamas/src
COPY crates/madhyamas-core/src ./crates/madhyamas-core/src
COPY crates/madhyamas-api/src ./crates/madhyamas-api/src
COPY crates/madhyamas-cli/src ./crates/madhyamas-cli/src
COPY crates/madhyamas-mcp/src ./crates/madhyamas-mcp/src

# Touch source files to invalidate cache and force rebuild
RUN find crates -name "*.rs" -exec touch {} \;

# Build the application (main server, CLI, and MCP)
RUN cargo build --release -p madhyamas -p madhyamas-cli -p madhyamas-mcp

# Runtime stage
FROM alpine:3.19

RUN apk add --no-cache ca-certificates openssl

# Create non-root user
RUN addgroup -S madhyamas && adduser -S madhyamas -G madhyamas

WORKDIR /app

# Copy binaries from builder
COPY --from=builder /app/target/release/madhyamas /usr/local/bin/madhyamas
COPY --from=builder /app/target/release/madhyamas-cli /usr/local/bin/madhyamas-cli
COPY --from=builder /app/target/release/madhyamas-mcp /usr/local/bin/madhyamas-mcp

# Copy web assets (built separately)
COPY --from=frontend-builder /app/web/dist ./web/dist

# Create directories for data
RUN mkdir -p /data/certs /data/sessions && chown -R madhyamas:madhyamas /data

USER madhyamas

# Expose ports
EXPOSE 3001 8888

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3001/api/health || exit 1

# Default configuration
ENV MADHYAMAS_PROXY_PORT=8888
ENV MADHYAMAS_PROXY_TLS_PORT=8443
ENV MADHYAMAS_API_PORT=3001
ENV MADHYAMAS_DATA_DIR=/data
ENV MADHYAMAS_LOG_LEVEL=info

ENTRYPOINT ["madhyamas"]
CMD []
