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
        if home.exists() {
            paths.push(home);
        }
        Self::with_paths(&paths, base_interval, max_idle_interval)
    }

    /// Starts watching an explicit list of directories/files.
    #[must_use]
    pub fn with_paths(
        paths: &[PathBuf],
        base_interval: Duration,
        max_idle_interval: Duration,
    ) -> Self {
        let dirty = Arc::new(AtomicBool::new(true));
        let wakeup = Arc::new(Notify::new());
        let dirty_cb = Arc::clone(&dirty);
        let wakeup_cb = Arc::clone(&wakeup);

        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if res.is_ok() {
                dirty_cb.store(true, Ordering::Release);
                wakeup_cb.notify_one();
            }
        })
        .ok()
        .map(|mut w| {
            for path in paths {
                if path.exists() {
                    let _ = w.watch(path, RecursiveMode::Recursive);
                }
            }
            w
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
        let was_dirty = self.dirty.swap(false, Ordering::AcqRel);
        let elapsed = self
            .last_scan
            .is_none_or(|t| t.elapsed() >= self.backoff.current());
        if !was_dirty && !elapsed {
            return Ok(None);
        }
        let ingested = ingest_all(db)?;
        self.last_scan = Some(Instant::now());
        if ingested > 0 || was_dirty {
            self.backoff.on_activity();
        } else {
            self.backoff.on_idle();
        }
        Ok(Some(ingested))
    }
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
