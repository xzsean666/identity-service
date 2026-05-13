# Build and Usage Guide

## Current Step

Step 2 - Documentation.

This repository currently contains architecture and planning documents only.

No implementation code exists yet, so there is no application build command, runtime command, package manager command, migration command, or test command at this stage.

## Repository Layout

Current documentation layout:

```text
identity-service/
  Agent.md
  docs/
    ARCHITECTURE.md
    SPEC.md
    BUILD.md
    TECH_STACK.md
    MVP.md
    nextsession.md
```

Expected future implementation layout is documented in:

- `docs/ARCHITECTURE.md`

## How to Use This Repository Now

Read documents in this order:

1. `Agent.md`
2. `docs/ARCHITECTURE.md`
3. `docs/SPEC.md`
4. `docs/BUILD.md`
5. `docs/TECH_STACK.md`
6. `docs/MVP.md`
7. `docs/nextsession.md`

Purpose of each document:

- `Agent.md` defines how AI agents must work in this repository.
- `docs/ARCHITECTURE.md` defines system architecture and module boundaries.
- `docs/SPEC.md` defines product and system requirements.
- `docs/BUILD.md` defines build and usage guidance.
- `docs/TECH_STACK.md` records the fixed technology stack decision.
- `docs/MVP.md` defines the first minimum viable product.
- `docs/nextsession.md` preserves context for the next AI-assisted session.

## Current Validation Commands

Use these commands to inspect repository state:

```bash
git status --short
find . -maxdepth 3 -type f | sort
```

Use this command to review the current commit history:

```bash
git log --oneline
```

## Git Workflow

After each major step:

```bash
git add .
git commit -m "feat: <describe current step>"
```

Do not push unless explicitly requested.

## Implementation Prerequisites

Before Step 4 implementation begins, the following decisions are already fixed:

- Programming language: Rust
- Web framework: Axum
- Package manager: Cargo
- Database: PostgreSQL
- Cache layer: no Redis in MVP
- Token signing approach: JWT access tokens plus server-tracked refresh tokens
- Test framework: Rust unit and integration tests through Cargo

The following items still need implementation-time detail:

- Local development strategy
- Deployment target
- Migration strategy

These details should be recorded before production code is added.

The fixed stack is documented in:

- `docs/TECH_STACK.md`

## Fixed Technology Decisions

### Language and Framework

- Rust with Axum is selected.
- Other languages and frameworks are not active implementation targets.

### Database

- PostgreSQL is selected.
- A relational database is required because identity, sessions, bindings, clients, and permissions require consistency and constraints.

### Cache

- Redis is excluded from the MVP.
- Add Redis only after the MVP when rate limiting, cache, distributed locks, or high-volume session workflows require it.

### Token Strategy

- Keep access tokens short-lived.
- Store refresh token state server-side.
- Separate token issuance from session lifecycle.
- Hash refresh tokens before storing them.

Implementation-time details still needed:

- JWT signing algorithm.
- Key storage.
- Key rotation plan.
- Access token lifetime.
- Refresh token rotation policy.
- Token revocation behavior.

### Identity Implementation Order

Fixed MVP identity implementation order:

1. Local username/password
2. Local password change
3. Supabase

Post-MVP provider order:

1. Email verification code
2. SMS verification code
3. OAuth2 generic provider
4. GitHub
5. Google
6. Apple Sign In
7. WeChat

Reason:

- Local username/password validates the internal identity, credential, session, and token foundation.
- Local password change validates credential update and refresh token invalidation behavior.
- Supabase validates the provider adapter and external identity binding model.
- Supabase upstream email, phone, social, OAuth, and OIDC methods remain inside the single `supabase` provider boundary for the MVP.
- Email and SMS should be added after the MVP because they require delivery infrastructure and abuse controls.
- Generic OAuth2 creates a reusable base for GitHub, Google, and other providers.
- WeChat and Apple have platform-specific edge cases and should be implemented after core provider contracts are stable.

## Future Local Development Guide

When implementation starts, this document should be updated with:

- Dependency installation command
- Environment variable setup
- Database migration command
- Local server command
- Test command
- Lint command
- Formatting command
- API documentation command

Example sections to add later:

```text
Install dependencies
Configure environment
Start database
Run migrations
Start service
Run tests
Run lint
Build production artifact
```

## Environment Configuration Guidance

Future configuration should be centralized.

Expected configuration categories:

- Service name
- Runtime environment
- HTTP host and port
- Database connection
- Cache connection
- Token issuer
- Token audience
- Signing key or key provider
- Refresh token policy
- Session policy
- Provider credentials
- OAuth2 client registry settings
- Observability settings

Configuration rules:

- Do not scatter provider secrets across modules.
- Do not read environment variables directly from business logic.
- Validate required settings at startup.
- Keep sensitive values out of logs.

## Testing Strategy

Future tests should be organized by risk and module boundary.

### Unit Tests

Target:

- Provider adapter normalization
- Identity binding decisions
- Session lifecycle policy
- Token claim creation
- Authorization decisions

### Integration Tests

Target:

- Database repositories
- Session persistence
- Refresh token rotation
- Provider callback handling with mocked providers
- Token verification with real signing keys

### Contract Tests

Target:

- Provider adapter interface behavior
- OAuth2/OIDC endpoint compatibility
- Gateway token verification expectations

### End-to-End Tests

Target:

- Login to token issuance
- Refresh token rotation
- Logout and token invalidation
- Account linking
- Authorization check

## Build Completion Criteria

This document is complete for the current step when:

- It states that no implementation build exists yet.
- It explains how to inspect and use the documentation.
- It records future implementation prerequisites.
- It defines what must be added once implementation begins.
