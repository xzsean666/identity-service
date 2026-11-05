# ==============================================================================
# Stage 1: Builder
# ==============================================================================
FROM rust:1-slim-bookworm AS builder

WORKDIR /app

# Install native build tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Copy database migrations (embedded into binary at compile time via sqlx::migrate!)
COPY migrations/ ./migrations/

# Copy application source code
COPY src/ ./src/

# Compile production binary in release mode using BuildKit cache mounts
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked --bin identity-service && \
    cp /app/target/release/identity-service /usr/local/bin/identity-service

# ==============================================================================
# Stage 2: Runtime
# ==============================================================================
FROM debian:bookworm-slim AS runtime

# Install minimal runtime dependencies:
# - ca-certificates: Required for HTTPS calls to external OAuth/OIDC providers (Google, X, Supabase)
# - curl: Required for container health checks
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create unprivileged system group and user
RUN groupadd -g 10001 appgroup && \
    useradd -u 10001 -g appgroup -m -s /bin/sh appuser

WORKDIR /app

# Copy production binary from builder
COPY --from=builder /usr/local/bin/identity-service /usr/local/bin/identity-service

# Configure strict file permissions
RUN chown -R appuser:appgroup /app /usr/local/bin/identity-service && \
    chmod 0755 /usr/local/bin/identity-service

# Default environment configuration (bind to all interfaces on 8080)
ENV APP_ENV=production \
    APP_HOST=0.0.0.0 \
    APP_PORT=8080 \
    RUST_LOG=identity_service=info,tower_http=info

# Expose default HTTP port
EXPOSE 8080

# Enforce non-root execution
USER appuser

# Container healthcheck using real liveness probe
HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://127.0.0.1:8080/healthz || exit 1

# Launch application (PID 1 receives SIGTERM for graceful shutdown)
ENTRYPOINT ["/usr/local/bin/identity-service"]
