use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::application::error::AppError;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct RefreshTokenHasher {
    secret: Vec<u8>,
}

impl RefreshTokenHasher {
    pub fn new(secret: String) -> Self {
        Self {
            secret: secret.into_bytes(),
        }
    }

    pub fn hash(&self, refresh_token: &str) -> Result<String, AppError> {
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        mac.update(refresh_token.as_bytes());
        Ok(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
    }

    pub fn verify(&self, refresh_token: &str, stored_hash: &str) -> Result<bool, AppError> {
        let computed_hash = self.hash(refresh_token)?;
        Ok(computed_hash
            .as_bytes()
            .ct_eq(stored_hash.as_bytes())
            .into())
    }
}
