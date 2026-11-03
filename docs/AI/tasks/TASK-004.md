# TASK-004: External Provider Verification Engine

## Objective
Implement a modular third-party provider verification framework supporting Google, X (Twitter), Supabase, and a Mock provider for automated testing.

## Scope
- Define `ProviderVerifier` async trait with `verify(token: &str) -> Result<ExternalIdentity, AppError>`.
- Implement Google ID token verification (OIDC / tokeninfo endpoint or JWKS verification).
- Implement X (Twitter) OAuth2 bearer token verification (`/2/users/me`).
- Implement Supabase JWT verification (using Supabase JWT secret or JWKS).
- Implement Mock provider adapter for integration tests without real network credentials.

## Allowed Files
- `src/providers/mod.rs`
- `src/providers/google.rs`
- `src/providers/x.rs`
- `src/providers/supabase.rs`
- `src/providers/mock.rs`
- `tests/provider_test.rs`

## Dependencies
- TASK-001

## Inputs and Outputs
- **Input**: Provider name string + external token/credential string
- **Output**: Verified `ExternalIdentity { provider, provider_user_id, email, name, avatar_url }`

## Acceptance Criteria
- Untrusted tokens are rejected.
- Mock provider allows fast, deterministic unit and integration tests.
- Real provider clients correctly handle HTTP timeouts and error responses.

## Verification Commands
- `cargo test --test provider_test`

## Risks and Assumptions
- Real external providers require live network and API credentials; mock provider ensures test suite remains hermetic.

## Status
DONE
