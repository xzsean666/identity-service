use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use identity_service::{
    application::bootstrap::build_application_services,
    config::{
        AppConfig, ClientConfig, FrontendDirectConfig, HttpConfig, IdentityProviderConfig,
        PersistenceBackend, PersistenceConfig, ProviderToggle, SecurityConfig, SessionConfig,
        SupabaseProviderConfig, TokenConfig,
    },
    infrastructure::{postgres, sqlite},
    interfaces::http::router,
};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::Serialize;
use serde_json::{Value, json};
use std::fs;
use tower::util::ServiceExt;
use uuid::Uuid;

const TEST_PRIVATE_KEY_PEM: &str = include_str!("fixtures/jwt_private.pem");
const TEST_PUBLIC_KEY_PEM: &str = include_str!("fixtures/jwt_public.pem");
const SUPABASE_SECRET: &[u8] = b"supabase-e2e-secret";
const SUPABASE_KEY_ID: &str = "supabase-e2e-key";
const SUPABASE_ISSUER: &str = "https://example.supabase.co/auth/v1";
const SUPABASE_AUDIENCE: &str = "authenticated";

#[tokio::test]
async fn e2e_full_identity_platform_flow_with_memory_backend() {
    let config = test_config(TestBackend::Memory, true, true);
    let mut app = test_app(config).await;

    run_full_identity_platform_flow(&mut app, "memory", "memory").await;
}

#[tokio::test]
async fn e2e_full_identity_platform_flow_with_sqlite_backend() {
    let database_url = sqlite_database_url("e2e");
    sqlite::run_pending_migrations(&database_url)
        .await
        .expect("SQLite E2E tests should run migrations");

    let config = test_config(TestBackend::Sqlite(database_url), true, true);
    let mut app = test_app(config).await;

    run_full_identity_platform_flow(&mut app, "sqlite", "sqlite").await;
}

#[tokio::test]
async fn e2e_full_identity_platform_flow_with_postgres_backend_when_configured() {
    let Some(database_url) = std::env::var("IDENTITY_DATABASE_URL").ok() else {
        return;
    };
    postgres::run_pending_migrations(&database_url)
        .await
        .expect("PostgreSQL E2E tests require a database that can run migrations");

    let config = test_config(TestBackend::Postgres(database_url), true, true);
    let mut app = test_app(config).await;

    run_full_identity_platform_flow(&mut app, "postgres", "postgres").await;
}

#[tokio::test]
async fn e2e_disabled_providers_return_stable_errors() {
    let config = test_config(TestBackend::Memory, false, false);
    let mut app = test_app(config).await;

    let register = post_json(
        &mut app,
        "/v1/auth/register",
        json!({
            "username": "disabled-local@example.test",
            "password": "correct horse battery staple"
        }),
    )
    .await;
    assert_error(register, StatusCode::FORBIDDEN, "provider_disabled");

    let login = post_json(
        &mut app,
        "/v1/auth/login",
        json!({
            "username": "disabled-local@example.test",
            "password": "correct horse battery staple"
        }),
    )
    .await;
    assert_error(login, StatusCode::FORBIDDEN, "provider_disabled");

    let supabase_exchange = post_json(
        &mut app,
        "/v1/auth/supabase/exchange",
        json!({ "access_token": supabase_jwt("disabled-supabase-user", SUPABASE_AUDIENCE) }),
    )
    .await;
    assert_error(
        supabase_exchange,
        StatusCode::FORBIDDEN,
        "provider_disabled",
    );
}

#[tokio::test]
async fn e2e_disabled_local_provider_blocks_password_change_for_existing_session() {
    let database_url = sqlite_database_url("disabled-local-password-change");
    sqlite::run_pending_migrations(&database_url)
        .await
        .expect("SQLite E2E tests should run migrations");

    let enabled_config = test_config(TestBackend::Sqlite(database_url.clone()), true, true);
    let mut enabled_app = test_app(enabled_config).await;
    let register = post_json(
        &mut enabled_app,
        "/v1/auth/register",
        json!({
            "username": format!("disabled-change-{}@example.test", Uuid::new_v4()),
            "password": "correct horse battery staple"
        }),
    )
    .await;
    assert_eq!(register.status, StatusCode::OK);
    let access_token = token(&register.body, "access_token");

    let disabled_config = test_config(TestBackend::Sqlite(database_url), false, true);
    let mut disabled_app = test_app(disabled_config).await;
    let change = post_json_with_bearer(
        &mut disabled_app,
        "/v1/auth/password/change",
        &access_token,
        json!({
            "current_password": "correct horse battery staple",
            "new_password": "new correct horse battery staple"
        }),
    )
    .await;

    assert_error(change, StatusCode::FORBIDDEN, "provider_disabled");
}

async fn run_full_identity_platform_flow(
    app: &mut axum::Router,
    backend_label: &str,
    expected_readiness_dependency: &str,
) {
    assert_health_and_readiness(app, expected_readiness_dependency).await;
    assert_missing_bearer_is_rejected(app).await;

    let username = format!("{backend_label}-local-{}@example.test", Uuid::new_v4());
    let original_password = "correct horse battery staple";
    let changed_password = "changed correct horse battery staple";

    let register = post_json(
        app,
        "/v1/auth/register",
        json!({
            "username": username,
            "password": original_password
        }),
    )
    .await;
    assert_eq!(register.status, StatusCode::OK);
    let local_user_id = string_at(&register.body, &["user", "internal_user_id"]);
    let initial_access_token = token(&register.body, "access_token");
    let initial_refresh_token = token(&register.body, "refresh_token");
    assert_current_user(app, &initial_access_token, &local_user_id).await;

    let duplicate_register = post_json(
        app,
        "/v1/auth/register",
        json!({
            "username": username,
            "password": original_password
        }),
    )
    .await;
    assert_error(
        duplicate_register,
        StatusCode::CONFLICT,
        "identity_conflict",
    );

    let bad_login = post_json(
        app,
        "/v1/auth/login",
        json!({
            "username": username,
            "password": "wrong password"
        }),
    )
    .await;
    assert_error(bad_login, StatusCode::UNAUTHORIZED, "invalid_credentials");

    let login = post_json(
        app,
        "/v1/auth/login",
        json!({
            "username": username,
            "password": original_password
        }),
    )
    .await;
    assert_eq!(login.status, StatusCode::OK);
    assert_eq!(
        string_at(&login.body, &["user", "internal_user_id"]),
        local_user_id
    );
    let login_refresh_token = token(&login.body, "refresh_token");

    let first_refresh = post_json(
        app,
        "/v1/auth/refresh",
        json!({ "refresh_token": login_refresh_token }),
    )
    .await;
    assert_eq!(first_refresh.status, StatusCode::OK);
    let rotated_refresh_token = token(&first_refresh.body, "refresh_token");
    assert_ne!(rotated_refresh_token, login_refresh_token);

    let reused_refresh = post_json(
        app,
        "/v1/auth/refresh",
        json!({ "refresh_token": login_refresh_token }),
    )
    .await;
    assert_error(
        reused_refresh,
        StatusCode::UNAUTHORIZED,
        "refresh_token_reused",
    );

    let revoked_rotated_refresh = post_json(
        app,
        "/v1/auth/refresh",
        json!({ "refresh_token": rotated_refresh_token }),
    )
    .await;
    assert_error(
        revoked_rotated_refresh,
        StatusCode::UNAUTHORIZED,
        "token_invalid",
    );

    let password_change_wrong_current = post_json_with_bearer(
        app,
        "/v1/auth/password/change",
        &initial_access_token,
        json!({
            "current_password": "wrong password",
            "new_password": changed_password
        }),
    )
    .await;
    assert_error(
        password_change_wrong_current,
        StatusCode::UNAUTHORIZED,
        "invalid_credentials",
    );

    let password_change = post_json_with_bearer(
        app,
        "/v1/auth/password/change",
        &initial_access_token,
        json!({
            "current_password": original_password,
            "new_password": changed_password
        }),
    )
    .await;
    assert_eq!(password_change.status, StatusCode::OK);
    let changed_access_token = token(&password_change.body, "access_token");
    let changed_refresh_token = token(&password_change.body, "refresh_token");

    let old_password_login = post_json(
        app,
        "/v1/auth/login",
        json!({
            "username": username,
            "password": original_password
        }),
    )
    .await;
    assert_error(
        old_password_login,
        StatusCode::UNAUTHORIZED,
        "invalid_credentials",
    );

    let new_password_login = post_json(
        app,
        "/v1/auth/login",
        json!({
            "username": username,
            "password": changed_password
        }),
    )
    .await;
    assert_eq!(new_password_login.status, StatusCode::OK);

    let old_refresh_after_password_change = post_json(
        app,
        "/v1/auth/refresh",
        json!({ "refresh_token": initial_refresh_token }),
    )
    .await;
    assert_error(
        old_refresh_after_password_change,
        StatusCode::UNAUTHORIZED,
        "token_invalid",
    );
    assert_current_user(app, &changed_access_token, &local_user_id).await;

    let logout = post_with_bearer(app, "/v1/auth/logout", &changed_access_token).await;
    assert_eq!(logout.status, StatusCode::OK);
    assert_eq!(logout.body["revoked"], true);
    let me_after_logout = get_with_bearer(app, "/v1/users/me", &changed_access_token).await;
    assert_error(me_after_logout, StatusCode::UNAUTHORIZED, "unauthorized");
    let changed_refresh_after_logout = post_json(
        app,
        "/v1/auth/refresh",
        json!({ "refresh_token": changed_refresh_token }),
    )
    .await;
    assert_error(
        changed_refresh_after_logout,
        StatusCode::UNAUTHORIZED,
        "token_invalid",
    );

    run_supabase_exchange_flow(app, backend_label).await;
}

async fn run_supabase_exchange_flow(app: &mut axum::Router, backend_label: &str) {
    let supabase_subject = format!("{backend_label}-supabase-{}", Uuid::new_v4());
    let access_token = supabase_jwt(&supabase_subject, SUPABASE_AUDIENCE);

    let exchange = post_json(
        app,
        "/v1/auth/supabase/exchange",
        json!({ "access_token": access_token }),
    )
    .await;
    assert_eq!(exchange.status, StatusCode::OK);
    let supabase_user_id = string_at(&exchange.body, &["user", "internal_user_id"]);
    let supabase_access_token = token(&exchange.body, "access_token");
    let supabase_refresh_token = token(&exchange.body, "refresh_token");
    assert_current_user(app, &supabase_access_token, &supabase_user_id).await;

    let second_exchange = post_json(
        app,
        "/v1/auth/supabase/exchange",
        json!({ "access_token": supabase_jwt(&supabase_subject, SUPABASE_AUDIENCE) }),
    )
    .await;
    assert_eq!(second_exchange.status, StatusCode::OK);
    assert_eq!(
        string_at(&second_exchange.body, &["user", "internal_user_id"]),
        supabase_user_id
    );

    let wrong_audience_exchange = post_json(
        app,
        "/v1/auth/supabase/exchange",
        json!({ "access_token": supabase_jwt(&supabase_subject, "wrong-audience") }),
    )
    .await;
    assert_error(
        wrong_audience_exchange,
        StatusCode::UNAUTHORIZED,
        "provider_verification_failed",
    );

    let refresh = post_json(
        app,
        "/v1/auth/refresh",
        json!({ "refresh_token": supabase_refresh_token }),
    )
    .await;
    assert_eq!(refresh.status, StatusCode::OK);
    let refreshed_access_token = token(&refresh.body, "access_token");
    assert_current_user(app, &refreshed_access_token, &supabase_user_id).await;

    let logout = post_with_bearer(app, "/v1/auth/logout", &refreshed_access_token).await;
    assert_eq!(logout.status, StatusCode::OK);
    let me_after_logout = get_with_bearer(app, "/v1/users/me", &refreshed_access_token).await;
    assert_error(me_after_logout, StatusCode::UNAUTHORIZED, "unauthorized");
}

async fn assert_health_and_readiness(app: &mut axum::Router, persistence_name: &str) {
    let health = get_json(app, "/health").await;
    assert_eq!(health.status, StatusCode::OK);
    assert_eq!(health.body["status"], "ok");

    let ready = get_json(app, "/ready").await;
    assert_eq!(ready.status, StatusCode::OK);
    assert_eq!(ready.body["status"], "ready");
    assert_eq!(ready.body["persistence"]["name"], persistence_name);
    assert_eq!(ready.body["persistence"]["status"], "ready");
}

async fn assert_missing_bearer_is_rejected(app: &mut axum::Router) {
    let response = get_json(app, "/v1/users/me").await;
    assert_error(response, StatusCode::UNAUTHORIZED, "unauthorized");
}

async fn assert_current_user(app: &mut axum::Router, access_token: &str, expected_user_id: &str) {
    let response = get_with_bearer(app, "/v1/users/me", access_token).await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body["internal_user_id"], expected_user_id);
    assert_eq!(response.body["account_status"], "Active");
}

async fn test_app(config: AppConfig) -> axum::Router {
    router(
        build_application_services(config)
            .await
            .expect("test config must build services"),
    )
}

enum TestBackend {
    Memory,
    Sqlite(String),
    Postgres(String),
}

fn test_config(
    backend: TestBackend,
    local_password_enabled: bool,
    supabase_enabled: bool,
) -> AppConfig {
    let (persistence_backend, database_url) = match backend {
        TestBackend::Memory => (PersistenceBackend::Memory, None),
        TestBackend::Sqlite(database_url) => (PersistenceBackend::Sqlite, Some(database_url)),
        TestBackend::Postgres(database_url) => (PersistenceBackend::Postgres, Some(database_url)),
    };

    AppConfig {
        http: HttpConfig {
            host: "127.0.0.1".to_owned(),
            port: 3000,
            frontend_direct: FrontendDirectConfig {
                enabled: false,
                allowed_origins: Vec::new(),
            },
        },
        persistence: PersistenceConfig {
            backend: persistence_backend,
            database_url,
        },
        identity_providers: IdentityProviderConfig {
            local_password: ProviderToggle {
                enabled: local_password_enabled,
            },
            supabase: SupabaseProviderConfig {
                enabled: supabase_enabled,
                auto_provision_enabled: true,
                project_url: "https://example.supabase.co".to_owned(),
                issuer: SUPABASE_ISSUER.to_owned(),
                audience: SUPABASE_AUDIENCE.to_owned(),
                jwks_url: "https://example.supabase.co/auth/v1/.well-known/jwks.json".to_owned(),
                jwks_json: Some(supabase_jwks_json()),
                fixture_tokens_enabled: false,
            },
        },
        client: ClientConfig {
            client_id: "identity-service-e2e".to_owned(),
            trusted_origin: None,
        },
        tokens: TokenConfig {
            issuer: "identity-service-e2e".to_owned(),
            audience: "platform-api-e2e".to_owned(),
            access_token_lifetime_seconds: 900,
            key_id: "e2e-key".to_owned(),
            private_key_pem: TEST_PRIVATE_KEY_PEM.to_owned(),
            public_key_pem: TEST_PUBLIC_KEY_PEM.to_owned(),
        },
        sessions: SessionConfig {
            refresh_token_lifetime_seconds: 2_592_000,
            session_lifetime_seconds: 2_592_000,
        },
        security: SecurityConfig {
            refresh_token_hmac_secret: "identity-service-e2e-refresh-secret".to_owned(),
        },
    }
}

fn sqlite_database_url(label: &str) -> String {
    let path = std::env::temp_dir().join(format!("identity-service-{label}-{}.db", Uuid::new_v4()));
    let _ = fs::remove_file(&path);

    format!("sqlite://{}", path.display())
}

fn supabase_jwks_json() -> String {
    let encoded_secret = URL_SAFE_NO_PAD.encode(SUPABASE_SECRET);
    serde_json::json!({
        "keys": [{
            "kty": "oct",
            "k": encoded_secret,
            "kid": SUPABASE_KEY_ID,
            "alg": "HS256",
            "use": "sig"
        }]
    })
    .to_string()
}

fn supabase_jwt(subject: &str, audience: &str) -> String {
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some(SUPABASE_KEY_ID.to_owned());
    let claims = SupabaseClaims {
        sub: subject.to_owned(),
        exp: Utc::now().timestamp() + 900,
        iss: SUPABASE_ISSUER.to_owned(),
        aud: audience.to_owned(),
        email: Some(format!("{subject}@example.test")),
        phone: Some("+15555550123".to_owned()),
    };

    encode(&header, &claims, &EncodingKey::from_secret(SUPABASE_SECRET)).unwrap()
}

#[derive(Serialize)]
struct SupabaseClaims {
    sub: String,
    exp: i64,
    iss: String,
    aud: String,
    email: Option<String>,
    phone: Option<String>,
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

async fn post_json_with_bearer(
    app: &mut axum::Router,
    uri: &str,
    bearer_token: &str,
    body: Value,
) -> TestResponse {
    let request = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {bearer_token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should build");

    call(app, request).await
}

async fn get_json(app: &mut axum::Router, uri: &str) -> TestResponse {
    let request = Request::builder()
        .method(Method::GET)
        .uri(uri)
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

fn string_at(body: &Value, path: &[&str]) -> String {
    let mut value = body;
    for segment in path {
        value = &value[*segment];
    }
    value
        .as_str()
        .unwrap_or_else(|| panic!("response should include {}", path.join(".")))
        .to_owned()
}

fn assert_error(response: TestResponse, expected_status: StatusCode, expected_error_code: &str) {
    assert_eq!(response.status, expected_status);
    assert_eq!(response.body["error_code"], expected_error_code);
    assert_eq!(response.body["retryable"], false);
}

struct TestResponse {
    status: StatusCode,
    body: Value,
}
