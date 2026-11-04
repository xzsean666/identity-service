use crate::config::Config;
use crate::error::AppError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::pkcs8::DecodePrivateKey;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtHeader {
    pub alg: String,
    pub typ: String,
    pub kid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub iat: u64,
    pub exp: u64,
    pub jti: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwkKey {
    pub kty: String,
    pub crv: String,
    pub x: String,
    #[serde(rename = "use")]
    pub key_use: String,
    pub alg: String,
    pub kid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksResponse {
    pub keys: Vec<JwkKey>,
}

#[derive(Clone)]
pub struct JwtEngine {
    signing_key: Arc<SigningKey>,
    verifying_key: VerifyingKey,
    key_id: String,
    issuer: String,
    audience: String,
    access_token_ttl: u64,
}

impl JwtEngine {
    pub fn new(config: &Config) -> Result<Self, AppError> {
        let signing_key = if let Some(ref pem_str) = config.jwt_private_key_pem {
            Self::load_from_pem_str(pem_str)?
        } else if let Some(ref path) = config.jwt_private_key_path {
            let content = fs::read_to_string(path)
                .map_err(|e| AppError::Config(format!("Failed to read private key at {path}: {e}")))?;
            Self::load_from_pem_str(&content)?
        } else if config.env.is_production() {
            return Err(AppError::Config(
                "In production environment, JWT_PRIVATE_KEY_PEM or JWT_PRIVATE_KEY_PATH is strictly required. Ephemeral keys are forbidden in production to prevent key desynchronization across restarts and replicas.".to_string(),
            ));
        } else {
            tracing::info!("No Ed25519 private key configured; generating ephemeral key for development");
            SigningKey::generate(&mut OsRng)
        };

        let verifying_key = signing_key.verifying_key();

        Ok(Self {
            signing_key: Arc::new(signing_key),
            verifying_key,
            key_id: config.jwt_key_id.clone(),
            issuer: config.jwt_issuer.clone(),
            audience: config.jwt_audience.clone(),
            access_token_ttl: config.jwt_access_token_ttl_seconds,
        })
    }

    pub fn load_from_pem_str(pem_str: &str) -> Result<SigningKey, AppError> {
        let trimmed = pem_str.trim();
        // 1. Try standard PKCS#8 PEM decode
        if let Ok(key) = SigningKey::from_pkcs8_pem(trimmed) {
            return Ok(key);
        }

        // 2. Try raw base64 decode if it's a 32-byte seed
        if let Ok(decoded) = URL_SAFE_NO_PAD.decode(trimmed) {
            if let Ok(bytes) = decoded.as_slice().try_into() {
                return Ok(SigningKey::from_bytes(bytes));
            }
        }
        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(trimmed) {
            if let Ok(bytes) = decoded.as_slice().try_into() {
                return Ok(SigningKey::from_bytes(bytes));
            }
        }

        Err(AppError::Config("Invalid Ed25519 private key format (expected PKCS#8 PEM or 32-byte base64 seed)".into()))
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    pub fn sign(&self, user_id: Uuid, email: Option<String>) -> Result<String, AppError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let claims = Claims {
            iss: self.issuer.clone(),
            sub: user_id.to_string(),
            aud: self.audience.clone(),
            iat: now,
            exp: now + self.access_token_ttl,
            jti: Uuid::new_v4().to_string(),
            email,
        };

        self.sign_claims(&claims)
    }

    pub fn sign_claims(&self, claims: &Claims) -> Result<String, AppError> {
        let header = JwtHeader {
            alg: "EdDSA".to_string(),
            typ: "JWT".to_string(),
            kid: self.key_id.clone(),
        };

        let header_json = serde_json::to_vec(&header)
            .map_err(|e| AppError::Internal(format!("Failed to serialize JWT header: {e}")))?;
        let payload_json = serde_json::to_vec(claims)
            .map_err(|e| AppError::Internal(format!("Failed to serialize JWT claims: {e}")))?;

        let header_b64 = URL_SAFE_NO_PAD.encode(header_json);
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload_json);

        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let signature: Signature = self.signing_key.sign(signing_input.as_bytes());
        let signature_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

        Ok(format!("{}.{}", signing_input, signature_b64))
    }

    pub fn verify(&self, token: &str) -> Result<Claims, AppError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AppError::Crypto("Invalid JWT format (expected 3 parts)".to_string()));
        }

        let (header_b64, payload_b64, signature_b64) = (parts[0], parts[1], parts[2]);

        // Decode and verify header
        let header_bytes = URL_SAFE_NO_PAD
            .decode(header_b64)
            .map_err(|_| AppError::Crypto("Invalid base64 in JWT header".to_string()))?;
        let header: JwtHeader = serde_json::from_slice(&header_bytes)
            .map_err(|_| AppError::Crypto("Invalid JSON in JWT header".to_string()))?;

        if header.alg != "EdDSA" {
            return Err(AppError::Crypto(format!("Unsupported JWT algorithm: {}", header.alg)));
        }

        // Verify signature
        let signature_bytes = URL_SAFE_NO_PAD
            .decode(signature_b64)
            .map_err(|_| AppError::Crypto("Invalid base64 in JWT signature".to_string()))?;
        let signature_arr: [u8; 64] = signature_bytes
            .as_slice()
            .try_into()
            .map_err(|_| AppError::Crypto("Invalid Ed25519 signature length".to_string()))?;
        let signature = Signature::from_bytes(&signature_arr);

        let signing_input = format!("{}.{}", header_b64, payload_b64);
        self.verifying_key
            .verify(signing_input.as_bytes(), &signature)
            .map_err(|_| AppError::Crypto("JWT signature verification failed".to_string()))?;

        // Decode and verify claims
        let payload_bytes = URL_SAFE_NO_PAD
            .decode(payload_b64)
            .map_err(|_| AppError::Crypto("Invalid base64 in JWT payload".to_string()))?;
        let claims: Claims = serde_json::from_slice(&payload_bytes)
            .map_err(|_| AppError::Crypto("Invalid JSON in JWT claims".to_string()))?;

        if claims.iss != self.issuer {
            return Err(AppError::Crypto(format!(
                "JWT issuer mismatch: expected '{}', got '{}'",
                self.issuer, claims.iss
            )));
        }

        if claims.aud != self.audience {
            return Err(AppError::Crypto(format!(
                "JWT audience mismatch: expected '{}', got '{}'",
                self.audience, claims.aud
            )));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if claims.exp < now {
            return Err(AppError::Crypto("JWT token has expired".to_string()));
        }

        Ok(claims)
    }

    pub fn jwks(&self) -> JwksResponse {
        let pub_bytes = self.verifying_key.to_bytes();
        let pub_b64 = URL_SAFE_NO_PAD.encode(pub_bytes);

        JwksResponse {
            keys: vec![JwkKey {
                kty: "OKP".to_string(),
                crv: "Ed25519".to_string(),
                x: pub_b64,
                key_use: "sig".to_string(),
                alg: "EdDSA".to_string(),
                kid: self.key_id.clone(),
            }],
        }
    }
}
