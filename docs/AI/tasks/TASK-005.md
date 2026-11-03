# TASK-005: Identity & User Core Services

## Objective
Implement core business services for identity resolution, user provisioning, account linking, and session/refresh token lifecycle.

## Scope
- Implement `IdentityService`: given a verified `ExternalIdentity`, lookup existing `identities` entry or atomically create `users` + `identities` record.
- Implement `UserService`: lookup user profile by ID, update user profile (`name`, `avatar_url`).
- Implement `SessionService`: issue random refresh tokens, store SHA-256 hash, validate refresh token, revoke tokens.

## Allowed Files
- `src/services/identity.rs`
- `src/services/user.rs`
- `src/services/session.rs`
- `src/services/mod.rs`
- `tests/service_test.rs`

## Dependencies
- TASK-002
- TASK-003
- TASK-004

## Inputs and Outputs
- **Input**: Verified `ExternalIdentity`, user updates, or refresh tokens
- **Output**: User model, issued tokens (access_token, refresh_token)

## Acceptance Criteria
- Concurrent login with the same external identity resolves to the exact same internal `user_id`.
- Revoked or expired refresh tokens cannot be used to issue new access tokens.

## Verification Commands
- `cargo test --test service_test`

## Risks and Assumptions
- Database transaction atomicity must be preserved when creating user and identity.

## Status
DONE
