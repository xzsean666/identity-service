# Task Index

| Task ID | Title | Status | Dependencies | Description |
|---|---|---|---|---|
| [TASK-001](tasks/TASK-001.md) | Project Skeleton, Configuration & Health Check | DONE | None | Rust project setup, dependencies, strongly-typed config, error module, health check handler |
| [TASK-002](tasks/TASK-002.md) | Database Models, Connection Pool & Migrations | DONE | TASK-001 | SQLx migrations for users, identities, refresh_tokens, pool configuration, and db models |
| [TASK-003](tasks/TASK-003.md) | Ed25519 Key Engine, JWT Issuance & JWKS Endpoint | DONE | TASK-001 | Ed25519 key loading, JWT claim signing, token verification, and RFC 7517 JWKS output |
| [TASK-004](tasks/TASK-004.md) | External Provider Verification Engine | DONE | TASK-001 | Provider trait and adapters for Google, X, Supabase, plus Mock provider for testing |
| [TASK-005](tasks/TASK-005.md) | Identity & User Core Services | DONE | TASK-002, TASK-003, TASK-004 | Identity mapping, user provisioning, refresh token issuance/revocation logic |
| [TASK-006](tasks/TASK-006.md) | Auth & User REST API Handlers | DONE | TASK-005 | HTTP endpoints for exchange, refresh, revoke, users/me, users/:id |
| [TASK-007](tasks/TASK-007.md) | Integration Tests, OpenAPI Documentation & Hardening | DONE | TASK-006 | End-to-end integration tests, OpenAPI documentation, graceful shutdown, production docker setup |
