# Backend Integration Guide

## Current Step

Step 4 - MVP implementation has started.

This document defines how other projects, backend services, API gateways, and internal microservices should integrate with the Identity Platform.

This file is the public integration contract for the current MVP implementation boundary.

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

MVP clients must send access tokens as:

```http
Authorization: Bearer <platform_access_token>
```

MVP access token verification must check:

- Signature.
- `kid`.
- Issuer.
- Audience.
- Expiration.
- Subject.
- Issued time.
- JWT identifier presence and UUID format.

MVP platform token key distribution:

- A platform JWKS endpoint is available at `GET /.well-known/jwks.json`.
- Backend services may load public keys from JWKS or receive the platform public key PEM through deployment configuration.
- Backend services must map configured or discovered `kid` values to public keys.
- Unknown `kid` values must be rejected.
- Key rotation in MVP is manual: deploy the new `kid -> public key` mapping to consumers, switch the issuer to the new signing key, then remove the old key after all old access tokens expire.

MVP access token claims must include:

- `iss` as platform issuer.
- `sub` as `internal_user_id`.
- `aud` as configured audience.
- `iat`.
- `exp`.
- `jti`.
- `sid` as session identifier.
- `client_id` from MVP static client context.

MVP audience semantics:

- MVP uses one shared platform API audience from `tokens.audience`.
- All backend services that accept MVP access tokens validate this same audience.
- Per-service audiences are post-MVP and require client/application registry design.

MVP claim contract v1:

| Field | Location | Required | Format |
| --- | --- | --- | --- |
| `alg` | JOSE header | yes | `RS256` |
| `kid` | JOSE header | yes | configured key identifier |
| `iss` | claim | yes | configured issuer string |
| `sub` | claim | yes | UUID-formatted `internal_user_id` |
| `aud` | claim | yes | configured platform API audience |
| `iat` | claim | yes | unix timestamp |
| `exp` | claim | yes | unix timestamp |
| `jti` | claim | yes | UUID |
| `sid` | claim | yes | UUID-formatted session identifier |
| `client_id` | claim | yes | configured MVP client identifier |

`jti` is an access token identifier only. The MVP does not maintain a server-side `jti` denylist or replay-detection store.

## MVP Current User Contract

`GET /v1/users/me` requires `Authorization: Bearer <platform_access_token>` and returns the platform user for the token subject.

Response shape:

```json
{
  "internal_user_id": "uuid",
  "account_status": "Active",
  "created_at": "RFC3339 timestamp",
  "updated_at": "RFC3339 timestamp"
}
```

## Frontend Direct Mode

Browser frontends may call the identity service directly when frontend direct mode is enabled.

Required configuration:

```bash
IDENTITY_FRONTEND_DIRECT_ENABLED=true
IDENTITY_FRONTEND_ALLOWED_ORIGINS=http://localhost:5173,https://app.example.com
```

Frontend direct mode adds CORS for the configured exact origins only.
It allows JSON requests and bearer-token requests using:

```http
Authorization: Bearer <platform_access_token>
Content-Type: application/json
```

The browser receives the same MVP token response as other clients.
Frontend applications should treat refresh tokens as sensitive secrets and avoid storing them in long-lived browser storage.

Other backend services should not depend on frontend direct mode.
They should validate the platform JWT locally and read `sub` as `internal_user_id`.

## MVP Stateless Revocation Limit

External backend services and gateways that verify JWTs locally do not observe session revocation in real time.

MVP guarantees:

- Logout revokes the current platform session and refresh-token state.
- Local password change revokes existing refresh-token families.
- Already issued access tokens may remain accepted by local JWT verification until `exp`.

If a backend requires immediate revocation awareness, add a post-MVP token verification, session check, or introspection endpoint before relying on that behavior.

## MVP Non-Goals for Integration

The MVP does not include:

- Full client application registry.
- OAuth2 introspection endpoint.
- OAuth2/OIDC discovery endpoint.
- Central permission-check API.
- RBAC.
- Tenant-aware authorization.
- Official SDKs.

These are post-MVP integration modules.

## Post-MVP Integration Surface

After MVP acceptance, add integration capabilities in this order when needed:

1. Token verification endpoint for trusted internal services that cannot verify JWT locally.
2. Token introspection endpoint.
3. Permission check endpoint.
4. Client application registry.
5. OAuth2/OIDC discovery metadata.
6. Userinfo endpoint.
7. Service-to-service authentication.
8. Official middleware or SDK packages.

## Gateway Integration

API gateways should:

- Verify platform access token signature.
- Validate issuer and audience.
- Reject expired tokens.
- Drop or overwrite inbound identity headers before forwarding.
- Forward only normalized identity context to upstream services.

Recommended forwarded context:

| Header | Required | Value |
| --- | --- | --- |
| `X-Identity-User-Id` | yes | `internal_user_id` from token `sub` |
| `X-Identity-Session-Id` | yes | session identifier from token `sid` |
| `X-Identity-Client-Id` | yes | client identifier from token `client_id` |
| `X-Request-Id` | recommended | gateway-generated request identifier |

Post-MVP authorization modules may add scope, tenant, organization, or permission headers after their contracts are defined.

Gateway security rules:

- Public ingress must never trust client-supplied `X-Identity-*` headers.
- Gateways must drop or overwrite inbound identity headers before forwarding.
- Upstream services may trust these headers only over authenticated internal paths such as mTLS, private network policy, or signed gateway headers.

Gateways must not forward:

- Raw refresh tokens.
- External provider tokens.
- Provider-specific user IDs unless explicitly required and approved.

## Internal Backend Integration

Internal backends should:

- Accept identity context from a trusted gateway, or verify the platform access token directly.
- Trust forwarded identity headers only over authenticated internal paths such as mTLS, private network policy, or signed gateway headers.
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

Current MVP error response shape:

```json
{
  "error_code": "provider_disabled",
  "message": "provider disabled",
  "retryable": false
}
```

Current fields:

- `error_code`: stable machine-readable code.
- `message`: human-readable message safe for logs.
- `retryable`: boolean.

Correlation identifiers are post-MVP unless a gateway adds `X-Request-Id`.

Common MVP error codes:

- `validation_failed`
- `invalid_credentials`
- `provider_disabled`
- `provider_verification_failed`
- `identity_conflict`
- `unauthorized`
- `token_invalid`
- `refresh_token_reused`
- `account_disabled`
- `dependency_unavailable`
- `not_found`
- `internal_error`

Common status mapping:

| Error code | HTTP status |
| --- | --- |
| `validation_failed` | `400` |
| `invalid_credentials` | `401` |
| `provider_disabled` | `403` |
| `provider_verification_failed` | `401` |
| `identity_conflict` | `409` |
| `unauthorized` | `401` |
| `token_invalid` | `401` |
| `refresh_token_reused` | `401` |
| `account_disabled` | `403` |
| `dependency_unavailable` | `503` |
| `not_found` | `404` |
| `internal_error` | `500` |

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
