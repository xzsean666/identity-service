use std::{collections::HashMap, sync::Arc};

use parking_lot::Mutex;
use uuid::Uuid;

use crate::{
    domain::{
        identity::ExternalIdentity,
        session::{RefreshTokenRecord, Session},
        user::InternalUser,
    },
    providers::local_password::LocalCredential,
};

pub type SharedState = Arc<Mutex<InMemoryState>>;

#[derive(Default)]
pub struct InMemoryState {
    pub users: HashMap<Uuid, InternalUser>,
    pub identities_by_provider_subject: HashMap<(String, String), ExternalIdentity>,
    pub local_credentials_by_username: HashMap<String, LocalCredential>,
    pub sessions: HashMap<Uuid, Session>,
    pub refresh_tokens_by_hash: HashMap<String, RefreshTokenRecord>,
}

impl InMemoryState {
    pub fn shared() -> SharedState {
        Arc::new(Mutex::new(Self::default()))
    }
}
