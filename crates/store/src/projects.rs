//! Projects and project reports.

use super::*;

/// Local-only project usage rollup; repository identity is not published to cloud.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProjectUsage {
    /// Priced invocation count.
    #[serde(default)]
    pub priced_turns: u64,
    /// Unpriced invocation count; cost is partial when nonzero.
    #[serde(default)]
    pub unpriced_turns: u64,
    /// Stable canonical project ID.
    pub id: String,
    /// Local project display name.
    pub name: String,
    /// Measured tokens with cache counted once.
    pub total_tokens: u64,
    /// API-equivalent spend.
    pub cost_usd: f64,
    /// Pricing trust; unknown if any invocation is unpriced.
    pub cost_basis: TrustLevel,
    /// Number of sessions with recorded usage in the selected period.
    pub sessions: u64,
    /// Local folder of the project; empty for shared scratch buckets. Never synced to cloud.
    #[serde(default)]
    pub repo_root: String,
    /// Timestamp of the latest recorded usage in the selected period.
    #[serde(default)]
    pub last_active: Option<String>,
    /// Agent identifiers with recorded usage in the selected period.
    #[serde(default)]
    pub agents: Vec<String>,
}

pub(crate) fn antigravity_workspace(root: &std::path::Path, conv_id: &str) -> Option<String> {
    [
        obfstr::obfstr!("antigravity-cli"),
        obfstr::obfstr!("antigravity"),
    ]
    .iter()
    .find_map(|product| {
        let path = root
            .join(product)
            .join(obfstr::obfstr!("conversation_summaries.db"));
        let conn =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .ok()?;
        let uris: String = conn
            .query_row(
                obfstr::obfstr!(
                    "SELECT workspace_uris FROM conversation_summaries WHERE conversation_id = ?1"
                ),
                params![conv_id],
                |row| row.get(0),
            )
            .ok()?;
        let value: serde_json::Value = serde_json::from_str(&uris).ok()?;
        let uri = value
            .as_array()?
            .first()?
            .as_str()?
            .replace("%3A", ":")
            .replace("%3a", ":");
        let path = uri.strip_prefix("file://").unwrap_or(&uri);
        let path = if path.as_bytes().get(2) == Some(&b':') {
            path.strip_prefix('/').unwrap_or(path)
        } else {
            path
        };
        Some(path.to_owned())
    })
}

impl Database {
    /// Every stored project, so derived identity can be recomputed.
    ///
    /// # Errors
    /// Returns [`Error`] if the query fails.
    pub fn list_projects(&self) -> Result<Vec<gnomcode_core::Project>, Error> {
        let mut statement = self
            .conn
            .prepare("SELECT id, canonical_key, repo_root, name, git_branch FROM project")?;
        let rows = statement.query_map([], |row| {
            Ok(gnomcode_core::Project {
                id: row.get(0)?,
                canonical_key: row.get(1)?,
                repo_root: row.get(2)?,
                name: row.get(3)?,
                git_branch: row.get(4)?,
            })
        })?;
        rows.collect::<Result<_, _>>().map_err(Error::from)
    }

    /// Moves every session of project `old_id` to `project`, merging into an existing row with
    /// the same canonical key. Only derived project fields change; sessions, usage and chat are kept.
    ///
    /// # Errors
    /// Returns [`Error`] if the transaction fails.
    pub fn reassign_project(
        &mut self,
        old_id: &str,
        project: &gnomcode_core::Project,
    ) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        let target: Option<String> = tx
            .query_row(
                "SELECT id FROM project WHERE canonical_key = ?1",
                params![project.canonical_key],
                |row| row.get(0),
            )
            .optional()?;
        match target {
            Some(id) if id == old_id => {
                tx.execute(
                    "UPDATE project SET name = ?2, repo_root = ?3 WHERE id = ?1",
                    params![old_id, project.name, project.repo_root],
                )?;
            }
            Some(id) => {
                tx.execute(
                    "UPDATE session SET project_id = ?1 WHERE project_id = ?2",
                    params![id, old_id],
                )?;
                tx.execute("DELETE FROM project WHERE id = ?1", params![old_id])?;
            }
            None => {
                tx.execute(
                    "INSERT INTO project (id, canonical_key, repo_root, name, git_branch) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        project.id,
                        project.canonical_key,
                        project.repo_root,
                        project.name,
                        project.git_branch
                    ],
                )?;
                tx.execute(
                    "UPDATE session SET project_id = ?1 WHERE project_id = ?2",
                    params![project.id, old_id],
                )?;
                tx.execute("DELETE FROM project WHERE id = ?1", params![old_id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Reconciles Antigravity session projects by querying the local Antigravity
    /// metadata SQLite database (`~/.gemini/antigravity/conversation_summaries.db`).
    ///
    /// # Errors
    /// Returns [`Error`] if querying or updating the local SQLite store fails.
    pub fn reconcile_antigravity_projects(&mut self) -> Result<usize, Error> {
        let gemini_base = std::env::var("GEMINI_HOME")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map_or_else(
                || {
                    let home = std::env::var("USERPROFILE")
                        .or_else(|_| std::env::var("HOME"))
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|_| std::path::PathBuf::from("."));
                    home.join(".gemini")
                },
                std::path::PathBuf::from,
            );

        let agy_db_path = gemini_base
            .join("antigravity")
            .join("conversation_summaries.db");

        if !agy_db_path.exists() {
            return Ok(0);
        }

        let agy_conn = match rusqlite::Connection::open_with_flags(
            &agy_db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        ) {
            Ok(c) => c,
            Err(_) => return Ok(0),
        };

        // Antigravity's own summaries are authoritative for subagent links; repair any drift.
        if let Ok(mut links) = agy_conn
            .prepare("SELECT conversation_id, parent_conversation_id FROM conversation_summaries")
        {
            let rows: Vec<(String, String)> = links
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .filter_map(Result::ok)
                .collect();
            for (conversation, parent) in rows {
                let parent = (!parent.trim().is_empty()).then(|| format!("antigravity:{parent}"));
                self.conn.execute(
                    "UPDATE session SET parent_session_id = ?1 WHERE id = ?2 AND parent_session_id IS NOT ?1",
                    params![parent, format!("antigravity:{conversation}")],
                )?;
            }
        }

        let mut stmt = self.conn.prepare(
            r#"
            SELECT s.id, p.name
            FROM session s
            LEFT JOIN project p ON s.project_id = p.id
            WHERE s.agent = 'antigravity'
            "#,
        )?;

        let sessions: Vec<(String, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(Result::ok)
            .collect();

        let mut updated = 0;
        let mut agy_stmt = match agy_conn.prepare(
            "SELECT title, workspace_uris FROM conversation_summaries WHERE conversation_id = ?1",
        ) {
            Ok(s) => s,
            Err(_) => return Ok(0),
        };

        for (sid, current_project) in sessions {
            let conv_id = sid.strip_prefix("antigravity:").unwrap_or(&sid);
            if let Ok(mut rows) = agy_stmt.query(params![conv_id])
                && let Ok(Some(row)) = rows.next()
            {
                let title: Option<String> =
                    row.get(0).ok().filter(|s: &String| !s.trim().is_empty());
                let uris_json: Option<String> = row.get(1).ok();
                let mut detected_ws = None;
                if let Some(raw_uris) = uris_json
                    && let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw_uris)
                    && let Some(arr) = val.as_array()
                    && let Some(first) = arr.first().and_then(|v| v.as_str())
                {
                    let unquoted = first.replace("%3A", ":").replace("%3a", ":");
                    let clean = unquoted
                        .strip_prefix("file:///")
                        .or_else(|| unquoted.strip_prefix("file://"))
                        .unwrap_or(&unquoted);
                    detected_ws = Some(clean.to_owned());
                }

                // A session that already has a project keeps it; ingest resolves it to the repo root.
                let unassigned = current_project
                    .as_deref()
                    .is_none_or(|name| name == "unknown");
                if let Some(ws) = detected_ws.filter(|_| unassigned) {
                    let project = gnomcode_core::Project::from_origin(None, &ws, None);
                    let _ = self.conn.execute(
                        r#"
                            INSERT INTO project (id, canonical_key, repo_root, name, git_branch)
                            VALUES (?1, ?2, ?3, ?4, ?5)
                            ON CONFLICT(canonical_key) DO UPDATE SET
                                name = excluded.name,
                                repo_root = excluded.repo_root,
                                git_branch = coalesce(excluded.git_branch, project.git_branch)
                            "#,
                        params![
                            project.id,
                            project.canonical_key,
                            project.repo_root,
                            project.name,
                            project.git_branch
                        ],
                    );

                    let proj_id: String = self
                        .conn
                        .query_row(
                            "SELECT id FROM project WHERE canonical_key = ?1",
                            params![project.canonical_key],
                            |r| r.get(0),
                        )
                        .unwrap_or(project.id);

                    let _ = self.conn.execute(
                            "UPDATE session SET project_id = ?1, title = COALESCE(?2, title) WHERE id = ?3",
                            params![proj_id, title, sid],
                        );
                    updated += 1;
                } else if title.is_some() {
                    let _ = self.conn.execute(
                        "UPDATE session SET title = ?1 WHERE id = ?2 AND title IS NOT ?1",
                        params![title, sid],
                    );
                    updated += 1;
                }
            }
        }

        Ok(updated)
    }

    /// Queries the workspace path for a specific Antigravity conversation from
    /// the IDE and CLI summary databases under `GEMINI_HOME` (default `~/.gemini`).
    #[must_use]
    pub fn find_antigravity_workspace(&self, conv_id: &str) -> Option<String> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."));

        let root = std::env::var_os("GEMINI_HOME")
            .map_or_else(|| home.join(".gemini"), std::path::PathBuf::from);
        antigravity_workspace(&root, conv_id)
    }

    /// Returns local project rollups using the same time bounds as the timeline report.
    ///
    /// # Errors
    /// Returns an error if timeline or project query execution fails.
    pub fn generate_project_report(
        &self,
        range: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Vec<ProjectUsage>, Error> {
        let timeline = self.generate_usage_report_timeline(range, from, to)?;
        if timeline.items_count == 0 {
            return Ok(Vec::new());
        }
        let mut statement = self.conn.prepare(obfstr::obfstr!(r#"
            SELECT p.id, p.name,
                SUM(u.input_tokens + u.output_tokens + CASE WHEN s.agent IN ('codex', 'gemini-cli') THEN 0
                    ELSE u.cache_read_tokens + u.cache_write_5m_tokens + u.cache_write_1h_tokens END),
                SUM(CASE WHEN u.cost_basis = 'unknown' THEN 0 ELSE u.cost_usd END), COUNT(DISTINCT s.id), SUM(CASE WHEN u.cost_basis = 'unknown' THEN 1 ELSE 0 END), COUNT(*),
                CASE WHEN p.canonical_key LIKE 'scratch/%' THEN '' ELSE p.repo_root END, MAX(u.timestamp), GROUP_CONCAT(DISTINCT s.agent)
            FROM usage_record u JOIN session s ON s.id = u.session_id JOIN project p ON p.id = s.project_id
            WHERE s.surface <> 'cloud' AND p.canonical_key <> '/unknown' AND julianday(u.timestamp) >= julianday(?1) AND julianday(u.timestamp) <= julianday(?2)
            GROUP BY p.id ORDER BY SUM(CASE WHEN u.cost_basis = 'unknown' THEN 0 ELSE u.cost_usd END) DESC, p.id
        "#))?;
        let rows =
            statement.query_map(params![timeline.earliest_at, timeline.latest_at], |row| {
                let tokens: i64 = row.get(2)?;
                let sessions: i64 = row.get(4)?;
                let unpriced: i64 = row.get(5)?;
                let agents: Option<String> = row.get(9)?;
                Ok(ProjectUsage {
                    repo_root: row.get(7)?,
                    last_active: row.get(8)?,
                    agents: agents
                        .iter()
                        .flat_map(|list| list.split(','))
                        .map(str::to_owned)
                        .collect(),
                    priced_turns: (row.get::<_, i64>(6)? - unpriced).max(0) as u64,
                    unpriced_turns: unpriced.max(0) as u64,
                    id: row.get(0)?,
                    name: row.get(1)?,
                    total_tokens: tokens.max(0) as u64,
                    cost_usd: row.get(3)?,
                    sessions: sessions.max(0) as u64,
                    cost_basis: if unpriced > 0 {
                        TrustLevel::Unknown
                    } else {
                        TrustLevel::Estimated
                    },
                })
            })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gnomcode_core::Project;

    #[test]
    fn antigravity_workspace_lookup_includes_cli_and_preserves_ide() {
        let root = tempfile::tempdir().expect("gemini home");
        for (product, id) in [("antigravity", "ide-test"), ("antigravity-cli", "cli-test")] {
            let directory = root.path().join(product);
            std::fs::create_dir_all(&directory).expect("directory");
            let db = rusqlite::Connection::open(directory.join("conversation_summaries.db"))
                .expect("summary db");
            db.execute_batch(
                "CREATE TABLE conversation_summaries(conversation_id TEXT, workspace_uris TEXT);",
            )
            .expect("schema");
            db.execute(
                "INSERT INTO conversation_summaries VALUES (?1, ?2)",
                rusqlite::params![id, r#"["file:///C:/test-workspace"]"#],
            )
            .expect("summary");
        }
        for id in ["ide-test", "cli-test"] {
            assert_eq!(
                super::antigravity_workspace(root.path(), id).as_deref(),
                Some("C:/test-workspace")
            );
        }
        assert_eq!(super::antigravity_workspace(root.path(), "missing"), None);
    }

    #[test]
    fn project_report_lists_folder_agents_and_last_activity_but_not_unassigned_sessions() {
        let db = heatmap_db(&["2026-11-10T10:00:00Z", "2026-11-11T12:00:00Z"]);
        let unassigned = Project::from_origin(None, "C:/", None);
        assert!(unassigned.is_unassigned());
        db.conn
            .execute(
                "INSERT INTO project(id,canonical_key,repo_root,name) VALUES (?1,?2,?3,?4)",
                rusqlite::params![
                    unassigned.id,
                    unassigned.canonical_key,
                    unassigned.repo_root,
                    unassigned.name
                ],
            )
            .expect("unassigned project");
        db.conn
            .execute(
                "INSERT INTO session(id,agent,surface,project_id,state,started_at,last_event_at) VALUES ('codex:none','codex','cli',?1,'idle','2026-11-11T00:00:00Z','2026-11-11T00:00:00Z')",
                rusqlite::params![unassigned.id],
            )
            .expect("unassigned session");
        db.conn
            .execute(
                "INSERT INTO usage_record VALUES ('none:1','codex:none','2026-11-11T00:00:00Z','gpt',5,5,0,0,0,0,10,NULL,1.0,0.0,'exact','v')",
                [],
            )
            .expect("unassigned usage");
        let projects = db
            .generate_project_report("all", None, None)
            .expect("project rollups");
        assert_eq!(
            projects.len(),
            1,
            "sessions without a folder are not a project"
        );
        assert_eq!(projects[0].repo_root, "/workspace/gnomcode");
        assert_eq!(projects[0].agents, ["antigravity"]);
        assert!(
            projects[0]
                .last_active
                .as_deref()
                .is_some_and(|stamp| stamp.starts_with("2026-11-11T12:00:00"))
        );
    }

    #[test]
    fn reassigning_a_project_merges_sessions_and_keeps_them() {
        let mut db = heatmap_db(&["2026-11-10T10:00:00Z"]);
        let old = db.list_projects().expect("projects").remove(0);
        let renamed = Project::from_origin(None, "/workspace/renamed", None);
        db.reassign_project(&old.id, &renamed).expect("move");
        let projects = db.list_projects().expect("projects");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "renamed");
        let (rows, total) = db
            .list_sessions_filtered(None, None, Some(&renamed.id), 0, 10)
            .expect("sessions");
        assert_eq!((total, rows[0].project_name.as_str()), (1, "renamed"));
        db.reassign_project(&renamed.id, &renamed)
            .expect("same key");
        assert_eq!(db.list_projects().expect("projects").len(), 1);
    }
}
