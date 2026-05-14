# Build and Usage Guide

## Current Step

Step 4 - MVP implementation has started.

The repository now contains a Rust/Axum service skeleton with both in-memory and PostgreSQL persistence adapters.

PostgreSQL is the selected production persistence target.
The PostgreSQL schema exists in `migrations/`, and startup wiring selects either `memory` or `postgres` through centralized configuration.

## Repository Layout

```text
identity-service/
  Agent.md
  Cargo.toml
  Cargo.lock
  docs/
    ARCHITECTURE.md
    SPEC.md
    BUILD.md
    DOCKER.md
    TECH_STACK.md
    MVP.md
    MODULE_EXPANSION.md
    INTEGRATION.md
    nextsession.md
  docker/
    Dockerfile.release
    Dockerfile.source
    configure_debian_apt_mirror.sh
  docker-compose.release.yml
  docker-compose.source.yml
  src/
    application/
    config/
    domain/
    infrastructure/
      postgres/
    interfaces/
    providers/
    security/
    lib.rs
    main.rs
  scripts/
    build_release.sh
    docker_start_release.sh
    docker_start_source.sh
```

## Implemented MVP Increment

Current executable scope:

- Axum HTTP service.
- Local username/password registration.
- Local username/password login.
- Authenticated local password change.
- Supabase provider exchange through the provider adapter boundary.
- Central provider enable/disable configuration.
- `internal_user_id` identity binding.
- Session creation and revocation.
- RS256 JWT access token issuance and verification.
- Server-tracked refresh token rotation and reuse detection.
- Logout and current-user endpoints.

Current implementation limits:

- `IDENTITY_PERSISTENCE_BACKEND` defaults to `memory`, which resets on process restart.
- `IDENTITY_PERSISTENCE_BACKEND=postgres` wires business repositories to PostgreSQL and requires a migrated database.
- Supabase verification supports JWT access token validation through a configured Supabase JWKS.
- Supabase fixture tokens are available only when explicitly enabled for local tests.
- Migration execution is handled by the `migrate` binary, backed by the SQL files in `migrations/`.
- JWT revocation is stateless for external consumers; logout and password change revoke refresh-token state, while already issued access tokens remain valid until `exp`.
- Release builds are created by `scripts/build_release.sh`; the generated `release/` directory is local-only and ignored by Git.
- Docker startup supports both a prebuilt `release/identity-service` image path and a source-compiling image path.

## Prerequisites

- Rust toolchain with edition 2024 support.
- Cargo.
- OpenSSL CLI for generating local RSA development keys.

## Local Key Setup

Do not commit generated private keys.

```bash
mkdir -p secrets
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out secrets/jwt_private.pem
openssl rsa -pubout -in secrets/jwt_private.pem -out secrets/jwt_public.pem
```

The service reads key PEM files from these default paths:

- `./secrets/jwt_private.pem`
- `./secrets/jwt_public.pem`

Alternatively, provide PEM contents directly through:

- `IDENTITY_TOKEN_PRIVATE_KEY_PEM`
- `IDENTITY_TOKEN_PUBLIC_KEY_PEM`

## Required Environment

Minimum local environment:

```bash
export IDENTITY_REFRESH_TOKEN_HMAC_SECRET="replace-with-a-long-local-secret"
```

Optional environment variables:

```bash
export IDENTITY_HTTP_HOST="127.0.0.1"
export IDENTITY_HTTP_PORT="3000"
export IDENTITY_FRONTEND_DIRECT_ENABLED="false"
# Required only when IDENTITY_FRONTEND_DIRECT_ENABLED="true".
# export IDENTITY_FRONTEND_ALLOWED_ORIGINS="http://localhost:5173,http://127.0.0.1:5173"
export IDENTITY_PERSISTENCE_BACKEND="memory"
# Required only when IDENTITY_PERSISTENCE_BACKEND="postgres".
# export IDENTITY_DATABASE_URL="postgres://identity:identity@localhost:5432/identity"
export IDENTITY_TOKEN_PRIVATE_KEY_PEM_PATH="./secrets/jwt_private.pem"
export IDENTITY_TOKEN_PUBLIC_KEY_PEM_PATH="./secrets/jwt_public.pem"
export IDENTITY_TOKEN_KEY_ID="mvp-local-key"
export IDENTITY_TOKEN_ISSUER="identity-service-local"
export IDENTITY_TOKEN_AUDIENCE="platform-api"
export IDENTITY_ACCESS_TOKEN_LIFETIME_SECONDS="900"
export IDENTITY_REFRESH_TOKEN_LIFETIME_SECONDS="2592000"
export IDENTITY_SESSION_LIFETIME_SECONDS="2592000"
export IDENTITY_PROVIDER_LOCAL_PASSWORD_ENABLED="true"
export IDENTITY_PROVIDER_SUPABASE_ENABLED="true"
export IDENTITY_PROVIDER_SUPABASE_PROJECT_ID="ahjhppptrqnrhcpdpcew"
export IDENTITY_PROVIDER_SUPABASE_AUTO_PROVISION_ENABLED="true"
export IDENTITY_PROVIDER_SUPABASE_AUDIENCE="authenticated"
# Optional advanced overrides for custom Supabase auth domains.
# export IDENTITY_PROVIDER_SUPABASE_PROJECT_URL="https://example.supabase.co"
# export IDENTITY_PROVIDER_SUPABASE_ISSUER="https://example.supabase.co/auth/v1"
# export IDENTITY_PROVIDER_SUPABASE_JWKS_URL="https://example.supabase.co/auth/v1/.well-known/jwks.json"
# Optional: inline JWKS JSON for controlled test or legacy environments.
# export IDENTITY_PROVIDER_SUPABASE_JWKS_JSON='{"keys":[]}'
export IDENTITY_PROVIDER_SUPABASE_FIXTURE_TOKENS_ENABLED="false"
```

Production issuer values must be environment-unique and stable, for example:

- `https://identity.example.com`
- `urn:identity-service:prod`

The default issuer is only for local development.

## Run

```bash
cargo run
```

Health check:

```bash
curl http://127.0.0.1:3000/health
```

Readiness check:

```bash
curl http://127.0.0.1:3000/ready
```

`/health` reports that the HTTP process is running.
`/ready` reports whether required runtime dependencies are available.
When `IDENTITY_PERSISTENCE_BACKEND=postgres`, `/ready` checks PostgreSQL with `SELECT 1`.

## MVP API

```text
POST /v1/auth/register
POST /v1/auth/login
POST /v1/auth/password/change
POST /v1/auth/supabase/exchange
POST /v1/auth/refresh
POST /v1/auth/logout
GET  /v1/users/me
GET  /health
GET  /ready
GET  /.well-known/jwks.json
```

Disabled providers keep their public routes registered and return `provider_disabled`.

## Supabase Input

Current Supabase adapter input is a Supabase JWT access token passed as `access_token`.

Supabase keys are intentionally not part of this service's MVP backend configuration:

- The Supabase `anon` key is used by the frontend or client app to log in with Supabase.
- The Supabase `service_role` key is not used by this IAM MVP and must not be exposed to browser clients.
- This IAM service verifies the resulting Supabase access token through the configured Supabase JWKS URL.

For a standard Supabase project, this IAM service only needs:

```bash
IDENTITY_PROVIDER_SUPABASE_ENABLED=true
IDENTITY_PROVIDER_SUPABASE_PROJECT_ID=ahjhppptrqnrhcpdpcew
```

The service derives the Supabase project URL, issuer, and JWKS URL from the project ID.

The adapter:

- Reads `kid` from the token header.
- Loads the matching key from configured Supabase JWKS.
- Caches remote JWKS for a short interval.
- Forces one remote JWKS refresh when the cached set does not contain the token `kid`.
- Checks JWK `alg` against the token header algorithm.
- Validates issuer, audience, expiration, and subject.
- Rejects shared-secret JWKs from remote JWKS; use asymmetric Supabase signing keys for normal integration.

For local tests only, set `IDENTITY_PROVIDER_SUPABASE_FIXTURE_TOKENS_ENABLED=true` to accept a JSON fixture string passed as `access_token`.

JWT request body:

```json
{
  "access_token": "<supabase-jwt-access-token>"
}
```

Fixture request body:

```json
{
  "access_token": "{\"sub\":\"supabase-user-1\",\"exp\":4102444800,\"iss\":\"https://example.supabase.co/auth/v1\",\"aud\":\"authenticated\",\"email\":\"user@example.com\"}"
}
```

Fixture mode exists only to exercise provider normalization and identity binding without a Supabase project.

## Release Build

Build the latest release package:

```bash
./scripts/build_release.sh
```

The script performs these steps:

- Reads the current package version from `Cargo.toml`.
- Increments the patch version, for example `0.1.0` to `0.1.1`.
- Builds all Rust binaries with `cargo build --release --bins`.
- Writes the generated files into `release/`.
- Replaces `release/` only after a successful build.

`release/` keeps only the latest successful build. Older local release artifacts are removed when the new build succeeds. If the build fails, the previous `release/` directory is left untouched and the version files are restored.

Generated release files:

```text
release/
  identity-service
  migrate
  VERSION
  BUILD_INFO
  SHA256SUMS
```

`release/` is ignored by Git. Commit the script, docs, `Cargo.toml`, and `Cargo.lock` version changes, but do not commit compiled binaries unless a separate distribution process explicitly requires it.

## Docker Startup

Detailed Docker instructions live in `docs/DOCKER.md`.

Minimal Docker runtime configuration lives in `.env.example`.
Use a local `.env` only for values you actually need to override.

Start from the existing local release binary:

```bash
./scripts/build_release.sh
./scripts/docker_start_release.sh -d
```

Start from a Docker build that compiles the Rust project:

```bash
./scripts/docker_start_source.sh -d
```

Both Docker paths default to China-friendly mirrors:

- Debian apt: `http://mirrors.aliyun.com`.
- Cargo sparse registry: `sparse+https://rsproxy.cn/index/`.
- Rustup: `https://rsproxy.cn`.

Override examples:

```bash
IDENTITY_DOCKER_APT_MIRROR=http://mirrors.tuna.tsinghua.edu.cn ./scripts/docker_start_source.sh
IDENTITY_DOCKER_CARGO_REGISTRY_MIRROR=sparse+https://rsproxy.cn/index/ ./scripts/docker_start_source.sh
```

## Development Commands

Format:

```bash
cargo fmt
```

Compile check:

```bash
cargo check
```

Tests:

```bash
cargo test
```

Full E2E flow tests:

```bash
cargo test --test e2e_full_flow
```

The default E2E suite runs against the in-memory backend and covers:

- `/health` and `/ready`.
- Local username/password registration.
- Duplicate registration conflict.
- Failed and successful local login.
- Refresh token rotation and reuse detection.
- Authenticated password change.
- Old password rejection after password change.
- Refresh-token invalidation after password change and logout.
- Supabase JWT exchange through configured JWKS.
- Supabase identity reuse into the same `internal_user_id`.
- Disabled provider errors.

PostgreSQL repository integration tests are opt-in. They run only when `IDENTITY_DATABASE_URL` is present, and they expect the MVP migration to already be applied:

```bash
export IDENTITY_DATABASE_URL="postgres://identity:identity@localhost:5432/identity"
cargo run --bin migrate -- up
cargo test postgres_repositories
```

The E2E suite also runs the full HTTP flow against PostgreSQL when `IDENTITY_DATABASE_URL` is present:

```bash
export IDENTITY_DATABASE_URL="postgres://identity:identity@localhost:5432/identity"
cargo test --test e2e_full_flow
```

Whitespace check before commit:

```bash
git diff --check
```

## PostgreSQL Target

PostgreSQL is still the required production persistence target because identity bindings, sessions, refresh tokens, and credential updates need transactional consistency.

Current persistence configuration:

- `IDENTITY_PERSISTENCE_BACKEND=memory` uses the current in-memory MVP adapter and is the default.
- `IDENTITY_PERSISTENCE_BACKEND=postgres` uses PostgreSQL repositories and requires `IDENTITY_DATABASE_URL` during configuration loading.
- `IDENTITY_DATABASE_URL` is optional for the default memory backend.

The MVP schema lives in `migrations/` as plain PostgreSQL SQL and is applied through the migration binary:

```bash
export IDENTITY_DATABASE_URL="postgres://identity:identity@localhost:5432/identity"
cargo run --bin migrate -- up
```

Rollback for local development:

```bash
cargo run --bin migrate -- down 0
```

The migration creates only the current MVP persistence tables: `internal_users`, `external_identities`, `local_credentials`, `sessions`, and `refresh_token_records`.
The migration runner also creates SQLx migration tracking metadata.

Apply migrations to a fresh database or a database already managed by this runner.
If the schema was previously applied manually with `psql`, the runner cannot safely infer that state.

Implemented PostgreSQL behavior:

- Identity binding, local credential, and session repositories are implemented under `src/infrastructure/postgres/`.
- Runtime wiring selects PostgreSQL repositories when `IDENTITY_PERSISTENCE_BACKEND=postgres`.
- Session creation, refresh token exchange, reuse detection, logout revocation, and refresh-family rotation use PostgreSQL transactions.
- Refresh token exchange locks the current refresh-token row with `FOR UPDATE`.
- Local password change updates the credential hash, revokes old refresh-token state, and inserts the new refresh-token family in one PostgreSQL transaction.
- The `migrate` binary applies and reverts SQLx-tracked migrations.

Known persistence hardening left after this increment:

- Add deployment-specific backup, restore, and migration rollout guidance after the runtime target is selected.

Do not add Redis to the MVP unless a specific runtime need appears.

## Git Workflow

After each major step:

```bash
git add .
git commit -m "feat: <describe current step>"
```

Do not push unless explicitly requested.
