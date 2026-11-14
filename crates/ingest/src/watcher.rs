//! File-system watcher and exponential idle back-off for ingestion (`SPEC §3.5`, `§4.4`).

use crate::{Error, ingest_all};
use gnomcode_platform::{discover_agent_roots, gnomcode_home};
use gnomcode_store::Database;
use notify::{Event, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

/// Exponential idle back-off for background ingestion sweeps.
#[derive(Debug, Clone)]
pub struct IdleBackoff {
    base: Duration,
    max: Duration,
    current: Duration,
}

impl IdleBackoff {
    /// Creates a back-off schedule from `base` up to `max`.
    #[must_use]
    pub fn new(base: Duration, max: Duration) -> Self {
        let max = max.max(base);
        Self {
            base,
            max,
            current: base,
        }
    }

    /// Current wait duration before the next fallback scan.
    #[must_use]
    pub fn current(&self) -> Duration {
        self.current
    }

    /// Resets the interval back to `base` when activity is observed.
    pub fn on_activity(&mut self) {
        self.current = self.base;
    }

    /// Doubles the interval up to `max` after an idle pass.
    pub fn on_idle(&mut self) {
        let doubled = self.current.saturating_mul(2);
        self.current = doubled.min(self.max);
    }
}

/// File watcher + idle back-off coordinator for [`ingest_all`].
pub struct IngestWatcher {
    dirty: Arc<AtomicBool>,
    wakeup: Arc<Notify>,
    backoff: IdleBackoff,
    last_scan: Option<Instant>,
    _watcher: Option<notify::RecommendedWatcher>,
}

impl IngestWatcher {
    /// Starts watching discovered agent log directories and `GNOMCODE_HOME`.
    /// Falls back to periodic back-off scans if the OS file watcher cannot be created.
    #[must_use]
    pub fn new(base_interval: Duration, max_idle_interval: Duration) -> Self {
        let mut paths: Vec<PathBuf> = discover_agent_roots()
            .into_iter()
            .filter(|r| r.exists)
            .map(|r| r.path)
            .collect();
        let home = gnomcode_home();
        let mut watched: Vec<(PathBuf, RecursiveMode)> = paths
            .drain(..)
            .map(|p| (p, RecursiveMode::Recursive))
            .collect();
        // Only top-level files (status-line limits) matter in the Gnomcode home; recursive
        // watching would pick up worktrees and our own SQLite writes.
        // `sessions/` holds launched PTY records the daemon binds to conversations.
        let pty_sessions = home.join("sessions");
        if pty_sessions.exists() {
            watched.push((pty_sessions, RecursiveMode::NonRecursive));
        }
        if home.exists() {
            watched.push((home, RecursiveMode::NonRecursive));
        }
        Self::watch(&watched, base_interval, max_idle_interval)
    }

    /// Starts watching an explicit list of directories/files.
    #[must_use]
    pub fn with_paths(
        paths: &[PathBuf],
        base_interval: Duration,
        max_idle_interval: Duration,
    ) -> Self {
        let watched: Vec<(PathBuf, RecursiveMode)> = paths
            .iter()
            .map(|p| (p.clone(), RecursiveMode::Recursive))
            .collect();
        Self::watch(&watched, base_interval, max_idle_interval)
    }

    fn watch(
        paths: &[(PathBuf, RecursiveMode)],
        base_interval: Duration,
        max_idle_interval: Duration,
    ) -> Self {
        let dirty = Arc::new(AtomicBool::new(true));
        let wakeup = Arc::new(Notify::new());
        let dirty_cb = Arc::clone(&dirty);
        let wakeup_cb = Arc::clone(&wakeup);

        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            // Fail open: a watcher error marks dirty so the next tick rescans.
            if res.as_ref().is_ok_and(is_own_db_write) {
                return;
            }
            dirty_cb.store(true, Ordering::Release);
            wakeup_cb.notify_one();
        })
        .ok()
        .and_then(|mut w| {
            let mut watching = 0;
            for (path, mode) in paths {
                if path.exists() && w.watch(path, *mode).is_ok() {
                    watching += 1;
                }
            }
            (watching > 0).then_some(w)
        });

        Self {
            dirty,
            wakeup,
            backoff: IdleBackoff::new(base_interval, max_idle_interval),
            last_scan: None,
            _watcher: watcher,
        }
    }

    /// Marks the watcher dirty and wakes the loop immediately.
    pub fn trigger(&self) {
        self.dirty.store(true, Ordering::Release);
        self.wakeup.notify_one();
    }

    /// Whether an OS watcher is active on at least one path. When `false`, callers
    /// should fall back to plain polling.
    #[must_use]
    pub fn is_watching(&self) -> bool {
        self._watcher.is_some()
    }

    /// Returns `Some(was_dirty)` when a scan is due (filesystem event or back-off
    /// elapsed) and clears the dirty flag; `None` while idle.
    pub fn take_due(&mut self) -> Option<bool> {
        let was_dirty = self.dirty.swap(false, Ordering::AcqRel);
        let elapsed = self
            .last_scan
            .is_none_or(|t| t.elapsed() >= self.backoff.current());
        (was_dirty || elapsed).then_some(was_dirty)
    }

    /// Records a finished scan: `activity` resets the back-off, otherwise it doubles.
    pub fn record_scan(&mut self, activity: bool) {
        self.last_scan = Some(Instant::now());
        if activity {
            self.backoff.on_activity();
        } else {
            self.backoff.on_idle();
        }
    }

    /// Current idle back-off interval.
    #[must_use]
    pub fn current_interval(&self) -> Duration {
        self.backoff.current()
    }

    /// Waits until either a filesystem event arrives or the idle back-off elapses.
    pub async fn wait_next(&self) {
        if self.dirty.load(Ordering::Acquire) {
            return;
        }
        let sleep_dur = self.backoff.current();
        tokio::select! {
            () = self.wakeup.notified() => {}
            () = tokio::time::sleep(sleep_dur) => {}
        }
    }

    /// Runs [`ingest_all`] only if filesystem events arrived or the back-off elapsed.
    /// Returns `Ok(Some(count))` when a scan ran, or `Ok(None)` when skipped while idle.
    ///
    /// # Errors
    /// Returns [`Error`] if [`ingest_all`] fails.
    pub fn poll_if_needed(&mut self, db: &mut Database) -> Result<Option<u64>, Error> {
        let Some(was_dirty) = self.take_due() else {
            return Ok(None);
        };
        let ingested = ingest_all(db)?;
        self.record_scan(ingested > 0 || was_dirty);
        Ok(Some(ingested))
    }
}

/// SQLite files written by Gnomcode itself must not wake ingestion (feedback loop).
fn is_own_db_write(event: &Event) -> bool {
    !event.paths.is_empty()
        && event.paths.iter().all(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains(".db"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn idle_backoff_doubles_to_cap_and_resets_on_activity() {
        let mut backoff = IdleBackoff::new(Duration::from_secs(2), Duration::from_secs(30));
        assert_eq!(backoff.current(), Duration::from_secs(2));
        backoff.on_idle();
        assert_eq!(backoff.current(), Duration::from_secs(4));
        backoff.on_idle();
        assert_eq!(backoff.current(), Duration::from_secs(8));
        backoff.on_idle();
        assert_eq!(backoff.current(), Duration::from_secs(16));
        backoff.on_idle();
        assert_eq!(backoff.current(), Duration::from_secs(30));
        backoff.on_idle();
        assert_eq!(backoff.current(), Duration::from_secs(30));
        backoff.on_activity();
        assert_eq!(backoff.current(), Duration::from_secs(2));
    }

    #[tokio::test]
    async fn ingest_watcher_wakes_on_file_write_and_skips_idle_rescan() {
        const CHILD: &str = "GNOMCODE_WATCHER_TEST_CHILD";
        const NAME: &str =
            "watcher::tests::ingest_watcher_wakes_on_file_write_and_skips_idle_rescan";
        if std::env::var_os(CHILD).is_none() {
            // `ingest_all` discovers agent roots from the environment. Environment mutation
            // is unsafe in-process, so rerun this test in a child with every root redirected
            // to a temp dir; it must never read the real home directory.
            let home = tempdir().expect("home");
            let output = std::process::Command::new(std::env::current_exe().expect("exe"))
                .args(["--exact", NAME, "--test-threads=1"])
                .env(CHILD, "1")
                .env("HOME", home.path())
                .env("USERPROFILE", home.path())
                .env("GNOMCODE_HOME", home.path().join("gnomcode"))
                .env("SPAWNT_HOME", home.path().join("gnomcode"))
                .env("CLAUDE_CONFIG_DIR", home.path().join("claude"))
                .env("CODEX_HOME", home.path().join("codex"))
                .env("GEMINI_HOME", home.path().join("gemini"))
                .output()
                .expect("spawn child test");
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success() && stdout.contains("1 passed"),
                "child failed: {stdout}{}",
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("ingest.db");
        let mut db = Database::open(&db_path).expect("db");
        let watch_dir = dir.path().join("logs");
        fs::create_dir_all(&watch_dir).expect("mkdir");

        let mut watcher = IngestWatcher::with_paths(
            std::slice::from_ref(&watch_dir),
            Duration::from_secs(2),
            Duration::from_secs(30),
        );

        // Initial pass runs because dirty starts true.
        assert!(watcher.poll_if_needed(&mut db).expect("first").is_some());
        // Immediate second pass while clean and within backoff skips scanning.
        assert_eq!(watcher.poll_if_needed(&mut db).expect("idle"), None);

        // Write a file and verify the OS watcher marks dirty / wakes wait_next.
        fs::write(watch_dir.join("rollout-test.jsonl"), "{}\n").expect("write");
        tokio::time::timeout(Duration::from_secs(3), watcher.wait_next())
            .await
            .expect("watcher should wake on file write");
        assert!(watcher.poll_if_needed(&mut db).expect("dirty").is_some());
    }
}
