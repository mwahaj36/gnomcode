//! SQLite storage schema, migrations, queries, and rollups (`SPEC §3.6`, `§4.3`).

use chrono::{DateTime, Datelike, Utc};

use gnomcode_core::rules::{GuardrailRule, ResumeQueueEntry};

use gnomcode_core::{
    ActivityEvent, ActivityKind, AgentId, ChatEvent, ChatRole, Project, QuotaSnapshot, QuotaWindow,
    RoutingPolicy, Session, SessionState, Surface, TrustLevel, UsageRecord,
};

use rusqlite::{Connection, OptionalExtension, params};

use sha2::{Digest, Sha256};

use std::path::Path;

use thiserror::Error;

mod automation;

mod conversation;

mod chat;
mod ingest;
mod pricing;
mod projects;
mod quota;
mod relay;
mod report;
mod rules;
mod schema;
mod sessions;
mod settings;
mod trajectory;
mod usage_sync;

pub use automation::{AutomationNotice, PendingPushNotice, ScheduledMessage};

pub use conversation::ConversationLaunch;

pub use ingest::*;
pub use pricing::*;
pub use projects::*;
pub use relay::*;
pub use report::*;
pub use rules::*;
pub use sessions::*;
pub use settings::*;
pub use trajectory::{TrajectoryRows, read_trajectory, trajectory_max_generation};
pub use usage_sync::{CloudSessionMeta, CloudUsageRecord, UsageSyncWatermark};

/// Storage layer error type.
#[derive(Debug, Error)]
pub enum Error {
    /// SQLite driver failure.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Schema migration failure.
    #[error("migration error: {0}")]
    Migration(String),
    /// Invalid stored rule or queue payload.
    #[error("invalid stored JSON: {0}")]
    Json(#[from] serde_json::Error),
}

fn event_marker(id: &str) -> Vec<u8> {
    Sha256::digest(id.as_bytes()).to_vec()
}

fn mark_deleted<P: rusqlite::Params>(
    tx: &rusqlite::Transaction<'_>,
    query: &str,
    params: P,
) -> Result<(), Error> {
    let mut statement = tx.prepare(query)?;
    let ids = statement.query_map(params, |row| row.get::<_, String>(0))?;
    let mut insert =
        tx.prepare_cached("INSERT OR IGNORE INTO deleted_chat_event(event_id) VALUES (?1)")?;
    for id in ids {
        insert.execute([event_marker(&id?)])?;
    }
    Ok(())
}

/// Managed SQLite database handle wrapped with Gnomcode schema & queries.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens or creates the SQLite database at `path` and runs schema migrations.
    /// Configures WAL mode, normal synchronous, and 5-second busy timeout (`SPEC §4.3`).
    ///
    /// # Errors
    /// Returns [`Error`] if opening the connection or applying migrations fails.
    pub fn open(path: &Path) -> Result<Self, Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::Migration(e.to_string()))?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let mut db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Creates an in-memory SQLite database for unit tests.
    ///
    /// # Errors
    /// Returns [`Error`] if initialization or migration fails.
    pub fn open_in_memory() -> Result<Self, Error> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let mut db = Self { conn };
        db.migrate()?;
        Ok(db)
    }
}
