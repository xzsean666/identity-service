# System Specification

## Current Step

Step 2 - Documentation.

This document specifies the Identity Platform / IAM Service behavior, scope, interfaces, data model, and development boundaries.

No implementation code is included in this step.

## Product Positioning

The Identity Platform is a lightweight, extensible identity authentication and authorization platform for modern full-platform applications and microservice systems.

It is similar in role to Keycloak, Auth0, Okta, or Clerk, but the intended direction is:

- Lighter operational footprint
- Clearer modular architecture
- Strong AI-assisted maintainability
- Provider adapter based extension model
- Practical support for web, mobile, OpenAPI, third-party applications, and internal services

## Primary Goals

- Provide unified user identity management.
- Support multiple login providers through a consistent provider adapter model.
- Support local username/password registration and login as the first MVP provider.
- Map all external identities to one stable `internal_user_id`.
- Issue and verify platform tokens.
- Manage refresh tokens and session lifecycle.
- Track device login state.
- Support standard JWT, OAuth2, and OIDC integration patterns.
- Provide a foundation for RBAC, tenants, MFA, SSO, Passkey, risk control, and audit logging.

## MVP Scope

The MVP is defined in:

- `docs/MVP.md`

The MVP includes:

- Local username/password registration.
- Local username/password login.
- Supabase provider adapter.
- Provider enable/disable configuration.
- Internal user identity mapping.
- Session creation.
- Access token and refresh token issuance.
- Refresh token exchange.
- Current session logout.

The MVP excludes all other providers and enterprise features unless explicitly added as modules after the foundation is stable.

## Technology Stack Direction

The recommended stack is documented in:

- `docs/TECH_STACK.md`

Current recommendation:

- Rust.
- Axum.
- PostgreSQL.
- Argon2id for password hashing.
- JWT access tokens with server-tracked refresh token state.

TypeScript/NestJS remains the main alternative if fastest MVP delivery becomes more important than long-term identity-service correctness.

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

Initial provider categories:

- Local username/password
- Supabase
- WeChat
- SMS verification code
- Email verification code
- OAuth2 provider
- GitHub
- Google
- Apple Sign In

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

The system must:

- Create sessions.
- List active sessions for a user.
- Revoke one session.
- Revoke all sessions for a user.
- Track device metadata.
- Track client application metadata.
- Support refresh token rotation.
- Reject revoked, expired, or reused refresh tokens.

### Token Management

The system must:

- Issue access tokens.
- Issue refresh tokens.
- Verify access token signature and claims.
- Rotate refresh tokens when configured.
- Support token revocation through session state.
- Support signing key rotation in a future phase.

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

The system must:

- Verify authenticated subject context.
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

Input depends on provider type:

- OAuth2 authorization code
- Provider access token
- SMS code
- Email verification code
- Supabase session token
- WeChat login code
- Apple identity token

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
- Disabled providers must not register public login routes.
- Disabled providers must not execute provider-specific verification logic.
- Disabled provider usage must return an explicit provider-disabled error.
- Business logic must not read environment variables directly to decide feature availability.

MVP provider toggles:

- `local_password`
- `supabase`

Future provider toggles:

- `wechat`
- `sms`
- `email`
- `oauth2`
- `github`
- `google`
- `apple`

## API Surface

The recommended framework is Axum, but exact route names and handler structure should be finalized during implementation.

The API surface should be designed around these categories.

### Authentication APIs

Planned capabilities:

- Start provider login
- Complete provider callback
- Login with SMS code
- Login with email code
- Login with Supabase token
- Login with OAuth2 authorization code
- Logout

### Session APIs

Planned capabilities:

- Refresh token
- List active sessions
- Revoke current session
- Revoke selected session
- Revoke all sessions

### User APIs

Planned capabilities:

- Get current user
- Update current user profile
- Disable account
- List linked identities
- Link identity provider
- Unlink identity provider

### Authorization APIs

Planned capabilities:

- Verify token
- Introspect token
- Check permission
- Resolve subject context

### OAuth2/OIDC APIs

Planned capabilities:

- Authorize
- Token
- Userinfo
- JWKS
- Discovery metadata
- Revoke token
- Introspect token

### Administration APIs

Planned capabilities:

- Manage users
- Manage sessions
- Manage client applications
- Manage provider settings
- Manage roles and permissions
- View audit events

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

- Unique client identifier
- Allowed redirect URIs
- Allowed scopes
- Allowed grant types
- Client status

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
- Validate redirect URIs strictly.
- Validate token issuer and audience.
- Expire access tokens.
- Rotate or revoke refresh tokens according to policy.
- Detect refresh token reuse when rotation is enabled.
- Rate limit high-risk endpoints.
- Keep provider secrets centralized in configuration or secret management.
- Record security-sensitive events for audit when audit is available.

## Non-Functional Requirements

### Maintainability

- Modules must be locally understandable.
- Files should avoid mixed responsibilities.
- Naming must be descriptive.
- Dependencies must be explicit.

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

- Select implementation language and framework.
- Create project skeleton.
- Add centralized configuration.
- Add feature toggle support.
- Add internal user model.
- Add external identity model.
- Add provider adapter contract.
- Add local username/password provider.
- Add Supabase provider adapter.
- Add session and token foundation.

### Phase 2 - Multi-Provider Authentication

- Add WeChat provider.
- Add SMS provider.
- Add email provider.
- Add GitHub, Google, and Apple providers.
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

## Acceptance Criteria for Documentation Step

Step 2 is complete when:

- `docs/SPEC.md` exists.
- `docs/BUILD.md` exists.
- System capabilities are specified.
- Initial build and usage guidance is documented.
- No implementation code has been written.
