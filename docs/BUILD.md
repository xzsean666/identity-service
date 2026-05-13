# Build and Usage Guide

## Current Step

Step 4 - MVP implementation has started.

The repository now contains a Rust/Axum service skeleton with an in-memory development storage adapter.

PostgreSQL remains the selected production persistence target, but the current implementation is intentionally the first executable MVP increment, not the final production storage layer.

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
    TECH_STACK.md
    MVP.md
    MODULE_EXPANSION.md
    INTEGRATION.md
    nextsession.md
  src/
    application/
    config/
    domain/
    infrastructure/
    interfaces/
    providers/
    security/
    lib.rs
    main.rs
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

- Persistence is in memory and resets on process restart.
- Supabase verification supports JWT access token validation through a configured Supabase JWKS.
- Supabase fixture tokens are available only when explicitly enabled for local tests.
- PostgreSQL persistence is required before production deployment.
- JWT revocation is stateless for external consumers; logout and password change revoke refresh-token state, while already issued access tokens remain valid until `exp`.

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
export IDENTITY_PROVIDER_SUPABASE_AUTO_PROVISION_ENABLED="true"
export IDENTITY_PROVIDER_SUPABASE_PROJECT_URL="https://example.supabase.co"
export IDENTITY_PROVIDER_SUPABASE_ISSUER="https://example.supabase.co/auth/v1"
export IDENTITY_PROVIDER_SUPABASE_AUDIENCE="authenticated"
export IDENTITY_PROVIDER_SUPABASE_JWKS_URL="https://example.supabase.co/auth/v1/.well-known/jwks.json"
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
```

Disabled providers keep their public routes registered and return `provider_disabled`.

## Supabase Input

Current Supabase adapter input is a Supabase JWT access token passed as `access_token`.

The adapter:

- Reads `kid` from the token header.
- Loads the matching key from configured Supabase JWKS.
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

Whitespace check before commit:

```bash
git diff --check
```

## PostgreSQL Target

PostgreSQL is still the required production persistence target because identity bindings, sessions, refresh tokens, and credential updates need transactional consistency.

The MVP schema lives in `migrations/` as plain PostgreSQL SQL:

```bash
psql "$DATABASE_URL" -f migrations/0001_mvp_identity_schema.up.sql
```

Rollback for local development:

```bash
psql "$DATABASE_URL" -f migrations/0001_mvp_identity_schema.down.sql
```

The migration creates only the current MVP persistence tables: `internal_users`, `external_identities`, `local_credentials`, `sessions`, and `refresh_token_records`.

The next persistence increment should add:

- Repository contracts owned by the application layer.
- PostgreSQL implementations under infrastructure.
- Transactional password-change and refresh-token rotation behavior.

Do not add Redis to the MVP unless a specific runtime need appears.

## Git Workflow

After each major step:

```bash
git add .
git commit -m "feat: <describe current step>"
```

Do not push unless explicitly requested.
