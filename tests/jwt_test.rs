use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use identity_service::{
    api::create_router,
    config::Config,
    db::{init_pool, run_migrations},
    providers::ProviderRegistry,
    services::token::{Claims, JwksResponse, JwtEngine},
    AppState,
};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_jwt_signing_and_verification() {
    let config = Config::from_env().unwrap();
    let jwt_engine = JwtEngine::new(&config).expect("JwtEngine should initialize");

    let user_id = Uuid::new_v4();
    let email = Some("alice@example.com".to_string());

    // 1. Sign token
    let token = jwt_engine
        .sign(user_id, email.clone())
        .expect("Signing should succeed");

    // 2. Verify token
    let claims = jwt_engine
        .verify(&token)
        .expect("Verification should succeed");
    assert_eq!(claims.sub, user_id.to_string());
    assert_eq!(claims.email, email);
    assert_eq!(claims.iss, config.jwt_issuer);
    assert_eq!(claims.aud, config.jwt_audience);

    // 3. Reject tampered token
    let tampered_token = format!("{}x", token);
    assert!(jwt_engine.verify(&tampered_token).is_err());

    // 4. Reject expired token
    let expired_claims = Claims {
        iss: config.jwt_issuer.clone(),
        sub: user_id.to_string(),
        aud: config.jwt_audience.clone(),
        iat: 1000,
        exp: 1010, // past epoch
        jti: Uuid::new_v4().to_string(),
        email: None,
    };
    let expired_token = jwt_engine.sign_claims(&expired_claims).unwrap();
    let err = jwt_engine.verify(&expired_token).unwrap_err();
    assert!(err.to_string().contains("expired"));

    // 5. Reject mismatched issuer
    let wrong_iss_claims = Claims {
        iss: "wrong-issuer".to_string(),
        sub: user_id.to_string(),
        aud: config.jwt_audience.clone(),
        iat: chrono::Utc::now().timestamp() as u64,
        exp: chrono::Utc::now().timestamp() as u64 + 3600,
        jti: Uuid::new_v4().to_string(),
        email: None,
    };
    let wrong_iss_token = jwt_engine.sign_claims(&wrong_iss_claims).unwrap();
    assert!(jwt_engine.verify(&wrong_iss_token).is_err());
}

#[tokio::test]
async fn test_jwks_endpoint() {
    let config = Config::from_env().unwrap();
    let pool = init_pool(&config.database_url).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let jwt_engine = JwtEngine::new(&config).unwrap();
    let providers = ProviderRegistry::from_config(&config);
    let state = AppState::new(pool, jwt_engine.clone(), config.clone(), providers);

    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/jwks.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let jwks: JwksResponse = serde_json::from_slice(&body).expect("JWKS response should deserialize");

    assert_eq!(jwks.keys.len(), 1);
    let key = &jwks.keys[0];
    assert_eq!(key.kty, "OKP");
    assert_eq!(key.crv, "Ed25519");
    assert_eq!(key.alg, "EdDSA");
    assert_eq!(key.key_use, "sig");
    assert_eq!(key.kid, config.jwt_key_id);
    assert!(!key.x.is_empty());
}
