//! `gnomcode evals handoff`: run the handoff quality evals and print a report (ADR 0055).

use anyhow::{Context, Result};
use clap::Subcommand;
use gnomcode_core::handoff_eval::EvalReport;
use std::path::PathBuf;

/// Evaluation suites.
#[derive(Subcommand, Debug)]
pub(crate) enum EvalsCommands {
    /// Score how much of a conversation survives a handoff to another agent. Report only.
    Handoff {
        /// Print the report as JSON.
        #[arg(long)]
        json: bool,
        /// Run the `*.json` cases in this folder instead of the built-in set (not stored).
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Do not store the built-in report for the console.
        #[arg(long)]
        no_save: bool,
    },
}

/// Runs an evals subcommand. A low score never fails the command: this is a report.
pub(crate) fn run(action: EvalsCommands) -> Result<()> {
    match action {
        EvalsCommands::Handoff { json, dir, no_save } => {
            let report = match &dir {
                Some(dir) => gnomcode_daemon::handoff_eval::run_dir(dir)
                    .with_context(|| format!("could not run evals in {}", dir.display()))?,
                None => gnomcode_daemon::handoff_eval::run_embedded()?,
            };
            if dir.is_none() && !no_save {
                let path = gnomcode_daemon::handoff_eval::report_path();
                gnomcode_daemon::handoff_eval::save_report(&path, &report)
                    .with_context(|| format!("could not store the report at {}", path.display()))?;
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", render(&report));
            }
            Ok(())
        }
    }
}

fn render(report: &EvalReport) -> String {
    let mut out = String::from("=== HANDOFF QUALITY EVALS (estimated, synthetic cases) ===\n");
    for case in &report.cases {
        let badge = if case.outcome == "passed" {
            "PASS"
        } else {
            "FAIL"
        };
        out.push_str(&format!(
            "  [{badge}] {:>5.1}%  {:<18} {} -> {}  (packet {:.1}%)\n         {}\n",
            case.score_pct,
            case.id,
            case.source_agent,
            case.target_agent,
            case.packet_score_pct,
            case.summary
        ));
        for (label, coverage) in [
            ("goals", &case.details.goals),
            ("files", &case.details.files),
            ("open tasks", &case.details.open_tasks),
        ] {
            if !coverage.missing.is_empty() {
                out.push_str(&format!(
                    "         missing {label}: {}\n",
                    coverage.missing.join("; ")
                ));
            }
        }
        if !case.details.leaked.is_empty() {
            out.push_str("         a secret reached the handoff\n");
        }
    }
    out.push_str("\nBy receiving provider:\n");
    for provider in &report.by_provider {
        out.push_str(&format!(
            "  {:<12} {}/{} passed ({:.1}%), mean score {:.1}%\n",
            provider.target_agent,
            provider.passed_cases,
            provider.cases,
            provider.pass_rate_pct,
            provider.mean_score_pct
        ));
    }
    out.push_str(&format!(
        "\nTotal: {}/{} passed ({:.1}%), mean score {:.1}% [EST]\n",
        report.summary.passed_cases,
        report.summary.total_cases,
        report.summary.pass_rate_pct,
        report.summary.mean_score_pct
    ));
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn report_lists_each_case_provider_and_total() {
        let report = gnomcode_daemon::handoff_eval::run_embedded().expect("run");
        let text = super::render(&report);
        assert!(text.contains("[PASS]") && !text.contains("[FAIL]"));
        assert!(text.contains("By receiving provider") && text.contains("[EST]"));
        assert!(text.contains("long-session") && text.contains("Total: 9/9 passed"));
    }
}
