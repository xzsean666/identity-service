# MVP Plan

## Current Step

Step 4 - MVP implementation has started.

This document defines the fixed MVP boundary for the Identity Platform / IAM Service.

Implementation must stay inside this boundary unless the user explicitly changes the MVP scope.

## MVP Goal

Build the smallest useful identity service that proves the core architecture:

```text
Local Password or Supabase Identity
        -> Provider Adapter
        -> Identity Binding
        -> internal_user_id
        -> Session
        -> Token
```

## Hard Boundary

The MVP includes only:

- Local username/password registration.
- Local username/password login.
- Local username/password change for authenticated users.
- Supabase identity provider integration.
- Centralized provider enable/disable configuration.
- Internal user identity mapping through `internal_user_id`.
- Basic session creation and revocation.
- JWT access token issuance.
- Server-tracked refresh token issuance and exchange.

Anything outside this list is post-MVP unless the user explicitly changes the MVP boundary.

## Included Scope

### Local Username and Password

The MVP must support:

- Registering a user with username and password.
- Logging in with username and password.
- Changing the local password while authenticated.
- Verifying the current password before accepting a new password.
- Storing only Argon2id password hashes.
- Creating or resolving an `internal_user_id`.
- Binding the local credential as provider `local_password`.

The MVP does not include local forgot-password or password-reset-by-email flows.

Those require email delivery infrastructure and are post-MVP.

### Supabase Provider

The MVP must support:

- Verifying a Supabase user or session identity.
- Normalizing Supabase identity into the provider adapter output shape.
- Binding Supabase identity as provider `supabase`.
- Resolving Supabase identity to `internal_user_id`.

MVP product input is a Supabase JWT access token supplied by the client.

The current executable implementation verifies Supabase JWT access tokens through configured JWKS, caches remote JWKS briefly, and refreshes remote JWKS once when a token `kid` is not found in cache. A local JSON fixture payload can be enabled only for development and tests.

The MVP does not implement Supabase callback routes.

Supabase must remain an external provider.

It must not replace the internal user model, platform session model, or platform token model.

Supabase Auth may use its own enabled upstream methods, including email/password, magic link, email OTP, phone auth, social login, SSO, OAuth, and OIDC.

For this service's MVP, all of those Supabase-authenticated identities are still treated as one provider:

- provider name: `supabase`
- provider subject identifier: Supabase user identifier

The MVP must not duplicate Supabase's upstream email, phone, social, OAuth, or OIDC login methods as separate first-party providers.

Supabase-side credential management, including Supabase email/password change and password reset flows, stays inside Supabase Auth.

This service only verifies the resulting Supabase-authenticated user or session.

### Configuration Switches

The MVP must support centralized provider enablement:

```yaml
identity_providers:
  local_password:
    enabled: true
  supabase:
    enabled: true
    auto_provision_enabled: true
  wechat:
    enabled: false
  sms_code:
    enabled: false
  email_code:
    enabled: false
  oauth2:
    enabled: false
  github:
    enabled: false
  google:
    enabled: false
  apple:
    enabled: false
```

Feature toggle rules:

- The MVP uses stable public auth routes.
- Disabled providers must not execute provider verification logic.
- Disabled provider requests must return an explicit `provider_disabled` error.
- Provider availability must be decided from centralized configuration at startup.
- Business logic must not read environment variables directly.

### Session and Token

The MVP must support:

- Creating a session after successful authentication.
- Issuing a short-lived JWT access token.
- Issuing a refresh token.
- Storing only a hash of the refresh token.
- Exchanging a valid refresh token for a new token pair.
- Logging out of the current session.

## Excluded Scope

The MVP must not include:

- WeChat login.
- SMS login.
- Email code login.
- SMS vendor adapters.
- Email vendor adapters.
- Local forgot-password email flow.
- GitHub login.
- Google login.
- Apple Sign In.
- Generic OAuth2 provider login.
- OAuth2 provider mode.
- OIDC provider mode.
- RBAC.
- Organization model.
- Tenant model.
- MFA.
- Passkey.
- SSO.
- Risk engine.
- Audit pipeline beyond minimal local logging.
- Administrative console UI.
- User management admin APIs.
- Client application registry.
- Official SDKs.
- Token introspection endpoint.
- Permission check endpoint.

These are post-MVP modules.

## MVP API Boundary

The MVP API must cover only these product capabilities:

- Register with username/password.
- Login with username/password.
- Change local password while authenticated.
- Login or exchange Supabase identity.
- Refresh token.
- Logout current session.
- Get current user.
- Expose platform public signing keys for backend JWT verification.

Health and readiness endpoints are allowed as operational endpoints, but they are not product scope.

## MVP HTTP API Contract

All routes are stable in the MVP.

Disabled providers return `provider_disabled` instead of disappearing from the router.

| Method | Path | Request | Success |
| --- | --- | --- | --- |
| `POST` | `/v1/auth/register` | `username`, `password` | token pair and current user |
| `POST` | `/v1/auth/login` | `username`, `password` | token pair and current user |
| `POST` | `/v1/auth/password/change` | bearer access token, `current_password`, `new_password` | new token pair |
| `POST` | `/v1/auth/supabase/exchange` | `access_token` | token pair and current user |
| `POST` | `/v1/auth/refresh` | `refresh_token` | new token pair |
| `POST` | `/v1/auth/logout` | bearer access token | logout result |
| `GET` | `/v1/users/me` | bearer access token | current user |
| `GET` | `/.well-known/jwks.json` | none | platform public signing keys |
| `GET` | `/health` | none | process health |
| `GET` | `/ready` | none | dependency readiness |

MVP error codes:

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

General HTTP error responses may also return:

- `not_found`
- `internal_error`

## MVP Data Model

### Internal User

Required fields:

- `internal_user_id` as UUID v4
- account status
- created time
- updated time

### Local Credential

Required fields:

- credential identifier
- `internal_user_id`
- username
- password hash
- password hash algorithm
- password hash parameters
- credential status
- created time
- updated time

### External Identity

Required fields:

- provider name
- provider subject identifier
- linked `internal_user_id`
- provider metadata allowed by policy
- binding status
- created time
- updated time

For local username/password:

- provider name is `local_password`
- provider subject identifier is the local credential identifier.

For Supabase:

- provider name is `supabase`
- provider subject identifier is the Supabase user identifier.

### Session

Required fields:

- session identifier
- `internal_user_id`
- provider name used at login
- device metadata when available
- session status
- issued time
- expiration time
- revoked time

### Refresh Token Record

Required fields:

- refresh token identifier
- session identifier
- token family identifier
- token hash
- issued time
- expiration time
- consumed time
- revoked time
- reuse detection status

Allowed refresh token states:

- `active`
- `consumed`
- `revoked`
- `reused`
- `expired`

Refresh token record ownership:

- Session module owns refresh token records, token family state, rotation, reuse detection, and revocation.
- Token module generates opaque refresh token secrets but does not persist refresh token records.

### MVP Client Context

The MVP does not include a client application registry.

Instead, the MVP uses static client context from centralized configuration.

Required fields:

- client identifier
- trusted origin when needed

The full client application registry is post-MVP.

## MVP Configuration Schema

Minimum Step 4 configuration:

```yaml
http:
  host: "127.0.0.1"
  port: 3000
identity_providers:
  local_password:
    enabled: true
  supabase:
    enabled: true
    auto_provision_enabled: true
    project_url: "https://example.supabase.co"
    issuer: "https://example.supabase.co/auth/v1"
    audience: "authenticated"
client:
  client_id: "identity-service-mvp"
  trusted_origin: "http://localhost:3000"
tokens:
  issuer: "identity-service"
  audience: "platform-api"
  access_token_lifetime_seconds: 900
  key_id: "mvp-local-key"
  private_key_pem_path: "./secrets/jwt_private.pem"
  public_key_pem_path: "./secrets/jwt_public.pem"
sessions:
  refresh_token_lifetime_seconds: 2592000
  session_lifetime_seconds: 2592000
security:
  refresh_token_hmac_secret: "change-me"
```

MVP refresh token hashes use keyed HMAC-SHA256 over high-entropy opaque refresh tokens and constant-time comparison.

MVP backend verification of this platform's own JWTs uses `/.well-known/jwks.json` or a statically distributed public key PEM plus `key_id`.

`tokens.audience` is the only MVP JWT `aud` source. Per-client or per-service audiences require the post-MVP client application registry.

## MVP Security Requirements

The MVP must:

- Hash passwords with Argon2id.
- Use unique salts managed by the password hashing library.
- Never store plaintext passwords.
- Never log passwords or tokens.
- Require current-password verification before changing a local password.
- Hash the new password with Argon2id before storing it.
- Revoke or rotate refresh token state after a successful local password change.
- Use short-lived access tokens.
- Store refresh token state server-side.
- Hash refresh tokens before storage.
- Reject disabled accounts.
- Reject disabled providers.
- Use explicit authentication errors.
- Avoid leaking whether a username exists when doing so would increase account enumeration risk.

The MVP must not:

- Store provider access tokens unless explicitly required and documented.
- Put provider-specific logic inside the core authentication module.
- Let Supabase issue this platform's final access token.
- Add authorization policy logic beyond authenticated-user checks.
- Treat Supabase email, phone, social, OAuth, or OIDC identities as separate MVP providers.

Password change policy:

- Successful local password change revokes all existing refresh token families for the user.
- The current authenticated session receives a new refresh token family.
- Password hash update, old family revocation, and new family creation must happen in one transaction.

Current Step 4 implementation note:

- The executable increment supports an in-memory storage adapter for local development and a PostgreSQL adapter for persisted deployments.
- `IDENTITY_PERSISTENCE_BACKEND=memory` is the default.
- `IDENTITY_PERSISTENCE_BACKEND=postgres` wires the identity, local credential, and session repositories to PostgreSQL.
- `cargo run --bin migrate -- up` applies the MVP PostgreSQL migration.
- `/ready` checks memory readiness for the default backend and PostgreSQL readiness for the `postgres` backend.
- PostgreSQL refresh-token exchange, reuse detection, logout revocation, and refresh-family rotation run inside repository transactions.
- PostgreSQL local password change updates the password hash, revokes old refresh-token state, and creates the new refresh-token family in one transaction.

Refresh token exchange policy:

- Exchange consumes the old refresh token and inserts the new refresh token in one transaction.
- Reuse of a consumed refresh token marks the token family as `reused` and revokes the family.

## MVP Acceptance Criteria

The MVP is complete only when:

- A user can register with username and password.
- A user can log in with username and password.
- An authenticated local-password user can change password after current-password verification.
- A successful local password change updates only the password hash and invalidates old refresh token state according to policy.
- Passwords are hashed with Argon2id.
- A Supabase identity can be verified through the Supabase provider adapter.
- A Supabase user authenticated through Supabase email, phone, social, OAuth, or OIDC methods is still normalized as provider `supabase`.
- A Supabase identity maps to an `internal_user_id`.
- A successful login creates a session.
- A successful login returns a JWT access token and refresh token.
- A refresh token can be exchanged for a new token pair.
- Logout revokes the current session or refresh token family according to policy.
- Disabled providers cannot be used.
- Provider enablement is controlled by centralized configuration.
- Unit tests cover provider normalization, identity binding, password verification, password change, and refresh token behavior.
- E2E tests cover local password, Supabase JWT exchange, refresh rotation/reuse detection, password change, logout, readiness, provider toggles, and optional PostgreSQL HTTP flow.

## Implementation Order

After explicit approval for Step 4:

1. Create Rust project skeleton.
2. Add centralized configuration.
3. Add provider feature toggle model.
4. Add internal user domain model.
5. Add external identity domain model.
6. Add provider adapter contract.
7. Add local password provider.
8. Add password hashing service.
9. Add local password change flow.
10. Add identity binding service.
11. Add session model.
12. Add token service.
13. Add minimal HTTP interface.
14. Add Supabase provider adapter.
15. Add tests for MVP flows.
16. Add PostgreSQL repository implementations.
17. Wire runtime persistence selection.
18. Add opt-in PostgreSQL repository integration test.
19. Add PostgreSQL migration runner.
20. Add backend-aware readiness endpoint.

## Post-MVP Module Roadmap

After the MVP, add modules in this order only when needed:

1. Delivery adapter contract.
2. Email delivery adapter module.
3. SMS delivery adapter module.
4. Email verification code provider.
5. SMS verification code provider.
6. Generic OAuth2 provider login.
7. GitHub provider.
8. Google provider.
9. Apple Sign In provider.
10. WeChat provider.
11. OAuth2/OIDC provider mode.
12. Scope-based authorization.
13. RBAC.
14. Organizations and tenants.
15. MFA and Passkey.
16. Audit logging and risk controls.

SMS and email vendors are delivery adapters, not identity providers.

Changing a delivery vendor must not change authentication, identity binding, session, or token modules.

All post-MVP modules must follow:

- `docs/MODULE_EXPANSION.md`

Backend and gateway integration must follow:

- `docs/INTEGRATION.md`
