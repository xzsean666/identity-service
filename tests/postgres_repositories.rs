use std::sync::Arc;

use identity_service::{
    application::{
        error::AppError, identity_binding::IdentityBindingService, session::SessionService,
    },
    config::{ClientConfig, SessionConfig},
    domain::identity::{BindingMode, NormalizedExternalIdentity},
    infrastructure::postgres::{
        PostgresIdentityRepository, PostgresLocalCredentialRepository, PostgresSessionRepository,
        PostgresState,
    },
    providers::{
        IdentityProviderAdapter, ProviderVerificationRequest, local_password::LocalPasswordProvider,
    },
    security::RefreshTokenHasher,
};
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
async fn postgres_repositories_support_mvp_identity_and_session_flow() {
    let Some(database_url) = std::env::var("IDENTITY_DATABASE_URL").ok() else {
        return;
    };
    let state = PostgresState::connect(&database_url)
        .await
        .expect("IDENTITY_DATABASE_URL must point to a running PostgreSQL database");
    assert_mvp_schema_exists(&state.pool).await;

    let identity_repository = Arc::new(PostgresIdentityRepository::new(state.pool.clone()));
    let local_password_provider = LocalPasswordProvider::new(
        Arc::new(PostgresLocalCredentialRepository::new(state.pool.clone())),
        true,
    );
    let session_service = SessionService::new(
        Arc::new(PostgresSessionRepository::new(state.pool.clone())),
        SessionConfig {
            refresh_token_lifetime_seconds: 3600,
            session_lifetime_seconds: 3600,
        },
        ClientConfig {
            client_id: "postgres-repository-test".to_owned(),
            trusted_origin: None,
        },
        RefreshTokenHasher::new("postgres-refresh-token-test-secret".to_owned()),
    );
    let identity_binding = IdentityBindingService::new(identity_repository);

    let unique_username = format!("postgres-{}@example.test", Uuid::new_v4());
    let user = identity_binding.create_active_user().await.unwrap();
    let credential = local_password_provider
        .create_credential_for_user(
            user.internal_user_id,
            &unique_username,
            "correct horse battery staple",
        )
        .await
        .unwrap();
    let normalized_identity =
        NormalizedExternalIdentity::local_password(credential.credential_id, &credential.username);
    let user = identity_binding
        .resolve_identity(
            normalized_identity,
            BindingMode::LinkToExisting(user.internal_user_id),
        )
        .await
        .unwrap();

    let login_identity = local_password_provider
        .verify(ProviderVerificationRequest::LocalPassword {
            username: unique_username,
            password: "correct horse battery staple".to_owned(),
        })
        .await
        .unwrap();
    let login_user = identity_binding
        .resolve_identity(login_identity, BindingMode::LoginOnly)
        .await
        .unwrap();
    assert_eq!(user.internal_user_id, login_user.internal_user_id);

    let (session, first_refresh) = session_service
        .create_session(&user, "local_password", "first-refresh-secret".to_owned())
        .await
        .unwrap();
    let (rotated_session, second_refresh) = session_service
        .exchange_refresh_token("first-refresh-secret", "second-refresh-secret".to_owned())
        .await
        .unwrap();
    assert_eq!(session.session_id, rotated_session.session_id);
    assert_eq!(
        first_refresh.token_family_id,
        second_refresh.token_family_id
    );

    let reused = session_service
        .exchange_refresh_token("first-refresh-secret", "third-refresh-secret".to_owned())
        .await;
    assert!(matches!(reused, Err(AppError::RefreshTokenReused)));
    assert_refresh_status(&state.pool, first_refresh.refresh_token_id, "reused").await;
    assert_refresh_status(&state.pool, second_refresh.refresh_token_id, "revoked").await;

    session_service
        .revoke_session(rotated_session.session_id)
        .await
        .unwrap();
    assert!(matches!(
        session_service
            .session_by_id(rotated_session.session_id)
            .await,
        Err(AppError::Unauthorized)
    ));

    identity_binding
        .delete_user(user.internal_user_id)
        .await
        .unwrap();
}

async fn assert_mvp_schema_exists(pool: &PgPool) {
    let table_name: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('public.internal_users')::text AS table_name")
            .fetch_one(pool)
            .await
            .expect("schema check query must run");

    assert!(
        table_name.is_some(),
        "IDENTITY_DATABASE_URL tests require the MVP migration first: cargo run --bin migrate -- up"
    );
}

async fn assert_refresh_status(pool: &PgPool, refresh_token_id: Uuid, expected_status: &str) {
    let status: String =
        sqlx::query_scalar("SELECT status FROM refresh_token_records WHERE refresh_token_id = $1")
            .bind(refresh_token_id)
            .fetch_one(pool)
            .await
            .expect("refresh token status should exist");

    assert_eq!(status, expected_status);
}
