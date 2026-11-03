use crate::error::AppError;
use std::env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
    Test,
}

impl Environment {
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub env: Environment,
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub jwt_private_key_path: Option<String>,
    pub jwt_private_key_pem: Option<String>,
    pub jwt_key_id: String,
    pub jwt_access_token_ttl_seconds: u64,
    pub jwt_refresh_token_ttl_seconds: u64,
    pub google_client_id: Option<String>,
    pub x_client_id: Option<String>,
    pub supabase_url: Option<String>,
    pub supabase_jwt_secret: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let _ = dotenvy::dotenv();

        let env_str = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
        let env = match env_str.to_lowercase().as_str() {
            "production" | "prod" => Environment::Production,
            "test" => Environment::Test,
            _ => Environment::Development,
        };

        let host = env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("APP_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(8080);

        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/identity".to_string()
        });

        let jwt_issuer = env::var("JWT_ISSUER").unwrap_or_else(|_| "identity-service".to_string());
        let jwt_audience =
            env::var("JWT_AUDIENCE").unwrap_or_else(|_| "backend-services".to_string());

        let jwt_private_key_path = env::var("JWT_PRIVATE_KEY_PATH").ok();
        let jwt_private_key_pem = env::var("JWT_PRIVATE_KEY_PEM").ok();

        let jwt_key_id = env::var("JWT_KEY_ID").unwrap_or_else(|_| "key-1".to_string());

        let jwt_access_token_ttl_seconds = env::var("JWT_ACCESS_TOKEN_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(3600);

        let jwt_refresh_token_ttl_seconds = env::var("JWT_REFRESH_TOKEN_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30 * 24 * 3600);

        let google_client_id = env::var("GOOGLE_CLIENT_ID").ok();
        let x_client_id = env::var("X_CLIENT_ID").ok();
        let supabase_url = env::var("SUPABASE_URL").ok();
        let supabase_jwt_secret = env::var("SUPABASE_JWT_SECRET").ok();

        Ok(Self {
            env,
            host,
            port,
            database_url,
            jwt_issuer,
            jwt_audience,
            jwt_private_key_path,
            jwt_private_key_pem,
            jwt_key_id,
            jwt_access_token_ttl_seconds,
            jwt_refresh_token_ttl_seconds,
            google_client_id,
            x_client_id,
            supabase_url,
            supabase_jwt_secret,
        })
    }
}
