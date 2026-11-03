use axum::{
    body::Body,
    http::{header::AUTHORIZATION, header::CONTENT_TYPE, Request, StatusCode},
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
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
    services::{
        token::{Claims, JwksResponse, JwtEngine},
        TokenPair,
    },
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
async fn test_full_system_lifecycle_and_offline_verification() {
    let app = setup_app().await;

    // 1. Verify OpenAPI JSON endpoint
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let openapi_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let openapi_json: serde_json::Value = serde_json::from_slice(&openapi_bytes).unwrap();
    assert_eq!(openapi_json["openapi"], "3.0.3");

    // 2. Client logs in: exchange external token (Mock provider)
    let unique_user_id = format!("full_flow_{}", Uuid::new_v4());
    let exchange_payload = serde_json::json!({
        "provider": "mock",
        "token": format!("mock:{}:full_flow@example.com:Full Flow User", unique_user_id)
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/exchange")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&exchange_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let exchange_res: ExchangeResponse = serde_json::from_slice(&body).unwrap();
    let access_token = exchange_res.tokens.access_token.clone();
    let refresh_token = exchange_res.tokens.refresh_token.clone();
    let user_id = exchange_res.user.id;

    // 3. Downstream service offline JWT verification via JWKS:
    // A downstream API fetches the JWKS from identity-service:
    let jwks_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/.well-known/jwks.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(jwks_res.status(), StatusCode::OK);
    let jwks_body = jwks_res.into_body().collect().await.unwrap().to_bytes();
    let jwks: JwksResponse = serde_json::from_slice(&jwks_body).unwrap();
    let jwk = &jwks.keys[0];

    // Downstream decodes JWK public key bytes
    let pub_bytes = URL_SAFE_NO_PAD.decode(&jwk.x).unwrap();
    let pub_arr: [u8; 32] = pub_bytes.try_into().unwrap();
    let downstream_verifier = VerifyingKey::from_bytes(&pub_arr).unwrap();

    // Downstream decodes and verifies the access token offline
    let token_parts: Vec<&str> = access_token.split('.').collect();
    assert_eq!(token_parts.len(), 3);
    let signing_input = format!("{}.{}", token_parts[0], token_parts[1]);
    let sig_bytes = URL_SAFE_NO_PAD.decode(token_parts[2]).unwrap();
    let sig_arr: [u8; 64] = sig_bytes.try_into().unwrap();
    let signature = Signature::from_bytes(&sig_arr);

    // Cryptographic signature check by downstream succeeds without contacting identity-service!
    downstream_verifier
        .verify(signing_input.as_bytes(), &signature)
        .expect("Downstream offline cryptographic verification must succeed");

    let payload_bytes = URL_SAFE_NO_PAD.decode(token_parts[1]).unwrap();
    let claims: Claims = serde_json::from_slice(&payload_bytes).unwrap();
    assert_eq!(claims.sub, user_id.to_string());
    assert_eq!(claims.email, Some("full_flow@example.com".to_string()));

    // 4. Authenticated request: GET /v1/users/me
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
    let me_body = res.into_body().collect().await.unwrap().to_bytes();
    let me_res: CurrentUserResponse = serde_json::from_slice(&me_body).unwrap();
    assert_eq!(me_res.user.id, user_id);
    assert_eq!(me_res.identities.len(), 1);

    // 5. Update user profile: PATCH /v1/users/me
    let update_payload = serde_json::json!({
        "name": "Updated Flow Name",
        "avatar_url": "https://example.com/avatar_flow.png"
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/v1/users/me")
                .header(AUTHORIZATION, format!("Bearer {access_token}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&update_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let update_body = res.into_body().collect().await.unwrap().to_bytes();
    let updated_user: UserResponse = serde_json::from_slice(&update_body).unwrap();
    assert_eq!(updated_user.name, Some("Updated Flow Name".to_string()));

    // 6. Link secondary identity: POST /v1/users/link
    let second_provider_uid = format!("second_{}", Uuid::new_v4());
    let link_payload = serde_json::json!({
        "provider": "mock",
        "token": format!("mock:{}:linked_second@example.com:Secondary Account", second_provider_uid)
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/users/link")
                .header(AUTHORIZATION, format!("Bearer {access_token}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&link_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let link_body = res.into_body().collect().await.unwrap().to_bytes();
    let link_res: LinkResponse = serde_json::from_slice(&link_body).unwrap();
    assert_eq!(link_res.identity.user_id, user_id);

    // 7. Verify user profile by ID: GET /v1/users/{id}
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

    // 8. Token refresh flow: POST /v1/auth/refresh
    let refresh_payload = serde_json::json!({
        "refresh_token": refresh_token
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/refresh")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&refresh_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let refresh_body = res.into_body().collect().await.unwrap().to_bytes();
    let new_tokens: TokenPair = serde_json::from_slice(&refresh_body).unwrap();
    assert_ne!(new_tokens.refresh_token, refresh_token);

    // 9. Revoke session: POST /v1/auth/revoke
    let revoke_payload = serde_json::json!({
        "refresh_token": new_tokens.refresh_token
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/revoke")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&revoke_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 10. Attempt refresh with revoked token should fail
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/refresh")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&revoke_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
