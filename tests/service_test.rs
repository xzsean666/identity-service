use identity_service::{
    config::Config,
    db::{init_pool, run_migrations},
    models::{ExternalIdentity, UpdateUserDto},
    services::{IdentityService, JwtEngine, SessionService, UserService},
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_identity_service_and_account_linking() {
    let config = Config::from_env().unwrap();
    let pool = init_pool(&config.database_url).await.unwrap();
    run_migrations(&pool).await.unwrap();

    let identity_svc = IdentityService::new(pool.clone());
    let user_svc = UserService::new(pool.clone());

    let provider_uid_1 = format!("google_{}", Uuid::new_v4());
    let ext_1 = ExternalIdentity {
        provider: "google".to_string(),
        provider_user_id: provider_uid_1.clone(),
        email: Some(format!("{}@example.com", provider_uid_1)),
        name: Some("First Google User".to_string()),
        avatar_url: None,
    };

    // 1. Resolve new identity creates new user
    let user1 = identity_svc
        .resolve_or_create_user(&ext_1)
        .await
        .expect("Creation should succeed");

    // 2. Resolving again with same identity returns same user
    let user1_again = identity_svc
        .resolve_or_create_user(&ext_1)
        .await
        .expect("Second resolve should succeed");
    assert_eq!(user1.id, user1_again.id);

    // 3. Link a second identity (e.g. X / Twitter) to user1
    let provider_uid_2 = format!("x_{}", Uuid::new_v4());
    let ext_2 = ExternalIdentity {
        provider: "x".to_string(),
        provider_user_id: provider_uid_2.clone(),
        email: None,
        name: Some("Twitter Handle".to_string()),
        avatar_url: Some("https://x.com/pic.jpg".to_string()),
    };
    let linked = identity_svc
        .link_identity(user1.id, &ext_2)
        .await
        .expect("Linking second identity should succeed");
    assert_eq!(linked.user_id, user1.id);
    assert_eq!(linked.provider, "x");

    // 4. Resolving with the linked second identity should now resolve to user1!
    let user1_via_x = identity_svc
        .resolve_or_create_user(&ext_2)
        .await
        .expect("Resolve linked identity should succeed");
    assert_eq!(user1_via_x.id, user1.id);

    // 5. Check all identities for user1
    let all_idents = identity_svc
        .get_user_identities(user1.id)
        .await
        .expect("Should list identities");
    assert_eq!(all_idents.len(), 2);

    // 6. Test UserService profile update
    let update_dto = UpdateUserDto {
        name: Some("Updated Full Name".to_string()),
        avatar_url: Some("https://avatar.com/new.png".to_string()),
    };
    let updated = user_svc
        .update_user(user1.id, &update_dto)
        .await
        .expect("Update profile should succeed");
    assert_eq!(updated.name, Some("Updated Full Name".to_string()));
    assert_eq!(updated.avatar_url, Some("https://avatar.com/new.png".to_string()));
}

#[tokio::test]
async fn test_session_service_lifecycle() {
    let config = Config::from_env().unwrap();
    let pool = init_pool(&config.database_url).await.unwrap();
    run_migrations(&pool).await.unwrap();

    let jwt_engine = Arc::new(JwtEngine::new(&config).unwrap());
    let identity_svc = IdentityService::new(pool.clone());
    let session_svc = SessionService::new(pool.clone(), jwt_engine.clone(), &config);

    // Setup user
    let uid = format!("session_user_{}", Uuid::new_v4());
    let ext = ExternalIdentity {
        provider: "mock".to_string(),
        provider_user_id: uid.clone(),
        email: Some(format!("{}@mock.com", uid)),
        name: Some("Session User".to_string()),
        avatar_url: None,
    };
    let user = identity_svc.resolve_or_create_user(&ext).await.unwrap();

    // 1. Issue tokens
    let tokens = session_svc
        .issue_tokens(user.id, user.email.clone())
        .await
        .expect("Token issue should succeed");

    assert!(!tokens.access_token.is_empty());
    assert!(!tokens.refresh_token.is_empty());

    // Verify access token
    let claims = jwt_engine.verify(&tokens.access_token).unwrap();
    assert_eq!(claims.sub, user.id.to_string());

    // 2. Refresh token rotation
    let new_tokens = session_svc
        .refresh_tokens(&tokens.refresh_token)
        .await
        .expect("Refresh should succeed");

    assert_ne!(new_tokens.refresh_token, tokens.refresh_token);

    // 3. Old refresh token must now be revoked and rejected
    let reuse_err = session_svc.refresh_tokens(&tokens.refresh_token).await;
    assert!(reuse_err.is_err());
    assert!(reuse_err.unwrap_err().to_string().contains("revoked"));

    // 4. Revoke explicitly
    let revoked = session_svc
        .revoke_token(&new_tokens.refresh_token)
        .await
        .expect("Revoke should succeed");
    assert!(revoked);

    let refresh_after_revoke = session_svc.refresh_tokens(&new_tokens.refresh_token).await;
    assert!(refresh_after_revoke.is_err());
}
