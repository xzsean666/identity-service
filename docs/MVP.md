# MVP Plan

## Current Step

Documentation update.

This document defines the first minimum viable product for the Identity Platform / IAM Service.

No implementation code is included in this step.

## MVP Goal

Build the smallest useful identity service that proves the core architecture:

```text
User Credential or Supabase Identity
        -> Provider Adapter
        -> Identity Binding
        -> internal_user_id
        -> Session
        -> Token
```

The MVP must support:

- Local username/password registration.
- Local username/password login.
- Supabase identity provider integration.
- Centralized configuration for enabling and disabling providers.
- Internal user identity mapping.
- Basic session lifecycle.
- Access token and refresh token issuance.

## Explicit MVP Scope

### Included

The MVP includes:

- User registration with username and password.
- User login with username and password.
- Password hashing with Argon2id.
- Internal user creation.
- Local password external identity binding.
- Supabase provider adapter.
- Supabase identity binding to `internal_user_id`.
- Session creation.
- Refresh token record creation.
- Access token issuance.
- Refresh token exchange.
- Logout for current session.
- Basic current-user endpoint.
- Provider enable/disable configuration.

### Excluded

The MVP excludes:

- WeChat login.
- SMS login.
- Email code login.
- GitHub login.
- Google login.
- Apple Sign In.
- Full OAuth2/OIDC provider mode.
- RBAC.
- Organization and tenant model.
- MFA.
- Passkey.
- SSO.
- Risk engine.
- Administrative console UI.

These must be added later as modules.

## MVP Provider Strategy

Every login method is a provider.

MVP providers:

- `local_password`
- `supabase`

Future providers:

- `wechat`
- `sms`
- `email`
- `oauth2`
- `github`
- `google`
- `apple`

Provider modules must share the same adapter contract.

Provider modules must not:

- Create platform sessions directly.
- Issue platform tokens directly.
- Decide authorization.
- Bypass identity binding.

## Feature Toggle Strategy

Providers and optional capabilities must be controlled by centralized configuration.

Conceptual configuration shape:

```yaml
identity_providers:
  local_password:
    enabled: true
  supabase:
    enabled: true
  wechat:
    enabled: false
  sms:
    enabled: false
  email:
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

- Disabled providers must not register public login routes.
- Disabled providers must not execute provider verification logic.
- Disabled provider requests must return an explicit provider-disabled error.
- Provider availability must be decided from centralized configuration at startup.
- Business logic must not read environment variables directly.

## MVP API Capabilities

The exact route names can be chosen during implementation.

Required capability categories:

- Register with username/password.
- Login with username/password.
- Login or exchange Supabase identity.
- Refresh token.
- Logout current session.
- Get current user.
- List current user's linked identities.

Optional in MVP:

- Revoke all sessions for current user.
- Basic health check.
- Basic readiness check.

## MVP Data Model

### Internal User

Required fields:

- `internal_user_id`
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
- provider subject identifier is the stable username identity or local credential identifier

For Supabase:

- provider name is `supabase`
- provider subject identifier is the Supabase user identifier

### Session

Required fields:

- session identifier
- `internal_user_id`
- provider name used at login
- client application identifier when available
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

## MVP Security Requirements

The MVP must:

- Hash passwords with Argon2id.
- Use unique salts managed by the password hashing library.
- Never store plaintext passwords.
- Never log passwords or tokens.
- Use short-lived access tokens.
- Store refresh token state server-side.
- Hash refresh tokens before storage.
- Reject disabled accounts.
- Reject disabled providers.
- Use explicit authentication errors.
- Avoid leaking whether a username exists when doing so would increase account enumeration risk.

The MVP should:

- Add basic rate limiting before production exposure.
- Record basic security events before production exposure.
- Support password hash parameter upgrades later.

## MVP Acceptance Criteria

The MVP is complete when:

- A user can register with username and password.
- A user can log in with username and password.
- Passwords are hashed with Argon2id.
- A Supabase identity can be verified through the Supabase provider adapter.
- A Supabase identity maps to an `internal_user_id`.
- A successful login creates a session.
- A successful login returns an access token and refresh token.
- A refresh token can be exchanged for a new token pair.
- Logout revokes the current session or refresh token family according to policy.
- Disabled providers cannot be used.
- Provider enablement is controlled by centralized configuration.
- Unit tests cover provider normalization, identity binding, password verification, and refresh token behavior.

## Recommended Implementation Order

After explicit approval for Step 4:

1. Create Rust project skeleton.
2. Add centralized configuration.
3. Add feature toggle model.
4. Add internal user domain model.
5. Add external identity domain model.
6. Add provider adapter contract.
7. Add local password provider.
8. Add password hashing service.
9. Add identity binding service.
10. Add session model.
11. Add token service.
12. Add minimal HTTP interface.
13. Add Supabase provider adapter.
14. Add tests for the MVP flows.

## Post-MVP Module Roadmap

After the MVP:

1. Add email verification code provider.
2. Add SMS verification code provider.
3. Add generic OAuth2 provider.
4. Add GitHub provider.
5. Add Google provider.
6. Add Apple Sign In provider.
7. Add WeChat provider.
8. Add OAuth2/OIDC provider mode.
9. Add scope-based authorization.
10. Add RBAC.
11. Add organizations and tenants.
12. Add MFA and Passkey.
13. Add audit logging and risk controls.
