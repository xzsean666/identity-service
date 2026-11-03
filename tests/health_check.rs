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
    services::token::JwtEngine,
    AppState,
};
use tower::ServiceExt;

#[tokio::test]
async fn test_health_check_endpoint() {
    let config = Config::from_env().unwrap();
    let pool = init_pool(&config.database_url).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let jwt_engine = JwtEngine::new(&config).unwrap();
    let providers = ProviderRegistry::from_config(&config);
    let state = AppState::new(pool, jwt_engine, config, providers);

    let app = create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body_json["status"], "ok");
    assert_eq!(body_json["version"], env!("CARGO_PKG_VERSION"));
    assert!(body_json["timestamp"].is_number());
}
