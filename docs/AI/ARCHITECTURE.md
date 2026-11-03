# Architecture Specification: identity-service

## 1. High-Level Architecture Overview

```text
                  Authentication Providers
                 ┌──────────┬──────────┐
                 │  Google  │    X     │
                 ├──────────┼──────────┤
                 │ Supabase │  Others  │
                 └─────┬────┴─────┬────┘
                       │          │
                       ▼          ▼
              ┌────────────────────────┐
              │    identity-service    │
              │                        │
              │  Provider Verification │
              │  Identity Management   │
              │  User Management       │
              │  JWT Issuer / JWKS     │
              └───────────┬────────────┘
                          │
                          ▼
                      PostgreSQL
                          │
               ┌──────────┴──────────┐
               ▼                     ▼
         Backend API A         Backend API B
               │                     │
               └────── Verify JWT ───┘
```

## 2. Module Boundaries

```text
src/
├── main.rs                 # Application entrypoint & graceful shutdown
├── config.rs               # Strongly-typed environment configuration
├── error.rs                # Unified error handling & HTTP JSON responses
├── state.rs                # Shared application state (db pool, jwt engine, config, providers)
├── api/                    # HTTP routing & handlers
│   ├── mod.rs              # Router assembly & middleware
│   ├── routes.rs           # Route definitions
│   ├── auth.rs             # /v1/auth/* endpoints (exchange, refresh, revoke)
│   ├── users.rs            # /v1/users/* endpoints (me, :id)
│   └── jwks.rs             # /.well-known/jwks.json public key endpoint
├── providers/              # External identity provider integrations
│   ├── mod.rs              # Provider trait & registry
│   ├── google.rs           # Google ID token verification
│   ├── x.rs                # X (Twitter) OAuth2 verification
│   └── supabase.rs         # Supabase JWT verification
├── services/               # Core business logic
│   ├── mod.rs
│   ├── identity.rs         # Provider identity resolution & account linking
│   ├── user.rs             # User retrieval and profile updates
│   └── token.rs            # Access token & refresh token issuance & revocation
├── models/                 # Domain entities & DTOs
│   ├── mod.rs
│   ├── user.rs             # User entity
│   └── identity.rs         # Identity entity & Provider enums
└── db/                     # Database access layer & migrations
    ├── mod.rs              # Database connection pool setup & health check
    └── migrations/         # SQL migration scripts
```

## 3. Database Schema

### Table: `users`
- `id`: UUID PRIMARY KEY DEFAULT gen_random_uuid()
- `email`: TEXT NULL
- `name`: TEXT NULL
- `avatar_url`: TEXT NULL
- `status`: TEXT NOT NULL DEFAULT 'active'
- `created_at`: TIMESTAMPTZ NOT NULL DEFAULT now()
- `updated_at`: TIMESTAMPTZ NOT NULL DEFAULT now()

### Table: `identities`
- `id`: UUID PRIMARY KEY DEFAULT gen_random_uuid()
- `user_id`: UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE
- `provider`: TEXT NOT NULL
- `provider_user_id`: TEXT NOT NULL
- `created_at`: TIMESTAMPTZ NOT NULL DEFAULT now()
- UNIQUE (provider, provider_user_id)
- INDEX on user_id

### Table: `refresh_tokens`
- `id`: UUID PRIMARY KEY DEFAULT gen_random_uuid()
- `user_id`: UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE
- `token_hash`: TEXT NOT NULL UNIQUE
- `expires_at`: TIMESTAMPTZ NOT NULL
- `revoked`: BOOLEAN NOT NULL DEFAULT false
- `created_at`: TIMESTAMPTZ NOT NULL DEFAULT now()

## 4. Cryptography and JWT Specification
- Key Algorithm: Ed25519 (EdDSA)
- Token claims:
  - `iss`: Configured issuer string (e.g. `identity-service`)
  - `sub`: Internal user UUID
  - `aud`: Configured audience string (e.g. `backend-services`)
  - `iat`: Unix epoch timestamp (seconds)
  - `exp`: Unix epoch timestamp (seconds)
  - `jti`: Unique token UUID
- Header includes: `kid` matching the public key ID exposed in `/.well-known/jwks.json`.
