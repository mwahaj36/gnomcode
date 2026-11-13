//! Ref-counted sleep inhibitor (SPEC 3.14.3). Prevents idle and system sleep, never display
//! sleep. Every failure is swallowed: a missing inhibitor must never break the daemon.

/// Starts and stops the OS-level inhibition.
trait Backend: Send {
    /// Begins inhibiting sleep; `false` means the OS refused or the helper is missing.
    fn start(&mut self) -> bool;
    /// Ends inhibition; safe to call when not started.
    fn stop(&mut self);
}

/// Ref-counted guard around one OS sleep inhibitor.
pub struct SleepInhibitor {
    holds: usize,
    backend: Box<dyn Backend>,
}

impl Default for SleepInhibitor {
    fn default() -> Self {
        Self::with_backend(Box::new(os::Os::default()))
    }
}

impl SleepInhibitor {
    fn with_backend(backend: Box<dyn Backend>) -> Self {
        Self { holds: 0, backend }
    }

    /// Adds one hold; the first hold starts inhibition. Failure to start is logged and ignored.
    pub fn acquire(&mut self) {
        self.holds += 1;
        if self.holds == 1 && !self.backend.start() {
            tracing::warn!("sleep inhibitor unavailable; the laptop may sleep");
        }
    }

    /// Drops one hold; the last release stops inhibition. Extra releases are ignored.
    pub fn release(&mut self) {
        match self.holds {
            0 => {}
            1 => {
                self.holds = 0;
                self.backend.stop();
            }
            _ => self.holds -= 1,
        }
    }

    /// Number of outstanding holds.
    #[must_use]
    pub fn holds(&self) -> usize {
        self.holds
    }
}

/// Reports whether the current platform has a sleep-inhibitor backend available.
#[must_use]
pub fn is_supported() -> bool {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        true
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        crate::executables::executable_available("systemd-inhibit")
    }
}

impl Drop for SleepInhibitor {
    fn drop(&mut self) {
        if self.holds > 0 {
            self.backend.stop();
        }
    }
}

#[cfg(target_os = "windows")]
mod os {
    #![allow(unsafe_code)]
    use std::sync::mpsc::{Sender, channel};

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetThreadExecutionState(flags: u32) -> u32;
    }
    const ES_CONTINUOUS: u32 = 0x8000_0000;
    const ES_SYSTEM_REQUIRED: u32 = 0x0000_0001;

    /// The execution state is per thread, so a dedicated thread holds it until told to stop.
    #[derive(Default)]
    pub(super) struct Os(Option<Sender<()>>);

    impl super::Backend for Os {
        fn start(&mut self) -> bool {
            let (stop, wait) = channel::<()>();
            let (ready, started) = channel::<bool>();
            let spawned = std::thread::Builder::new()
                .name("keep-awake".into())
                .spawn(move || {
                    // SAFETY: plain Win32 call that only sets this thread's execution state.
                    let ok =
                        unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED) } != 0;
                    let _ = ready.send(ok);
                    if ok {
                        let _ = wait.recv();
                        // SAFETY: clears the continuous state set above on this same thread.
                        unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
                    }
                });
            if spawned.is_ok() && started.recv().unwrap_or(false) {
                self.0 = Some(stop);
                true
            } else {
                false
            }
        }
        fn stop(&mut self) {
            self.0 = None; // dropping the sender wakes the thread, which clears the state
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod os {
    use std::process::{Child, Command, Stdio};

    /// Helper child: `caffeinate -w` (macOS) or `systemd-inhibit` (Linux), both tied to our pid.
    #[derive(Default)]
    pub(super) struct Os(Option<Child>);

    pub(super) fn helper() -> Command {
        let pid = std::process::id().to_string();
        #[cfg(target_os = "macos")]
        {
            let mut command = Command::new("caffeinate");
            command.args(["-i", "-w", &pid]);
            command
        }
        #[cfg(not(target_os = "macos"))]
        {
            let mut command = Command::new("systemd-inhibit");
            // The inner loop ends by itself if the daemon dies without calling stop.
            command.args([
                "--what=idle:sleep",
                "--who=Gnomcode",
                "--why=Agent work or scheduled message pending",
                "--mode=block",
                "sh",
                "-c",
                "while kill -0 \"$0\" 2>/dev/null; do sleep 5; done",
                &pid,
            ]);
            command
        }
    }

    impl super::Backend for Os {
        fn start(&mut self) -> bool {
            match helper()
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(child) => {
                    self.0 = Some(child);
                    true
                }
                Err(_) => false,
            }
        }
        fn stop(&mut self) {
            if let Some(mut child) = self.0.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct Fake {
        log: Arc<Mutex<Vec<&'static str>>>,
        works: bool,
    }
    impl Backend for Fake {
        fn start(&mut self) -> bool {
            self.log.lock().expect("log").push("start");
            self.works
        }
        fn stop(&mut self) {
            self.log.lock().expect("log").push("stop");
        }
    }

    fn inhibitor(works: bool) -> (SleepInhibitor, Arc<Mutex<Vec<&'static str>>>) {
        let log = Arc::new(Mutex::new(Vec::new()));
        let fake = Fake {
            log: Arc::clone(&log),
            works,
        };
        (SleepInhibitor::with_backend(Box::new(fake)), log)
    }

    #[test]
    fn starts_on_first_hold_and_stops_on_last() {
        let (mut guard, log) = inhibitor(true);
        guard.acquire();
        guard.acquire();
        guard.release();
        assert_eq!(*log.lock().expect("log"), ["start"]);
        guard.release();
        guard.release(); // extra release is ignored
        assert_eq!(guard.holds(), 0);
        assert_eq!(*log.lock().expect("log"), ["start", "stop"]);
    }

    #[test]
    fn backend_failure_fails_open_and_drop_releases() {
        let (mut guard, log) = inhibitor(false);
        guard.acquire();
        assert_eq!(guard.holds(), 1);
        drop(guard);
        assert_eq!(*log.lock().expect("log"), ["start", "stop"]);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn unix_helper_command_uses_expected_backend() {
        let cmd = os::helper();
        let program = cmd.get_program().to_string_lossy();
        let args: Vec<_> = cmd
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        if cfg!(target_os = "macos") {
            assert_eq!(program, "caffeinate");
            assert_eq!(args.first().map(String::as_str), Some("-i"));
            assert_eq!(args.get(1).map(String::as_str), Some("-w"));
        } else {
            assert_eq!(program, "systemd-inhibit");
            assert!(args.iter().any(|arg| arg == "--what=idle:sleep"));
            assert!(args.iter().any(|arg| arg == "--mode=block"));
        }
    }
}
