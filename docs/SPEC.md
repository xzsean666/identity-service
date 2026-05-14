# System Specification

## Current Step

Step 4 - MVP implementation has started.

This document specifies the Identity Platform / IAM Service behavior, scope, interfaces, data model, and development boundaries.

Implementation must follow `docs/MVP.md`. Post-MVP sections are design notes and must not be implemented until explicitly approved.

## Product Positioning

The Identity Platform is a lightweight, extensible identity authentication and authorization platform for modern full-platform applications and microservice systems.

It is similar in role to Keycloak, Auth0, Okta, or Clerk, but the intended direction is:

- Lighter operational footprint
- Clearer modular architecture
- Strong AI-assisted maintainability
- Provider adapter based extension model
- Practical support for web, mobile, OpenAPI, third-party applications, and internal services
- Stable integration contract for other backend projects and API gateways

## Primary Goals

- Provide unified user identity management.
- Support multiple login providers through a consistent provider adapter model.
- Support local username/password registration and login as the first MVP provider.
- Map all external identities to one stable `internal_user_id`.
- Issue and verify platform tokens.
- Manage refresh tokens and session lifecycle.
- Track device login state.
- Provide a stable backend integration contract based on platform tokens and `internal_user_id`.
- Support standard JWT, OAuth2, and OIDC integration patterns.
- Provide a foundation for RBAC, tenants, MFA, SSO, Passkey, risk control, and audit logging.

## MVP Scope

The MVP is defined in:

- `docs/MVP.md`

The MVP includes:

- Local username/password registration.
- Local username/password login.
- Local username/password change for authenticated users.
- Supabase provider adapter.
- Provider enable/disable configuration.
- Internal user identity mapping.
- Session creation.
- Access token and refresh token issuance.
- Refresh token exchange.
- Current session logout.

The MVP excludes all other providers and enterprise features.

They are post-MVP modules only.

Supabase Auth may provide email/password, magic link, email OTP, phone auth, social login, SSO, OAuth, or OIDC upstream.

Within this service's MVP, all Supabase-authenticated users are normalized as provider `supabase`; they are not separate first-party providers.

Supabase-side credential management, including Supabase email/password change and password reset flows, remains in Supabase Auth.

## Technology Stack Direction

The fixed stack is documented in:

- `docs/TECH_STACK.md`

Selected stack:

- Rust.
- Axum.
- PostgreSQL.
- Argon2id for password hashing.
- JWT access tokens with server-tracked refresh token state.

This stack is no longer open for normal implementation discussion.

## Non-Goals for Initial Version

The initial version should not attempt to fully implement every enterprise feature.

Deferred capabilities:

- Full RBAC policy engine
- Full organization and tenant hierarchy
- SAML SSO
- Passkey
- Advanced risk engine
- Full SIEM-grade audit pipeline
- User behavior analytics
- Administrative console UI

These features must be added incrementally after the core identity foundation is stable.

## System Actors

### End User

A person who registers, logs in, refreshes sessions, links accounts, and accesses applications.

### Client Application

A web app, mobile app, third-party app, OpenAPI consumer, or internal product that depends on the identity service.

### API Gateway

A gateway that verifies tokens and enforces coarse-grained access rules before forwarding requests.

### Internal Microservice

A backend service that needs to verify user identity, service identity, or authorization decisions.

### External Identity Provider

A third-party or platform identity source such as Supabase, WeChat, GitHub, Google, Apple, SMS, email, or an OAuth2 provider.

### Administrator

A privileged operator who manages clients, users, sessions, providers, policies, and future tenant settings.

## Core Concepts

### Internal User

The platform-owned user identity.

All authentication providers eventually resolve to an `internal_user_id`.

MVP `internal_user_id` values are UUID v4 identifiers.

### External Identity

An identity record from an external provider.

Examples:

- WeChat OpenID or UnionID
- GitHub user ID
- Google subject identifier
- Apple subject identifier
- Supabase user ID
- Verified phone number
- Verified email address

### Identity Binding

The relationship between an external identity and an internal user.

One internal user can have multiple external identities.

### Session

A server-side record of a login state, usually tied to a user, client application, device, and refresh token family.

### Token

A credential issued by this service.

Initial token types:

- Access token
- Refresh token

Future token types:

- ID token
- Authorization code
- Device code

### Client Application

A registered application that can request authentication or authorization from the identity service.

### Authorization Decision

The result of checking whether a subject can perform an action on a resource.

## Functional Requirements

### User Identity

The system must:

- Create internal users.
- Retrieve users by `internal_user_id`.
- Track account status.
- Support account disabling.
- Support future user profile extension.
- Keep internal user identity independent from external providers.

### Provider Login

The system must:

- Support provider adapter based login.
- Normalize provider identities into a shared external identity shape.
- Resolve an external identity to an `internal_user_id`.
- Create an internal user when allowed by registration policy.
- Reject login when account status or provider status is invalid.
- Support local password change for authenticated users with current-password verification.

Provider categories:

- MVP: Local username/password.
- MVP: Supabase.
- Post-MVP: WeChat.
- Post-MVP: SMS verification code.
- Post-MVP: Email verification code.
- Post-MVP: OAuth2 provider.
- Post-MVP: GitHub.
- Post-MVP: Google.
- Post-MVP: Apple Sign In.

### Supabase Provider

The Supabase provider adapter must:

- Verify Supabase-authenticated user or session identity.
- Normalize Supabase identity into provider `supabase`.
- Use the Supabase user identifier as the provider subject identifier.
- Ignore which upstream Supabase method was used for this service's provider selection.
- Accept a Supabase JWT access token in the MVP product contract.
- Current Step 4 code verifies Supabase JWT access tokens through configured JWKS.
- Local fixture tokens require explicit development/test configuration.
- Store no Supabase provider token.
- Return only allowlisted Supabase metadata.

Supabase upstream methods may include:

- Email/password.
- Magic link.
- Email OTP.
- Phone auth.
- Social login.
- SSO.
- OAuth.
- OIDC.

This service must not split those upstream methods into separate MVP providers.

### Registration and Binding Policy

MVP registration behavior:

- Local registration creates a new internal user and local credential.
- Local login never auto-creates a user.
- Supabase exchange uses `RegisterOrLogin` only when `identity_providers.supabase.auto_provision_enabled` is enabled.
- Supabase exchange uses `LoginOnly` when auto-provisioning is disabled.

Binding behavior:

- `LoginOnly` resolves only existing identity bindings.
- `RegisterOrLogin` resolves an existing binding or creates a new internal user and binding atomically.
- `LinkToExisting` binds a verified external identity to an already authenticated `internal_user_id`.
- Provider subject uniqueness must be enforced by provider name and provider subject identifier.

### Post-MVP SMS and Email Verification Providers

SMS verification code login and email verification code login are post-MVP providers.

They must be implemented as provider modules that depend on delivery adapters.

The provider module owns:

- Verification code creation.
- Verification code validation.
- Verification code expiration.
- Verification code retry limits.
- Verification code consumption.
- Normalization into a verified phone or email identity.

The provider module must not own:

- Vendor-specific SMS APIs.
- Vendor-specific email APIs.
- Vendor credentials.
- Vendor template formats.

### Post-MVP Delivery Adapters

Different SMS and email vendors must be implemented as delivery adapters.

Examples of SMS delivery adapters:

- Aliyun SMS.
- Tencent Cloud SMS.
- Twilio.
- AWS SNS.

Examples of email delivery adapters:

- Resend.
- SendGrid.
- AWS SES.
- SMTP.

Delivery adapters must:

- Send a message using one vendor.
- Map vendor errors into internal delivery errors.
- Keep vendor-specific templates and request formats isolated.
- Read credentials only from centralized configuration or secret management.

Delivery adapters must not:

- Create internal users.
- Validate verification codes.
- Issue tokens.
- Create sessions.
- Decide identity binding.

Configuration must allow selecting enabled SMS and email delivery adapters after the MVP.

### Account Binding

The system must:

- Bind a verified external identity to an internal user.
- Prevent binding the same external identity to multiple users.
- Support unbinding when policy allows.
- Record binding and unbinding events for audit when audit is introduced.

### Authentication

The system must:

- Accept login requests from supported clients.
- Validate credentials through provider adapters.
- Create sessions after successful authentication.
- Issue tokens after session creation.
- Return explicit failure reasons.

### Session Management

MVP session behavior:

- Create sessions.
- Track device metadata.
- Track static MVP client context from configuration.
- Support refresh token rotation.
- Reject revoked, expired, or reused refresh tokens.

Post-MVP session behavior:

- List active sessions for a user.
- Revoke one selected session.
- Revoke all sessions for a user.
- Track full client application registry context.

### Token Management

MVP token behavior:

- Issue access tokens.
- Issue refresh tokens.
- Verify access token signature and claims.
- Rotate refresh tokens.
- Support token revocation through session state.
- Keep refresh token persistence, family state, reuse detection, and revocation in the session module.

Post-MVP token behavior:

- Support signing key rotation.
- Support OIDC ID tokens.
- Support OAuth2 protocol tokens.

Access token claims should include:

- issuer
- subject as `internal_user_id`
- audience
- expiration
- issued time
- session identifier
- client identifier
- scopes or permissions when available

### Authorization

MVP authorization behavior:

- Verify authenticated subject context for current-user access.

Post-MVP authorization behavior:

- Evaluate requested resource and action.
- Return explicit allow or deny decisions.
- Prepare for RBAC and tenant-aware authorization.

Initial authorization can be scope-based.

RBAC and organization-aware rules should be introduced after the authentication foundation is stable.

### OAuth2 and OIDC Provider Mode

The system should support becoming an OAuth2/OIDC provider.

Planned endpoints:

- Authorization endpoint
- Token endpoint
- Userinfo endpoint
- JWKS endpoint
- Discovery metadata endpoint
- Revocation endpoint
- Introspection endpoint

This capability must depend on the core session, token, and client application modules.

### Client Application Registry

The system must eventually manage:

- Client identifier
- Client secret metadata when applicable
- Client type
- Redirect URIs
- Allowed grant types
- Allowed scopes
- Trusted origins
- Status

### Audit Logging

The system should record:

- Login success
- Login failure
- Logout
- Refresh token use
- Refresh token reuse detection
- Account binding
- Account unbinding
- Session revocation
- Authorization denial
- Administrative changes

Audit events must be append-only when implemented.

## Provider Adapter Specification

Each provider adapter must expose a small, explicit behavior surface.

### Provider Adapter Purpose

- Verify provider-specific credentials.
- Retrieve or validate provider identity.
- Normalize provider result.
- Return structured provider errors.

### Provider Adapter Input

MVP provider inputs:

- Local username and password.
- Supabase JWT access token.

Current Step 4 implementation note:

- The Supabase adapter verifies JWT access tokens using configured JWKS.
- A local JSON fixture payload can be enabled only for tests and development.

Post-MVP provider input examples:

- OAuth2 authorization code.
- Provider access token.
- SMS code.
- Email verification code.
- WeChat login code.
- Apple identity token.

### Provider Adapter Output

Every successful provider adapter result must normalize into:

- provider name
- provider subject identifier
- verified email when available
- verified phone number when available
- provider display name when available
- provider avatar URL when available
- provider raw metadata when safe to store

### Provider Adapter Restrictions

Provider adapters must not:

- Create internal users.
- Create sessions.
- Issue platform tokens.
- Write authorization rules.
- Know about RBAC or tenant policy.

## Feature Toggle Requirements

The system must support centralized feature toggles for optional providers and capabilities.

Required behavior:

- Provider enablement must be controlled from centralized configuration.
- MVP public auth routes remain registered.
- Disabled providers must not execute provider-specific verification logic.
- Disabled provider usage must return an explicit `provider_disabled` error.
- Business logic must not read environment variables directly to decide feature availability.
- Configuration loads toggles.
- Startup builds the provider registry with configured availability.
- MVP public auth routes remain registered and disabled provider handlers return `provider_disabled`.
- Post-MVP module-specific routes may be omitted when a module is disabled if the module contract documents that behavior.
- Authentication uses the provider registry and rejects disabled provider usage.
- Provider adapters expose descriptors but do not read environment variables or feature toggles directly.

MVP provider toggles:

- `local_password`
- `supabase`

Future provider toggles:

- `wechat`
- `sms_code`
- `email_code`
- `oauth2`
- `github`
- `google`
- `apple`

## API Surface

The HTTP framework is Axum.

The MVP API surface must stay inside the boundaries defined in `docs/MVP.md`.

Backend and gateway integration contracts are defined in:

- `docs/INTEGRATION.md`

### MVP Authentication APIs

MVP capabilities:

- Register with username/password.
- Login with username/password.
- Change local password while authenticated.
- Login or exchange Supabase identity.
- Logout current session.

### MVP Session APIs

MVP capabilities:

- Refresh token.
- Revoke current session through logout.

### MVP User APIs

MVP capabilities:

- Get current user.

### MVP Operational APIs

MVP capabilities:

- Process health check.
- Runtime readiness check.
- PostgreSQL dependency readiness when PostgreSQL persistence is enabled.

### Post-MVP APIs

Post-MVP capabilities:

- SMS login.
- Email code login.
- Local forgot-password email flow.
- GitHub login.
- Google login.
- Apple Sign In.
- WeChat login.
- Generic OAuth2 provider login.
- OAuth2/OIDC provider mode.
- RBAC permission checks.
- Administration APIs.
- JWKS endpoint.
- Token introspection endpoint.
- Permission check endpoint.
- Service-to-service authentication.

## Data Requirements

### Internal Users

Required behavior:

- Stable `internal_user_id`
- Account status tracking
- Creation time
- Update time

### External Identities

Required behavior:

- Unique provider and provider subject pair
- Link to one `internal_user_id`
- Provider-specific metadata storage with strict boundaries
- Binding status tracking

### Sessions

Required behavior:

- Session status
- Refresh token family tracking
- Device metadata
- Client application context
- Revocation tracking

### Client Applications

Required behavior:

- MVP: static client identifier
- MVP: JWT audience from `tokens.audience`
- MVP: trusted origin when needed
- Post-MVP: redirect URIs
- Post-MVP: allowed scopes
- Post-MVP: allowed grant types
- Post-MVP: client status

### Authorization Records

Required behavior:

- Role and permission relationship when RBAC is introduced
- Resource and action representation
- Tenant or organization scope when multi-tenancy is introduced

## Security Requirements

The system must:

- Use secure token signing.
- Avoid storing plaintext secrets.
- Avoid logging credentials or tokens.
- Require current-password verification before local password change.
- Hash new local passwords with Argon2id before storage.
- Validate redirect URIs strictly.
- Validate token issuer and audience.
- Expire access tokens.
- Rotate or revoke refresh tokens according to policy.
- Detect refresh token reuse when rotation is enabled.
- Rate limit high-risk endpoints.
- Keep provider secrets centralized in configuration or secret management.
- Record security-sensitive events for audit when audit is available.
- Use one abuse-control policy interface for high-risk operations.
- Emit structured redacted security events through one security event sink.

### MVP Token Policy

Access token behavior:

- JWT access tokens are signed with RS256.
- Access tokens include `kid`, issuer, subject, audience, issued time, expiration, session identifier, client identifier, and `jti`.
- MVP access token lifetime is configured centrally and should be short-lived.
- Token verification must check signature, issuer, audience, expiration, and `kid`.

Refresh token behavior:

- Refresh tokens are opaque random secrets.
- Only refresh token hashes are stored.
- Session module owns refresh token records, families, rotation, reuse detection, and revocation.
- Token module may generate refresh token secrets but must not persist refresh token state.
- Refresh token exchange consumes the old token and creates the new token in one transaction.

### MVP Client Context

The MVP uses static client context from centralized configuration.

Required configuration:

- client identifier
- trusted origin when needed

The client application registry is post-MVP.

`tokens.audience` is the single MVP JWT audience source.

## Non-Functional Requirements

### Maintainability

- Modules must be locally understandable.
- Files should avoid mixed responsibilities.
- Naming must be descriptive.
- Dependencies must be explicit.
- Post-MVP capabilities must follow `docs/MODULE_EXPANSION.md`.
- New modules must use explicit extension points instead of modifying core flows directly.

### Reliability

- Login and token flows must fail explicitly.
- Provider failures must not corrupt internal identity state.
- Session state must remain consistent with refresh token behavior.

### Scalability

- Token verification should support high read volume.
- Session and refresh token writes should support horizontal scaling.
- Provider adapters should isolate external latency and failure behavior.

### Observability

- Authentication attempts should be observable.
- Provider failure rates should be measurable.
- Token refresh and session revocation should be traceable.
- Security events should be audit-friendly.

### Compatibility

- The service should be compatible with JWT, OAuth2, and OIDC expectations.
- It should integrate with API gateways and internal microservices without custom identity assumptions.

## Development Phases

### Phase 1 - Foundation

- Use the fixed Rust and Axum stack.
- Create project skeleton.
- Add centralized configuration.
- Add feature toggle support.
- Add internal user model.
- Add external identity model.
- Add provider adapter contract.
- Add local username/password provider.
- Add local password change flow.
- Add Supabase provider adapter.
- Add session and token foundation.

### Phase 2 - Multi-Provider Authentication

- Add email code provider.
- Add SMS code provider.
- Add generic OAuth2 provider.
- Add GitHub provider.
- Add Google provider.
- Add Apple Sign In provider.
- Add WeChat provider.
- Add account linking.
- Add active session listing and revocation.

### Phase 3 - OAuth2 and OIDC Provider Mode

- Add client application registry.
- Add OAuth2 authorization flow.
- Add token endpoint.
- Add OIDC discovery and JWKS.
- Add userinfo endpoint.

### Phase 4 - Authorization

- Add scope-based authorization.
- Add RBAC.
- Add permission checks for APIs and internal services.

### Phase 5 - Enterprise Capabilities

- Add organization and tenant model.
- Add MFA.
- Add Passkey.
- Add SSO.
- Add risk control.
- Expand audit logging.

## Historical Documentation Criteria

Step 2 was complete when:

- `docs/SPEC.md` exists.
- `docs/BUILD.md` exists.
- System capabilities are specified.
- Initial build and usage guidance is documented.
