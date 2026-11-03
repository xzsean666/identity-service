# Stage 1: Build stage
FROM rust:1.80-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code and migrations
COPY src/ ./src/
COPY migrations/ ./migrations/

# Build release binary
RUN cargo build --release

# Stage 2: Runtime stage
FROM debian:bookworm-slim AS runner

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create unprivileged user
RUN useradd -m -u 10001 -s /bin/sh appuser

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/identity-service /usr/local/bin/identity-service

# Use non-root user
USER appuser

EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/healthz || exit 1

ENTRYPOINT ["/usr/local/bin/identity-service"]
