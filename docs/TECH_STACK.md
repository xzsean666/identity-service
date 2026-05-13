# Technology Stack Decision

## Current Step

Documentation update.

This document records the recommended technology stack for the Identity Platform / IAM Service.

No implementation code is included in this step.

## Recommendation

Recommended default stack:

- Language: Rust
- HTTP framework: Axum
- Async runtime: Tokio
- Database: PostgreSQL
- Database access: SQLx or an equivalent explicit query layer
- Cache: Redis, optional after the first MVP milestone
- Password hashing: Argon2id
- Token format: JWT access token plus server-tracked refresh token state
- Configuration: centralized typed configuration loaded at startup

Decision status:

- Recommended for implementation.
- Not yet implemented.
- Can be revisited before Step 4 if the user prioritizes delivery speed over long-term service correctness.

## Why Rust Fits This Project

Rust is a strong fit for this project because IAM services are security-sensitive infrastructure.

Important project needs:

- Strong compile-time checks.
- Explicit error handling.
- Predictable runtime behavior.
- Low memory overhead.
- Good concurrency safety.
- Clear module boundaries.
- Fewer hidden side effects.

These match the repository's AI-oriented architecture principles:

- Explicit dependencies.
- Local understandability.
- Predictable behavior.
- Controlled complexity.
- No hidden global state.

Rust also fits the long-term direction of this service:

- Token verification can become high-volume.
- Session and refresh token logic must remain precise.
- Provider adapters need strict input and output contracts.
- Authorization logic benefits from strongly typed decisions.

## Why Axum Fits This Project

Axum is a good HTTP framework choice because:

- Routing is explicit.
- Request extraction is declarative.
- Error handling can stay predictable.
- Middleware can use the Tower ecosystem.
- It does not force a large application framework structure.

This matches the goal of keeping each module understandable in isolation.

## Main Tradeoffs

### Rust Advantages

- Better fit for a security-sensitive core service.
- Strong type system helps protect identity, session, and authorization boundaries.
- Good performance and resource usage for gateway and microservice integration.
- Explicitness helps AI agents reason about code across sessions.

### Rust Costs

- MVP implementation may take longer than TypeScript.
- Some third-party provider SDKs may be less mature than JavaScript/TypeScript SDKs.
- OAuth2/OIDC provider mode may require careful library selection and more contract testing.
- Developer onboarding can be slower if the team is not comfortable with Rust.

## TypeScript / NestJS Alternative

TypeScript with NestJS is the best alternative if the main priority is fastest MVP delivery.

Advantages:

- Faster scaffolding.
- Broad OAuth and provider SDK ecosystem.
- Supabase JavaScript examples and SDK support are first-class.
- NestJS provides a conventional application architecture out of the box.

Costs:

- More runtime behavior can remain implicit.
- Type guarantees are weaker than Rust at service boundaries.
- Dependency and framework abstraction layers can grow quickly.

## Decision Rule

Use Rust when:

- This service is expected to become long-lived identity infrastructure.
- Security, correctness, and predictable behavior are more important than fastest first demo.
- The team is willing to accept slower early implementation for a stronger core.

Use TypeScript/NestJS when:

- The team needs the fastest possible MVP.
- Most contributors are already TypeScript-focused.
- Supabase integration speed matters more than owning the service core cleanly.

Current recommendation:

- Use Rust for this project.
- Keep the MVP narrow to control Rust implementation cost.

## Recommended MVP Stack

For the MVP:

- Rust
- Axum
- PostgreSQL
- SQLx
- Argon2id
- JWT access tokens
- Server-side refresh token records
- Supabase adapter behind the provider adapter contract

Redis should not be mandatory in the MVP.

Reason:

- The first version can use PostgreSQL-backed sessions and refresh token state.
- Redis can be added later for rate limiting, cache, distributed locks, or high-volume token/session workflows.

## Supabase Integration Boundary

Supabase should be treated as an external identity provider, not as the owner of the platform's internal identity.

Supabase adapter responsibilities:

- Verify Supabase user/session identity.
- Normalize the Supabase subject into an external identity.
- Return provider metadata allowed by policy.

Supabase adapter must not:

- Replace `internal_user_id`.
- Own platform session lifecycle.
- Issue platform access tokens directly.
- Decide platform authorization.

## Password Security Decision

Local username/password support must use password hashing, not reversible encryption.

Recommended default:

- Argon2id.

Password handling rules:

- Never store plaintext passwords.
- Never log passwords.
- Never emit passwords in events.
- Store algorithm and work factor metadata with the hash.
- Allow future hash parameter upgrades.
- Tune work factors on the actual deployment environment.

## References Reviewed

- Rust official site: https://www.rust-lang.org/
- Axum docs: https://docs.rs/axum/latest/axum/
- NestJS docs: https://docs.nestjs.com/
- Supabase password auth docs: https://supabase.com/docs/guides/auth/passwords
- OWASP Password Storage Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
