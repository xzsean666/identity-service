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
- Support JWT, OAuth2, and OIDC standards.
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
- Technology decision checklist.
- Future local development guide.
- Environment configuration guidance.
- Future testing strategy.

## Pending Tasks

### Step 4 - Implementation Preparation

Pending because implementation requires explicit user approval.

Before writing code, decide:

1. Programming language.
2. Web framework.
3. Package manager.
4. Database.
5. Cache strategy.
6. Token signing algorithm and key management approach.
7. Migration tool.
8. Test framework.
9. First provider implementation order.
10. Deployment target.

### Step 4 - Suggested First Implementation Increment

Recommended first implementation increment after approval:

1. Create project skeleton.
2. Add centralized configuration module.
3. Add internal user domain model.
4. Add external identity domain model.
5. Add provider adapter contract.
6. Add email verification code provider stub or mock provider.
7. Add session model.
8. Add token issuance interface.
9. Add minimal authentication flow.
10. Add unit tests for identity binding and provider normalization.

### Step 4 - Suggested Second Implementation Increment

Recommended second implementation increment:

1. Add persistence layer.
2. Add refresh token rotation.
3. Add session listing and revocation.
4. Add token verification.
5. Add gateway-compatible verification endpoint.
6. Add integration tests.

### Step 4 - Suggested Third Implementation Increment

Recommended third implementation increment:

1. Add OAuth2 generic provider adapter.
2. Add GitHub provider.
3. Add Google provider.
4. Add Supabase provider.
5. Add account linking.
6. Add provider contract tests.

### Step 4 - Suggested Fourth Implementation Increment

Recommended fourth implementation increment:

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
5. Confirm whether the user has approved Step 4 implementation.
6. If implementation is approved, ask for or choose the technology stack based on project constraints.
7. Update `docs/BUILD.md` with stack-specific commands before or during implementation.
8. Create the implementation skeleton in small increments.
9. Commit each major step.

## Risks and Unknowns

### Technology Stack

The programming language and framework are not selected.

Impact:

- Cannot define exact build, test, migration, or runtime commands yet.

### Database

The database is not selected.

Impact:

- Persistence model and migration strategy cannot be finalized.

### Supabase Boundary

Supabase may be used as an external identity provider, backend platform, or both.

Impact:

- Adapter responsibility must be clarified before implementation.

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

- Must be specified and tested carefully before production use.

## Git Commits Created

Current known commits:

- `feat: add architecture design docs`
- `feat: add system documentation`

This handoff document should be committed with:

```bash
git add .
git commit -m "feat: add context handoff documentation"
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
