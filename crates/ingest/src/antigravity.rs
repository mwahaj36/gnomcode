//! Antigravity conversations that live in more than one data directory (`SPEC §3.5`).
//!
//! The CLI (`antigravity-cli`), the IDE (`antigravity-ide`) and the 2.0 app (`antigravity`) each
//! keep `brain/<id>/` transcripts and `conversations/<id>.db` trajectories. The 2.0 app also holds
//! copies of the conversations the IDE migrated, so one id can sit in several directories. Each id
//! has one owner surface, and token usage comes from the trajectory once per model call.

use crate::Error;
use chrono::{DateTime, Utc};
use gnomcode_adapters::antigravity::{
    GENERATION_KEY_PREFIX, generation_usage, owner_surface, parse_generation,
    parse_step_response_time, surface_of_path,
};
use gnomcode_core::{AgentId, Session, Surface};
use gnomcode_platform::AgentRoot;
use gnomcode_store::{Database, SourceCheckpoint, read_trajectory, trajectory_max_generation};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// One `conversations/<id>.db` file and the surface of the directory it sits in.
#[derive(Debug, Clone)]
pub(crate) struct Trajectory {
    pub(crate) conversation: String,
    pub(crate) path: PathBuf,
    pub(crate) surface: Surface,
}

/// Every Antigravity conversation id found on disk with its owner surface and trajectory files.
#[derive(Debug, Default)]
pub(crate) struct Catalog {
    pub(crate) owners: HashMap<String, Surface>,
    pub(crate) trajectories: Vec<Trajectory>,
}

impl Catalog {
    /// Scans the data directories next to each Antigravity `brain` root. Only directory listings
    /// are read, never conversation content.
    pub(crate) fn scan(roots: &[AgentRoot]) -> Self {
        let mut seen: HashMap<String, Vec<Surface>> = HashMap::new();
        let mut trajectories = Vec::new();
        for root in roots
            .iter()
            .filter(|root| root.agent == AgentId::Antigravity && root.exists)
        {
            let Some(data) = root.path.parent() else {
                continue;
            };
            let surface = surface_of_path(&data.to_string_lossy());
            for entry in fs::read_dir(&root.path).into_iter().flatten().flatten() {
                if let Some(id) = entry.file_name().to_str() {
                    seen.entry(id.to_owned()).or_default().push(surface);
                }
            }
            for entry in fs::read_dir(data.join("conversations"))
                .into_iter()
                .flatten()
                .flatten()
            {
                let path = entry.path();
                let (Some(stem), Some(ext)) = (
                    path.file_stem().and_then(|s| s.to_str()),
                    path.extension().and_then(|s| s.to_str()),
                ) else {
                    continue;
                };
                // `.pb` files are the encrypted pre-2.0 format and cannot be read.
                if ext.eq_ignore_ascii_case("db") {
                    trajectories.push(Trajectory {
                        conversation: stem.to_owned(),
                        path: path.clone(),
                        surface,
                    });
                }
                if ext.eq_ignore_ascii_case("db") || ext.eq_ignore_ascii_case("pb") {
                    seen.entry(stem.to_owned()).or_default().push(surface);
                }
            }
        }
        let owners = seen
            .into_iter()
            .filter_map(|(id, surfaces)| Some((id, owner_surface(surfaces)?)))
            .collect();
        Self {
            owners,
            trajectories,
        }
    }

    /// The surface that owns a conversation, when it was found on disk.
    pub(crate) fn owner(&self, conversation: &str) -> Option<Surface> {
        self.owners.get(conversation).copied()
    }

    /// Whether a trajectory of this conversation has already supplied its usage, so transcripts
    /// must not count the same calls again.
    pub(crate) fn usage_from_trajectory(&self, db: &Database, conversation: &str) -> bool {
        self.trajectories
            .iter()
            .filter(|t| t.conversation == conversation)
            .any(|t| {
                db.get_checkpoint(&t.path.to_string_lossy())
                    .is_ok_and(|checkpoint| checkpoint.is_some())
            })
    }

    /// Points stored sessions at their owner surface (idempotent).
    pub(crate) fn repair_surfaces(&self, db: &mut Database) -> Result<(), Error> {
        let surfaces: Vec<(String, Surface)> = self
            .owners
            .iter()
            .map(|(id, surface)| (Session::compose_id(AgentId::Antigravity, id), *surface))
            .collect();
        db.set_session_surfaces(&surfaces)?;
        Ok(())
    }
}

fn fingerprint(path: &Path) -> Option<String> {
    let stamp = |path: &Path| {
        fs::metadata(path).ok().map(|meta| {
            let modified = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |elapsed| elapsed.as_nanos());
            format!("{}:{modified}", meta.len())
        })
    };
    let mut wal = path.as_os_str().to_owned();
    wal.push("-wal");
    Some(format!(
        "{}|{}",
        stamp(path)?,
        stamp(Path::new(&wal)).unwrap_or_default()
    ))
}

fn owner_generation_max(catalog: &Catalog, conversation: &str) -> Option<i64> {
    let owner = catalog.owner(conversation)?;
    let t = catalog
        .trajectories
        .iter()
        .find(|t| t.conversation == conversation && t.surface == owner)?;
    trajectory_max_generation(&t.path).ok().flatten()
}

/// Records each model call of every readable trajectory once and returns how many were new.
///
/// A copy held by a surface that does not own the conversation is read only when it has model
/// calls the owner's copy lacks (the conversation was continued there). Sessions come from the
/// transcript pass, so a trajectory whose session is not stored yet waits for the next pass.
pub(crate) fn ingest_trajectories(db: &mut Database, catalog: &Catalog) -> Result<u64, Error> {
    let mut added = 0;
    for t in &catalog.trajectories {
        let session_id = Session::compose_id(AgentId::Antigravity, &t.conversation);
        if !db.session_exists(&session_id)? {
            continue;
        }
        let path_str = t.path.to_string_lossy().into_owned();
        let Some(fingerprint) = fingerprint(&t.path) else {
            continue;
        };
        let checkpoint = db.get_checkpoint(&path_str)?;
        if checkpoint
            .as_ref()
            .is_some_and(|checkpoint| checkpoint.fingerprint == fingerprint)
        {
            continue;
        }
        let owned = catalog.owner(&t.conversation) == Some(t.surface);
        if !owned
            && let Some(owner) = owner_generation_max(catalog, &t.conversation)
            && trajectory_max_generation(&t.path).ok().flatten() <= Some(owner)
        {
            // Nothing the owner's copy lacks; remember the file so it is not opened every pass.
            db.commit_trajectory_usage(
                &session_id,
                &[],
                GENERATION_KEY_PREFIX,
                &SourceCheckpoint {
                    file_path: path_str,
                    last_read_offset: 0,
                    file_size: fs::metadata(&t.path).map_or(0, |meta| meta.len()),
                    fingerprint,
                    last_ingested_at: Utc::now(),
                },
            )?;
            continue;
        }
        let after = checkpoint.as_ref().map_or(-1, |checkpoint| {
            i64::try_from(checkpoint.last_read_offset).map_or(-1, |next| next - 1)
        });
        let rows = match read_trajectory(&t.path, after) {
            Ok(rows) => rows,
            Err(error) => {
                tracing::warn!("Antigravity trajectory unreadable: {error}");
                continue;
            }
        };
        let generations: Vec<_> = rows
            .generations
            .iter()
            .filter_map(|(idx, blob)| parse_generation(*idx, blob))
            .collect();
        let mut started: HashMap<String, i64> = HashMap::new();
        let mut earliest = i64::MAX;
        for (response, seconds) in rows
            .step_metadata
            .iter()
            .filter_map(|metadata| parse_step_response_time(metadata))
        {
            earliest = earliest.min(seconds);
            started
                .entry(response)
                .and_modify(|seen| *seen = (*seen).min(seconds))
                .or_insert(seconds);
        }
        let usage = generation_usage(&t.conversation, &generations, |generation| {
            let seconds = generation
                .response_id
                .as_ref()
                .and_then(|response| started.get(response))
                .copied()
                .unwrap_or(earliest);
            DateTime::from_timestamp(seconds, 0).unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
        });
        let last = rows.generations.last().map_or(after, |(idx, _)| *idx);
        let size = fs::metadata(&t.path).map_or(0, |meta| meta.len());
        db.commit_trajectory_usage(
            &session_id,
            &usage,
            GENERATION_KEY_PREFIX,
            &SourceCheckpoint {
                file_path: path_str,
                last_read_offset: u64::try_from(last + 1).unwrap_or(0),
                file_size: size,
                fingerprint,
                last_ingested_at: Utc::now(),
            },
        )?;
        added += usage.len() as u64;
    }
    Ok(added)
}
