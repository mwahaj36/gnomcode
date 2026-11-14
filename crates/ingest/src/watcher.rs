//! File-system watcher and exponential idle back-off for ingestion (`SPEC §3.5`, `§4.4`).

use crate::{Error, ingest_all};
use gnomcode_platform::{discover_agent_roots, gnomcode_home};
use gnomcode_store::Database;
use notify::event::ModifyKind;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
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

/// Roots shared with the watcher callback so it can classify events.
#[derive(Default)]
struct Shared {
    /// Roots that exist and are watched directly.
    active: Vec<PathBuf>,
    /// Roots that do not exist yet; their nearest existing ancestor is watched.
    pending: Vec<PathBuf>,
}

/// File watcher + idle back-off coordinator for [`ingest_all`].
pub struct IngestWatcher {
    dirty: Arc<AtomicBool>,
    roots_changed: Arc<AtomicBool>,
    wakeup: Arc<Notify>,
    backoff: IdleBackoff,
    last_scan: Option<Instant>,
    roots: Vec<(PathBuf, RecursiveMode)>,
    shared: Arc<Mutex<Shared>>,
    watched: HashMap<PathBuf, RecursiveMode>,
    watcher: Option<notify::RecommendedWatcher>,
}

impl IngestWatcher {
    /// Starts watching discovered agent log directories and `GNOMCODE_HOME`.
    /// Missing roots are picked up when created; falls back to periodic back-off
    /// scans if the OS file watcher cannot be created.
    #[must_use]
    pub fn new(base_interval: Duration, max_idle_interval: Duration) -> Self {
        let mut roots: Vec<(PathBuf, RecursiveMode)> = discover_agent_roots()
            .into_iter()
            .map(|r| (r.path, RecursiveMode::Recursive))
            .collect();
        let home = gnomcode_home();
        // Only top-level files (status-line limits) matter in the Gnomcode home; recursive
        // watching would pick up worktrees and our own SQLite writes.
        // `sessions/` holds launched PTY records the daemon binds to conversations.
        roots.push((home.join("sessions"), RecursiveMode::NonRecursive));
        roots.push((home, RecursiveMode::NonRecursive));
        Self::watch(roots, base_interval, max_idle_interval)
    }

    /// Starts watching an explicit list of directories (recursive). Missing ones are
    /// watched through their nearest existing ancestor until they appear.
    #[must_use]
    pub fn with_paths(
        paths: &[PathBuf],
        base_interval: Duration,
        max_idle_interval: Duration,
    ) -> Self {
        let roots = paths
            .iter()
            .map(|p| (p.clone(), RecursiveMode::Recursive))
            .collect();
        Self::watch(roots, base_interval, max_idle_interval)
    }

    fn watch(
        roots: Vec<(PathBuf, RecursiveMode)>,
        base_interval: Duration,
        max_idle_interval: Duration,
    ) -> Self {
        let dirty = Arc::new(AtomicBool::new(true));
        let roots_changed = Arc::new(AtomicBool::new(false));
        let wakeup = Arc::new(Notify::new());
        let shared = Arc::new(Mutex::new(Shared::default()));
        let dirty_cb = Arc::clone(&dirty);
        let changed_cb = Arc::clone(&roots_changed);
        let wakeup_cb = Arc::clone(&wakeup);
        let shared_cb = Arc::clone(&shared);

        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            // Fail open: errors or a poisoned lock mark everything for a recheck.
            let (data, layout) = match (&res, shared_cb.lock()) {
                (Ok(event), Ok(roots)) => classify(event, &roots),
                _ => (true, true),
            };
            if layout {
                changed_cb.store(true, Ordering::Release);
            }
            if data || layout {
                dirty_cb.store(true, Ordering::Release);
                wakeup_cb.notify_one();
            }
        })
        .ok();

        let mut this = Self {
            dirty,
            roots_changed,
            wakeup,
            backoff: IdleBackoff::new(base_interval, max_idle_interval),
            last_scan: None,
            roots,
            shared,
            watched: HashMap::new(),
            watcher,
        };
        this.refresh_watches();
        this
    }

    /// Re-derives the watch set: existing roots directly, missing roots through their
    /// nearest existing ancestor (non-recursive). Returns whether a missing root appeared.
    fn refresh_watches(&mut self) -> bool {
        let Some(w) = self.watcher.as_mut() else {
            return false;
        };
        let mut desired: HashMap<PathBuf, RecursiveMode> = HashMap::new();
        let mut next = Shared::default();
        for (root, mode) in &self.roots {
            // 9P shares deliver no reliable change events and an open watch would hold a
            // handle on the distro; WSL roots rely on the back-off poll (ADR 0060).
            if gnomcode_platform::wsl::is_wsl_path(root) {
                continue;
            }
            if root.is_dir() {
                next.active.push(root.clone());
                merge_mode(&mut desired, root.clone(), *mode);
                // A watch cannot report its own directory's deletion on every OS; the
                // parent sees it, so delete-and-recreate is caught. Other parent
                // entries are classified as unrelated and never wake ingestion.
                if let Some(parent) = root.parent().filter(|p| p.is_dir()) {
                    merge_mode(
                        &mut desired,
                        parent.to_path_buf(),
                        RecursiveMode::NonRecursive,
                    );
                }
            } else if let Some(ancestor) = root.ancestors().skip(1).find(|a| a.is_dir()) {
                next.pending.push(root.clone());
                merge_mode(
                    &mut desired,
                    ancestor.to_path_buf(),
                    RecursiveMode::NonRecursive,
                );
            }
        }
        let appeared = match self.shared.lock() {
            Ok(mut current) => {
                let appeared = next.active.iter().any(|r| current.pending.contains(r));
                *current = next;
                appeared
            }
            Err(_) => true,
        };
        // Re-add every watch so a deleted and recreated root gets a fresh OS handle and
        // ancestor watches no longer needed are dropped.
        for path in self.watched.keys() {
            let _ = w.unwatch(path);
        }
        self.watched.clear();
        for (path, mode) in desired {
            if w.watch(&path, mode).is_ok() {
                self.watched.insert(path, mode);
            }
        }
        appeared
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
        self.watcher.is_some() && !self.watched.is_empty()
    }

    /// Returns `Some(was_dirty)` when a scan is due (filesystem event, a root
    /// appearing, or back-off elapsed) and clears the dirty flag; `None` while idle.
    pub fn take_due(&mut self) -> Option<bool> {
        let elapsed = self
            .last_scan
            .is_none_or(|t| t.elapsed() >= self.backoff.current());
        // Root create/delete events and the slow fallback both recheck the roots.
        let recheck = self.roots_changed.swap(false, Ordering::AcqRel) || elapsed;
        if recheck && self.refresh_watches() {
            self.dirty.store(true, Ordering::Release);
        }
        let was_dirty = self.dirty.swap(false, Ordering::AcqRel);
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

fn merge_mode(map: &mut HashMap<PathBuf, RecursiveMode>, path: PathBuf, mode: RecursiveMode) {
    let entry = map.entry(path).or_insert(mode);
    if mode == RecursiveMode::Recursive {
        *entry = mode;
    }
}

/// Classifies an event as `(data, layout)`: `data` when it touches a watched root's
/// contents, `layout` when it creates or removes a root or one of its ancestors.
/// Unrelated churn in an ancestor (e.g. the home dir) is neither.
fn classify(event: &Event, roots: &Shared) -> (bool, bool) {
    if is_own_db_write(event) {
        return (false, false);
    }
    // Parents report content changes of a root as `Modify` on the root itself;
    // only creates, removes and renames change which roots exist.
    let structural = matches!(
        event.kind,
        EventKind::Create(_)
            | EventKind::Remove(_)
            | EventKind::Modify(ModifyKind::Name(_))
            | EventKind::Any
            | EventKind::Other
    );
    let mut data = false;
    let mut layout = false;
    for path in &event.paths {
        if structural
            && roots
                .active
                .iter()
                .chain(&roots.pending)
                .any(|r| r.starts_with(path))
        {
            layout = true;
        } else if roots
            .active
            .iter()
            .any(|r| path.starts_with(r) && path != r)
        {
            data = true;
        }
    }
    (data, layout)
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

    #[test]
    fn wsl_roots_are_polled_not_watched() {
        // Skipped before any `is_dir` probe, so a stopped distro's share is never opened.
        let root = PathBuf::from(r"\\wsl.localhost\NoSuchDistro\home\a\.claude\projects");
        let mut watcher = IngestWatcher::with_paths(&[root], Duration::ZERO, Duration::ZERO);
        assert!(!watcher.is_watching(), "9P roots get no OS watch");
        assert_eq!(watcher.take_due(), Some(true));
        watcher.record_scan(false);
        // No events will ever arrive; the back-off poll still makes a scan due.
        assert_eq!(watcher.take_due(), Some(false));
    }
}
