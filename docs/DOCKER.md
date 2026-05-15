# Docker Guide

## Goal

This project supports two Docker startup paths:

- Release artifact image: copy and run the existing `release/identity-service` binary.
- Source build image: compile the Rust project inside Docker, then run the compiled binary.

Both paths use domestic mirror defaults for China-based development:

- Debian apt mirror: `http://mirrors.aliyun.com`.
- Cargo sparse registry mirror: `sparse+https://rsproxy.cn/index/`.
- Rustup distribution mirror: `https://rsproxy.cn`.

## Prerequisites

Generate local JWT keys before starting either Docker path:

```bash
mkdir -p secrets
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out secrets/jwt_private.pem
openssl rsa -pubout -in secrets/jwt_private.pem -out secrets/jwt_public.pem
```

The startup scripts run the container as the current host user by default so bind-mounted key files remain readable.

## Minimal Environment

Docker Compose reads a local `.env` file when it exists.
Start from the small template:

```bash
cp .env.example .env
```

For the default in-memory MVP, the only value you normally need to change is:

```bash
IDENTITY_REFRESH_TOKEN_HMAC_SECRET=replace-with-a-long-local-secret
```

Supabase is still part of the MVP.
It is disabled in the minimal local `.env.example` so Docker can start without a real Supabase project.
For a normal Supabase project, enable it with only:

```bash
IDENTITY_PROVIDER_SUPABASE_ENABLED=true
IDENTITY_PROVIDER_SUPABASE_PROJECT_ID=ahjhppptrqnrhcpdpcew
```

The service derives these values automatically:

- project URL: `https://ahjhppptrqnrhcpdpcew.supabase.co`
- issuer: `https://ahjhppptrqnrhcpdpcew.supabase.co/auth/v1`
- JWKS URL: `https://ahjhppptrqnrhcpdpcew.supabase.co/auth/v1/.well-known/jwks.json`
- audience: `authenticated`

Supabase keys are not configured in this IAM service for the MVP.

- Supabase `anon` key belongs in the frontend project, for example `VITE_SUPABASE_ANON_KEY`.
- Supabase `service_role` key must not be exposed to the frontend and is not used by this IAM MVP.
- This service receives a Supabase `access_token` from the client and verifies it through Supabase JWKS.

Frontend-side example:

```bash
VITE_SUPABASE_URL=https://your-project.supabase.co
VITE_SUPABASE_ANON_KEY=your-supabase-anon-key
```

Enable browser frontend direct mode only when a frontend needs to call this service directly:

```bash
IDENTITY_FRONTEND_DIRECT_ENABLED=true
IDENTITY_FRONTEND_ALLOWED_ORIGINS=http://localhost:5173,http://127.0.0.1:5173
```

Do not use wildcard origins. Add the exact frontend origins that are allowed to call this service.

## Option 1 - Run Existing Release Binary

Build the local release artifact first:

```bash
./scripts/build_release.sh
```

Start the Docker container from `release/identity-service`:

```bash
./scripts/docker_start_release.sh
```

Detached mode:

```bash
./scripts/docker_start_release.sh -d
```

This path does not compile Rust inside Docker. It is fastest when `release/identity-service` already exists.

## Option 2 - Compile Inside Docker

Start the Docker container and compile the project in the Docker builder stage:

```bash
./scripts/docker_start_source.sh
```

Detached mode:

```bash
./scripts/docker_start_source.sh -d
```

This path is slower on the first run, but it creates a runtime image from a binary compiled inside the container.
Use this path if the host-built release binary has Linux or glibc compatibility issues.

## Service URL

Default local endpoint:

```bash
curl http://127.0.0.1:3000/health
curl http://127.0.0.1:3000/ready
```

Change the host port:

```bash
IDENTITY_DOCKER_HTTP_PORT=3001 ./scripts/docker_start_source.sh -d
```

## Mirror Overrides

Override mirrors when needed:

```bash
IDENTITY_DOCKER_APT_MIRROR=http://mirrors.tuna.tsinghua.edu.cn \
IDENTITY_DOCKER_CARGO_REGISTRY_MIRROR=sparse+https://rsproxy.cn/index/ \
./scripts/docker_start_source.sh
```

Override base images if your environment uses a private registry or Docker Hub mirror:

```bash
IDENTITY_DOCKER_RUST_IMAGE=rust:1-bookworm \
IDENTITY_DOCKER_DEBIAN_IMAGE=debian:bookworm-slim \
./scripts/docker_start_source.sh
```

The release image uses only `IDENTITY_DOCKER_DEBIAN_IMAGE`.

## Runtime Configuration

The compose files now set only the container-specific defaults:

- `IDENTITY_HTTP_HOST=0.0.0.0`
- `IDENTITY_HTTP_PORT=3000`
- JWT key paths under `/app/secrets`
- a local-only default `IDENTITY_REFRESH_TOKEN_HMAC_SECRET`

Other `IDENTITY_*` variables should live in `.env` for Docker use.

Example `.env` values for PostgreSQL:

```bash
IDENTITY_PERSISTENCE_BACKEND=postgres
IDENTITY_DATABASE_URL=postgres://identity:identity@postgres:5432/identity
```

Example `.env` values for SQLite:

```bash
IDENTITY_PERSISTENCE_BACKEND=sqlite
IDENTITY_DATABASE_URL=sqlite:///app/data/identity.db
```

Then start the container:

```bash
./scripts/docker_start_source.sh -d
```

Do not use the default HMAC secret in production.

## Backend JWT Integration

Other backend services do not need to call this service for every request.
They can verify the access token as a JWT and read:

- `sub`: platform `internal_user_id`.
- `sid`: platform session id.
- `client_id`: platform client id.

Public signing keys are exposed at:

```text
GET /.well-known/jwks.json
```

Backends must validate signature, `kid`, `iss`, `aud`, and `exp` before trusting `sub`.
