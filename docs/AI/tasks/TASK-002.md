# TASK-002: Database Models, Connection Pool & Migrations

## Objective
Implement PostgreSQL schema migrations and connection pool management using SQLx, covering `users`, `identities`, and `refresh_tokens`.

## Scope
- Create SQL migration files for `users`, `identities`, and `refresh_tokens`.
- Implement database connection initialization and automatic migration runner.
- Define database entity structs and repository query helpers.

## Allowed Files
- `src/db/mod.rs`
- `src/db/migrations/`
- `src/models/mod.rs`
- `src/models/user.rs`
- `src/models/identity.rs`
- `tests/db_test.rs`

## Dependencies
- TASK-001

## Inputs and Outputs
- **Input**: Database URL string
- **Output**: Initialized `PgPool` and migrated database tables

## Acceptance Criteria
- SQL migrations execute cleanly.
- Unique constraints on `(provider, provider_user_id)` and foreign keys are verified.

## Verification Commands
- `cargo check`
- `cargo test --lib models`

## Risks and Assumptions
- Requires PostgreSQL connection or offline SQLx checks during testing.

## Status
DONE
