use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;
use chrono::Utc;

use bat_types::audit::{AuditCategory, AuditEntry, AuditFilter, AuditLevel, AuditStats, AuditLevelCounts, AuditCategoryCounts};
use bat_types::memory::{Observation, ObservationFilter, ObservationKind, ObservationSummary};
use bat_types::message::Message;
use bat_types::session::{SessionKind, SessionMeta, SessionStatus, SubagentInfo, SubagentStatus};
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
            );

            CREATE TABLE IF NOT EXISTS audit_log (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                ts          TEXT NOT NULL,
                session_id  TEXT,
                level       TEXT NOT NULL,
                category    TEXT NOT NULL,
                event       TEXT NOT NULL,
                summary     TEXT NOT NULL,
                detail_json TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_audit_ts ON audit_log(ts);
            CREATE INDEX IF NOT EXISTS idx_audit_category ON audit_log(category);
            CREATE INDEX IF NOT EXISTS idx_audit_level ON audit_log(level);

            CREATE TABLE IF NOT EXISTS observations (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                ts          TEXT NOT NULL,
                session_id  TEXT,
                kind        TEXT NOT NULL,
                key         TEXT NOT NULL,
                value       TEXT,
                count       INTEGER NOT NULL DEFAULT 1,
                created_at  TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_obs_kind ON observations(kind);
            CREATE INDEX IF NOT EXISTS idx_obs_key ON observations(key);
            CREATE INDEX IF NOT EXISTS idx_obs_ts ON observations(ts);"
        )?;

        // Migration: add subagent columns to sessions (safe if they already exist)
        let _ = conn.execute("ALTER TABLE sessions ADD COLUMN kind TEXT NOT NULL DEFAULT 'main'", []);
        let _ = conn.execute("ALTER TABLE sessions ADD COLUMN parent_id TEXT", []);
        let _ = conn.execute("ALTER TABLE sessions ADD COLUMN label TEXT", []);
        let _ = conn.execute("ALTER TABLE sessions ADD COLUMN task TEXT", []);
        let _ = conn.execute("ALTER TABLE sessions ADD COLUMN subagent_status TEXT", []);
        let _ = conn.execute("ALTER TABLE sessions ADD COLUMN summary TEXT", []);

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
            kind: SessionKind::Main,
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

    /// List all user-created sessions (excludes subagents).
    pub fn list_sessions(&self) -> Result<Vec<SessionMeta>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, key, model, status, token_input, token_output, created_at, updated_at
             FROM sessions WHERE kind = 'main' ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(row_to_session(row))
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row??);
        }
        Ok(result)
    }

    /// Delete a session and all its messages.
    pub fn delete_session(&self, session_id: Uuid) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let id_str = session_id.to_string();
        conn.execute("DELETE FROM tool_calls WHERE session_id = ?1", params![id_str])?;
        conn.execute("DELETE FROM messages WHERE session_id = ?1", params![id_str])?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![id_str])?;
        Ok(())
    }

    /// Rename a session.
    pub fn rename_session(&self, session_id: Uuid, new_key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET key = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_key, Utc::now().to_rfc3339(), session_id.to_string()],
        )?;
        Ok(())
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
            "SELECT id, path, access, recursive, description FROM path_policies"
        )?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let access_str: String = row.get(2)?;
            let recursive: bool = row.get(3)?;
            Ok((id, row.get::<_, String>(1)?, access_str, recursive, row.get::<_, Option<String>>(4)?))
        })?;
        let mut policies = Vec::new();
        for row in rows {
            let (id, path, access_str, recursive, desc) = row?;
            let access = match access_str.as_str() {
                "read-only" => AccessLevel::ReadOnly,
                "read-write" => AccessLevel::ReadWrite,
                "write-only" => AccessLevel::WriteOnly,
                _ => continue,
            };
            policies.push(PathPolicy {
                id: Some(id),
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
    pub fn delete_path_policy(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM path_policies WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    // ── Audit Log ────────────────────────────────────────────────

    /// Insert an audit log entry. Returns the row id.
    pub fn insert_audit_log(
        &self,
        ts: &str,
        session_id: Option<&str>,
        level: AuditLevel,
        category: AuditCategory,
        event: &str,
        summary: &str,
        detail_json: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO audit_log (ts, session_id, level, category, event, summary, detail_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                ts,
                session_id,
                level.to_string(),
                category.to_string(),
                event,
                summary,
                detail_json,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Query audit log entries with optional filters.
    pub fn query_audit_log(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>> {
        let conn = self.conn.lock().unwrap();

        let mut sql = String::from(
            "SELECT id, ts, session_id, level, category, event, summary, detail_json FROM audit_log WHERE 1=1"
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(ref level) = filter.level {
            sql.push_str(&format!(" AND level = ?{idx}"));
            param_values.push(Box::new(level.to_string()));
            idx += 1;
        }
        if let Some(ref category) = filter.category {
            sql.push_str(&format!(" AND category = ?{idx}"));
            param_values.push(Box::new(category.to_string()));
            idx += 1;
        }
        if let Some(ref session_id) = filter.session_id {
            sql.push_str(&format!(" AND session_id = ?{idx}"));
            param_values.push(Box::new(session_id.clone()));
            idx += 1;
        }
        if let Some(ref since) = filter.since {
            sql.push_str(&format!(" AND ts >= ?{idx}"));
            param_values.push(Box::new(since.clone()));
            idx += 1;
        }
        if let Some(ref until) = filter.until {
            sql.push_str(&format!(" AND ts <= ?{idx}"));
            param_values.push(Box::new(until.clone()));
            idx += 1;
        }
        if let Some(ref search) = filter.search {
            sql.push_str(&format!(" AND summary LIKE ?{idx}"));
            param_values.push(Box::new(format!("%{search}%")));
            idx += 1;
        }

        sql.push_str(" ORDER BY id DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT ?{idx}"));
            param_values.push(Box::new(limit));
            idx += 1;
        } else {
            sql.push_str(&format!(" LIMIT ?{idx}"));
            param_values.push(Box::new(500i64));
            idx += 1;
        }
        if let Some(offset) = filter.offset {
            sql.push_str(&format!(" OFFSET ?{idx}"));
            param_values.push(Box::new(offset));
        }

        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt
            .query_map(params_ref.as_slice(), |row| {
                let level_str: String = row.get(3)?;
                let cat_str: String = row.get(4)?;
                Ok(AuditEntry {
                    id: row.get(0)?,
                    ts: row.get(1)?,
                    session_id: row.get(2)?,
                    level: level_str.parse().unwrap_or(AuditLevel::Info),
                    category: cat_str.parse().unwrap_or(AuditCategory::Gateway),
                    event: row.get(5)?,
                    summary: row.get(6)?,
                    detail_json: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    /// Get summary statistics for the audit log.
    pub fn get_audit_stats(&self) -> Result<AuditStats> {
        let conn = self.conn.lock().unwrap();
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0))?;

        let count_level = |level: &str| -> Result<i64> {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM audit_log WHERE level = ?1",
                params![level],
                |r| r.get(0),
            )?)
        };
        let count_cat = |cat: &str| -> Result<i64> {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM audit_log WHERE category = ?1",
                params![cat],
                |r| r.get(0),
            )?)
        };

        Ok(AuditStats {
            total,
            by_level: AuditLevelCounts {
                debug: count_level("debug")?,
                info: count_level("info")?,
                warn: count_level("warn")?,
                error: count_level("error")?,
            },
            by_category: AuditCategoryCounts {
                agent: count_cat("agent")?,
                tool: count_cat("tool")?,
                gateway: count_cat("gateway")?,
                ipc: count_cat("ipc")?,
                config: count_cat("config")?,
            },
        })
    }

    // ── Observations ─────────────────────────────────────────────

    /// Record a behavioral observation. If the same kind+key was recorded
    /// in the last hour, increment the count instead of inserting a new row.
    pub fn record_observation(
        &self,
        kind: ObservationKind,
        key: &str,
        value: Option<&str>,
        session_id: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let ts = Utc::now().to_rfc3339();
        let one_hour_ago = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();

        // Try to increment an existing recent observation
        let updated = conn.execute(
            "UPDATE observations SET count = count + 1, ts = ?1
             WHERE kind = ?2 AND key = ?3 AND ts > ?4",
            params![ts, kind.to_string(), key, one_hour_ago],
        )?;

        if updated == 0 {
            conn.execute(
                "INSERT INTO observations (ts, session_id, kind, key, value, count, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
                params![ts, session_id, kind.to_string(), key, value, ts],
            )?;
        }
        Ok(())
    }

    /// Query observations with optional filters.
    pub fn get_observations(&self, filter: &ObservationFilter) -> Result<Vec<Observation>> {
        let conn = self.conn.lock().unwrap();
        let mut sql = String::from(
            "SELECT id, ts, session_id, kind, key, value, count FROM observations WHERE 1=1"
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(ref kind) = filter.kind {
            sql.push_str(&format!(" AND kind = ?{idx}"));
            param_values.push(Box::new(kind.to_string()));
            idx += 1;
        }
        if let Some(ref since) = filter.since {
            sql.push_str(&format!(" AND ts >= ?{idx}"));
            param_values.push(Box::new(since.clone()));
            idx += 1;
        }
        if let Some(ref key) = filter.key {
            sql.push_str(&format!(" AND key LIKE ?{idx}"));
            param_values.push(Box::new(format!("%{key}%")));
            idx += 1;
        }

        sql.push_str(" ORDER BY ts DESC");

        let limit = filter.limit.unwrap_or(200);
        sql.push_str(&format!(" LIMIT ?{idx}"));
        param_values.push(Box::new(limit));

        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params_ref.as_slice(), |row| {
                let kind_str: String = row.get(3)?;
                Ok(Observation {
                    id: row.get(0)?,
                    ts: row.get(1)?,
                    session_id: row.get(2)?,
                    kind: kind_str.parse().unwrap_or(ObservationKind::ToolUse),
                    key: row.get(4)?,
                    value: row.get(5)?,
                    count: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Get aggregated observation summary.
    pub fn get_observation_summary(&self) -> Result<ObservationSummary> {
        let conn = self.conn.lock().unwrap();

        let total: i64 = conn.query_row("SELECT COUNT(*) FROM observations", [], |r| r.get(0))?;

        // Count distinct sessions
        let total_sessions: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT session_id) FROM observations WHERE session_id IS NOT NULL",
            [], |r| r.get(0),
        )?;

        // Top tools (by total count)
        let mut stmt = conn.prepare(
            "SELECT key, SUM(count) as total FROM observations WHERE kind = 'tool_use'
             GROUP BY key ORDER BY total DESC LIMIT 10"
        )?;
        let top_tools: Vec<(String, i64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        // Top paths
        let mut stmt = conn.prepare(
            "SELECT key, SUM(count) as total FROM observations WHERE kind = 'path_access'
             GROUP BY key ORDER BY total DESC LIMIT 10"
        )?;
        let top_paths: Vec<(String, i64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ObservationSummary {
            total_observations: total,
            total_sessions,
            top_tools,
            top_paths,
            last_consolidation: None, // TODO: track this
        })
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

impl Database {
    /// Create a subagent session.
    pub fn create_subagent_session(
        &self,
        parent_id: Uuid,
        model: &str,
        label: &str,
        task: &str,
    ) -> Result<SessionMeta> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4();
        let key = format!("subagent:{}", &id.to_string()[..8]);
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        conn.execute(
            "INSERT INTO sessions (id, key, model, status, token_input, token_output, created_at, updated_at, kind, parent_id, label, task, subagent_status)
             VALUES (?1, ?2, ?3, 'active', 0, 0, ?4, ?4, 'subagent', ?5, ?6, ?7, 'running')",
            params![id.to_string(), key, model, now_str, parent_id.to_string(), label, task],
        )?;
        Ok(SessionMeta {
            id,
            key,
            model: model.to_string(),
            status: SessionStatus::Active,
            token_input: 0,
            token_output: 0,
            created_at: now,
            updated_at: now,
            kind: SessionKind::Subagent {
                parent_id,
                label: label.to_string(),
                task: task.to_string(),
            },
        })
    }

    /// Get all subagent sessions for a parent session.
    pub fn get_subagents(&self, parent_id: Uuid) -> Result<Vec<SubagentInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, key, label, task, subagent_status, created_at, updated_at, summary, token_input, token_output
             FROM sessions WHERE parent_id = ?1 ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map(params![parent_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
            ))
        })?;

        let mut result = Vec::new();
        for row in rows {
            let (id_str, key, label, task, status_str, created, _updated, summary, tok_in, tok_out) = row?;
            let status = match status_str.as_deref() {
                Some("completed") => SubagentStatus::Completed,
                Some("failed") => SubagentStatus::Failed,
                Some("cancelled") => SubagentStatus::Cancelled,
                _ => SubagentStatus::Running,
            };
            result.push(SubagentInfo {
                session_id: id_str.parse().unwrap_or_default(),
                session_key: key,
                label: label.unwrap_or_default(),
                task: task.unwrap_or_default(),
                status,
                started_at: created,
                completed_at: None, // Could track separately
                summary,
                token_input: tok_in,
                token_output: tok_out,
            });
        }
        Ok(result)
    }

    /// Update subagent status and optionally set summary.
    pub fn update_subagent_status(
        &self,
        session_id: Uuid,
        status: SubagentStatus,
        summary: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE sessions SET subagent_status = ?1, summary = ?2, updated_at = ?3 WHERE id = ?4",
            params![status.to_string(), summary, now, session_id.to_string()],
        )?;
        Ok(())
    }
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
        kind: SessionKind::Main, // Default; subagent sessions are identified by DB columns
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
            id: None,
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

    #[test]
    fn test_audit_log_insert_and_query() {
        let db = Database::open_in_memory().unwrap();

        // Insert entries
        let id1 = db.insert_audit_log(
            "2026-02-22T01:00:00Z", Some("session-1"), AuditLevel::Info, AuditCategory::Gateway,
            "gateway_start", "Gateway started", None,
        ).unwrap();
        assert!(id1 > 0);

        db.insert_audit_log(
            "2026-02-22T01:00:01Z", Some("session-1"), AuditLevel::Info, AuditCategory::Agent,
            "agent_spawn", "Agent spawned (pid: 1234)", None,
        ).unwrap();

        db.insert_audit_log(
            "2026-02-22T01:00:02Z", Some("session-1"), AuditLevel::Error, AuditCategory::Tool,
            "tool_error", "fs.read failed", Some(r#"{"path":"/etc/shadow"}"#),
        ).unwrap();

        // Query all
        let all = db.query_audit_log(&AuditFilter::default()).unwrap();
        assert_eq!(all.len(), 3);

        // Query by level
        let errors = db.query_audit_log(&AuditFilter { level: Some(AuditLevel::Error), ..Default::default() }).unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].event, "tool_error");

        // Query by category
        let agent = db.query_audit_log(&AuditFilter { category: Some(AuditCategory::Agent), ..Default::default() }).unwrap();
        assert_eq!(agent.len(), 1);

        // Search
        let search = db.query_audit_log(&AuditFilter { search: Some("spawned".to_string()), ..Default::default() }).unwrap();
        assert_eq!(search.len(), 1);

        // Stats
        let stats = db.get_audit_stats().unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.by_level.info, 2);
        assert_eq!(stats.by_level.error, 1);
        assert_eq!(stats.by_category.gateway, 1);
        assert_eq!(stats.by_category.agent, 1);
        assert_eq!(stats.by_category.tool, 1);
    }

    #[test]
    fn test_observations() {
        let db = Database::open_in_memory().unwrap();

        // Record observations
        db.record_observation(ObservationKind::ToolUse, "fs_read", None, Some("s1")).unwrap();
        db.record_observation(ObservationKind::ToolUse, "fs_read", None, Some("s1")).unwrap(); // should increment
        db.record_observation(ObservationKind::ToolUse, "fs_write", None, Some("s1")).unwrap();
        db.record_observation(ObservationKind::PathAccess, "/tmp/test.txt", Some("fs_read"), Some("s1")).unwrap();

        // Query all
        let all = db.get_observations(&ObservationFilter::default()).unwrap();
        assert_eq!(all.len(), 3); // fs_read was incremented, not duplicated

        // The fs_read observation should have count 2
        let fs_read = all.iter().find(|o| o.key == "fs_read").unwrap();
        assert_eq!(fs_read.count, 2);

        // Query by kind
        let paths = db.get_observations(&ObservationFilter {
            kind: Some(ObservationKind::PathAccess),
            ..Default::default()
        }).unwrap();
        assert_eq!(paths.len(), 1);

        // Summary
        let summary = db.get_observation_summary().unwrap();
        assert_eq!(summary.total_observations, 3);
        assert_eq!(summary.total_sessions, 1);
        assert_eq!(summary.top_tools.len(), 2);
    }
}
