# Next Session Handoff

## Current Step

Step 3 - Context Handoff.

This document preserves project state for future AI-assisted sessions.

No implementation code has been written.

## Current Progress

Date:

- 2026-05-13

Repository state:

- Documentation-only repository.
- Initial Git history has been created.
- Architecture, specification, build guidance, and AI agent workflow are documented.
- Fixed technology stack decision has been documented.
- MVP scope has been documented.
- Module expansion rules have been documented.
- Architecture audit findings have been incorporated into module ownership rules.
- Implementation has not started.

Current completed workflow steps:

- Step 1 - Architecture Design: completed.
- Step 2 - Documentation: completed.
- Step 3 - Context Handoff: completed by this document.

Current blocked workflow step:

- Step 4 - Implementation: pending explicit user approval.

## Architecture Summary

The project is an Identity Platform / IAM Service for unified authentication and authorization across web, mobile, OpenAPI consumers, third-party applications, API gateways, and internal microservices.

The system maps all external identities into a stable platform identity:

```text
External Identity Provider Identity
        -> Provider Adapter
        -> Identity Binding
        -> internal_user_id
        -> Session / Token / Authorization
```

Core architecture decisions:

- Use Provider Adapter pattern for all identity sources.
- Use `internal_user_id` as the stable subject identifier.
- Keep authentication, session, token, and authorization modules separate.
- Keep provider-specific logic out of core identity modules.
- Centralize security policy and configuration.
- Use centralized feature toggles to enable or disable provider modules.
- Start the MVP with local username/password registration, login, password change, and Supabase provider support.
- Treat Supabase upstream email, phone, social, OAuth, and OIDC methods as one provider named `supabase`.
- Treat post-MVP SMS and email vendors as delivery adapters, not identity providers.
- Use a startup composition root to build provider and delivery adapter registries.
- Keep concrete providers out of authentication, session, token, and identity binding modules.
- Let session own refresh token records, families, rotation, reuse detection, and revocation.
- Let token own JWT signing/verification and opaque refresh token secret generation only.
- Support JWT in the MVP.
- Keep OAuth2 and OIDC provider mode post-MVP.
- Add enterprise features incrementally instead of mixing them into the first version.

## Completed Parts

### Root Agent Guide

File:

- `Agent.md`

Completed content:

- AI execution workflow.
- Required documentation layout.
- Architecture-first rule.
- Documentation-first rule.
- Implementation approval boundary.
- Engineering principles for AI-readable code.
- Git workflow requirement.
- Self-correction rule.

### Architecture Document

File:

- `docs/ARCHITECTURE.md`

Completed content:

- Overall system architecture.
- Proposed directory structure.
- Module definitions.
- Module inputs, outputs, and dependencies.
- Core data model.
- Login, refresh token, authorization, and account linking flows.
- Key design decisions.
- Module boundaries.
- Risks and unknowns.

### System Specification

File:

- `docs/SPEC.md`

Completed content:

- Product positioning.
- Goals and non-goals.
- System actors.
- Core concepts.
- Functional requirements.
- Provider adapter specification.
- API surface categories.
- Data requirements.
- Security requirements.
- Non-functional requirements.
- Development phases.

### Build and Usage Guide

File:

- `docs/BUILD.md`

Completed content:

- Current documentation-only state.
- Repository usage order.
- Current validation commands.
- Git workflow.
- Implementation prerequisites.
- Fixed technology decisions.
- Future local development guide.
- Environment configuration guidance.
- Future testing strategy.

### Technology Stack Decision

File:

- `docs/TECH_STACK.md`

Completed content:

- Rust fixed as implementation language.
- Axum fixed as HTTP framework.
- PostgreSQL fixed as MVP database.
- Redis excluded from MVP.
- Supabase integration boundary.
- Password hashing decision.

### MVP Plan

File:

- `docs/MVP.md`

Completed content:

- MVP goal.
- Included and excluded scope.
- Local username/password provider plan.
- Authenticated local password change plan.
- Supabase provider plan.
- Supabase upstream method boundary.
- Provider feature toggle strategy.
- MVP API capabilities.
- MVP data model.
- MVP security requirements.
- MVP acceptance criteria.
- Post-MVP provider roadmap.

### Module Expansion Rules

File:

- `docs/MODULE_EXPANSION.md`

Completed content:

- Locked MVP core.
- Identity provider extension point.
- Delivery adapter extension point.
- Verification code extension point.
- Authorization extension point.
- Protocol surface extension point.
- Module addition checklist.
- Configuration rules.
- Dependency direction rules.
- Composition root and registry rules.
- Security support interface rules.

### Architecture Audit Updates

Completed content:

- Provider registry and descriptor rules.
- Feature toggle gate path.
- Refresh token ownership split between session and token modules.
- MVP Supabase input narrowed to Supabase access or session token.
- MVP static client context.
- Local credential operation boundary for password change.
- Registration and binding policy.
- Token policy with RS256 access tokens.

## Pending Tasks

### Step 4 - Implementation Preparation

Pending because implementation requires explicit user approval.

Before writing code, decide:

1. Migration tool.
2. Key storage strategy.
3. Access token lifetime.
4. Local development strategy.
5. Deployment target.

Fixed decisions:

- Language: Rust.
- HTTP framework: Axum.
- Package manager: Cargo.
- Database: PostgreSQL.
- JWT signing algorithm: RS256 for MVP access tokens.
- MVP providers: local username/password and Supabase only.
- MVP local password flow includes authenticated password change.
- Redis: excluded from MVP.

### Step 4 - First MVP Implementation Increment

First implementation increment after approval:

1. Create project skeleton.
2. Add centralized configuration module.
3. Add feature toggle model.
4. Add internal user domain model.
5. Add external identity domain model.
6. Add provider adapter contract.
7. Add local username/password provider.
8. Add password hashing service.
9. Add authenticated local password change flow.
10. Add session model.
11. Add token issuance interface.
12. Add minimal authentication flow.
13. Add Supabase provider adapter.
14. Add unit tests for identity binding, password verification, password change, refresh token behavior, and provider normalization.

### Step 4 - MVP Hardening Increment

Second implementation increment:

1. Add persistence layer.
2. Add refresh token rotation.
3. Add current-session revocation.
4. Add token verification.
5. Add integration tests.

### Post-MVP Provider Increment

Only after MVP acceptance:

1. Add delivery adapter contract.
2. Add email delivery adapter module.
3. Add SMS delivery adapter module.
4. Add email verification code provider.
5. Add SMS verification code provider.
6. Add OAuth2 generic provider adapter.
7. Add GitHub provider.
8. Add Google provider.
9. Add account linking.
10. Add provider contract tests.

### Post-MVP OAuth2/OIDC Increment

Only after MVP acceptance:

1. Add client application registry.
2. Add OAuth2 authorization endpoint.
3. Add OAuth2 token endpoint.
4. Add JWKS endpoint.
5. Add OIDC discovery metadata.
6. Add userinfo endpoint.

## Next Actions

For the next AI session:

1. Read `Agent.md`.
2. Read `docs/ARCHITECTURE.md`.
3. Read `docs/SPEC.md`.
4. Read `docs/BUILD.md`.
5. Read `docs/MODULE_EXPANSION.md`.
6. Confirm whether the user has approved Step 4 implementation.
7. If implementation is approved, use Rust, Axum, Cargo, PostgreSQL, Argon2id, JWT access tokens, and server-tracked refresh tokens.
8. Update `docs/BUILD.md` with stack-specific commands before or during implementation.
9. Create the implementation skeleton in small increments.
10. Commit each major step.

## Risks and Unknowns

### Implementation Details

The stack is fixed, but some implementation details remain open.

Impact:

- Exact build, test, and runtime commands will be finalized when the Rust project skeleton is created.
- Migration tooling still needs to be selected.

### Supabase Boundary

Supabase is an external identity provider for this service.

Impact:

- The adapter must verify Supabase identity and map it to `internal_user_id`.
- Supabase must not replace platform sessions or platform tokens.
- Supabase upstream email, phone, social, OAuth, and OIDC methods must remain inside provider `supabase`.

### WeChat Login Modes

WeChat login differs across web, mobile, and mini-program environments.

Impact:

- Provider adapter may need separate sub-adapters or explicit flow types.

### OAuth2/OIDC Scope

OAuth2/OIDC provider mode can grow large.

Impact:

- Must be phased after core user, provider, session, and token modules are stable.

### Multi-Tenancy

Organization and tenant support can affect user, authorization, token, and client application models.

Impact:

- Do not add tenant assumptions implicitly before the model is explicitly designed.

### Refresh Token Security

Refresh token rotation, reuse detection, and session revocation require precise state transitions.

Impact:

- Session owns refresh token state.
- Implementation must test active, consumed, revoked, reused, and expired states.

### SMS and Email Delivery Vendors

SMS and email providers differ by vendor.

Impact:

- Add delivery adapters after the MVP.
- Keep vendor-specific APIs, credentials, templates, and errors outside authentication provider modules.
- Do not make SMS or email vendors identity providers.

## Git Commits Created

Current known commits:

- `feat: add architecture design docs`
- `feat: add system documentation`
- `feat: add context handoff documentation`
- `feat: document technology stack and mvp scope`
- `feat: lock rust stack and mvp boundary`
- `feat: add password change and supabase auth boundary`
- `feat: document verification delivery adapters`
- `feat: document module expansion rules`

This documentation hardening should be committed with:

```bash
git add .
git commit -m "feat: refine modular architecture audit findings"
```

## Handoff Completion Criteria

Step 3 is complete when:

- Current progress is recorded.
- Architecture summary is recorded.
- Completed parts are recorded.
- Pending tasks are listed step by step.
- Next actions are clear.
- Risks and unknowns are documented.
- No implementation code has been written.
