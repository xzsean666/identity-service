# Backend Integration Guide

## Current Step

Documentation hardening.

This document defines how other projects, backend services, API gateways, and internal microservices should integrate with the Identity Platform.

No implementation code is included in this step.

## Purpose

This service is a shared identity and authorization platform.

Other backend projects must be able to integrate without knowing provider-specific login details such as local password, Supabase, WeChat, SMS, email, GitHub, Google, or Apple.

Downstream services should depend on platform identity only:

- `internal_user_id`
- access token claims
- session identifier when needed
- scopes or permissions when available
- tenant or organization context when introduced

## Integration Principles

- Other services must not verify external identity provider tokens directly.
- Other services must not depend on Supabase user IDs, WeChat OpenIDs, GitHub IDs, phone numbers, or email addresses as stable subjects.
- Other services must treat `internal_user_id` as the stable subject.
- Other services should verify platform access tokens through documented token verification rules.
- Provider-specific behavior remains inside this identity service.
- Authorization checks must use stable platform contracts, not provider-specific assumptions.

## MVP Integration Contract

The MVP supports JWT access tokens issued by this service.

Other backend services may integrate by:

1. Receiving a bearer access token from a client.
2. Verifying the token signature and standard claims.
3. Reading `sub` as `internal_user_id`.
4. Reading `sid` as platform session identifier when present.
5. Reading `aud` as allowed service or API audience.
6. Rejecting expired or invalid tokens.

MVP access token verification must check:

- Signature.
- `kid`.
- Issuer.
- Audience.
- Expiration.
- Subject.
- Issued time.
- JWT identifier.

MVP access token claims must include:

- `iss` as platform issuer.
- `sub` as `internal_user_id`.
- `aud` as configured audience.
- `iat`.
- `exp`.
- `jti`.
- `sid` as session identifier.
- `client_id` from MVP static client context.

## MVP Non-Goals for Integration

The MVP does not include:

- Full client application registry.
- OAuth2 introspection endpoint.
- OAuth2/OIDC discovery endpoint.
- JWKS endpoint.
- Central permission-check API.
- RBAC.
- Tenant-aware authorization.
- Official SDKs.

These are post-MVP integration modules.

## Post-MVP Integration Surface

After MVP acceptance, add integration capabilities in this order when needed:

1. JWKS endpoint for public key discovery.
2. Token verification endpoint for trusted internal services that cannot verify JWT locally.
3. Token introspection endpoint.
4. Permission check endpoint.
5. Client application registry.
6. OAuth2/OIDC discovery metadata.
7. Userinfo endpoint.
8. Service-to-service authentication.
9. Official middleware or SDK packages.

## Gateway Integration

API gateways should:

- Verify platform access token signature.
- Validate issuer and audience.
- Reject expired tokens.
- Forward only normalized identity context to upstream services.

Recommended forwarded context:

- `internal_user_id`.
- session identifier.
- client identifier.
- scopes when available.
- request correlation identifier.

Gateways must not forward:

- Raw refresh tokens.
- External provider tokens.
- Provider-specific user IDs unless explicitly required and approved.

## Internal Backend Integration

Internal backends should:

- Accept identity context from a trusted gateway, or verify the platform access token directly.
- Treat `internal_user_id` as the user key.
- Use platform permission checks when post-MVP authorization APIs are introduced.
- Avoid storing provider-specific identifiers as primary foreign keys.

Internal backends may store:

- `internal_user_id`.
- tenant identifier when introduced.
- organization identifier when introduced.
- permission or role snapshots only when explicitly designed.

## SDK and Middleware Boundary

SDKs and middleware are post-MVP.

When added, they must stay thin.

SDKs may:

- Verify JWTs.
- Load JWKS.
- Parse platform claims.
- Call introspection.
- Call permission check APIs.
- Provide framework middleware for common backends.

SDKs must not:

- Implement login provider logic.
- Store refresh tokens for backend services.
- Decide authorization rules locally beyond documented claim checks.
- Depend on Supabase, WeChat, GitHub, Google, Apple, SMS, or email provider details.

## Versioning and Compatibility

Public integration contracts must be versioned.

Versioned surfaces include:

- Token claims.
- HTTP APIs.
- Error response shape.
- JWKS format.
- Introspection response.
- Permission check request and response.
- SDK major versions.

Compatibility rules:

- Do not remove token claims without a major contract version change.
- Do not change claim meaning without a major contract version change.
- Additive claims are allowed when downstream services can ignore unknown claims.
- Error responses must remain machine-readable.
- Deprecated integration contracts must have a migration path.

## Error Response Contract

Public integration APIs should return structured errors.

Required fields:

- error code.
- human-readable message safe for logs.
- correlation identifier.
- retryable flag when applicable.

Errors must not expose:

- Secrets.
- Raw tokens.
- Passwords.
- Verification codes.
- Provider access tokens.

## Documentation Requirement

Every public integration surface must update:

- `docs/INTEGRATION.md`.
- `docs/SPEC.md`.
- `docs/BUILD.md`.
- `docs/nextsession.md`.

If the change adds a new module, it must also follow:

- `docs/MODULE_EXPANSION.md`.
