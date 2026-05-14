use std::{env, fs};

use thiserror::Error;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub http: HttpConfig,
    pub persistence: PersistenceConfig,
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
    pub frontend_direct: FrontendDirectConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontendDirectConfig {
    pub enabled: bool,
    pub allowed_origins: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistenceConfig {
    pub backend: PersistenceBackend,
    pub database_url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PersistenceBackend {
    Memory,
    Postgres,
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
                frontend_direct: parse_frontend_direct_config()?,
            },
            persistence: parse_persistence_config()?,
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

fn parse_frontend_direct_config() -> Result<FrontendDirectConfig, ConfigError> {
    let enabled = parse_bool_env("IDENTITY_FRONTEND_DIRECT_ENABLED", false)?;
    let allowed_origins = parse_csv_env("IDENTITY_FRONTEND_ALLOWED_ORIGINS");

    if enabled && allowed_origins.is_empty() {
        return Err(ConfigError::MissingRequired(
            "IDENTITY_FRONTEND_ALLOWED_ORIGINS",
        ));
    }

    if !enabled {
        return Ok(FrontendDirectConfig {
            enabled,
            allowed_origins,
        });
    }

    for origin in &allowed_origins {
        if origin.contains('*') {
            return Err(ConfigError::InvalidValue {
                name: "IDENTITY_FRONTEND_ALLOWED_ORIGINS",
                message: "wildcard origins are not allowed in frontend direct mode".to_owned(),
            });
        }
        if origin.chars().any(char::is_whitespace) {
            return Err(ConfigError::InvalidValue {
                name: "IDENTITY_FRONTEND_ALLOWED_ORIGINS",
                message: "origins must be comma-separated without whitespace".to_owned(),
            });
        }
        if !(origin.starts_with("http://") || origin.starts_with("https://")) {
            return Err(ConfigError::InvalidValue {
                name: "IDENTITY_FRONTEND_ALLOWED_ORIGINS",
                message: "origins must start with http:// or https://".to_owned(),
            });
        }
    }

    Ok(FrontendDirectConfig {
        enabled,
        allowed_origins,
    })
}

fn parse_persistence_config() -> Result<PersistenceConfig, ConfigError> {
    let backend_name = optional_env("IDENTITY_PERSISTENCE_BACKEND", "memory");
    let backend = match backend_name.as_str() {
        "memory" => PersistenceBackend::Memory,
        "postgres" => PersistenceBackend::Postgres,
        _ => {
            return Err(ConfigError::InvalidValue {
                name: "IDENTITY_PERSISTENCE_BACKEND",
                message: "must be one of: memory, postgres".to_owned(),
            });
        }
    };
    let database_url = env::var("IDENTITY_DATABASE_URL").ok();

    if backend == PersistenceBackend::Postgres && database_url.is_none() {
        return Err(ConfigError::MissingRequired("IDENTITY_DATABASE_URL"));
    }

    Ok(PersistenceConfig {
        backend,
        database_url,
    })
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

fn parse_csv_env(name: &str) -> Vec<String> {
    env::var(name)
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    fn persistence_defaults_to_memory() {
        with_env_lock(|| {
            unsafe {
                env::remove_var("IDENTITY_PERSISTENCE_BACKEND");
                env::remove_var("IDENTITY_DATABASE_URL");
            }

            let config = parse_persistence_config().unwrap();

            assert_eq!(config.backend, PersistenceBackend::Memory);
            assert_eq!(config.database_url, None);
        });
    }

    #[test]
    fn postgres_persistence_requires_database_url() {
        with_env_lock(|| {
            unsafe {
                env::set_var("IDENTITY_PERSISTENCE_BACKEND", "postgres");
                env::remove_var("IDENTITY_DATABASE_URL");
            }

            let result = parse_persistence_config();

            assert!(matches!(
                result,
                Err(ConfigError::MissingRequired("IDENTITY_DATABASE_URL"))
            ));
        });
    }

    #[test]
    fn invalid_persistence_backend_is_rejected() {
        with_env_lock(|| {
            unsafe {
                env::set_var("IDENTITY_PERSISTENCE_BACKEND", "sqlite");
                env::remove_var("IDENTITY_DATABASE_URL");
            }

            let result = parse_persistence_config();

            assert!(matches!(
                result,
                Err(ConfigError::InvalidValue {
                    name: "IDENTITY_PERSISTENCE_BACKEND",
                    ..
                })
            ));
        });
    }

    #[test]
    fn frontend_direct_mode_requires_explicit_origins() {
        with_env_lock(|| {
            unsafe {
                env::set_var("IDENTITY_FRONTEND_DIRECT_ENABLED", "true");
                env::remove_var("IDENTITY_FRONTEND_ALLOWED_ORIGINS");
            }

            let result = parse_frontend_direct_config();

            assert!(matches!(
                result,
                Err(ConfigError::MissingRequired(
                    "IDENTITY_FRONTEND_ALLOWED_ORIGINS"
                ))
            ));
        });
    }

    #[test]
    fn frontend_direct_mode_parses_allowed_origins() {
        with_env_lock(|| {
            unsafe {
                env::set_var("IDENTITY_FRONTEND_DIRECT_ENABLED", "true");
                env::set_var(
                    "IDENTITY_FRONTEND_ALLOWED_ORIGINS",
                    "http://localhost:5173,https://app.example.com",
                );
            }

            let config = parse_frontend_direct_config().unwrap();

            assert!(config.enabled);
            assert_eq!(
                config.allowed_origins,
                vec![
                    "http://localhost:5173".to_owned(),
                    "https://app.example.com".to_owned()
                ]
            );
        });
    }

    fn with_env_lock(run: impl FnOnce()) {
        let lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock.lock().unwrap();
        run();
        unsafe {
            env::remove_var("IDENTITY_PERSISTENCE_BACKEND");
            env::remove_var("IDENTITY_DATABASE_URL");
            env::remove_var("IDENTITY_FRONTEND_DIRECT_ENABLED");
            env::remove_var("IDENTITY_FRONTEND_ALLOWED_ORIGINS");
        }
    }
}
