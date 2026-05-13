# Architecture Design

## Current Step

Step 1 - Architecture Design.

This document defines the initial architecture for the Identity Platform / IAM Service.

No implementation code is included in this step.

## System Overview

The Identity Platform is a unified identity authentication and authorization microservice.

It provides one stable internal identity system for multiple clients, applications, identity providers, APIs, gateways, and internal services.

The core architectural goal is to normalize many external login methods into one internal account model:

```text
External Identity Provider Identity
        -> Provider Adapter
        -> Identity Binding
        -> internal_user_id
        -> Session / Token / Authorization
```

Supported and planned identity providers include:

- Local username and password
- Supabase Auth
- WeChat login
- SMS verification code
- Email verification code
- OAuth2 provider login
- GitHub
- Google
- Apple Sign In

The service is designed to support future enterprise capabilities:

- MFA
- SSO
- RBAC
- Organization and tenant model
- Passkey
- Risk control
- Audit logging
- OAuth2 and OIDC provider mode

## Architecture Goals

- Provide one identity authority for all platform applications.
- Decouple external identity providers from internal user identity.
- Keep authentication, session, token, and authorization responsibilities separate.
- Make each module understandable in isolation.
- Support standard protocols such as JWT, OAuth2, and OIDC.
- Allow incremental delivery without forcing all enterprise features into the first version.
- Avoid hidden provider-specific logic leaking into core identity modules.

## Proposed Directory Structure

The future implementation should follow this structure or an equivalent structure with the same boundaries:

```text
identity-service/
  Agent.md
  docs/
    ARCHITECTURE.md
    SPEC.md
    BUILD.md
    TECH_STACK.md
    MVP.md
    MODULE_EXPANSION.md
    nextsession.md
  src/
    application/
      authentication/
      authorization/
      feature_toggle/
      identity_binding/
      session/
      token/
    domain/
      users/
      identities/
      sessions/
      permissions/
      tenants/
    providers/
      common/
      local_password/
      supabase/
      verification_code/
      wechat/
      sms/
      email/
      oauth2/
      github/
      google/
      apple/
    delivery_adapters/
      sms/
      email/
    infrastructure/
      database/
      cache/
      message_queue/
      external_http/
      observability/
    interfaces/
      http/
      openapi/
      internal_rpc/
      gateway/
    config/
    security/
    shared/
    main/
  tests/
    unit/
    integration/
    contract/
    e2e/
```

## Module Breakdown

### Main Entry Module

Purpose:

- Start the service.
- Load centralized configuration.
- Initialize infrastructure dependencies.
- Register API routes and protocol handlers.

Input:

- Runtime configuration
- Environment variables
- Infrastructure connection settings

Output:

- Running Identity Platform service

Dependencies:

- Configuration module
- Interface modules
- Infrastructure modules

### Configuration Module

Purpose:

- Provide one centralized configuration entry point.
- Validate required settings before service startup.
- Expose typed configuration to other modules.

Input:

- Environment variables
- Secret manager values
- Deployment configuration

Output:

- Validated service configuration

Dependencies:

- None beyond standard configuration loading mechanisms

Design rule:

- Configuration must not be scattered across provider adapters or application services.

### Authentication Module

Purpose:

- Coordinate login, registration, credential verification, and provider callback flows.
- Convert provider results into internal authentication outcomes.

Input:

- Login requests
- Provider callback payloads
- Verification codes
- OAuth2 authorization responses

Output:

- Authenticated `internal_user_id`
- Authentication result
- Session creation request
- Token issuance request

Dependencies:

- Provider adapter interfaces
- Identity binding module
- User module
- Session module
- Token module

### Provider Adapter Modules

Purpose:

- Isolate provider-specific behavior.
- Normalize each provider response into a common external identity format.

Input:

- Provider-specific login data
- Provider access tokens
- Authorization codes
- Callback payloads
- Verification code validation requests

Output:

- Normalized external identity
- Provider verification result
- Provider error mapped to internal error type

Dependencies:

- External provider SDKs or HTTP clients
- Provider-specific configuration
- Shared provider adapter contracts

Design rule:

- Provider adapters must not create sessions, issue tokens, or directly decide authorization.

### Local Password Provider Module

Purpose:

- Provide first-party username and password registration and login.
- Provide authenticated local password change.
- Treat local credentials as one provider type behind the same provider adapter boundary.
- Keep password hashing and verification isolated from general authentication flow coordination.

Input:

- Username
- Plaintext password during registration or login
- Current password and new password during password change
- Password policy context

Output:

- Normalized external identity using provider name `local_password`
- Password verification result
- Password change result
- Explicit credential error

Dependencies:

- Credential repository
- Password hashing library
- Security policy module

Design rule:

- Plaintext passwords must never be stored, logged, emitted in events, or passed beyond the local password provider boundary.
- Password hashes must use a modern password hashing algorithm selected in the security policy.
- Password change must verify the current password before replacing the stored password hash.

### Post-MVP Verification Code Provider Module

Purpose:

- Provide phone verification code login and email verification code login after the MVP.
- Own verification code lifecycle, including creation, validation, expiration, retry limits, and consumption.
- Normalize verified phone or email identities into the provider adapter output shape.

Input:

- Phone number or email address.
- Verification code request.
- Verification code submit request.
- Delivery channel selection from centralized configuration.

Output:

- Verification code challenge result.
- Verification code validation result.
- Normalized external identity for `sms` or `email`.
- Explicit verification error.

Dependencies:

- Verification code repository.
- Delivery adapter contract.
- Security policy module.
- Rate-limit module when introduced.

Design rule:

- Verification code providers must not call SMS or email vendors directly.
- Delivery must go through delivery adapters.
- Verification code login is post-MVP and must not be added to the MVP implementation.

### Post-MVP Delivery Adapter Modules

Purpose:

- Isolate vendor-specific SMS and email delivery behavior.
- Allow different SMS and email vendors to be enabled through configuration.
- Keep provider login logic independent from vendor APIs, signatures, templates, throttling behavior, and error formats.

Input:

- Delivery request.
- Destination phone number or email address.
- Template identifier.
- Template variables.
- Vendor configuration.

Output:

- Delivery accepted result.
- Delivery failed result.
- Vendor error mapped to internal delivery error.

Dependencies:

- External SMS vendor APIs.
- External email vendor APIs.
- Centralized provider and delivery configuration.

Design rule:

- SMS vendors and email vendors are delivery adapters, not identity providers.
- Changing SMS or email vendors must not change authentication, identity binding, session, or token modules.
- Vendor credentials must be loaded only through centralized configuration or secret management.

### Identity Binding Module

Purpose:

- Map external provider identities to platform `internal_user_id`.
- Manage account linking and unlinking.
- Prevent duplicate or conflicting identity bindings.

Input:

- Normalized external identity
- Existing user context when linking accounts
- Binding operation request

Output:

- Resolved `internal_user_id`
- New identity binding
- Binding conflict result

Dependencies:

- User module
- Identity persistence repository
- Audit module when introduced

### User Module

Purpose:

- Manage internal users as platform-owned identities.
- Store stable user profile and account lifecycle state.

Input:

- User creation request
- User lookup request
- Account status change request

Output:

- Internal user record
- User lifecycle status

Dependencies:

- User persistence repository

Design rule:

- External provider profile data must not replace the platform-owned user identity model.

### Session Module

Purpose:

- Manage session lifecycle.
- Track device login state.
- Support refresh token rotation and session revocation.

Input:

- Authenticated `internal_user_id`
- Device metadata
- Refresh token request
- Logout or revocation request

Output:

- Session record
- Session status
- Refresh validation result

Dependencies:

- Session repository
- Token module
- Security policy module

### Token Module

Purpose:

- Issue and verify platform tokens.
- Support JWT access tokens and refresh token workflows.
- Prepare for OAuth2 and OIDC provider mode.

Input:

- Token issuance request
- Session state
- User identity
- Client application context

Output:

- Access token
- Refresh token
- ID token when OIDC is introduced
- Token validation result

Dependencies:

- Key management
- Session module
- Authorization module for claim enrichment when needed

Design rule:

- Token generation must be deterministic from explicit input and configured signing keys.

### Authorization Module

Purpose:

- Decide whether an authenticated subject can access a resource or action.
- Support future RBAC, tenant, organization, and policy-based authorization.

Input:

- Authenticated subject
- Requested resource
- Requested action
- Tenant or organization context when available

Output:

- Authorization decision
- Deny reason
- Required permissions when applicable

Dependencies:

- Permission repository
- Role repository when RBAC is introduced
- Tenant module when multi-tenancy is introduced

### OAuth2 / OIDC Provider Module

Purpose:

- Allow the platform to act as an identity provider for third-party clients and internal applications.
- Expose standard authorization, token, userinfo, discovery, and JWKS endpoints.

Input:

- OAuth2 authorization requests
- Token exchange requests
- Client credentials
- OIDC userinfo requests

Output:

- Authorization code
- Access token
- Refresh token
- ID token
- Userinfo response
- Discovery metadata
- JWKS response

Dependencies:

- Authentication module
- Session module
- Token module
- Client application registry

### Client Application Module

Purpose:

- Manage registered applications that use the identity service.
- Store redirect URIs, allowed grants, scopes, and client trust level.

Input:

- Client registration data
- OAuth2 client lookup request
- Client credential validation request

Output:

- Client application record
- Client validation result

Dependencies:

- Client application repository

### Security Policy Module

Purpose:

- Centralize security rules that must remain consistent across authentication, token, and session behavior.

Input:

- Passwordless login policy
- Refresh token policy
- Session lifetime policy
- Device trust policy
- Risk signals when introduced

Output:

- Policy decision
- Required challenge
- Security violation result

Dependencies:

- Configuration module
- Risk module when introduced

### Feature Toggle Module

Purpose:

- Control which identity providers and optional capabilities are enabled.
- Provide one explicit place where runtime feature availability is decided.
- Prevent disabled provider modules from registering routes or executing provider logic.

Input:

- Centralized service configuration
- Provider capability definitions

Output:

- Enabled feature list
- Enabled provider list
- Disabled feature decision

Dependencies:

- Configuration module

Design rule:

- Feature toggles must be evaluated at startup and exposed as explicit provider availability.
- Business logic must not read environment variables directly to decide whether a module is enabled.

### Audit Module

Purpose:

- Record security-sensitive events.
- Support compliance, incident investigation, and behavior analysis.

Input:

- Login event
- Logout event
- Token refresh event
- Binding event
- Permission decision event
- Administrative change event

Output:

- Immutable audit event record

Dependencies:

- Audit persistence repository
- Observability infrastructure

### Infrastructure Modules

Purpose:

- Provide concrete implementations for persistence, cache, external HTTP, message queues, key storage, and observability.

Input:

- Repository calls
- Cache operations
- External HTTP requests
- Metrics and log events

Output:

- Stored data
- Loaded data
- External service responses
- Metrics and logs

Dependencies:

- Database driver
- Cache client
- HTTP client
- Observability libraries

Design rule:

- Infrastructure modules must implement contracts defined by application or domain modules, not own business logic.

### Interface Modules

Purpose:

- Expose the service through HTTP, OpenAPI, gateway integration, and internal service interfaces.
- Convert transport requests into application-level commands.

Input:

- HTTP requests
- OpenAPI requests
- Internal RPC requests
- Gateway token validation requests

Output:

- HTTP responses
- OpenAPI responses
- Internal service responses

Dependencies:

- Application modules
- Request validation
- Response serialization

Design rule:

- Interface modules must not contain provider-specific login logic or persistence logic.

## Core Data Model

### Internal User

Represents the platform-owned user identity.

Key fields:

- `internal_user_id`
- account status
- display profile
- primary contact fields when verified
- created time
- updated time

### External Identity

Represents one identity from an external provider.

Key fields:

- provider name
- provider subject identifier
- provider account metadata
- verified contact information
- linked `internal_user_id`
- binding status

MVP provider identities:

- `local_password` identity for username/password login
- `supabase` identity for Supabase user mapping

### Session

Represents one authenticated login state.

Key fields:

- session identifier
- `internal_user_id`
- client application identifier
- device identifier
- issued time
- last active time
- expiration time
- revoked time
- refresh token family identifier

### Token

Represents signed or opaque credentials issued by the platform.

Key fields:

- token type
- subject
- audience
- scopes
- expiration
- signing key identifier
- session identifier

### Client Application

Represents an application allowed to use the identity service.

Key fields:

- client identifier
- client type
- redirect URIs
- allowed grant types
- allowed scopes
- secret metadata when applicable
- status

### Authorization Policy

Represents access rules.

Key fields:

- role
- permission
- resource
- action
- tenant or organization scope when introduced

## Data Flow

### Login Flow

```text
Client
  -> Interface Module
  -> Authentication Module
  -> Provider Adapter
  -> Identity Binding Module
  -> User Module
  -> Session Module
  -> Token Module
  -> Client
```

Flow description:

1. A client submits a login request or provider callback.
2. The interface module validates request shape and passes a command to the authentication module.
3. The authentication module selects the correct provider adapter.
4. The provider adapter verifies the provider-specific credential and returns a normalized external identity.
5. The identity binding module resolves or creates the corresponding `internal_user_id`.
6. The session module creates a session and records device login state.
7. The token module issues access and refresh tokens.
8. The interface module returns the authentication response.

### Refresh Token Flow

```text
Client
  -> Interface Module
  -> Session Module
  -> Token Module
  -> Session Module
  -> Client
```

Flow description:

1. A client submits a refresh token.
2. The session module validates the token family, session status, rotation rules, and expiration.
3. The token module issues a new access token and refresh token when valid.
4. The session module records the rotation result.
5. The client receives new credentials.

### Local Password Change Flow

```text
Authenticated User
  -> Interface Module
  -> Authentication Module
  -> Local Password Provider Module
  -> Session Module
  -> Token Module
  -> Client
```

Flow description:

1. An authenticated user submits current password and new password.
2. The interface module validates request shape and passes a command to the authentication module.
3. The authentication module confirms the authenticated `internal_user_id`.
4. The local password provider verifies the current password.
5. The local password provider hashes and stores the new password.
6. The session module revokes or rotates old refresh token state according to policy.
7. The token module issues a new token pair when policy requires it.
8. The interface module returns the password change result.

### Authorization Flow

```text
API / Gateway / Microservice
  -> Token Verification
  -> Authorization Module
  -> Policy / Role / Permission Lookup
  -> Authorization Decision
```

Flow description:

1. A protected service submits a token or authenticated subject context.
2. The token module verifies token signature, issuer, audience, expiration, and session linkage when required.
3. The authorization module evaluates resource and action permissions.
4. The authorization decision is returned with an allow or deny result.

### Account Linking Flow

```text
Authenticated User
  -> Interface Module
  -> Authentication Module
  -> Provider Adapter
  -> Identity Binding Module
  -> Audit Module
```

Flow description:

1. A logged-in user starts linking another identity provider.
2. The provider adapter verifies the external identity.
3. The identity binding module checks whether the external identity is already linked.
4. If safe, the external identity is bound to the existing `internal_user_id`.
5. A binding event is recorded for audit.

## Key Design Decisions

### Selected Technology Direction

The implementation direction is fixed as Rust with Axum.

This is fixed and is not an open question.

AI agents must not reopen the language or HTTP framework decision unless the user explicitly asks to change the stack.

Reason:

- Identity services are security-sensitive and benefit from strong types, explicit error handling, memory safety, and predictable resource usage.
- Rust makes hidden shared mutable state harder to introduce.
- Axum's explicit routing and extractor model matches the architecture goal of locally understandable modules.

The decision is recorded in:

- `docs/TECH_STACK.md`

### MVP Hard Boundary

The MVP includes only:

- Local username/password registration.
- Local username/password login.
- Local username/password change for authenticated users.
- Supabase provider adapter.
- Internal user identity mapping.
- Basic session lifecycle.
- JWT access token issuance.
- Server-tracked refresh token issuance and exchange.
- Provider feature toggles through centralized configuration.

All other providers and enterprise features are post-MVP.

The MVP plan is recorded in:

- `docs/MVP.md`

### Supabase Provider Boundary

Supabase is one provider from this service's point of view.

Supabase Auth may authenticate users through its own enabled methods, including email/password, magic link, email OTP, phone auth, social login, SSO, OAuth, and OIDC.

The MVP must not split those Supabase upstream methods into separate first-party providers.

All Supabase-authenticated users must normalize to:

- provider name: `supabase`
- provider subject identifier: Supabase user identifier

Supabase-side credential management, including Supabase email/password change and password reset flows, remains in Supabase Auth.

### Provider Adapter Pattern

All provider-specific behavior must be isolated behind adapter contracts.

Reason:

- Providers have different protocols, callback shapes, errors, and profile formats.
- Core authentication should operate on normalized identity results only.
- New providers should not require changes across session, token, or authorization modules.

### Internal User ID as the Stable Identity

All platform behavior must use `internal_user_id` as the stable subject identifier.

Reason:

- Provider identifiers can change, disappear, or conflict.
- One user may link multiple provider identities.
- Authorization and session state must not depend on external identity providers.

### Session and Token Separation

Session state and token issuance are separate modules.

Reason:

- Tokens are credentials.
- Sessions are lifecycle state.
- Revocation, refresh token rotation, and device state require server-side session tracking.

### Authorization Separate from Authentication

Authentication proves identity.

Authorization decides access.

Reason:

- The system must support many clients, tenants, roles, scopes, and policies.
- Combining login and permission logic makes future RBAC and organization support harder to reason about.

### Centralized Security Policies

Security-sensitive rules must be centralized in a security policy module.

Reason:

- Token lifetime, refresh behavior, MFA requirements, device trust, and risk challenges must be consistent.
- Scattered policy decisions make security behavior unpredictable.

### Standards-Compatible Protocol Surface

The service should support JWT, OAuth2, and OIDC patterns.

Reason:

- Gateways, microservices, third-party applications, and modern clients expect standard authentication flows.
- A standards-compatible interface reduces custom integration burden.

### Incremental Enterprise Feature Expansion

Enterprise features must be introduced as modules, not mixed into the first authentication flow.

Recommended order:

1. MVP local username/password and Supabase provider foundation.
2. Core user, provider adapter, session, and token hardening.
3. OAuth2/OIDC provider surface.
4. Authorization and RBAC.
5. Organization and tenant support.
6. MFA and Passkey.
7. Risk control and audit expansion.

## Architecture Boundaries

Detailed module expansion rules are defined in:

- `docs/MODULE_EXPANSION.md`

Post-MVP capabilities must be added through explicit extension points:

- Identity provider modules.
- Delivery adapter modules.
- Verification code modules.
- Authorization modules.
- Protocol surface modules.

New modules must not rewrite MVP core authentication, identity binding, session, or token behavior.

Provider adapters may:

- Verify provider-specific credentials.
- Fetch provider-specific profile data.
- Normalize external identities.
- Map provider errors into internal error types.

Provider adapters must not:

- Create internal users directly.
- Issue platform tokens.
- Create sessions.
- Make authorization decisions.

Authentication may:

- Coordinate login flows.
- Call provider adapters.
- Request identity binding.
- Request session creation and token issuance.

Authentication must not:

- Store provider-specific implementation details.
- Decide business permissions.
- Own token signing keys.

Authorization may:

- Evaluate subject, action, resource, scope, tenant, and role context.
- Return explicit allow or deny results.

Authorization must not:

- Verify external provider credentials.
- Create sessions.
- Link identities.

## Risks and Unknowns

- Migration tooling still needs to be selected.
- Token storage strategy and refresh token rotation details need specification.
- Supabase must remain an external identity provider and must not replace `internal_user_id`, platform sessions, or platform tokens.
- WeChat login requires environment-specific behavior for web, mobile, and mini-program scenarios.
- OAuth2/OIDC provider mode needs careful client registry and redirect URI validation.
- Multi-tenant authorization can significantly affect the data model and should not be added implicitly.

## Architecture Completion Criteria

Step 1 is complete when:

- Overall architecture is documented.
- Module responsibilities are explicit.
- Data flow is documented.
- Key design decisions are recorded.
- No implementation code has been written.
