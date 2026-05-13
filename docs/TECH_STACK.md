# Technology Stack Decision

## Current Step

Documentation hardening.

This document records the fixed technology stack for the Identity Platform / IAM Service.

No implementation code is included in this step.

## Locked Decision

The implementation language is Rust.

This is fixed and is not an open question.

AI agents must not reopen the language decision unless the user explicitly asks to change the stack.

## Selected MVP Stack

The MVP stack is:

- Language: Rust
- HTTP framework: Axum
- Async runtime: Tokio
- Database: PostgreSQL
- Database access: SQLx
- Password hashing: Argon2id
- Access token format: JWT
- Refresh token strategy: server-tracked refresh token records
- Configuration: centralized typed configuration loaded at startup

Redis is not part of the MVP.

Redis may be added after the MVP for rate limiting, cache, distributed locks, or high-volume session workflows.

## Non-Selected Stacks

TypeScript, NestJS, Go, Java, Kotlin, Python, and other stacks are not active implementation targets for this project.

They should not be proposed again during normal implementation planning.

## Why Rust Is Fixed

This project is identity infrastructure.

The implementation needs:

- Strong compile-time checks.
- Explicit error handling.
- Predictable runtime behavior.
- Memory safety.
- Good concurrency safety.
- Clear module boundaries.
- Fewer hidden side effects.

These requirements match the repository's AI-oriented architecture principles:

- Explicit dependencies.
- Local understandability.
- Predictable behavior.
- Controlled complexity.
- No hidden global state.

Rust is selected because identity, session, token, and authorization logic must stay precise over time.

## Why Axum Is Fixed

Axum is selected as the HTTP framework because:

- Routing is explicit.
- Request extraction is declarative.
- Error handling can stay predictable.
- Middleware can use the Tower ecosystem.
- The framework does not force a large application structure.

This supports the repository goal of keeping each module understandable in isolation.

## Database Decision

PostgreSQL is selected for the MVP.

Reason:

- Identity data requires consistency.
- External identity bindings require uniqueness constraints.
- Sessions and refresh token records require reliable state transitions.
- Future client applications, scopes, roles, and permissions fit a relational model.

## Password Security Decision

Local username/password support must use password hashing, not reversible encryption.

Argon2id is selected for password hashing.

Password handling rules:

- Never store plaintext passwords.
- Never log passwords.
- Never emit passwords in events.
- Verify the current password before changing a local password.
- Hash the new password with Argon2id before storing it.
- Store algorithm and work factor metadata with the hash.
- Allow future hash parameter upgrades.
- Tune work factors on the actual deployment environment.

## Token Decision

The MVP uses:

- Short-lived JWT access tokens.
- Server-side refresh token records.
- Hashed refresh tokens at rest.
- Refresh token rotation when implemented.

Token issuance must remain separate from session lifecycle.

## Supabase Integration Boundary

Supabase is an external identity provider.

Supabase must not own the platform's internal identity.

Supabase Auth may authenticate users through its own enabled methods, including email/password, magic link, email OTP, phone auth, social login, SSO, OAuth, and OIDC.

This service must treat all Supabase-authenticated users as provider `supabase`.

Supabase-side credential management, including Supabase email/password change and password reset flows, remains Supabase Auth responsibility.

Supabase adapter responsibilities:

- Verify Supabase user or session identity.
- Normalize the Supabase subject into an external identity.
- Return provider metadata allowed by policy.

Supabase adapter must not:

- Replace `internal_user_id`.
- Own platform session lifecycle.
- Issue platform access tokens directly.
- Decide platform authorization.
- Split Supabase upstream login methods into separate MVP providers.

## References Reviewed

- Rust official site: https://www.rust-lang.org/
- Axum docs: https://docs.rs/axum/latest/axum/
- Supabase Auth docs: https://supabase.com/docs/guides/auth
- Supabase password auth docs: https://supabase.com/docs/guides/auth/passwords
- OWASP Password Storage Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
