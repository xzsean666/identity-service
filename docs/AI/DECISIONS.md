# Architectural Decision Records (ADRs)

## ADR-001: Asymmetric Ed25519 for JWT Signing
- **Context**: Backend services need to verify JWTs issued by identity-service without making network requests to identity-service on every API call.
- **Decision**: Use Ed25519 (EdDSA) asymmetric signatures. `identity-service` holds the private key; backend services retrieve public keys once via `/.well-known/jwks.json` and cache them.
- **Consequences**: Fast verification, tiny key size (32 bytes raw / standard RFC 7517 JWK format), strong cryptographic security, zero cross-service RPC overhead on verification.

## ADR-002: Modular External Provider Verification
- **Context**: Identity service supports multiple providers (Google, X, Supabase, with more to follow).
- **Decision**: Define an async `ProviderVerifier` trait. Each provider implements token verification, validating issuer, audience, and signature with the external provider, extracting a canonical `ExternalUserIdentity(provider, provider_user_id, email, name, avatar_url)`.
- **Consequences**: Adding new providers requires zero modification to core identity resolution logic.

## ADR-003: Account Mapping and Identity Binding
- **Context**: Users might sign in via different providers.
- **Decision**: A unique constraint is placed on `(provider, provider_user_id)`. The lookup checks `identities` table first. If not found, a new `users` record is created atomically with the `identities` record in a database transaction.
- **Consequences**: Guaranteed consistency, no duplicate user creation under concurrent calls.

## ADR-004: Refresh Token Storage with Hashing
- **Context**: Need secure refresh token issuance and revocation without storing raw tokens.
- **Decision**: Issue high-entropy cryptographically random refresh tokens to clients, store SHA-256 hashes of refresh tokens in the database, with explicit revocation and expiry checking.
- **Consequences**: Protection against database read leakages; simple and fast revocation.
