//! Windowless launcher for agent hooks and the Claude status line.
//!
//! GUI agents (the Claude desktop app) start hook commands without hiding their window, so a
//! console-subsystem `gnomcode.exe` flashes a terminal on every tool call. This shim is built
//! for the Windows GUI subsystem: it owns no console, starts the sibling `gnomcode` binary with
//! `CREATE_NO_WINDOW` and the inherited stdin/stdout/stderr pipes, and returns its exit code.
//! It fails open: if the engine cannot start, it exits 0 so the agent is never blocked.
#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    let engine = std::env::current_exe().ok().map(|exe| {
        exe.with_file_name(if cfg!(windows) {
            "gnomcode.exe"
        } else {
            "gnomcode"
        })
    });
    let Some(engine) = engine else {
        std::process::exit(0);
    };
    let mut command = std::process::Command::new(engine);
    command.args(std::env::args_os().skip(1));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0800_0000);
    }
    let code = command
        .status()
        .ok()
        .and_then(|status| status.code())
        .unwrap_or(0);
    std::process::exit(code);
}
