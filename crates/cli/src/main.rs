//! The `fleethud` command-line interface.
//!
//! Scaffold only: argument parsing (`clap`) and subcommands arrive in M1 (see `docs/SPEC.md` §5).

use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut out = io::stdout().lock();
    let result = writeln!(
        out,
        "FleetHUD {} — live mission control for AI coding agents\n\
         Pre-release scaffold. Roadmap: https://github.com/mwahaj36/fleethud/blob/main/docs/SPEC.md",
        env!("CARGO_PKG_VERSION")
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
