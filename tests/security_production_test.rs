use identity_service::{
    config::{Config, Environment},
    db::{cleanup_expired_refresh_tokens, init_pool, run_migrations},
    providers::{supabase::SupabaseProvider, ProviderRegistry, ProviderVerifier},
    services::token::JwtEngine,
};

#[tokio::test]
async fn test_production_jwt_key_enforcement() {
    let mut config = Config::from_env().unwrap();
    config.env = Environment::Production;
    config.jwt_private_key_pem = None;
    config.jwt_private_key_path = None;

    // In production, ephemeral keys must be rejected
    match JwtEngine::new(&config) {
        Ok(_) => panic!("Should have failed in production without persistent key"),
        Err(err) => {
            assert!(err.to_string().contains("JWT_PRIVATE_KEY_PEM or JWT_PRIVATE_KEY_PATH is strictly required"));
        }
    }
}

#[tokio::test]
async fn test_production_mock_provider_blocked() {
    let mut config = Config::from_env().unwrap();
    config.env = Environment::Production;
    config.enable_mock_provider = false;

    let registry = ProviderRegistry::from_config(&config);
    let verify_result = registry.verify("mock", "admin").await;
    assert!(verify_result.is_err());
    let err_msg = verify_result.unwrap_err().to_string();
    assert!(err_msg.contains("Unsupported authentication provider: 'mock'"));
}

#[tokio::test]
async fn test_production_supabase_credentials_enforced() {
    let provider = SupabaseProvider::new(None, None, true);
    let result = provider.verify("fake.jwt.token").await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Supabase provider requires SUPABASE_URL or SUPABASE_JWT_SECRET in production"));
}

#[tokio::test]
async fn test_cleanup_expired_tokens() {
    let config = Config::from_env().unwrap();
    let pool = init_pool(&config.database_url).await.unwrap();
    run_migrations(&pool).await.unwrap();

    let cleaned = cleanup_expired_refresh_tokens(&pool).await;
    assert!(cleaned.is_ok());
}
