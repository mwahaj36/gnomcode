//! Background desktop update-manifest polling, verification, and staging (`SPEC §6.4`, ADR 0033).
//!
//! Checks `{baseUrl}/downloads/latest.json` on daemon start and every 6 hours unless opted out,
//! downloads verified installers in the background, and fails open on any error. Never executes
//! an installer automatically; Electron installs only on explicit user click or quit.

use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use chrono::Utc;
use gnomcode_core::update::{
    UPDATE_CHECK_INTERVAL_SECS, UpdateManifest, UpdateStateKind, UpdateStatus,
    is_allowed_release_url, is_newer_version, platform_target, safe_asset_filename,
    verify_ed25519_hex, verify_sha256_hex,
};
use gnomcode_platform::gnomcode_home;
use gnomcode_store::Database;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;

const MAX_MANIFEST_BYTES: usize = 256 * 1024;
const MAX_INSTALLER_BYTES: usize = 256 * 1024 * 1024;

static UPDATE_STATE: OnceLock<Arc<RwLock<UpdateStatus>>> = OnceLock::new();

/// Shared daemon-wide desktop update status state.
#[must_use]
pub(crate) fn shared_state() -> &'static Arc<RwLock<UpdateStatus>> {
    UPDATE_STATE.get_or_init(|| {
        Arc::new(RwLock::new(UpdateStatus::initial(
            env!("CARGO_PKG_VERSION"),
            true,
        )))
    })
}

/// Starts the fail-open startup + 6-hour background update checker.
#[must_use]
pub fn spawn(db: Arc<Mutex<Database>>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .ok();
        let Some(client) = client else {
            return;
        };
        let state = Arc::clone(shared_state());
        let mut ticker = tokio::time::interval(Duration::from_secs(UPDATE_CHECK_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let enabled = is_update_enabled(&db).await;
            let updates_dir = gnomcode_home().join("updates");
            let manifest_override = manifest_override_from_env();
            let mut next = check_and_stage_update(
                &client,
                &state,
                enabled,
                manifest_override.as_deref(),
                &updates_dir,
                env!("CARGO_PKG_VERSION"),
            )
            .await;
            next.auto_update_enabled = enabled;
            *state.write().await = next;
        }
    })
}

/// Triggers an immediate update check using the persisted opt-out preference.
pub(crate) async fn check_now(db: &Arc<Mutex<Database>>) -> UpdateStatus {
    let enabled = is_update_enabled(db).await;
    let state = Arc::clone(shared_state());
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
    else {
        return state.read().await.clone();
    };
    let updates_dir = gnomcode_home().join("updates");
    let manifest_override = manifest_override_from_env();
    let next = check_and_stage_update(
        &client,
        &state,
        enabled,
        manifest_override.as_deref(),
        &updates_dir,
        env!("CARGO_PKG_VERSION"),
    )
    .await;
    *state.write().await = next.clone();
    next
}

/// Reads the test-only manifest override, honouring it only for loopback hosts so the
/// release flow can never be redirected to an arbitrary origin (AGENTS.md invariant 4).
fn manifest_override_from_env() -> Option<String> {
    let raw = std::env::var("GNOMCODE_UPDATE_MANIFEST_URL").ok()?;
    loopback_url(&raw).map(str::to_owned)
}

fn loopback_url(raw: &str) -> Option<&str> {
    let raw = raw.trim();
    let rest = raw
        .strip_prefix("http://")
        .or_else(|| raw.strip_prefix("https://"))?;
    let authority = rest.split('/').next().unwrap_or_default();
    let host = if authority.starts_with('[') {
        authority.split_once(']').map_or("", |(name, _)| name)
    } else {
        authority.split(':').next().unwrap_or_default()
    };
    matches!(host, "127.0.0.1" | "localhost" | "[::1").then_some(raw)
}

pub(crate) async fn is_update_enabled(db: &Arc<Mutex<Database>>) -> bool {
    if std::env::var("GNOMCODE_AUTO_UPDATE").as_deref() == Ok("0") {
        return false;
    }
    let db = Arc::clone(db);
    tokio::task::spawn_blocking(move || {
        db.blocking_lock().get_auto_update_enabled().unwrap_or(true)
    })
    .await
    .unwrap_or(true)
}

fn default_manifest_url() -> Option<String> {
    gnomcode_core::branding::branding().map(|b| format!("{}/downloads/latest.json", b.base_url))
}

async fn check_and_stage_update(
    client: &reqwest::Client,
    state: &Arc<RwLock<UpdateStatus>>,
    enabled: bool,
    manifest_override: Option<&str>,
    updates_dir: &Path,
    current_version: &str,
) -> UpdateStatus {
    if !enabled {
        return UpdateStatus::initial(current_version, false);
    }
    let now = Utc::now().to_rfc3339();
    let Some(manifest_url) = manifest_override
        .map(str::to_owned)
        .or_else(default_manifest_url)
    else {
        return fail_open(current_version, &now, "Update manifest URL is unavailable.");
    };

    let manifest_res = match client.get(&manifest_url).send().await {
        Ok(res) if res.status().is_success() => res,
        Ok(res) => {
            return fail_open(
                current_version,
                &now,
                &format!("Manifest request returned HTTP {}", res.status().as_u16()),
            );
        }
        Err(_) => return fail_open(current_version, &now, "Update server unreachable."),
    };

    let manifest_bytes = match manifest_res.bytes().await {
        Ok(bytes) if bytes.len() <= MAX_MANIFEST_BYTES => bytes,
        _ => return fail_open(current_version, &now, "Invalid update manifest size."),
    };
    let manifest: UpdateManifest = match serde_json::from_slice(&manifest_bytes) {
        Ok(m) => m,
        Err(_) => return fail_open(current_version, &now, "Malformed update manifest."),
    };

    if !is_newer_version(current_version, &manifest.version) {
        return UpdateStatus {
            state: UpdateStateKind::UpToDate,
            current_version: current_version.to_owned(),
            available_version: None,
            downloaded_path: None,
            sha256: None,
            auto_update_enabled: true,
            last_checked_at: Some(now),
            last_error: None,
        };
    }

    let Some(target) = platform_target(std::env::consts::OS, std::env::consts::ARCH) else {
        return UpdateStatus {
            state: UpdateStateKind::Available,
            current_version: current_version.to_owned(),
            available_version: Some(manifest.version),
            downloaded_path: None,
            sha256: None,
            auto_update_enabled: true,
            last_checked_at: Some(now),
            last_error: None,
        };
    };

    let Some(asset) = manifest.platforms.get(target) else {
        return UpdateStatus {
            state: UpdateStateKind::UpToDate,
            current_version: current_version.to_owned(),
            available_version: None,
            downloaded_path: None,
            sha256: None,
            auto_update_enabled: true,
            last_checked_at: Some(now),
            last_error: None,
        };
    };

    if !url_is_trusted(&asset.url, manifest_override) {
        return fail_open(current_version, &now, "Update asset URL rejected.");
    }

    let Some(filename) = safe_asset_filename(&asset.url, asset.file_name.as_deref()) else {
        return fail_open(current_version, &now, "Unsafe installer file name.");
    };

    let dest_dir = updates_dir.join(format!("v{}", manifest.version));
    let dest_path = dest_dir.join(&filename);
    if let Ok(existing) = tokio::fs::read(&dest_path).await
        && verify_downloaded_bytes(&existing, &asset.sha256, asset.signature.as_deref())
    {
        return ready_status(
            current_version,
            &manifest.version,
            &dest_path,
            &asset.sha256,
            &now,
        );
    }

    {
        let mut guard = state.write().await;
        *guard = UpdateStatus {
            state: UpdateStateKind::Downloading,
            current_version: current_version.to_owned(),
            available_version: Some(manifest.version.clone()),
            downloaded_path: None,
            sha256: None,
            auto_update_enabled: true,
            last_checked_at: Some(now.clone()),
            last_error: None,
        };
    }

    let asset_res = match client.get(&asset.url).send().await {
        Ok(res) if res.status().is_success() => res,
        _ => return fail_open(current_version, &now, "Failed to download update package."),
    };
    let asset_bytes = match asset_res.bytes().await {
        Ok(bytes) if !bytes.is_empty() && bytes.len() <= MAX_INSTALLER_BYTES => bytes,
        _ => return fail_open(current_version, &now, "Downloaded package size invalid."),
    };

    if !verify_downloaded_bytes(&asset_bytes, &asset.sha256, asset.signature.as_deref()) {
        return fail_open(
            current_version,
            &now,
            "Update signature or checksum verification failed.",
        );
    }

    if tokio::fs::create_dir_all(&dest_dir).await.is_err() {
        return fail_open(current_version, &now, "Could not prepare update directory.");
    }
    let part_path = dest_dir.join(format!("{filename}.part"));
    if tokio::fs::write(&part_path, &asset_bytes).await.is_err()
        || tokio::fs::rename(&part_path, &dest_path).await.is_err()
    {
        let _ = tokio::fs::remove_file(&part_path).await;
        return fail_open(current_version, &now, "Could not save verified installer.");
    }

    ready_status(
        current_version,
        &manifest.version,
        &dest_path,
        &asset.sha256,
        &now,
    )
}

fn verify_downloaded_bytes(bytes: &[u8], sha256: &str, signature: Option<&str>) -> bool {
    if !verify_sha256_hex(bytes, sha256) {
        return false;
    }
    // An embedded key cannot be removed or replaced by the runtime environment.
    // Preserve malformed embedded values so verification fails closed for installation.
    let pub_key = option_env!("GNOMCODE_UPDATE_PUBLIC_KEY")
        .map(str::to_owned)
        .or_else(|| {
            std::env::var("GNOMCODE_UPDATE_PUBLIC_KEY")
                .ok()
                .filter(|k| !k.trim().is_empty())
        });
    if let Some(key_hex) = pub_key {
        let Some(sig_hex) = signature else {
            return false;
        };
        if !verify_ed25519_hex(bytes, sig_hex, &key_hex) {
            return false;
        }
    }
    true
}

fn url_is_trusted(url: &str, manifest_override: Option<&str>) -> bool {
    if let Some(over) = manifest_override
        && let Some((scheme, rest)) = over.split_once("://")
        && let Some(host) = rest.split('/').next()
        && url.starts_with(&format!("{scheme}://{host}/"))
    {
        return !url.contains('@') && !url.contains("..");
    }
    let Some(brand) = gnomcode_core::branding::branding() else {
        return false;
    };
    let downloads = format!("{}/downloads", brand.base_url);
    let releases = format!("{}/releases", brand.releases_repository);
    let allowed = [
        downloads.as_str(),
        releases.as_str(),
        "https://objects.githubusercontent.com",
        "https://release-assets.githubusercontent.com",
    ];
    is_allowed_release_url(url, &allowed)
}

fn ready_status(
    current_version: &str,
    available_version: &str,
    path: &Path,
    sha256: &str,
    checked_at: &str,
) -> UpdateStatus {
    UpdateStatus {
        state: UpdateStateKind::Ready,
        current_version: current_version.to_owned(),
        available_version: Some(available_version.to_owned()),
        downloaded_path: Some(path.to_string_lossy().into_owned()),
        sha256: Some(sha256.to_ascii_lowercase()),
        auto_update_enabled: true,
        last_checked_at: Some(checked_at.to_owned()),
        last_error: None,
    }
}

fn fail_open(current_version: &str, checked_at: &str, message: &str) -> UpdateStatus {
    tracing::warn!("update check failed open: {message}");
    UpdateStatus {
        state: UpdateStateKind::Error,
        current_version: current_version.to_owned(),
        available_version: None,
        downloaded_path: None,
        sha256: None,
        auto_update_enabled: true,
        last_checked_at: Some(checked_at.to_owned()),
        last_error: Some(message.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use serde_json::json;
    use sha2::{Digest, Sha256};

    // RFC 8032 test vector 1: public key and signature only, no private key.
    #[test]
    #[ignore = "requires GNOMCODE_UPDATE_PUBLIC_KEY at build time; see docs/release-signing.md"]
    fn embedded_key_requires_signature_despite_runtime_override() {
        let public_key = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
        assert_eq!(option_env!("GNOMCODE_UPDATE_PUBLIC_KEY"), Some(public_key));
        let sha = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let signature = concat!(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155",
            "5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
        );
        assert!(!verify_downloaded_bytes(b"", sha, None));
        assert!(verify_downloaded_bytes(b"", sha, Some(signature)));
        assert!(!verify_downloaded_bytes(b"", sha, Some(&"0".repeat(128))));
        assert!(!verify_downloaded_bytes(b"tampered", sha, Some(signature)));
    }

    #[test]
    fn manifest_override_accepts_only_loopback_hosts() {
        assert!(loopback_url("http://127.0.0.1:8080/latest.json").is_some());
        assert!(loopback_url("http://localhost/latest.json").is_some());
        assert!(loopback_url("http://[::1]:9/latest.json").is_some());
        assert!(loopback_url("https://evil.example/latest.json").is_none());
        assert!(loopback_url("http://127.0.0.1.evil.example/latest.json").is_none());
        assert!(loopback_url("file:///etc/passwd").is_none());
    }

    #[tokio::test]
    async fn downloads_verifies_sha256_stages_installer_and_fails_open_on_tamper() {
        let installer_bytes = b"MZ-verified-setup-binary";
        let digest = Sha256::digest(installer_bytes);
        let valid_sha: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        let target =
            platform_target(std::env::consts::OS, std::env::consts::ARCH).expect("supported OS");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let base = format!("http://127.0.0.1:{port}");

        let valid_sha_clone = valid_sha.clone();
        let base_clone = base.clone();
        let app = Router::new()
            .route(
                "/good.json",
                get(move || {
                    let sha = valid_sha_clone.clone();
                    let url = format!("{base_clone}/Gnomcode-Setup.exe");
                    async move {
                        axum::Json(json!({
                            "version": "9.9.9",
                            "platforms": {
                                target: { "url": url, "sha256": sha }
                            }
                        }))
                    }
                }),
            )
            .route(
                "/bad.json",
                get({
                    let url = format!("{base}/Gnomcode-Setup.exe");
                    move || async move {
                        axum::Json(json!({
                            "version": "9.9.9",
                            "platforms": {
                                target: { "url": url, "sha256": "0".repeat(64) }
                            }
                        }))
                    }
                }),
            )
            .route(
                "/Gnomcode-Setup.exe",
                get(|| async { installer_bytes.as_slice() }),
            );

        let server = tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
        let client = reqwest::Client::new();
        let temp = tempfile::tempdir().expect("tempdir");
        let state = Arc::new(RwLock::new(UpdateStatus::initial("0.9.4", true)));

        // Opted-out returns Disabled without downloading.
        let disabled = check_and_stage_update(
            &client,
            &state,
            false,
            Some(&format!("{base}/good.json")),
            temp.path(),
            "0.9.4",
        )
        .await;
        assert_eq!(disabled.state, UpdateStateKind::Disabled);

        // Tampered hash fails open without leaving staged binary.
        let tampered = check_and_stage_update(
            &client,
            &state,
            true,
            Some(&format!("{base}/bad.json")),
            temp.path(),
            "0.9.4",
        )
        .await;
        assert_eq!(tampered.state, UpdateStateKind::Error);
        assert!(tampered.downloaded_path.is_none());

        // Valid manifest stages verified file cleanly.
        let ready = check_and_stage_update(
            &client,
            &state,
            true,
            Some(&format!("{base}/good.json")),
            temp.path(),
            "0.9.4",
        )
        .await;
        if option_env!("GNOMCODE_UPDATE_PUBLIC_KEY").is_some() {
            // A release key embedded at build time requires a signature this unsigned
            // fixture cannot carry; the embedded-key regression covers that path.
            assert_eq!(ready.state, UpdateStateKind::Error);
            assert!(ready.downloaded_path.is_none());
            server.abort();
            return;
        }
        assert_eq!(ready.state, UpdateStateKind::Ready);
        assert_eq!(ready.available_version.as_deref(), Some("9.9.9"));
        let staged = std::path::PathBuf::from(ready.downloaded_path.expect("staged path"));
        assert!(staged.exists());
        assert_eq!(
            tokio::fs::read(&staged).await.expect("read"),
            installer_bytes
        );

        server.abort();
    }
}
