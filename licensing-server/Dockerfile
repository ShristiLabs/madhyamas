# Multi-stage build for the Madhyamas licensing server.
#
# Stage 1: build the Rust binary from the workspace.
# Stage 2: copy the binary into a minimal runtime image.

FROM rust:1.88-bookworm AS builder

WORKDIR /build

# Copy the entire workspace (the licensing server is a workspace member and
# depends on workspace-level dependency resolution).
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY licensing-server/ licensing-server/
COPY web/ web/

# Build the web assets first (rust-embed requires them at compile time for
# the main madhyamas crate, which is a workspace member).
RUN cd web && npm ci && npm run build || true

# Build the licensing server binary in release mode.
RUN cargo build --release -p madhyamas-licensing

# Stage 2: minimal runtime image.
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/madhyamas-licensing /app/madhyamas-licensing

EXPOSE 8080

ENV PORT=8080
ENV BIND_ADDR=0.0.0.0
ENV RUST_LOG=info

CMD ["./madhyamas-licensing", "--port", "8080"]
