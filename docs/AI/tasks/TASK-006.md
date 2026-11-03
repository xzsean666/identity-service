# TASK-006: Auth & User REST API Handlers

## Objective
Implement HTTP REST endpoints for token exchange, refresh, revocation, current user retrieval, and profile updates.

## Scope
- Implement `POST /v1/auth/exchange`: validates request, calls provider verifier, resolves identity, returns tokens.
- Implement `POST /v1/auth/refresh`: validates refresh token, returns new access token (and rotated refresh token).
- Implement `POST /v1/auth/revoke`: invalidates current or specified refresh token.
- Implement `GET /v1/users/me`: authenticated via internal Bearer JWT, returns current user profile.
- Implement `PATCH /v1/users/me`: authenticated, updates user name and avatar.
- Implement `GET /v1/users/:id`: service-level or internal lookup of user by ID.
- Implement JWT authentication Axum extractor.

## Allowed Files
- `src/api/auth.rs`
- `src/api/users.rs`
- `src/api/middleware.rs`
- `src/api/routes.rs`
- `src/api/mod.rs`
- `tests/api_auth_test.rs`
- `tests/api_user_test.rs`

## Dependencies
- TASK-005

## Inputs and Outputs
- **Input**: HTTP requests with JSON payloads and Bearer tokens
- **Output**: JSON responses conforming to API specification with correct HTTP status codes

## Acceptance Criteria
- Full exchange-to-me request cycle succeeds.
- Invalid tokens return 401 Unauthorized with structured error JSON.
- Malformed requests return 400 Bad Request.

## Verification Commands
- `cargo test --test api_auth_test`
- `cargo test --test api_user_test`

## Risks and Assumptions
- Handlers should remain thin and delegate all business rules to services.

## Status
DONE
