# Centralized Identity Service (`identity-service`)

A high-performance, lightweight, centralized identity and authentication microservice written in Rust.

`identity-service` provides a unified user management layer across multiple backend applications. It delegates user authentication to external identity providers (Google, X / Twitter, Supabase, etc.), binds them to internal user accounts, and issues asymmetric **Ed25519 (EdDSA)** JWTs. Downstream microservices can verify tokens **offline** at high speed using the published JWKS endpoint without placing network load on the identity service.

---

## 🌟 Key Features

- **Delegated Authentication**: Verifies third-party tokens (Google ID tokens, X OAuth2 tokens, Supabase JWTs) rather than handling and storing raw passwords.
- **Unified Identity Mapping**: Maps `(provider, provider_user_id)` to a singular internal `user_id` (UUIDv4) and supports multi-account linking.
- **Asymmetric Ed25519 Cryptography**: Signs internal access tokens with Ed25519 (EdDSA / RFC 8037).
- **Offline JWT Verification (JWKS)**: Exposes RFC 7517 compliant JWKS at `/.well-known/jwks.json`. Downstream APIs cache public keys and verify JWTs locally with zero RPC latency.
- **Secure Refresh Token Rotation**: Issues cryptographically secure, high-entropy refresh tokens with SHA-256 storage hashing, single-use rotation, and instant revocation.
- **Production-Ready**: Automatic SQLx migrations on startup, graceful shutdown handling (SIGINT/SIGTERM), structured JSON error responses, CORS, and tracing observability.
- **Built-in Interactive API Docs**: OpenAPI 3.0 specification at `/openapi.json` and interactive Swagger UI at `/docs`.

---

## 🏛 Architecture Overview

```text
               External Identity Providers
           ┌──────────┬──────────┬──────────┐
           │  Google  │    X     │ Supabase │
           └────┬─────┴────┬─────┴────┬─────┘
                │          │          │
                ▼          ▼          ▼
         ┌─────────────────────────────────────┐
         │          identity-service           │
         │                                     │
         │  • Provider Verification Engine     │
         │  • Identity Resolution & Linking    │
         │  • Ed25519 JWT Engine               │
         │  • Refresh Token Lifecycle          │
         └──────────────┬──────────────────────┘
                        │
                        ▼
                 PostgreSQL DB
                 (users, identities, refresh_tokens)
                        │
            JWKS: /.well-known/jwks.json
                        │
         ┌──────────────┴──────────────┐
         ▼                             ▼
   Backend Service A             Backend Service B
   (Local offline Ed25519)       (Local offline Ed25519)
```

---

## 🚀 Quick Start

### 1. Prerequisites
- [Rust](https://rustup.rs/) (1.80+)
- [PostgreSQL](https://www.postgresql.org/) (14+) or [Docker](https://www.docker.com/)

### 2. Environment Configuration
Copy the example environment file:
```bash
cp .env.example .env
```

Key configuration options:
| Variable | Default | Description |
|---|---|---|
| `APP_ENV` | `development` | Environment mode (`development`, `production`, `test`) |
| `APP_HOST` | `0.0.0.0` | Server listening address |
| `APP_PORT` | `8080` | Server listening port |
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5432/identity` | PostgreSQL connection string |
| `JWT_ISSUER` | `identity-service` | Issuer (`iss`) claim in internal JWTs |
| `JWT_AUDIENCE` | `backend-services` | Audience (`aud`) claim in internal JWTs |
| `JWT_KEY_ID` | `key-1` | Key ID (`kid`) exposed in JWKS |
| `JWT_ACCESS_TOKEN_TTL_SECONDS` | `3600` | Access token lifespan (seconds) |
| `JWT_REFRESH_TOKEN_TTL_SECONDS` | `2592000` | Refresh token lifespan (seconds, default 30 days) |
| `JWT_PRIVATE_KEY_PEM` | *None* | Optional Ed25519 private key in PKCS#8 PEM format (auto-generates ephemeral key in dev mode) |
| `GOOGLE_CLIENT_ID` | *None* | Google OAuth2 Client ID for audience validation |
| `X_CLIENT_ID` | *None* | X (Twitter) Client ID |
| `SUPABASE_URL` | *None* | Supabase project URL |

### 3. Run with Docker Compose
```bash
docker compose up -d
```
Access Swagger UI at `http://localhost:8080/docs` and JWKS at `http://localhost:8080/.well-known/jwks.json`.

### 4. Run Locally
```bash
cargo run
```

---

## 📡 REST API Reference

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/healthz` | Public | Liveness probe returning service status and version |
| `GET` | `/.well-known/jwks.json` | Public | Public keys in RFC 7517 / RFC 8037 format |
| `GET` | `/openapi.json` | Public | OpenAPI 3.0 specification |
| `GET` | `/docs` | Public | Interactive Swagger UI documentation |
| `POST` | `/v1/auth/exchange` | Public | Exchange provider token for internal JWT & refresh token |
| `POST` | `/v1/auth/refresh` | Public | Rotate refresh token and issue new access token |
| `POST` | `/v1/auth/revoke` | Public | Invalidate an active refresh token |
| `GET` | `/v1/users/me` | Bearer JWT | Retrieve profile and linked identities of caller |
| `PATCH` | `/v1/users/me` | Bearer JWT | Update profile name and avatar URL |
| `POST` | `/v1/users/link` | Bearer JWT | Link an additional provider identity to current user |
| `GET` | `/v1/users/{id}` | Bearer JWT | Retrieve public user profile by UUID |

---

## 🔒 Offline JWT Verification for Downstream Services

Downstream microservices verify tokens **offline** using Ed25519 public keys retrieved once from `http://<identity-service>/.well-known/jwks.json`.

### Node.js / TypeScript Example (`jose` library)
```typescript
import { createRemoteJWKSet, jwtVerify } from 'jose';

const JWKS = createRemoteJWKSet(new URL('http://identity-service:8080/.well-known/jwks.json'));

async function authenticate(token: string) {
  const { payload } = await jwtVerify(token, JWKS, {
    issuer: 'identity-service',
    audience: 'backend-services',
  });
  return payload; // { sub: "user-uuid", email: "...", ... }
}
```

### Go Example (`lestrrat-go/jwx`)
```go
ctx := context.Background()
keySet, err := jwk.Fetch(ctx, "http://identity-service:8080/.well-known/jwks.json")

token, err := jwt.Parse(
    []byte(tokenString),
    jwt.WithKeySet(keySet),
    jwt.WithIssuer("identity-service"),
    jwt.WithAudience("backend-services"),
)
```

---

## 🧪 Testing

Run all unit, integration, and end-to-end tests:
```bash
cargo test
```

### Test Suites Included:
- `tests/health_check.rs`: Health endpoints and service versioning
- `tests/db_test.rs`: Database migrations, constraints, and models
- `tests/jwt_test.rs`: Ed25519 signing, verification, and JWKS RFC conformance
- `tests/provider_test.rs`: External provider adapters (Google, X, Supabase, Mock)
- `tests/service_test.rs`: Identity mapping, account linking, refresh token rotation
- `tests/api_auth_test.rs`: Auth REST handlers (`/v1/auth/*`)
- `tests/api_user_test.rs`: User REST handlers (`/v1/users/*`)
- `tests/integration_test.rs`: Full end-to-end client lifecycle with downstream offline JWKS verification

---

## 📄 License
MIT License.
