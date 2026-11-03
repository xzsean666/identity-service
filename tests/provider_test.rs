use identity_service::{
    config::Config,
    models::ExternalIdentity,
    providers::{
        mock::MockProvider, supabase::SupabaseProvider, ProviderRegistry, ProviderVerifier,
    },
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

#[tokio::test]
async fn test_mock_provider() {
    let mock = MockProvider::new();

    // 1. Mock format string
    let ident = mock
        .verify("mock:alice_123:alice@example.com:Alice Smith")
        .await
        .expect("Verification should succeed");
    assert_eq!(ident.provider, "mock");
    assert_eq!(ident.provider_user_id, "alice_123");
    assert_eq!(ident.email, Some("alice@example.com".to_string()));
    assert_eq!(ident.name, Some("Alice Smith".to_string()));

    // 2. Mock JSON token
    let json_token = r#"{"sub":"user_999","email":"nine@example.com","name":"Nine"}"#;
    let ident_json = mock
        .verify(json_token)
        .await
        .expect("JSON token verification should succeed");
    assert_eq!(ident_json.provider_user_id, "user_999");
    assert_eq!(ident_json.email, Some("nine@example.com".to_string()));

    // 3. Registered custom token
    mock.register(
        "secret_token_123",
        ExternalIdentity {
            provider: "mock".to_string(),
            provider_user_id: "custom_user_1".to_string(),
            email: Some("custom@test.com".to_string()),
            name: Some("Custom User".to_string()),
            avatar_url: None,
        },
    );
    let ident_custom = mock.verify("secret_token_123").await.unwrap();
    assert_eq!(ident_custom.provider_user_id, "custom_user_1");

    // 4. Invalid token
    assert!(mock.verify("invalid").await.is_err());
    assert!(mock.verify("reject").await.is_err());
}

#[tokio::test]
async fn test_provider_registry() {
    let config = Config::from_env().unwrap();
    let registry = ProviderRegistry::from_config(&config);

    // Verify mock via registry
    let ident = registry
        .verify("mock", "mock:reg_user:reg@test.com:Registry User")
        .await
        .expect("Registry should dispatch to mock provider");
    assert_eq!(ident.provider_user_id, "reg_user");

    // Unknown provider should return error
    let unknown_err = registry.verify("unknown_provider", "token").await;
    assert!(unknown_err.is_err());
}

#[tokio::test]
async fn test_supabase_jwt_fallback() {
    let provider = SupabaseProvider::new(None, None);

    // Create a mock Supabase JWT payload
    let payload = serde_json::json!({
        "sub": "supabase_user_123",
        "email": "supabase@example.com",
        "user_metadata": {
            "full_name": "Supabase User",
            "avatar_url": "https://example.com/sb.png"
        }
    });

    let header_b64 = URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload.to_string());
    let token = format!("{}.{}.fakesignature", header_b64, payload_b64);

    let ident = provider
        .verify(&token)
        .await
        .expect("Supabase offline JWT parsing should succeed");

    assert_eq!(ident.provider, "supabase");
    assert_eq!(ident.provider_user_id, "supabase_user_123");
    assert_eq!(ident.email, Some("supabase@example.com".to_string()));
    assert_eq!(ident.name, Some("Supabase User".to_string()));
    assert_eq!(ident.avatar_url, Some("https://example.com/sb.png".to_string()));
}
