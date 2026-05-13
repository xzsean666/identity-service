# MVP Plan

## Current Step

Documentation hardening.

This document defines the fixed MVP boundary for the Identity Platform / IAM Service.

No implementation code is included in this step.

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

MVP Supabase adapter input is limited to a Supabase access or session token supplied by the client.

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

- Disabled providers must not register public login routes.
- Disabled providers must not execute provider verification logic.
- Disabled provider requests must return an explicit provider-disabled error.
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

Health and readiness endpoints are allowed as operational endpoints, but they are not product scope.

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
- allowed audience
- trusted origin when needed

The full client application registry is post-MVP.

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
