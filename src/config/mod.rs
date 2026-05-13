use std::{env, fs};

use thiserror::Error;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub http: HttpConfig,
    pub identity_providers: IdentityProviderConfig,
    pub client: ClientConfig,
    pub tokens: TokenConfig,
    pub sessions: SessionConfig,
    pub security: SecurityConfig,
}

#[derive(Clone, Debug)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug)]
pub struct IdentityProviderConfig {
    pub local_password: ProviderToggle,
    pub supabase: SupabaseProviderConfig,
}

#[derive(Clone, Debug)]
pub struct ProviderToggle {
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct SupabaseProviderConfig {
    pub enabled: bool,
    pub auto_provision_enabled: bool,
    pub project_url: String,
    pub issuer: String,
    pub audience: String,
    pub jwks_url: String,
    pub jwks_json: Option<String>,
    pub fixture_tokens_enabled: bool,
}

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub client_id: String,
    pub trusted_origin: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TokenConfig {
    pub issuer: String,
    pub audience: String,
    pub access_token_lifetime_seconds: i64,
    pub key_id: String,
    pub private_key_pem: String,
    pub public_key_pem: String,
}

#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub refresh_token_lifetime_seconds: i64,
    pub session_lifetime_seconds: i64,
}

#[derive(Clone, Debug)]
pub struct SecurityConfig {
    pub refresh_token_hmac_secret: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable {0}")]
    MissingRequired(&'static str),
    #[error("invalid environment variable {name}: {message}")]
    InvalidValue { name: &'static str, message: String },
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let private_key_pem = pem_from_env_or_path(
            "IDENTITY_TOKEN_PRIVATE_KEY_PEM",
            "IDENTITY_TOKEN_PRIVATE_KEY_PEM_PATH",
            "./secrets/jwt_private.pem",
        )?;
        let public_key_pem = pem_from_env_or_path(
            "IDENTITY_TOKEN_PUBLIC_KEY_PEM",
            "IDENTITY_TOKEN_PUBLIC_KEY_PEM_PATH",
            "./secrets/jwt_public.pem",
        )?;
        let refresh_token_hmac_secret = required_env("IDENTITY_REFRESH_TOKEN_HMAC_SECRET")?;

        Ok(Self {
            http: HttpConfig {
                host: optional_env("IDENTITY_HTTP_HOST", "127.0.0.1"),
                port: parse_u16_env("IDENTITY_HTTP_PORT", 3000)?,
            },
            identity_providers: IdentityProviderConfig {
                local_password: ProviderToggle {
                    enabled: parse_bool_env("IDENTITY_PROVIDER_LOCAL_PASSWORD_ENABLED", true)?,
                },
                supabase: {
                    let issuer = optional_env(
                        "IDENTITY_PROVIDER_SUPABASE_ISSUER",
                        "https://example.supabase.co/auth/v1",
                    );
                    SupabaseProviderConfig {
                        enabled: parse_bool_env("IDENTITY_PROVIDER_SUPABASE_ENABLED", true)?,
                        auto_provision_enabled: parse_bool_env(
                            "IDENTITY_PROVIDER_SUPABASE_AUTO_PROVISION_ENABLED",
                            true,
                        )?,
                        project_url: optional_env(
                            "IDENTITY_PROVIDER_SUPABASE_PROJECT_URL",
                            "https://example.supabase.co",
                        ),
                        jwks_url: optional_env(
                            "IDENTITY_PROVIDER_SUPABASE_JWKS_URL",
                            &format!("{issuer}/.well-known/jwks.json"),
                        ),
                        jwks_json: env::var("IDENTITY_PROVIDER_SUPABASE_JWKS_JSON").ok(),
                        fixture_tokens_enabled: parse_bool_env(
                            "IDENTITY_PROVIDER_SUPABASE_FIXTURE_TOKENS_ENABLED",
                            false,
                        )?,
                        issuer,
                        audience: optional_env(
                            "IDENTITY_PROVIDER_SUPABASE_AUDIENCE",
                            "authenticated",
                        ),
                    }
                },
            },
            client: ClientConfig {
                client_id: optional_env("IDENTITY_CLIENT_ID", "identity-service-mvp"),
                trusted_origin: env::var("IDENTITY_CLIENT_TRUSTED_ORIGIN").ok(),
            },
            tokens: TokenConfig {
                issuer: optional_env("IDENTITY_TOKEN_ISSUER", "identity-service"),
                audience: optional_env("IDENTITY_TOKEN_AUDIENCE", "platform-api"),
                access_token_lifetime_seconds: parse_i64_env(
                    "IDENTITY_ACCESS_TOKEN_LIFETIME_SECONDS",
                    900,
                )?,
                key_id: optional_env("IDENTITY_TOKEN_KEY_ID", "mvp-local-key"),
                private_key_pem,
                public_key_pem,
            },
            sessions: SessionConfig {
                refresh_token_lifetime_seconds: parse_i64_env(
                    "IDENTITY_REFRESH_TOKEN_LIFETIME_SECONDS",
                    2_592_000,
                )?,
                session_lifetime_seconds: parse_i64_env(
                    "IDENTITY_SESSION_LIFETIME_SECONDS",
                    2_592_000,
                )?,
            },
            security: SecurityConfig {
                refresh_token_hmac_secret,
            },
        })
    }
}

fn required_env(name: &'static str) -> Result<String, ConfigError> {
    env::var(name).map_err(|_| ConfigError::MissingRequired(name))
}

fn pem_from_env_or_path(
    value_name: &'static str,
    path_name: &'static str,
    default_path: &'static str,
) -> Result<String, ConfigError> {
    if let Ok(value) = env::var(value_name) {
        return Ok(value);
    }

    let path = env::var(path_name).unwrap_or_else(|_| default_path.to_owned());
    fs::read_to_string(&path).map_err(|error| ConfigError::InvalidValue {
        name: path_name,
        message: format!("failed to read PEM file at {path}: {error}"),
    })
}

fn optional_env(name: &str, default_value: &str) -> String {
    env::var(name).unwrap_or_else(|_| default_value.to_owned())
}

fn parse_bool_env(name: &'static str, default_value: bool) -> Result<bool, ConfigError> {
    match env::var(name) {
        Ok(value) => value
            .parse::<bool>()
            .map_err(|error| ConfigError::InvalidValue {
                name,
                message: error.to_string(),
            }),
        Err(_) => Ok(default_value),
    }
}

fn parse_i64_env(name: &'static str, default_value: i64) -> Result<i64, ConfigError> {
    match env::var(name) {
        Ok(value) => value
            .parse::<i64>()
            .map_err(|error| ConfigError::InvalidValue {
                name,
                message: error.to_string(),
            }),
        Err(_) => Ok(default_value),
    }
}

fn parse_u16_env(name: &'static str, default_value: u16) -> Result<u16, ConfigError> {
    match env::var(name) {
        Ok(value) => value
            .parse::<u16>()
            .map_err(|error| ConfigError::InvalidValue {
                name,
                message: error.to_string(),
            }),
        Err(_) => Ok(default_value),
    }
}
