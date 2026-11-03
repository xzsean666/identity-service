use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use http_body_util::BodyExt;
use identity_service::{
    api::{auth::ExchangeResponse, create_router},
    config::Config,
    db::{init_pool, run_migrations},
    providers::ProviderRegistry,
    services::{token::JwtEngine, TokenPair},
    AppState,
};
use tower::ServiceExt;
use uuid::Uuid;

async fn setup_app() -> axum::Router {
    let config = Config::from_env().unwrap();
    let pool = init_pool(&config.database_url).await.unwrap();
    run_migrations(&pool).await.unwrap();

    let jwt_engine = JwtEngine::new(&config).unwrap();
    let providers = ProviderRegistry::from_config(&config);
    let state = AppState::new(pool, jwt_engine, config, providers);

    create_router(state)
}

#[tokio::test]
async fn test_auth_exchange_flow() {
    let app = setup_app().await;

    let uid = format!("user_{}", Uuid::new_v4());
    let token = format!("mock:{}:exchange@test.com:Exchange User", uid);

    // 1. Successful exchange
    let req_body = serde_json::json!({
        "provider": "mock",
        "token": token
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/exchange")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let exchange_res: ExchangeResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(exchange_res.user.email, Some("exchange@test.com".to_string()));
    assert!(!exchange_res.tokens.access_token.is_empty());
    assert!(!exchange_res.tokens.refresh_token.is_empty());

    // 2. Refresh tokens
    let refresh_body = serde_json::json!({
        "refresh_token": exchange_res.tokens.refresh_token
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/refresh")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&refresh_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let refreshed_tokens: TokenPair = serde_json::from_slice(&body).unwrap();
    assert_ne!(refreshed_tokens.refresh_token, exchange_res.tokens.refresh_token);

    // 3. Old refresh token should be rejected (revoked by rotation)
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/refresh")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&refresh_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 4. Revoke active refresh token
    let revoke_body = serde_json::json!({
        "refresh_token": refreshed_tokens.refresh_token
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/revoke")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&revoke_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // 5. Refresh after revocation should fail
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/refresh")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&revoke_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_exchange_validation() {
    let app = setup_app().await;

    // Empty fields return 400 Bad Request
    let req_body = serde_json::json!({
        "provider": "",
        "token": "some_token"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/exchange")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Invalid provider token returns 401 Unauthorized
    let req_body = serde_json::json!({
        "provider": "mock",
        "token": "invalid"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/exchange")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
