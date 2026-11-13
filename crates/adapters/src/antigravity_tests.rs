//! Per-surface Antigravity fixtures: CLI 1.5.8, IDE 1.107.0 and the 2.0 app 2.31.2.

use super::antigravity::*;
use crate::{FileCtx, SourceAdapter};
use chrono::{DateTime, Utc};
use gnomcode_core::{Project, Surface};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/antigravity")
        .join(name)
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).expect("hex"))
        .collect()
}

type Blob = Vec<u8>;
type Rows = (Vec<(i64, Blob)>, Vec<Blob>);

/// `(generations, step metadata)` of a surface's trajectory fixture.
fn trajectory(dir: &str) -> Rows {
    let bytes = std::fs::read(fixture(&format!("{dir}/trajectory.json"))).expect("fixture");
    let value: Value = serde_json::from_slice(&bytes).expect("json");
    let generations = value["generations"]
        .as_array()
        .expect("generations")
        .iter()
        .map(|g| {
            (
                g["idx"].as_i64().expect("idx"),
                unhex(g["blob"].as_str().expect("blob")),
            )
        })
        .collect();
    let steps = value["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| unhex(s.as_str().expect("step")))
        .collect();
    (generations, steps)
}

fn transcript(dir: &str, data_dir: &str, usage_from_trajectory: bool) -> crate::ParsedBatch {
    let bytes = std::fs::read(fixture(&format!("{dir}/transcript.jsonl"))).expect("fixture");
    let ctx = FileCtx {
        source_path: format!(
            "/h/.gemini/{data_dir}/brain/00000000-0000-4000-8000-000000000001/.system_generated/logs/transcript.jsonl"
        ),
        fallback_cwd: Some("/workspace".into()),
        ..FileCtx::default()
    };
    AntigravityAdapter::new()
        .with_usage_from_trajectory(usage_from_trajectory)
        .parse(&ctx, &bytes)
}

fn sums(records: &[gnomcode_core::UsageRecord]) -> (u64, u64, u64) {
    records.iter().fold((0, 0, 0), |acc, r| {
        (
            acc.0 + r.input_tokens,
            acc.1 + r.output_tokens,
            acc.2 + r.cache_read_tokens,
        )
    })
}

#[test]
fn data_directory_decides_the_surface_and_cli_then_ide_own_shared_ids() {
    assert_eq!(
        surface_of_path("/h/.gemini/antigravity-cli/brain/x"),
        Surface::Cli
    );
    assert_eq!(
        surface_of_path(r"C:\Users\u\.gemini\antigravity-ide\brain\x"),
        Surface::Ide
    );
    assert_eq!(
        surface_of_path("/h/.gemini/antigravity/brain/x"),
        Surface::DesktopApp
    );
    assert_eq!(
        owner_surface([Surface::DesktopApp, Surface::Ide]),
        Some(Surface::Ide)
    );
    assert_eq!(
        owner_surface([Surface::DesktopApp, Surface::Cli, Surface::Ide]),
        Some(Surface::Cli)
    );
    assert_eq!(owner_surface([]), None);
    for (surface, label) in [
        ("ide", "Antigravity IDE"),
        ("cli", "Antigravity CLI"),
        ("desktop_app", "Antigravity 2.0"),
    ] {
        assert_eq!(
            gnomcode_core::surface_label("antigravity", surface),
            Some(label)
        );
    }
    assert_eq!(gnomcode_core::surface_label("codex", "cli"), None);
    assert_eq!(gnomcode_core::surface_label("claude-code", "cli"), None);
    assert_eq!(
        gnomcode_core::surface_label("claude-code", "desktop_app"),
        Some("Claude desktop app")
    );
    assert_eq!(
        gnomcode_core::surface_label("codex", "desktop_app"),
        Some("Codex desktop app")
    );
}

#[test]
fn each_surface_transcript_parses_once_with_its_own_surface() {
    for (dir, data_dir, surface, tokens) in [
        ("cli-1.5.8", "antigravity-cli", Surface::Cli, true),
        ("ide-1.107.0", "antigravity-ide", Surface::Ide, false),
        ("app-2.31.2", "antigravity", Surface::DesktopApp, true),
    ] {
        let batch = transcript(dir, data_dir, false);
        assert_eq!(batch.parse_errors, 0, "{dir}");
        assert_eq!(batch.sessions.len(), 1, "{dir}");
        assert_eq!(batch.sessions[0].surface, surface, "{dir}");
        assert_eq!(!batch.usage_records.is_empty(), tokens, "{dir}");
        // The same file read again yields the same keys, so a re-ingest adds nothing.
        let again = transcript(dir, data_dir, false);
        let keys = |b: &crate::ParsedBatch| {
            b.usage_records
                .iter()
                .map(|r| r.dedup_key.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(keys(&batch), keys(&again), "{dir}");
        // Trajectory-backed conversations leave usage to the trajectory.
        assert!(
            transcript(dir, data_dir, true).usage_records.is_empty(),
            "{dir}"
        );
    }
}

#[test]
fn trajectory_tokens_match_the_transcript_and_add_unlisted_calls() {
    for (dir, data_dir) in [
        ("cli-1.5.8", "antigravity-cli"),
        ("app-2.31.2", "antigravity"),
    ] {
        let (generations, steps) = trajectory(dir);
        let parsed: Vec<_> = generations
            .iter()
            .filter_map(|(idx, blob)| parse_generation(*idx, blob))
            .collect();
        assert_eq!(parsed.len(), generations.len(), "{dir}");
        assert_eq!(steps.len(), parsed.len(), "{dir}");
        let records = generation_usage("conv", &parsed, |_| DateTime::<Utc>::UNIX_EPOCH);
        let from_transcript = sums(&transcript(dir, data_dir, false).usage_records);
        // The fixture's last call is a background call the transcript does not list.
        assert_eq!(
            sums(&records),
            (
                from_transcript.0 + 1200,
                from_transcript.1 + 30,
                from_transcript.2
            ),
            "{dir}"
        );
        assert!(
            records
                .iter()
                .all(|r| r.dedup_key.starts_with(GENERATION_KEY_PREFIX))
        );
    }
}

#[test]
fn ide_trajectory_supplies_the_tokens_its_transcript_lacks() {
    assert!(
        transcript("ide-1.107.0", "antigravity-ide", false)
            .usage_records
            .is_empty()
    );
    let (generations, steps) = trajectory("ide-1.107.0");
    let parsed: Vec<_> = generations
        .iter()
        .filter_map(|(idx, blob)| parse_generation(*idx, blob))
        .collect();
    let mut started: HashMap<String, i64> = HashMap::new();
    for (response, seconds) in steps.iter().filter_map(|s| parse_step_response_time(s)) {
        let entry = started.entry(response).or_insert(seconds);
        *entry = (*entry).min(seconds);
    }
    let records = generation_usage("conv", &parsed, |g| {
        let seconds = started[g.response_id.as_deref().expect("response id")];
        DateTime::from_timestamp(seconds, 0).expect("time")
    });
    assert_eq!(sums(&records), (41_900, 832, 310_000));
    // Two steps of one response count once, dated at the earlier step.
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].timestamp.timestamp(), 1_794_555_304);
    assert_eq!(records[1].timestamp.timestamp(), 1_794_555_310);
    assert_eq!(records[0].model, "Gemini 3.8 Flash (Medium)");
    assert_eq!(records[2].model, "gemini-3.8-flash-n");
    assert_eq!(records[0].context_tokens, 170_000);
}

#[test]
fn malformed_trajectory_blobs_are_skipped_not_fatal() {
    for blob in [
        &b""[..],
        &[0xff; 9],
        &[0x0a, 0x7f, 0x01],
        &[0x0a, 0x02, 0x22, 0x00],
        b"\x0a\x04\x22\x02\x10\x00",
    ] {
        assert_eq!(parse_generation(0, blob), None);
    }
    assert_eq!(parse_step_response_time(&[0x0a, 0x05]), None);
}

#[test]
fn a_recorded_workspace_wins_over_folders_tool_calls_touch() {
    let repo = tempfile::tempdir().expect("repo");
    std::fs::create_dir(repo.path().join(".git")).expect("git dir");
    let line = serde_json::json!({
        "step_index": 1, "source": "MODEL", "type": "PLANNER_RESPONSE",
        "created_at": "2026-11-12T00:00:00Z", "input_tokens": 1,
        "tool_calls": [{"name": "view_file", "args": {"Path": repo.path().join("a.rs")}}]
    })
    .to_string();
    let parse = |cwd: Option<&str>| {
        let ctx = FileCtx {
            source_path: "/h/.gemini/antigravity/brain/c/.system_generated/logs/transcript.jsonl"
                .into(),
            fallback_cwd: cwd.map(str::to_owned),
            ..FileCtx::default()
        };
        AntigravityAdapter::new().parse(&ctx, line.as_bytes())
    };
    let declared = Project::from_origin(None, "/declared/workspace", None);
    assert_eq!(
        parse(Some("/declared/workspace")).sessions[0].project_id,
        declared.id
    );
    let touched = Project::from_origin(None, &repo.path().to_string_lossy(), None);
    assert_eq!(parse(None).sessions[0].project_id, touched.id);
}

#[test]
fn a_path_outside_any_project_has_no_root() {
    let dir = tempfile::tempdir().expect("dir");
    assert_eq!(
        find_repo_root_from_path(&dir.path().join("missing.txt").to_string_lossy()),
        None
    );
    let root = dir.path().ancestors().last().expect("root");
    assert_eq!(find_repo_root_from_path(&root.to_string_lossy()), None);
}
