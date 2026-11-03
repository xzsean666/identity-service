# TASK-003: Ed25519 Key Engine, JWT Issuance & JWKS Endpoint

## Objective
Implement cryptographic key loading/generation for Ed25519 (EdDSA), JWT claims signing, token verification, and RFC 7517 JWKS serialization.

## Scope
- Implement `JwtEngine` capable of signing claims using Ed25519 private key.
- Support key configuration via PEM file path or raw PEM/base64 string, or auto-generating an ephemeral key in development mode.
- Export public key as JWKS JSON (`/.well-known/jwks.json`).
- Implement JWT token parser and validator with `kid`, expiration, issuer, audience checks.

## Allowed Files
- `src/services/token.rs`
- `src/api/jwks.rs`
- `tests/jwt_test.rs`

## Dependencies
- TASK-001

## Inputs and Outputs
- **Input**: User ID, claims parameters, private key
- **Output**: Signed JWT string, JWKS JSON response

## Acceptance Criteria
- Tokens signed with the private key verify successfully using the corresponding public key.
- Expired tokens or tokens with mismatched audience/issuer are rejected.
- JWKS conforms to RFC 7517.

## Verification Commands
- `cargo test --test jwt_test`

## Risks and Assumptions
- Use standard Ed25519 crates (`ed25519-dalek` or `jsonwebtoken` with ed25519 support).

## Status
DONE
