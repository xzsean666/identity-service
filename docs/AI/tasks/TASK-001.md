# TASK-001: Project Skeleton, Configuration & Health Check Endpoint

## Objective
Establish the Rust project skeleton with Axum, Tokio, strongly-typed configuration loaded from environment variables, unified error handling, and a working `/health` / `/healthz` endpoint.

## Scope
- Initialize Cargo project with required dependencies.
- Define strongly-typed configuration in `src/config.rs` (server host/port, database URL, JWT settings, provider flags).
- Define unified application error type in `src/error.rs` with Axum `IntoResponse` implementation.
- Implement `/healthz` endpoint returning JSON status.
- Implement integration/unit test verifying the server builds and health check responds.

## Allowed Files
- `Cargo.toml`
- `.gitignore`
- `.env.example`
- `src/main.rs`
- `src/lib.rs`
- `src/config.rs`
- `src/error.rs`
- `tests/health_check.rs`

## Dependencies
None (initial task)

## Inputs and Outputs
- **Input**: Environment variables (via `.env` or process environment)
- **Output**: Running Axum HTTP server on configured address/port exposing `/healthz`

## Acceptance Criteria
1. Project compiles without warnings or errors via `cargo check` and `cargo test`.
2. Configuration parses valid environment variables and applies sensible defaults for local development.
3. Errors format cleanly into JSON responses (`{ "error": { "code": "...", "message": "..." } }`).
4. Health check endpoint responds with HTTP 200 OK and valid JSON `{ "status": "ok", "version": "..." }`.

## Verification Commands
- `cargo check`
- `cargo test --test health_check`

## Risks and Assumptions
- Must use modern, maintained crates compatible with Axum 0.8 / Tokio 1.x.

## Status
DONE
