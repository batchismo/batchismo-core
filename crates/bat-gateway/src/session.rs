use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use bat_types::message::Message;
use bat_types::session::SessionMeta;
use crate::db::Database;

pub struct SessionManager {
    db: Arc<Database>,
    default_model: String,
}

impl SessionManager {
    pub fn new(db: Arc<Database>, default_model: String) -> Self {
        Self { db, default_model }
    }

    pub fn create_session(&self, key: &str, model: &str) -> Result<SessionMeta> {
        self.db.create_session(key, model)
    }

    pub fn get_or_create_main(&self) -> Result<SessionMeta> {
        self.db.get_or_create_main(&self.default_model)
    }

    pub fn get_session(&self, id: Uuid) -> Result<Option<SessionMeta>> {
        self.db.get_session(id)
    }

    pub fn get_session_by_key(&self, key: &str) -> Result<Option<SessionMeta>> {
        self.db.get_session_by_key(key)
    }

    pub fn append_message(&self, msg: &Message) -> Result<()> {
        self.db.append_message(msg)
    }

    pub fn get_history(&self, session_id: Uuid) -> Result<Vec<Message>> {
        self.db.get_history(session_id)
    }

    pub fn update_token_usage(&self, session_id: Uuid, input: i64, output: i64) -> Result<()> {
        self.db.update_token_usage(session_id, input, output)
    }
}
