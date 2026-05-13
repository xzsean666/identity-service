# Identity Service

Rust/Axum Identity Platform MVP.

The current executable scope is intentionally small:

- local username/password register, login, and password change
- Supabase identity exchange through a provider adapter
- `internal_user_id` binding
- platform JWT access tokens
- server-tracked refresh tokens and sessions

Read first:

- `Agent.md`
- `docs/MVP.md`
- `docs/BUILD.md`
- `docs/INTEGRATION.md`

Local checks:

```bash
cargo fmt
cargo check
cargo test
```

Do not commit generated files under `secrets/`.
