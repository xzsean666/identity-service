# Module Expansion Rules

## Current Step

Documentation hardening.

This document defines how MVP and post-MVP modules must be added without weakening the architecture.

No implementation code is included in this step.

## Purpose

The project must remain easy to extend after the MVP.

New capabilities must be added through explicit module boundaries instead of modifying core authentication, identity binding, session, or token logic directly.

## Locked MVP Core

The MVP core includes:

- Local username/password registration.
- Local username/password login.
- Local username/password change for authenticated users.
- Supabase provider adapter.
- Internal user model.
- External identity model.
- Identity binding.
- Session lifecycle.
- JWT access token issuance.
- Server-tracked refresh token issuance and exchange.
- Centralized provider feature toggles.

Post-MVP modules must not rewrite the MVP core.

If a post-MVP feature cannot fit an existing extension point, update the architecture documents before implementation.

## Extension Point Types

### Identity Provider Module

Use this extension point for a new login or identity source.

Examples:

- Email verification code.
- SMS verification code.
- GitHub.
- Google.
- Apple Sign In.
- WeChat.
- Generic OAuth2 provider.

Responsibilities:

- Verify provider-specific credentials or callback payloads.
- Normalize provider identity into the shared external identity shape.
- Map provider errors into internal provider errors.
- Respect centralized provider feature toggles.
- Expose a provider descriptor for registration.

Must not:

- Create platform sessions.
- Issue platform tokens.
- Create internal users directly.
- Decide authorization.
- Call SMS or email vendors directly.

Provider descriptor must include:

- Provider name.
- Feature toggle key.
- Supported entry type, such as direct credential, token exchange, callback, or verification code.
- Route registration needs.
- Normalized identity output contract.
- Error mapping contract.

### Delivery Adapter Module

Use this extension point for a message delivery vendor.

Examples:

- Aliyun SMS.
- Tencent Cloud SMS.
- Twilio.
- AWS SNS.
- Resend.
- SendGrid.
- AWS SES.
- SMTP.

Responsibilities:

- Send one message through one vendor.
- Isolate vendor credentials, templates, request formats, and response formats.
- Map vendor errors into internal delivery errors.
- Respect centralized delivery configuration.

Must not:

- Validate verification codes.
- Create internal users.
- Issue tokens.
- Create sessions.
- Decide identity binding.

### Verification Code Module

Use this extension point for email or phone code verification flows.

Responsibilities:

- Create verification challenges.
- Enforce expiration.
- Enforce retry and resend limits.
- Consume verification codes exactly once.
- Use delivery adapters for sending messages.
- Normalize verified phone or email identities into provider identities.
- Return non-enumerating responses.

Must not:

- Depend on one vendor directly.
- Store plaintext verification codes when avoidable.
- Issue platform tokens directly.

Verification challenge data contract:

- purpose
- canonical destination
- destination hash
- code hash
- expiration time
- attempt count
- resend count
- consumed time
- correlation identifier
- idempotency key when needed

Verification logs must redact destination and code values.

### Authorization Module

Use this extension point for access decisions after authentication.

Examples:

- Scope checks.
- RBAC.
- Organization permissions.
- Tenant-aware policies.

Responsibilities:

- Evaluate subject, resource, action, and scope context.
- Return explicit allow or deny decisions.
- Keep authorization separate from login.

Must not:

- Verify external provider credentials.
- Create sessions.
- Link identities.

### Security Support Interface

Use this interface for cross-cutting abuse controls and security events.

Examples:

- Rate limit checks.
- Login attempt throttling.
- Verification code resend throttling.
- Structured security event logging.

Responsibilities:

- Expose one `AbuseControlPolicy` interface for high-risk operations.
- Expose one `SecurityEventSink` interface for redacted security events.
- Keep provider-specific modules from inventing their own throttling or audit paths.

Must not:

- Contain provider-specific login logic.
- Issue tokens.
- Own identity binding.

Initial MVP implementation may use local logs and simple configured limits.

### Protocol Surface Module

Use this extension point for protocol endpoints.

Examples:

- OAuth2 provider mode.
- OIDC provider mode.
- JWKS.
- Userinfo.
- Introspection.
- Revocation.

Responsibilities:

- Expose standards-compatible protocol endpoints.
- Depend on client application registry, session, token, and user modules.

Must not:

- Bypass platform session or token modules.
- Bypass identity binding.

## Module Addition Checklist

Every new module must define:

- Module type.
- Contract version.
- Purpose.
- Inputs.
- Outputs.
- Dependencies.
- Configuration keys.
- Feature toggle key.
- Registry descriptor.
- Error types.
- Test scope.
- Data ownership.
- Explicit non-responsibilities.

Every new module must include:

- Unit tests for local behavior.
- Contract tests for module interface behavior.
- Integration tests when infrastructure or external APIs are involved.
- Documentation updates in `docs/ARCHITECTURE.md`, `docs/SPEC.md`, `docs/BUILD.md`, and `docs/nextsession.md`.
- No empty scaffolding for future modules before their feature is approved.

## Configuration Rules

All optional modules must be controlled by centralized configuration.

Required configuration behavior:

- Disabled modules must not register public routes.
- Disabled modules must not execute provider or delivery logic.
- Disabled module usage must return an explicit disabled error.
- Business logic must not read environment variables directly.
- Secrets must be loaded through centralized configuration or secret management.

Configuration namespaces:

- Identity providers use `identity_providers.<provider_name>`.
- Delivery adapters use `delivery_adapters.sms.<vendor_name>` or `delivery_adapters.email.<vendor_name>`.
- Security policies use `security.<policy_name>`.
- Token policies use `tokens.<policy_name>`.

## Directory Rules

Future implementation should place modules by extension type:

```text
src/
  providers/
    <provider_name>/
  delivery_adapters/
    sms/
      <vendor_name>/
    email/
      <vendor_name>/
  application/
    authorization/
    feature_toggle/
    identity_binding/
    session/
    token/
  config/
```

Rules:

- Provider modules live under `src/providers/`.
- Delivery vendor modules live under `src/delivery_adapters/`.
- Shared contracts live in a common module, not inside a concrete provider.
- Core session and token modules must not import concrete providers or delivery vendors.
- MVP implementation must create only active MVP modules.
- Post-MVP directories must be added only when the module is approved for implementation.

## Composition Root and Registry

Concrete providers and delivery adapters must be wired at startup only.

The composition root builds:

- Enabled provider registry.
- Enabled delivery adapter registry.
- Route registrations for enabled modules.
- Disabled module decisions.

Rules:

- Contracts must not depend on concrete modules.
- Authentication depends on the provider registry, not concrete providers.
- Verification code modules depend on the delivery adapter registry, not concrete vendors.
- Concrete modules implement contracts and expose descriptors.
- Disabled modules must not register public routes.
- Disabled module requests must fail with an explicit disabled error.

## Dependency Direction

Allowed direction:

```text
interface
  -> application
  -> provider registry
  -> provider contract

main / composition root
  -> concrete provider
  -> provider contract

verification code provider
  -> delivery adapter registry
  -> delivery adapter contract

main / composition root
  -> concrete delivery adapter
  -> delivery adapter contract

authentication
  -> identity binding
  -> session
  -> token
```

Forbidden direction:

```text
token -> concrete provider
session -> concrete provider
identity binding -> delivery vendor
provider -> token signing
delivery adapter -> identity binding
provider contract -> concrete provider
delivery adapter contract -> concrete delivery adapter
```

## Review Rule

If adding a post-MVP feature requires changing multiple core modules, stop and update the architecture first.

Do not continue by spreading logic across files.

The expected solution is usually a new extension point, a better contract, or a narrower feature boundary.
