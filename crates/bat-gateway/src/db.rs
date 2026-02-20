use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;
use chrono::Utc;

use bat_types::message::Message;
use bat_types::session::{SessionMeta, SessionStatus};
use bat_types::policy::{PathPolicy, AccessLevel};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database at {}", path.display()))?;
        let db = Self { conn: Mutex::new(conn) };
        db.init_tables()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn: Mutex::new(conn) };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id          TEXT PRIMARY KEY,
                key         TEXT NOT NULL UNIQUE,
                model       TEXT NOT NULL,
                status      TEXT NOT NULL DEFAULT 'active',
                token_input INTEGER NOT NULL DEFAULT 0,
                token_output INTEGER NOT NULL DEFAULT 0,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id          TEXT PRIMARY KEY,
                session_id  TEXT NOT NULL REFERENCES sessions(id),
                role        TEXT NOT NULL,
                content     TEXT NOT NULL,
                tool_calls_json TEXT NOT NULL DEFAULT '[]',
                tool_results_json TEXT NOT NULL DEFAULT '[]',
                token_input INTEGER,
                token_output INTEGER,
                created_at  TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, created_at);

            CREATE TABLE IF NOT EXISTS tool_calls (
                id            TEXT PRIMARY KEY,
                message_id    TEXT NOT NULL REFERENCES messages(id),
                session_id    TEXT NOT NULL REFERENCES sessions(id),
                tool_name     TEXT NOT NULL,
                input_json    TEXT NOT NULL,
                result_text   TEXT,
                is_error      INTEGER NOT NULL DEFAULT 0,
                duration_ms   INTEGER,
                created_at    TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_tool_calls_session ON tool_calls(session_id, created_at);

            CREATE TABLE IF NOT EXISTS path_policies (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                path        TEXT NOT NULL,
                access      TEXT NOT NULL,
                recursive   INTEGER NOT NULL DEFAULT 1,
                description TEXT,
                created_at  TEXT NOT NULL
            );"
        )?;
        Ok(())
    }

    // --- Sessions ---

    pub fn create_session(&self, key: &str, model: &str) -> Result<SessionMeta> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        conn.execute(
            "INSERT INTO sessions (id, key, model, status, token_input, token_output, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'active', 0, 0, ?4, ?4)",
            params![id.to_string(), key, model, now_str],
        )?;
        Ok(SessionMeta {
            id,
            key: key.to_string(),
            model: model.to_string(),
            status: SessionStatus::Active,
            token_input: 0,
            token_output: 0,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn get_session_by_key(&self, key: &str) -> Result<Option<SessionMeta>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, key, model, status, token_input, token_output, created_at, updated_at
             FROM sessions WHERE key = ?1"
        )?;
        let mut rows = stmt.query(params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_session(row)?)),
            None => Ok(None),
        }
    }

    pub fn get_session(&self, id: Uuid) -> Result<Option<SessionMeta>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, key, model, status, token_input, token_output, created_at, updated_at
             FROM sessions WHERE id = ?1"
        )?;
        let mut rows = stmt.query(params![id.to_string()])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_session(row)?)),
            None => Ok(None),
        }
    }

    pub fn get_or_create_main(&self, model: &str) -> Result<SessionMeta> {
        if let Some(session) = self.get_session_by_key("main")? {
            return Ok(session);
        }
        self.create_session("main", model)
    }

    pub fn update_token_usage(&self, session_id: Uuid, input: i64, output: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET token_input = token_input + ?1, token_output = token_output + ?2, updated_at = ?3
             WHERE id = ?4",
            params![input, output, Utc::now().to_rfc3339(), session_id.to_string()],
        )?;
        Ok(())
    }

    // --- Messages ---

    pub fn append_message(&self, msg: &Message) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tool_calls_json = serde_json::to_string(&msg.tool_calls)?;
        let tool_results_json = serde_json::to_string(&msg.tool_results)?;
        conn.execute(
            "INSERT INTO messages (id, session_id, role, content, tool_calls_json, tool_results_json, token_input, token_output, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                msg.id.to_string(),
                msg.session_id.to_string(),
                msg.role.to_string(),
                msg.content,
                tool_calls_json,
                tool_results_json,
                msg.token_input,
                msg.token_output,
                msg.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_history(&self, session_id: Uuid) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content, tool_calls_json, tool_results_json, token_input, token_output, created_at
             FROM messages WHERE session_id = ?1 ORDER BY created_at ASC"
        )?;
        let rows = stmt.query_map(params![session_id.to_string()], |row| {
            Ok(MessageRow {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                tool_calls_json: row.get(4)?,
                tool_results_json: row.get(5)?,
                token_input: row.get(6)?,
                token_output: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;
        let mut messages = Vec::new();
        for row in rows {
            let r = row?;
            messages.push(Message {
                id: r.id.parse().context("invalid message id")?,
                session_id: r.session_id.parse().context("invalid session id")?,
                role: r.role.parse().context("invalid role")?,
                content: r.content,
                tool_calls: serde_json::from_str(&r.tool_calls_json)?,
                tool_results: serde_json::from_str(&r.tool_results_json)?,
                token_input: r.token_input,
                token_output: r.token_output,
                created_at: chrono::DateTime::parse_from_rfc3339(&r.created_at)?.with_timezone(&Utc),
            });
        }
        Ok(messages)
    }

    // --- Path Policies ---

    pub fn get_path_policies(&self) -> Result<Vec<PathPolicy>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT path, access, recursive, description FROM path_policies"
        )?;
        let rows = stmt.query_map([], |row| {
            let access_str: String = row.get(1)?;
            let recursive: bool = row.get(2)?;
            Ok((row.get::<_, String>(0)?, access_str, recursive, row.get::<_, Option<String>>(3)?))
        })?;
        let mut policies = Vec::new();
        for row in rows {
            let (path, access_str, recursive, desc) = row?;
            let access = match access_str.as_str() {
                "read-only" => AccessLevel::ReadOnly,
                "read-write" => AccessLevel::ReadWrite,
                "write-only" => AccessLevel::WriteOnly,
                _ => continue,
            };
            policies.push(PathPolicy {
                path: path.into(),
                access,
                recursive,
                description: desc,
            });
        }
        Ok(policies)
    }

    pub fn add_path_policy(&self, policy: &PathPolicy) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let access_str = match policy.access {
            AccessLevel::ReadOnly => "read-only",
            AccessLevel::ReadWrite => "read-write",
            AccessLevel::WriteOnly => "write-only",
        };
        conn.execute(
            "INSERT INTO path_policies (path, access, recursive, description, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                policy.path.to_string_lossy().to_string(),
                access_str,
                policy.recursive,
                policy.description,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    // --- Tool Calls ---

    pub fn record_tool_call(
        &self,
        id: &str,
        message_id: Uuid,
        session_id: Uuid,
        tool_name: &str,
        input: &serde_json::Value,
        result: Option<&str>,
        is_error: bool,
        duration_ms: Option<i64>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tool_calls (id, message_id, session_id, tool_name, input_json, result_text, is_error, duration_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                message_id.to_string(),
                session_id.to_string(),
                tool_name,
                serde_json::to_string(input)?,
                result,
                is_error as i32,
                duration_ms,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Delete a path policy by its path string.
    pub fn delete_path_policy(&self, path: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM path_policies WHERE path = ?1",
            rusqlite::params![path],
        )?;
        Ok(())
    }
}

struct MessageRow {
    id: String,
    session_id: String,
    role: String,
    content: String,
    tool_calls_json: String,
    tool_results_json: String,
    token_input: Option<i64>,
    token_output: Option<i64>,
    created_at: String,
}

fn row_to_session(row: &rusqlite::Row<'_>) -> Result<SessionMeta> {
    let id_str: String = row.get(0)?;
    let status_str: String = row.get(3)?;
    let created_str: String = row.get(6)?;
    let updated_str: String = row.get(7)?;
    Ok(SessionMeta {
        id: id_str.parse().context("invalid session id")?,
        key: row.get(1)?,
        model: row.get(2)?,
        status: status_str.parse().context("invalid status")?,
        token_input: row.get(4)?,
        token_output: row.get(5)?,
        created_at: chrono::DateTime::parse_from_rfc3339(&created_str)?.with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_str)?.with_timezone(&chrono::Utc),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bat_types::message::Role;

    #[test]
    fn test_create_and_get_session() {
        let db = Database::open_in_memory().unwrap();
        let session = db.create_session("test", "claude-opus").unwrap();
        assert_eq!(session.key, "test");
        assert_eq!(session.model, "claude-opus");

        let fetched = db.get_session_by_key("test").unwrap().unwrap();
        assert_eq!(fetched.id, session.id);
    }

    #[test]
    fn test_get_or_create_main() {
        let db = Database::open_in_memory().unwrap();
        let s1 = db.get_or_create_main("claude").unwrap();
        let s2 = db.get_or_create_main("claude").unwrap();
        assert_eq!(s1.id, s2.id);
    }

    #[test]
    fn test_message_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let session = db.create_session("test", "claude").unwrap();

        let msg = Message::user(session.id, "Hello");
        db.append_message(&msg).unwrap();

        let msg2 = Message::assistant(session.id, "Hi there!");
        db.append_message(&msg2).unwrap();

        let history = db.get_history(session.id).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].role, Role::User);
        assert_eq!(history[0].content, "Hello");
        assert_eq!(history[1].role, Role::Assistant);
        assert_eq!(history[1].content, "Hi there!");
    }

    #[test]
    fn test_token_usage() {
        let db = Database::open_in_memory().unwrap();
        let session = db.create_session("test", "claude").unwrap();
        db.update_token_usage(session.id, 100, 200).unwrap();
        db.update_token_usage(session.id, 50, 75).unwrap();

        let updated = db.get_session(session.id).unwrap().unwrap();
        assert_eq!(updated.token_input, 150);
        assert_eq!(updated.token_output, 275);
    }

    #[test]
    fn test_path_policies() {
        let db = Database::open_in_memory().unwrap();
        let policy = PathPolicy {
            path: "/tmp/test".into(),
            access: AccessLevel::ReadWrite,
            recursive: true,
            description: Some("Test folder".to_string()),
        };
        db.add_path_policy(&policy).unwrap();

        let policies = db.get_path_policies().unwrap();
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].path.to_string_lossy(), "/tmp/test");
        assert_eq!(policies[0].access, AccessLevel::ReadWrite);
    }
}
