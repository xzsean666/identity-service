use axum::{
    body::Body,
    http::{header::AUTHORIZATION, header::CONTENT_TYPE, Request, StatusCode},
};
use http_body_util::BodyExt;
use identity_service::{
    api::{
        auth::ExchangeResponse,
        create_router,
        users::{CurrentUserResponse, LinkResponse},
    },
    config::Config,
    db::{init_pool, run_migrations},
    models::UserResponse,
    providers::ProviderRegistry,
    services::token::JwtEngine,
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
async fn test_user_endpoints_and_account_linking() {
    let app = setup_app().await;

    // 1. Authenticate via exchange to get tokens
    let uid = format!("user_{}", Uuid::new_v4());
    let token = format!("mock:{}:user_api@test.com:API User", uid);
    let exchange_req = serde_json::json!({
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
                .body(Body::from(serde_json::to_vec(&exchange_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let exchange_res: ExchangeResponse = serde_json::from_slice(&body).unwrap();
    let access_token = exchange_res.tokens.access_token;
    let user_id = exchange_res.user.id;

    // 2. GET /v1/users/me without token -> 401
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/users/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 3. GET /v1/users/me with valid Bearer token -> 200
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/users/me")
                .header(AUTHORIZATION, format!("Bearer {access_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let me: CurrentUserResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(me.user.id, user_id);
    assert_eq!(me.identities.len(), 1);

    // 4. PATCH /v1/users/me -> updates name & avatar
    let patch_body = serde_json::json!({
        "name": "Updated Name VIA PATCH",
        "avatar_url": "https://example.com/avatar_patch.png"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/users/me")
                .header(AUTHORIZATION, format!("Bearer {access_token}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&patch_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let updated: UserResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated.name, Some("Updated Name VIA PATCH".to_string()));
    assert_eq!(
        updated.avatar_url,
        Some("https://example.com/avatar_patch.png".to_string())
    );

    // 5. GET /v1/users/{id}
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/users/{user_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 6. POST /v1/users/link -> links a new identity
    let link_uid = format!("link_uid_{}", Uuid::new_v4());
    let link_token = format!("mock:{link_uid}:linked@test.com:Linked Provider");
    let link_body = serde_json::json!({
        "provider": "mock",
        "token": link_token
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/users/link")
                .header(AUTHORIZATION, format!("Bearer {access_token}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&link_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let link_res: LinkResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(link_res.identity.user_id, user_id);
    assert_eq!(link_res.identity.provider_user_id, link_uid);

    // 7. Verify /v1/users/me now lists 2 identities
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/users/me")
                .header(AUTHORIZATION, format!("Bearer {access_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let me: CurrentUserResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(me.identities.len(), 2);
}
