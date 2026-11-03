# Goal: Centralized Lightweight Identity Service (identity-service)

## 1. High-Level Objective
Build a lightweight, production-ready, centralized identity service in Rust that provides a unified user system for backend applications.

Key characteristics:
- **Delegated Authentication**: Verifies external authentication tokens (Google, X, Supabase, OAuth/OIDC) instead of managing passwords.
- **Unified Identity**: Maps `(provider, provider_user_id)` to a single internal `user_id` (UUID).
- **Asymmetric JWT Issuance**: Issues internal JWTs signed with Ed25519 (EdDSA) with key rotation support.
- **Local Verification for Downstreams**: Publishes public keys at `/.well-known/jwks.json`, enabling downstream microservices/APIs to verify tokens offline without calling the identity service.
- **Production-Ready & Developer-Friendly**: Clean configuration, structured error handling, structured logging/tracing, automatic migrations option, and comprehensive tests.

## 2. Core Technical Decisions
- Language: Rust 2021/2024 edition
- HTTP Framework: Axum
- Async Runtime: Tokio
- Database & ORM: PostgreSQL with SQLx (compile-time checked or clean queries with migrations)
- Signing: Ed25519 asymmetric cryptography (`ed25519-dalek` / `jsonwebtoken`)
- Configuration: Environment variables with `dotenvy` / `envy` or `serde`
- Observability: `tracing` and `tracing-subscriber`

## 3. Scope Boundaries
Included in initial scope:
- Project structure, config, health check
- Database schema and migrations (`users`, `identities`, `refresh_tokens`)
- Key management and JWKS endpoint
- External identity provider verification (Google ID Token, X/Twitter OAuth2 token, Supabase JWT)
- Internal user and identity mapping service
- Auth endpoints: `/v1/auth/exchange`, `/v1/auth/refresh`, `/v1/auth/revoke`
- User endpoints: `/v1/users/me`, `/v1/users/:id`
- Unit and integration tests

Excluded from initial scope (future extensions):
- Multi-tenant tenant isolation
- Complex RBAC / permission hierarchies
- Admin web UI
