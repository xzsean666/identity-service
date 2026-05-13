use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub iss: String,
    pub sub: Uuid,
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: Uuid,
    pub sid: Uuid,
    pub client_id: String,
}
