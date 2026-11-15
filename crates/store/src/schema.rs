//! Schema migrations.

use super::*;

impl Database {
    pub(crate) fn migrate(&mut self) -> Result<(), Error> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS project (
                id TEXT PRIMARY KEY,
                canonical_key TEXT NOT NULL UNIQUE,
                repo_root TEXT NOT NULL,
                name TEXT NOT NULL,
                git_branch TEXT
            );

            CREATE TABLE IF NOT EXISTS session (
                id TEXT PRIMARY KEY,
                agent TEXT NOT NULL,
                surface TEXT NOT NULL,
                project_id TEXT NOT NULL REFERENCES project(id),
                parent_session_id TEXT,
                model TEXT,
                title TEXT,
                state TEXT NOT NULL,
                pid INTEGER,
                started_at TEXT NOT NULL,
                last_event_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_session_project ON session(project_id);
            CREATE INDEX IF NOT EXISTS idx_session_last_event ON session(last_event_at);

            CREATE TABLE IF NOT EXISTS usage_record (
                dedup_key TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES session(id),
                timestamp TEXT NOT NULL,
                model TEXT NOT NULL,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                reasoning_tokens INTEGER NOT NULL,
                cache_read_tokens INTEGER NOT NULL,
                cache_write_5m_tokens INTEGER NOT NULL,
                cache_write_1h_tokens INTEGER NOT NULL,
                context_tokens INTEGER NOT NULL,
                context_window INTEGER,
                cost_usd REAL NOT NULL,
                cache_savings_usd REAL NOT NULL,
                cost_basis TEXT NOT NULL,
                pricing_version TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_usage_session ON usage_record(session_id);
            CREATE INDEX IF NOT EXISTS idx_usage_timestamp ON usage_record(timestamp);

            CREATE TABLE IF NOT EXISTS activity_event (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES session(id),
                timestamp TEXT NOT NULL,
                kind TEXT NOT NULL,
                tool_name TEXT,
                summary_redacted TEXT,
                duration_ms INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_activity_session ON activity_event(session_id);
            CREATE INDEX IF NOT EXISTS idx_activity_timestamp ON activity_event(timestamp);

            CREATE TABLE IF NOT EXISTS quota_snapshot (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider TEXT NOT NULL,
                limit_id TEXT NOT NULL,
                window TEXT NOT NULL,
                used_pct REAL NOT NULL,
                resets_at TEXT,
                plan TEXT,
                basis TEXT NOT NULL,
                observed_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_quota_lookup ON quota_snapshot(provider, limit_id, window, observed_at);

            CREATE TABLE IF NOT EXISTS quota_pool_member (
                provider TEXT NOT NULL,
                limit_id TEXT NOT NULL,
                member TEXT NOT NULL,
                PRIMARY KEY (provider, limit_id, member)
            );

            CREATE TABLE IF NOT EXISTS source_file (
                file_path TEXT PRIMARY KEY,
                last_read_offset INTEGER NOT NULL,
                file_size INTEGER NOT NULL,
                fingerprint TEXT NOT NULL,
                last_ingested_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS user_profile (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                display_name TEXT NOT NULL,
                openai_plan_usd REAL NOT NULL DEFAULT 20.0,
                claude_plan_usd REAL NOT NULL DEFAULT 20.0,
                google_plan_usd REAL NOT NULL DEFAULT 0.0,
                other_plan_usd REAL NOT NULL DEFAULT 0.0,
                updated_at TEXT NOT NULL
            );

            INSERT OR IGNORE INTO user_profile (id, username, display_name, openai_plan_usd, claude_plan_usd, google_plan_usd, other_plan_usd, updated_at)
            VALUES ('local_user', 'developer', 'Local Developer', 20.0, 20.0, 0.0, 0.0, datetime('now'));

            CREATE TABLE IF NOT EXISTS custom_model_pricing (
                model_pattern TEXT PRIMARY KEY,
                input_per_mtok_usd REAL NOT NULL,
                output_per_mtok_usd REAL NOT NULL,
                cache_read_per_mtok_usd REAL NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )?;

        // The folder a session itself ran in; projects can merge folders, so it is stored per session.
        if !self
            .conn
            .prepare("SELECT 1 FROM pragma_table_info('session') WHERE name = 'cwd'")?
            .exists([])?
        {
            self.conn
                .execute("ALTER TABLE session ADD COLUMN cwd TEXT", [])?;
        }

        let has_v5: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = 5",
                [],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !has_v5 {
            self.conn.execute_batch(
                r#"
                DELETE FROM source_file;
                DELETE FROM activity_event;
                INSERT INTO schema_migrations (version, applied_at) VALUES (5, datetime('now'));
                "#,
            )?;
        }

        let has_v6: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = 6",
                [],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !has_v6 {
            self.conn.execute_batch(
                r#"
                UPDATE usage_record SET model = 'unknown' WHERE model LIKE '%PLACEHOLDER%';
                UPDATE session SET model = NULL WHERE model LIKE '%PLACEHOLDER%';
                DELETE FROM quota_snapshot WHERE limit_id LIKE '%placeholder%';
                INSERT INTO schema_migrations (version, applied_at) VALUES (6, datetime('now'));
                "#,
            )?;
            let _ = self.reconcile_antigravity_projects();
        }

        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS rule (
                id TEXT PRIMARY KEY,
                payload TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS alert (
                session_id TEXT NOT NULL,
                rule_id TEXT NOT NULL,
                action TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (session_id, rule_id)
            );
            CREATE TABLE IF NOT EXISTS resume_queue (
                session_id TEXT PRIMARY KEY,
                agent TEXT NOT NULL,
                cwd TEXT NOT NULL,
                resume_prompt TEXT NOT NULL,
                max_runs INTEGER NOT NULL,
                runs_completed INTEGER NOT NULL,
                eligible_after TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS context_relay (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                source_agent TEXT NOT NULL,
                target_agent TEXT NOT NULL,
                project_name TEXT NOT NULL,
                cwd TEXT NOT NULL,
                fill_pct INTEGER NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                executed_at TEXT
            );
            CREATE TABLE IF NOT EXISTS context_relay_config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                policy TEXT NOT NULL DEFAULT 'ask_first',
                strategy TEXT NOT NULL DEFAULT 'same_agent',
                threshold_pct INTEGER NOT NULL DEFAULT 90,
                updated_at TEXT NOT NULL
            );
            "#,
        )?;

        let has_v7: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = 7",
                [],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !has_v7 {
            self.conn.execute_batch(
                r#"
                UPDATE usage_record
                SET context_tokens = input_tokens + cache_read_tokens
                WHERE cache_read_tokens > 0 AND context_tokens < (input_tokens + cache_read_tokens);
                INSERT INTO schema_migrations (version, applied_at) VALUES (7, datetime('now'));
                "#,
            )?;
        }

        let has_v8: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = 8",
                [],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !has_v8 {
            self.conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS chat_event (
                    event_id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL REFERENCES session(id) ON DELETE CASCADE,
                    turn_index INTEGER NOT NULL,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL,
                    tool_name TEXT,
                    status TEXT,
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_chat_event_session ON chat_event(session_id, turn_index);
                INSERT INTO schema_migrations (version, applied_at) VALUES (8, datetime('now'));
                "#,
            )?;
        }

        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS conversation_member (
                host_id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                native_id TEXT UNIQUE,
                position INTEGER NOT NULL,
                needs_context INTEGER NOT NULL DEFAULT 0,
                agent TEXT, repo TEXT, requested_model TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_conversation_member ON conversation_member(conversation_id, position);
            INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (9, datetime('now'));
        "#)?;
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS deleted_chat_event (event_id BLOB PRIMARY KEY);
            INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (10, datetime('now'));
        "#)?;
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS scheduled_message (
                id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL, text TEXT NOT NULL,
                due_at INTEGER NOT NULL, status TEXT NOT NULL, created_at INTEGER NOT NULL, attempted_at INTEGER, finished_at INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_scheduled_due ON scheduled_message(status,due_at);
            CREATE TABLE IF NOT EXISTS automation_notice (
                id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL, message TEXT NOT NULL,
                created_at INTEGER NOT NULL, notified INTEGER NOT NULL DEFAULT 0
            );
            INSERT OR IGNORE INTO schema_migrations(version,applied_at) VALUES (12,datetime('now'));
        "#)?;
        let push_column_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=14)",
            [],
            |row| row.get(0),
        )?;
        if !push_column_exists {
            self.conn.execute_batch("BEGIN; ALTER TABLE automation_notice ADD COLUMN pushed INTEGER NOT NULL DEFAULT 0; INSERT INTO schema_migrations(version,applied_at) VALUES (14,datetime('now')); COMMIT;")?;
        }
        let v15_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=15)",
            [],
            |row| row.get(0),
        )?;
        if !v15_exists {
            self.conn.execute_batch(
                r#"BEGIN;
                CREATE TABLE IF NOT EXISTS notification_settings (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    approval_waiting INTEGER NOT NULL DEFAULT 1,
                    turn_finished INTEGER NOT NULL DEFAULT 1,
                    turn_finished_min_minutes INTEGER NOT NULL DEFAULT 2,
                    limit_low INTEGER NOT NULL DEFAULT 1,
                    limit_reset INTEGER NOT NULL DEFAULT 1,
                    updated_at TEXT NOT NULL
                );
                ALTER TABLE automation_notice ADD COLUMN push_payload TEXT;
                INSERT INTO schema_migrations(version,applied_at) VALUES (15,datetime('now'));
                COMMIT;"#,
            )?;
        }
        let v16_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=16)",
            [],
            |row| row.get(0),
        )?;
        if !v16_exists {
            // Antigravity titles drifted to the latest prompt; restore the first user prompt.
            self.conn.execute_batch(
                r#"BEGIN;
                UPDATE session SET title = (
                    SELECT substr(trim(CASE WHEN instr(t.c, char(10)) > 0
                        THEN substr(t.c, 1, instr(t.c, char(10)) - 1) ELSE t.c END), 1, 80)
                    FROM (SELECT ltrim(content, char(9,10,13,32)) AS c FROM chat_event
                        WHERE session_id = session.id AND role = 'user'
                          AND trim(content) <> '' AND content NOT LIKE '<%'
                        ORDER BY turn_index LIMIT 1) t)
                WHERE agent = 'antigravity' AND parent_session_id IS NULL
                  AND EXISTS (SELECT 1 FROM chat_event e WHERE e.session_id = session.id
                      AND e.role = 'user' AND trim(e.content) <> '' AND e.content NOT LIKE '<%');
                INSERT INTO schema_migrations(version,applied_at) VALUES (16,datetime('now'));
                COMMIT;"#,
            )?;
        }
        self.conn.execute_batch("CREATE TABLE IF NOT EXISTS desktop_preferences (key TEXT PRIMARY KEY, value TEXT NOT NULL);")?;
        // Conversations the person deleted from Gnomcode. The agent keeps its own history, so
        // this marker is what stops a re-read of that history from listing them again.
        self.conn.execute_batch("CREATE TABLE IF NOT EXISTS hidden_conversation (id TEXT PRIMARY KEY, hidden_at INTEGER NOT NULL);")?;
        self.conn.execute_batch(r#"
            DELETE FROM usage_record
            WHERE session_id IN (
                SELECT id FROM session
                WHERE agent = 'codex'
                  AND (id LIKE 'codex:ctc_%' OR id LIKE 'codex:fc_%' OR id LIKE 'codex:ctco_%' OR id LIKE 'codex:call_%' OR id LIKE 'codex:rs_%' OR id LIKE 'codex:msg_%')
            );
            DELETE FROM activity_event
            WHERE session_id IN (
                SELECT id FROM session
                WHERE agent = 'codex'
                  AND (id LIKE 'codex:ctc_%' OR id LIKE 'codex:fc_%' OR id LIKE 'codex:ctco_%' OR id LIKE 'codex:call_%' OR id LIKE 'codex:rs_%' OR id LIKE 'codex:msg_%')
            );
            DELETE FROM chat_event
            WHERE session_id IN (
                SELECT id FROM session
                WHERE agent = 'codex'
                  AND (id LIKE 'codex:ctc_%' OR id LIKE 'codex:fc_%' OR id LIKE 'codex:ctco_%' OR id LIKE 'codex:call_%' OR id LIKE 'codex:rs_%' OR id LIKE 'codex:msg_%')
            );
            DELETE FROM session
            WHERE agent = 'codex'
              AND (id LIKE 'codex:ctc_%' OR id LIKE 'codex:fc_%' OR id LIKE 'codex:ctco_%' OR id LIKE 'codex:call_%' OR id LIKE 'codex:rs_%' OR id LIKE 'codex:msg_%');

            -- Clean up self-referencing subagents where background tasks set parent to self
            UPDATE session SET parent_session_id = NULL WHERE parent_session_id = id;
            UPDATE session SET title = NULL WHERE title = 'Subagent' AND parent_session_id IS NULL;
        "#)?;
        Ok(())
    }
}
