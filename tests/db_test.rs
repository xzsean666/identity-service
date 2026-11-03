use identity_service::{
    config::Config,
    db::{
        create_user_with_identity, find_identity, find_refresh_token, find_user_by_id, init_pool,
        revoke_refresh_token, run_migrations, save_refresh_token, update_user,
    },
    models::{ExternalIdentity, UpdateUserDto},
};
use uuid::Uuid;

#[tokio::test]
async fn test_database_models_and_queries() {
    let config = Config::from_env().expect("Config must load");
    let pool = init_pool(&config.database_url)
        .await
        .expect("Database pool initialization should succeed");

    run_migrations(&pool)
        .await
        .expect("Migrations should execute without error");

    let unique_provider_id = format!("user_{}", Uuid::new_v4());
    let ext_identity = ExternalIdentity {
        provider: "google".to_string(),
        provider_user_id: unique_provider_id.clone(),
        email: Some(format!("{}@example.com", unique_provider_id)),
        name: Some("Test User".to_string()),
        avatar_url: Some("https://example.com/avatar.png".to_string()),
    };

    // 1. Create user with identity
    let (user, identity) = create_user_with_identity(&pool, &ext_identity)
        .await
        .expect("User and identity creation should succeed");

    assert_eq!(user.email, ext_identity.email);
    assert_eq!(user.name, ext_identity.name);
    assert_eq!(identity.user_id, user.id);
    assert_eq!(identity.provider, "google");
    assert_eq!(identity.provider_user_id, unique_provider_id);

    // 2. Lookup identity
    let found_identity = find_identity(&pool, "google", &unique_provider_id)
        .await
        .expect("Identity query should succeed")
        .expect("Identity must exist");
    assert_eq!(found_identity.user_id, user.id);

    // 3. Lookup user
    let found_user = find_user_by_id(&pool, user.id)
        .await
        .expect("User query should succeed")
        .expect("User must exist");
    assert_eq!(found_user.id, user.id);
    assert_eq!(found_user.status, "active");

    // 4. Update user
    let update = UpdateUserDto {
        name: Some("Updated User Name".to_string()),
        avatar_url: Some("https://example.com/new_avatar.png".to_string()),
    };
    let updated_user = update_user(&pool, user.id, &update)
        .await
        .expect("Update user should succeed");
    assert_eq!(updated_user.name, Some("Updated User Name".to_string()));
    assert_eq!(
        updated_user.avatar_url,
        Some("https://example.com/new_avatar.png".to_string())
    );

    // 5. Save and find refresh token
    let token_hash = format!("dummy_hash_{}", Uuid::new_v4());
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
    let rt = save_refresh_token(&pool, user.id, &token_hash, expires_at)
        .await
        .expect("Save refresh token should succeed");
    assert_eq!(rt.user_id, user.id);
    assert_eq!(rt.token_hash, token_hash);
    assert!(!rt.revoked);

    let found_rt = find_refresh_token(&pool, &token_hash)
        .await
        .expect("Find refresh token should succeed")
        .expect("Refresh token should exist");
    assert_eq!(found_rt.id, rt.id);

    // 6. Revoke refresh token
    let revoked = revoke_refresh_token(&pool, &token_hash)
        .await
        .expect("Revoke token should succeed");
    assert!(revoked);

    let found_revoked_rt = find_refresh_token(&pool, &token_hash)
        .await
        .expect("Find refresh token should succeed")
        .expect("Refresh token should exist");
    assert!(found_revoked_rt.revoked);
}
