use std::{fs, sync::Arc};

use identity_service::{
    application::{
        error::AppError, identity_binding::IdentityBindingService,
        password_change::PasswordChangeService, session::SessionService,
    },
    config::{ClientConfig, SessionConfig},
    domain::identity::{BindingMode, NormalizedExternalIdentity},
    infrastructure::sqlite::{
        SqliteIdentityRepository, SqliteLocalCredentialRepository, SqlitePasswordChangeRepository,
        SqliteSessionRepository, SqliteState, run_pending_migrations,
    },
    providers::{
        IdentityProviderAdapter, ProviderVerificationRequest, local_password::LocalPasswordProvider,
    },
    security::RefreshTokenHasher,
};
use sqlx::SqlitePool;
use uuid::Uuid;

#[tokio::test]
async fn sqlite_repositories_support_mvp_identity_and_session_flow() {
    let database_url = sqlite_database_url("identity-flow");
    run_pending_migrations(&database_url)
        .await
        .expect("SQLite migrations should run");
    let state = SqliteState::connect(&database_url)
        .await
        .expect("SQLite test database should connect");
    assert_mvp_schema_exists(&state.pool).await;

    let identity_repository = Arc::new(SqliteIdentityRepository::new(state.pool.clone()));
    let local_password_provider = LocalPasswordProvider::new(
        Arc::new(SqliteLocalCredentialRepository::new(state.pool.clone())),
        true,
    );
    let session_service = SessionService::new(
        Arc::new(SqliteSessionRepository::new(state.pool.clone())),
        SessionConfig {
            refresh_token_lifetime_seconds: 3600,
            session_lifetime_seconds: 3600,
        },
        ClientConfig {
            client_id: "sqlite-repository-test".to_owned(),
            trusted_origin: None,
        },
        RefreshTokenHasher::new("sqlite-refresh-token-test-secret".to_owned()),
    );
    let identity_binding = IdentityBindingService::new(identity_repository);

    let unique_username = format!("sqlite-{}@example.test", Uuid::new_v4());
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

#[tokio::test]
async fn sqlite_password_change_updates_hash_and_rotates_refresh_tokens() {
    let database_url = sqlite_database_url("password-change");
    run_pending_migrations(&database_url)
        .await
        .expect("SQLite migrations should run");
    let state = SqliteState::connect(&database_url)
        .await
        .expect("SQLite test database should connect");
    assert_mvp_schema_exists(&state.pool).await;

    let identity_binding =
        IdentityBindingService::new(Arc::new(SqliteIdentityRepository::new(state.pool.clone())));
    let local_password_provider = LocalPasswordProvider::new(
        Arc::new(SqliteLocalCredentialRepository::new(state.pool.clone())),
        true,
    );
    let session_config = SessionConfig {
        refresh_token_lifetime_seconds: 3600,
        session_lifetime_seconds: 3600,
    };
    let session_service = SessionService::new(
        Arc::new(SqliteSessionRepository::new(state.pool.clone())),
        session_config.clone(),
        ClientConfig {
            client_id: "sqlite-password-change-test".to_owned(),
            trusted_origin: None,
        },
        RefreshTokenHasher::new("sqlite-password-change-test-secret".to_owned()),
    );
    let password_change_service = PasswordChangeService::new(
        Arc::new(SqlitePasswordChangeRepository::new(state.pool.clone())),
        session_config,
        RefreshTokenHasher::new("sqlite-password-change-test-secret".to_owned()),
    );

    let unique_username = format!("sqlite-password-{}@example.test", Uuid::new_v4());
    let user = identity_binding.create_active_user().await.unwrap();
    let credential = local_password_provider
        .create_credential_for_user(
            user.internal_user_id,
            &unique_username,
            "old correct horse battery staple",
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
    let (session, old_refresh) = session_service
        .create_session(&user, "local_password", "old-refresh-secret".to_owned())
        .await
        .unwrap();

    let prepared_change = local_password_provider
        .prepare_password_change(
            user.internal_user_id,
            "old correct horse battery staple",
            "new correct horse battery staple",
        )
        .await
        .unwrap();
    let new_refresh = password_change_service
        .change_password_and_rotate_refresh_tokens(
            user.internal_user_id,
            session.session_id,
            prepared_change,
            "new-refresh-secret",
        )
        .await
        .unwrap();

    assert_refresh_status(&state.pool, old_refresh.refresh_token_id, "revoked").await;
    assert_refresh_status(&state.pool, new_refresh.refresh_token_id, "active").await;
    assert!(
        local_password_provider
            .verify(ProviderVerificationRequest::LocalPassword {
                username: unique_username.clone(),
                password: "old correct horse battery staple".to_owned(),
            })
            .await
            .is_err()
    );
    assert!(
        local_password_provider
            .verify(ProviderVerificationRequest::LocalPassword {
                username: unique_username,
                password: "new correct horse battery staple".to_owned(),
            })
            .await
            .is_ok()
    );

    identity_binding
        .delete_user(user.internal_user_id)
        .await
        .unwrap();
}

fn sqlite_database_url(label: &str) -> String {
    let path = std::env::temp_dir().join(format!("identity-service-{label}-{}.db", Uuid::new_v4()));
    let _ = fs::remove_file(&path);

    format!("sqlite://{}", path.display())
}

async fn assert_mvp_schema_exists(pool: &SqlitePool) {
    let table_name: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'internal_users'",
    )
    .fetch_optional(pool)
    .await
    .expect("schema check query must run");

    assert!(
        table_name.is_some(),
        "SQLite tests require the MVP migration to create internal_users"
    );
}

async fn assert_refresh_status(pool: &SqlitePool, refresh_token_id: Uuid, expected_status: &str) {
    let status: String =
        sqlx::query_scalar("SELECT status FROM refresh_token_records WHERE refresh_token_id = ?1")
            .bind(refresh_token_id.to_string())
            .fetch_one(pool)
            .await
            .expect("refresh token status should exist");

    assert_eq!(status, expected_status);
}
