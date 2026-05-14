use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use identity_service::{
    application::bootstrap::build_auth_service, config::AppConfig, interfaces::http::router,
};
use serde_json::{Value, json};
use std::sync::Once;
use tower::util::ServiceExt;

const TEST_PRIVATE_KEY_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQC7hWn6j1V5utRG
RgkDGEiyIyYmtk7T5UbKtVbuVKR0x46R/OwjmY+iNlQ6JotztmFsurEjRmcKHw2M
6hNF3S7xf9yeeH1yzSx8xt9YC9FXyYZJGspnanxB2rDGxePAliWjDSI/zPRtNuqp
ItxRNF3GLJy/sEJMnMxGCxau8UND0t0PZWhNmHUZCMSWWxccwPC/js8yqKnn+HgF
fcGMfrR5cXqCfsrwZoetlsvTJ0kD8uiMuSyZJiSjidRNw47gck3N9752nHCAztzU
/pKcKtBmt+VWMmj8gjuergMf7mNpxQ5g2ynDNlIy3UUw2fIEg0t4sMYz3JsQvSjo
9Udk4YpJAgMBAAECggEAWN1pM0HQxm7I4QKYi7xq2ux4THkx0w4A5dhY+XnoM6VM
RZfACkDgBgXMFYClrnDcK4wvnOFkvDGqGMDm4EFo2S54TSsZfBmKPxl5xz5Wd007
05IcIDUg7I5oHtKx01b0QBhdxjFpFgaj3wJzuRHhbKRApkCvsqHN1lWz6rTP5SgN
S4jURWS6jlfNYZgSv4cye5eoxJ7iBj2bDoMcR8lFwqXjs7/3TuIFfbMaiangh4g2
KoeC1L31uu2hx5MenoLYG+tiSSfooe/myTdqNsBhD8pRDyd392r0/NCFSFL+bumV
DRlIy5qADpkKbCGqUwUDEUncndm/TDdsn+nPdXvCZwKBgQDbU0CMIjVPRK5A5Oax
2t/drrcS6lhu3XaHsd6SIuexprO5+dqxGWuHmO7uYlHdC/ItZnZwvGKrHo3eCoYf
Z9vVhLeP0kz0Pc9DaTYyIz3rCWDKCb1VNgtduYPfEsgyHObyeF0m66wke7IEvEyR
6CYJ0Y2xA9kc8W394FVnrlYaqwKBgQDa4LZH2vqhOjXxiYoWc5I7c/gbI1XG3ckK
6annC73H0WV7P3oMyNIfDiKJQU1rIgOo48TUqawl2iMpszLEhvSKIB19o+i7klW5
6b9F9UZuNsG/sE5Ts/n17cplZKPNV0UWGHRE3hUDsnhCfnN7KosPk1amfUyNIOot
zfpKOUou2wKBgQCKudzxBk4r5mhFucNFqgjBolpAB6SJ82Cesd3zF0rv7l5t+uDd
9hMywIQYmm3nYD/9gXrXEgFi9T+Mu6FcSggdxQWKXd24+0OXAvx5uBrZCKSFBqYQ
OM/1p3sG5U1ljSxzH7jj/lty9B6EqknQXEN7IGX8GlAA46DL3VKH8xiZYwKBgQCo
KCgiwFv5bi1vagnLAfOA9bHRt6344P/KAIbl2SFu2LMsozHzjH3SGhvyc1c1Taae
JI9eCxUU56hIK0J/tmc9jzrZAgqVwPFXqfunla8Mkcj8qkkjCYyqoovypgUqhzeu
qA77sdtXQdAe1eOG5sJ7rujNdEpRys3fbvYx/B3ALQKBgQCbfpTAkpmeth3xh5Jy
2NuXslpwlIN+ge9RSHouuI65WN/biymDxW/37lIFLdZZizQq0UjKxCf+WukXsfpi
PnVmzKSSGo/qNGfVJziAQlAB45zVWfUM0bynUu6gqa38IPZP8uxghhtXE4bGZlBA
W6sEEB4p+zZk7qQgK8uIDIjYww==
-----END PRIVATE KEY-----"#;

const TEST_PUBLIC_KEY_PEM: &str = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAu4Vp+o9VebrURkYJAxhI
siMmJrZO0+VGyrVW7lSkdMeOkfzsI5mPojZUOiaLc7ZhbLqxI0ZnCh8NjOoTRd0u
8X/cnnh9cs0sfMbfWAvRV8mGSRrKZ2p8QdqwxsXjwJYlow0iP8z0bTbqqSLcUTRd
xiycv7BCTJzMRgsWrvFDQ9LdD2VoTZh1GQjEllsXHMDwv47PMqip5/h4BX3BjH60
eXF6gn7K8GaHrZbL0ydJA/LojLksmSYko4nUTcOO4HJNzfe+dpxwgM7c1P6SnCrQ
ZrflVjJo/II7nq4DH+5jacUOYNspwzZSMt1FMNnyBINLeLDGM9ybEL0o6PVHZOGK
SQIDAQAB
-----END PUBLIC KEY-----"#;

#[tokio::test]
async fn register_then_me_returns_registered_user() {
    let mut app = test_app(true).await;

    let register = post_json(
        &mut app,
        "/v1/auth/register",
        json!({ "username": "alice@example.test", "password": "correct horse battery staple" }),
    )
    .await;

    assert_eq!(register.status, StatusCode::OK);
    let user_id = register.body["user"]["internal_user_id"]
        .as_str()
        .expect("register response should include user id")
        .to_owned();
    let access_token = token(&register.body, "access_token");

    let me = get_with_bearer(&mut app, "/v1/users/me", &access_token).await;

    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["internal_user_id"], user_id);
    assert_eq!(me.body["account_status"], "Active");
}

#[tokio::test]
async fn login_refresh_then_logout_revokes_current_session() {
    let mut app = test_app(true).await;

    let credentials =
        json!({ "username": "bob@example.test", "password": "correct horse battery staple" });
    let register = post_json(&mut app, "/v1/auth/register", credentials.clone()).await;
    assert_eq!(register.status, StatusCode::OK);

    let login = post_json(&mut app, "/v1/auth/login", credentials).await;
    assert_eq!(login.status, StatusCode::OK);
    let user_id = login.body["user"]["internal_user_id"]
        .as_str()
        .expect("login response should include user id")
        .to_owned();
    let access_token = token(&login.body, "access_token");
    let refresh_token = token(&login.body, "refresh_token");

    let refresh = post_json(
        &mut app,
        "/v1/auth/refresh",
        json!({ "refresh_token": refresh_token }),
    )
    .await;
    assert_eq!(refresh.status, StatusCode::OK);
    let refreshed_access_token = token(&refresh.body, "access_token");
    assert_ne!(refreshed_access_token, access_token);

    let me_before_logout = get_with_bearer(&mut app, "/v1/users/me", &refreshed_access_token).await;
    assert_eq!(me_before_logout.status, StatusCode::OK);
    assert_eq!(me_before_logout.body["internal_user_id"], user_id);

    let logout = post_with_bearer(&mut app, "/v1/auth/logout", &refreshed_access_token).await;
    assert_eq!(logout.status, StatusCode::OK);
    assert_eq!(logout.body["revoked"], true);

    let me_after_logout = get_with_bearer(&mut app, "/v1/users/me", &refreshed_access_token).await;
    assert_eq!(me_after_logout.status, StatusCode::UNAUTHORIZED);
    assert_eq!(me_after_logout.body["error_code"], "unauthorized");

    let original_access_after_logout =
        get_with_bearer(&mut app, "/v1/users/me", &access_token).await;
    assert_eq!(
        original_access_after_logout.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        original_access_after_logout.body["error_code"],
        "unauthorized"
    );
}

#[tokio::test]
async fn disabled_local_provider_returns_provider_disabled() {
    let mut app = test_app(false).await;

    let register = post_json(
        &mut app,
        "/v1/auth/register",
        json!({ "username": "carol@example.test", "password": "correct horse battery staple" }),
    )
    .await;
    assert_provider_disabled(register);

    let login = post_json(
        &mut app,
        "/v1/auth/login",
        json!({ "username": "carol@example.test", "password": "correct horse battery staple" }),
    )
    .await;
    assert_provider_disabled(login);
}

async fn test_app(local_password_enabled: bool) -> axum::Router {
    router(
        build_auth_service(test_config(local_password_enabled))
            .await
            .expect("test config is valid"),
    )
}

fn test_config(local_password_enabled: bool) -> AppConfig {
    static ENV: Once = Once::new();
    ENV.call_once(|| {
        // These tests run in one integration-test process and use the same values in every case.
        unsafe {
            std::env::set_var("IDENTITY_TOKEN_PRIVATE_KEY_PEM", TEST_PRIVATE_KEY_PEM);
            std::env::set_var("IDENTITY_TOKEN_PUBLIC_KEY_PEM", TEST_PUBLIC_KEY_PEM);
            std::env::set_var(
                "IDENTITY_REFRESH_TOKEN_HMAC_SECRET",
                "http-test-refresh-token-hmac-secret",
            );
            std::env::set_var("IDENTITY_TOKEN_ISSUER", "identity-service-http-test");
            std::env::set_var("IDENTITY_TOKEN_AUDIENCE", "platform-api-http-test");
            std::env::set_var("IDENTITY_TOKEN_KEY_ID", "http-test-key");
            std::env::set_var("IDENTITY_CLIENT_ID", "identity-service-http-test");
            std::env::set_var("IDENTITY_PERSISTENCE_BACKEND", "memory");
            std::env::remove_var("IDENTITY_DATABASE_URL");
            std::env::set_var("IDENTITY_PROVIDER_SUPABASE_ENABLED", "false");
            std::env::set_var("IDENTITY_PROVIDER_SUPABASE_AUTO_PROVISION_ENABLED", "false");
        }
    });

    let mut config = AppConfig::from_env().expect("test environment config is valid");
    config.identity_providers.local_password.enabled = local_password_enabled;
    config.identity_providers.supabase.enabled = false;
    config
}

async fn post_json(app: &mut axum::Router, uri: &str, body: Value) -> TestResponse {
    let request = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should build");

    call(app, request).await
}

async fn post_with_bearer(app: &mut axum::Router, uri: &str, bearer_token: &str) -> TestResponse {
    let request = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {bearer_token}"))
        .body(Body::empty())
        .expect("request should build");

    call(app, request).await
}

async fn get_with_bearer(app: &mut axum::Router, uri: &str, bearer_token: &str) -> TestResponse {
    let request = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {bearer_token}"))
        .body(Body::empty())
        .expect("request should build");

    call(app, request).await
}

async fn call(app: &mut axum::Router, request: Request<Body>) -> TestResponse {
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("router should respond");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body = serde_json::from_slice(&bytes).expect("response body should be JSON");

    TestResponse { status, body }
}

fn token(body: &Value, field: &str) -> String {
    body["tokens"][field]
        .as_str()
        .unwrap_or_else(|| panic!("response should include tokens.{field}"))
        .to_owned()
}

fn assert_provider_disabled(response: TestResponse) {
    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert_eq!(response.body["error_code"], "provider_disabled");
    assert_eq!(response.body["retryable"], false);
}

struct TestResponse {
    status: StatusCode,
    body: Value,
}
