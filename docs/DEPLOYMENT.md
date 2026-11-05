# Identity Service Deployment Guide (`identity-service`)

This document defines the production deployment architecture, operational runbook, and security procedures for `identity-service`, adhering to the Universal AI Backend Deployment Specification.

---

## 1. Project & Architecture Overview

`identity-service` is a high-performance, stateless identity and authentication microservice written in Rust.

- **Language & Runtime**: Rust (Edition 2021), compiled to a native ELF x86_64 binary.
- **Web Framework**: Axum 0.8 with Tower HTTP middleware.
- **Async Runtime**: Tokio 1.0 (multi-threaded work-stealing executor).
- **Database Engine**: External PostgreSQL 14+ via SQLx 0.8 with Rustls.
- **Cryptography Engine**: Pure-Rust Ed25519 (EdDSA) signing and verification via `ed25519-dalek` with RFC 7517 / RFC 8037 JWKS exposure at `/.well-known/jwks.json`.
- **Database Migrations**: Embedded into the binary at compile time via `sqlx::migrate!("./migrations")` and automatically executed upon application startup.
- **Process Model**: Single non-root process per container with native Tokio concurrency; no external process manager (PM2/Gunicorn) is used or needed.
- **Background Workers**: Built-in async background worker running inside Tokio every 6 hours to clean up expired and revoked refresh tokens.

---

## 2. Docker Build & Runtime Architecture

The deployment uses an optimized, multi-stage Docker build separating compilation dependencies from the runtime image.

```text
┌────────────────────────────────────────────────────────────────────────┐
│ Stage 1: Builder (rust:1-slim-bookworm)                                │
│                                                                        │
│ • Dependency manifests: Cargo.toml, Cargo.lock                         │
│ • Database migrations:  migrations/ (embedded via sqlx::migrate!)     │
│ • Source code:          src/                                           │
│ • BuildKit Cache Mounts:                                               │
│     - /usr/local/cargo/registry (Crates.io download cache)             │
│     - /usr/local/cargo/git      (Git index cache)                      │
│     - /app/target               (Incremental compilation cache)        │
│ • Output: Optimized Release binary /usr/local/bin/identity-service     │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ COPY --from=builder binary
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Stage 2: Runtime (debian:bookworm-slim)                                │
│                                                                        │
│ • Base image: Minimal Debian 12 Bookworm (glibc matched ABI)           │
│ • Runtime dependencies: ca-certificates (OAuth TLS), curl (probes)     │
│ • Security: Non-root user (appuser:appgroup, UID/GID 10001)            │
│ • Listening: 0.0.0.0:8080                                              │
│ • Healthcheck: curl http://127.0.0.1:8080/healthz                     │
│ • Lifecycle: Exec-form ENTRYPOINT, PID 1 handles SIGTERM gracefully    │
│ • Image size: ~100MB (no compilers, no package caches, no dev tools)   │
└────────────────────────────────────────────────────────────────────────┘
```

### 2.1 Build Caching Strategy
- Cargo registry downloads and target compilation artifacts use BuildKit cache mounts (`--mount=type=cache`), allowing subsequent builds to complete in seconds.
- Database migrations from `migrations/` are baked into the binary by `sqlx::migrate!`, eliminating any runtime filesystem dependency on SQL files.

### 2.2 Security Hardening
- **Non-root execution**: Runs under dedicated unprivileged system user `appuser` (UID: `10001`, GID: `10001`).
- **No privilege escalation**: Supported via `security_opt: ["no-new-privileges:true"]`.
- **Clean filesystem**: Build compilers, package managers, and development tools are excluded from the runtime image.
- **Signal handling**: Binary runs as PID 1 (exec form `ENTRYPOINT ["/usr/local/bin/identity-service"]`) and handles `SIGTERM` and `SIGINT` gracefully.

---

## 3. Database Architecture & Connectivity

### 3.1 External Database First
In adherence to the External Infrastructure First principle:
- `identity-service` does **not** package a bundled database container in production.
- An external PostgreSQL 14+ database instance must be provided.
- The connection is configured exclusively via `DATABASE_URL`.

### 3.2 Connectivity Modes
1. **Managed Cloud Database** (AWS RDS, GCP Cloud SQL, Supabase, Neon):
   ```bash
   DATABASE_URL=postgres://identity_user:securepassword@db.prod.example.com:5432/identity?sslmode=require
   ```
2. **PostgreSQL on Docker Host Machine (Linux / macOS)**:
   The `docker-compose.yml` provides `extra_hosts: ["host.docker.internal:host-gateway"]`:
   ```bash
   DATABASE_URL=postgres://postgres:postgrespassword@host.docker.internal:5432/identity
   ```
3. **PostgreSQL on Docker Internal Network**:
   ```bash
   DATABASE_URL=postgres://postgres:postgrespassword@postgres-service-name:5432/identity
   ```

### 3.3 Database Migrations
- Migrations located in [`migrations/0001_initial_schema.sql`](file:///ssd0/git/identity-service/migrations/0001_initial_schema.sql) are run automatically upon container startup.
- Migrations are idempotent and transactional. No external CLI (`sqlx-cli`) is required in production.

---

## 4. Configuration & Environment Variables

All runtime configuration is supplied via environment variables or a `.env` file (which is never baked into the Docker image).

| Variable | Required | Default | Description |
|---|:---:|---|---|
| `APP_ENV` | Yes | `production` | Environment mode (`production`, `development`, `test`). |
| `APP_HOST` | No | `0.0.0.0` | IP interface to bind inside container. |
| `APP_PORT` | No | `8080` | Port to listen inside container. |
| `DATABASE_URL` | **Yes** | — | PostgreSQL connection URI. |
| `DATABASE_MAX_CONNECTIONS` | No | `20` | Maximum SQLx connection pool size. |
| `DATABASE_MIN_CONNECTIONS` | No | `5` | Minimum idle SQLx connections. |
| `JWT_ISSUER` | No | `identity-service` | Issuer (`iss` claim) in issued JWTs. |
| `JWT_AUDIENCE` | No | `backend-services` | Audience (`aud` claim) in issued JWTs. |
| `JWT_KEY_ID` | No | `key-1` | Key ID (`kid`) exposed in JWKS. |
| `JWT_ACCESS_TOKEN_TTL_SECONDS` | No | `3600` | Access token lifetime (default 1 hour). |
| `JWT_REFRESH_TOKEN_TTL_SECONDS` | No | `2592000` | Refresh token lifetime (default 30 days). |
| `JWT_PRIVATE_KEY_PEM` | **Prod Required\*** | — | Inline PKCS#8 Ed25519 private key in PEM format. |
| `JWT_PRIVATE_KEY_PATH` | **Prod Required\*** | — | Path to PKCS#8 Ed25519 private key file. |
| `GOOGLE_CLIENT_ID` | Optional | — | Google OAuth2 Client ID for ID token audience check. |
| `X_CLIENT_ID` | Optional | — | X (Twitter) OAuth2 Client ID. |
| `SUPABASE_URL` | Optional | — | Supabase project URL for JWT validation. |
| `SUPABASE_JWT_SECRET` | Optional | — | Supabase HMAC secret for offline verification. |
| `RUST_LOG` | No | `identity_service=info,tower_http=info` | Tracing log filter levels. |

> [!CAUTION]
> **Production Key Invariant**:
> When `APP_ENV=production`, either `JWT_PRIVATE_KEY_PEM` or `JWT_PRIVATE_KEY_PATH` **must** be provided. Ephemeral in-memory keys are strictly forbidden in production to avoid key desynchronization across restarts or multi-replica deployments.

### Generating an Ed25519 Private Key
```bash
# Generate Ed25519 PKCS#8 PEM private key
openssl genpkey -algorithm ed25519 -out private.pem
chmod 0600 private.pem
```

---

## 5. Deployment Commands & Runbook

### 5.1 Docker Compose Deployment (Recommended)

1. **Prepare Environment File**:
   ```bash
   cp .env.example .env
   # Edit .env with production credentials and database URL
   nano .env
   ```

2. **Validate Configuration**:
   ```bash
   docker compose config
   ```

3. **Build the Container Image**:
   ```bash
   docker compose build
   ```

4. **Start the Service in Background**:
   ```bash
   docker compose up -d
   ```

5. **Verify Running State & Health**:
   ```bash
   docker compose ps
   curl -i http://localhost:8080/healthz
   ```

### 5.2 Standalone Docker Commands

If deploying without Docker Compose (e.g., in Nomad, Kubernetes, or standalone Docker daemon):

```bash
# Build image
docker build -t identity-service:latest .

# Run container
docker run -d \
  --name identity-service \
  --restart unless-stopped \
  -p 8080:8080 \
  --env-file .env \
  --security-opt no-new-privileges:true \
  identity-service:latest
```

---

## 6. Operational Procedures

### 6.1 Inspecting Logs
```bash
# Follow live logs
docker compose logs -f identity-service

# View last 100 log lines with timestamps
docker compose logs --tail=100 -t identity-service
```

### 6.2 Service Probes & Health Checks
- **Liveness Probe**: `GET /healthz` (returns `200 OK` with service version and timestamp)
- **Readiness Probe**: `GET /ready` (tests live database connectivity `SELECT 1`; returns `200 OK` when healthy, `503 Service Unavailable` if database disconnected)
- **JWKS Endpoint**: `GET /.well-known/jwks.json` (returns Ed25519 public keys for downstream services)

```bash
# Liveness
curl http://localhost:8080/healthz

# Readiness (Database check)
curl http://localhost:8080/ready
```

### 6.3 Restarting the Service
```bash
docker compose restart identity-service
```

### 6.4 Stopping the Service
```bash
docker compose down
```

### 6.5 Zero-Downtime Application Update
```bash
# Pull/Rebuild and recreate container with new image
docker compose up -d --build identity-service
```

---

## 7. Troubleshooting & Diagnostics

| Symptom | Probable Cause | Resolution |
|---|---|---|
| Container exits immediately with error code 1 | `APP_ENV=production` set without `JWT_PRIVATE_KEY_PEM` or `JWT_PRIVATE_KEY_PATH` | Supply valid Ed25519 private key via `JWT_PRIVATE_KEY_PEM` or mount a key file and set `JWT_PRIVATE_KEY_PATH`. |
| Container exits with database connection failure | External PostgreSQL unreachable or incorrect credentials | Verify `DATABASE_URL`. Check that PostgreSQL allows connections from Docker network. If running on host, use `host.docker.internal`. |
| `/ready` returns HTTP 503 | Database connection dropped or connection pool exhausted | Check database logs, ensure maximum pool size (`DATABASE_MAX_CONNECTIONS`) does not exceed PostgreSQL `max_connections`. |
| Downstream services fail to verify JWTs | In-memory ephemeral key generated or key ID mismatch | Ensure a fixed `JWT_PRIVATE_KEY_PEM` is configured and downstream services fetch keys from `/.well-known/jwks.json`. |
| Outgoing OAuth calls fail with certificate error | Network proxy or SSL inspection intercepting traffic | Ensure container has access to default CA certificates (included via `ca-certificates` in runtime stage). |
