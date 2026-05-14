use chrono::{Duration, Utc};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
    jwk::{Jwk, JwkSet, PublicKeyUse},
};
use uuid::Uuid;

use crate::{
    application::error::AppError,
    config::TokenConfig,
    domain::{session::Session, token::AccessTokenClaims},
};

#[derive(Clone)]
pub struct TokenService {
    config: TokenConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl TokenService {
    pub fn new(config: TokenConfig) -> Result<Self, AppError> {
        let encoding_key = EncodingKey::from_rsa_pem(config.private_key_pem.as_bytes())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let decoding_key = DecodingKey::from_rsa_pem(config.public_key_pem.as_bytes())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        Ok(Self {
            config,
            encoding_key,
            decoding_key,
        })
    }

    pub fn issue_access_token(&self, session: &Session) -> Result<(String, i64), AppError> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(self.config.access_token_lifetime_seconds);
        let claims = AccessTokenClaims {
            iss: self.config.issuer.clone(),
            sub: session.internal_user_id,
            aud: self.config.audience.clone(),
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
            jti: Uuid::new_v4(),
            sid: session.session_id,
            client_id: session.client_id.clone(),
        };
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.config.key_id.clone());
        let token = encode(&header, &claims, &self.encoding_key)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        Ok((token, self.config.access_token_lifetime_seconds))
    }

    pub fn verify_access_token(&self, access_token: &str) -> Result<AccessTokenClaims, AppError> {
        let header = decode_header(access_token).map_err(|_| AppError::TokenInvalid)?;
        if header.kid.as_deref() != Some(self.config.key_id.as_str()) {
            return Err(AppError::TokenInvalid);
        }

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[self.config.audience.clone()]);
        validation.set_issuer(&[self.config.issuer.clone()]);
        decode::<AccessTokenClaims>(access_token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|_| AppError::TokenInvalid)
    }

    pub fn generate_refresh_token_secret(&self) -> String {
        format!("{}.{}", Uuid::new_v4(), Uuid::new_v4())
    }

    pub fn public_jwks(&self) -> Result<JwkSet, AppError> {
        let mut jwk = Jwk::from_encoding_key(&self.encoding_key, Algorithm::RS256)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        jwk.common.key_id = Some(self.config.key_id.clone());
        jwk.common.public_key_use = Some(PublicKeyUse::Signature);
        Ok(JwkSet { keys: vec![jwk] })
    }
}
