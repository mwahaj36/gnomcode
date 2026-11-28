//! Pure update manifest parsing, SemVer comparison, URL validation, and
//! checksum/signature verification (`SPEC §6.4`, ADR 0033).

use std::collections::BTreeMap;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Check interval for background update manifest polling (6 hours).
pub const UPDATE_CHECK_INTERVAL_SECS: u64 = 6 * 3600;

/// Platform entry in `latest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatePlatformAsset {
    /// HTTPS download URL for this platform installer or archive.
    pub url: String,
    /// Required lowercase/uppercase 64-char SHA-256 hex digest.
    pub sha256: String,
    /// Optional 64-byte Ed25519 signature encoded as 128-char hex or base64.
    #[serde(default)]
    pub signature: Option<String>,
    /// Optional explicit asset file name.
    #[serde(default)]
    pub file_name: Option<String>,
}

/// Signed/hash-verified update manifest JSON (`latest.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateManifest {
    /// Release SemVer string (e.g. `"0.9.5"`).
    pub version: String,
    /// Optional RFC 3339 publication timestamp.
    #[serde(default)]
    pub pub_date: Option<String>,
    /// Optional release notes URL.
    #[serde(default)]
    pub notes_url: Option<String>,
    /// Per-target packages (`win32-x64`, `darwin-arm64`, `linux-x64`, etc.).
    #[serde(default)]
    pub platforms: BTreeMap<String, UpdatePlatformAsset>,
    /// Optional current Remote relay WebSocket URL, published alongside releases so the
    /// relay backend can move (e.g. after a provider outage) without shipping a new build.
    /// Desktops only adopt this when [`is_valid_wss_relay_url`] accepts it; the connection
    /// itself is re-validated before use regardless (`SPEC` Remote relay notes).
    #[serde(default)]
    pub remote_relay_url: Option<String>,
}

/// Returns `true` when `url` is a clean, single-segment `wss://.../relay` endpoint: the
/// shape required of a Remote relay URL published in the update manifest. Stricter than
/// what a desktop accepts locally for development (plain `ws://127.0.0.1:*`/`localhost:*`
/// is never valid here), since this value arrives over the network.
#[must_use]
pub fn is_valid_wss_relay_url(url: &str) -> bool {
    url.starts_with("wss://")
        && url.len() <= 512
        && url.ends_with("/relay")
        && !url.contains(['?', '#', '&'])
}

/// Lifecycle state of in-app desktop auto-update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStateKind {
    /// Running the latest known release.
    UpToDate,
    /// Newer release found in manifest (pending download).
    Available,
    /// Downloading and verifying installer in the background.
    Downloading,
    /// Verified installer is staged on disk; waits for user click or quit.
    Ready,
    /// Automatic update checks opted out by user or env.
    Disabled,
    /// Last check or download failed open.
    Error,
}

/// Snapshot exposed to Electron and local console Settings via `/api/v1/update/status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateStatus {
    /// Current lifecycle state.
    pub state: UpdateStateKind,
    /// Running version (`CARGO_PKG_VERSION`).
    pub current_version: String,
    /// Manifest target version when newer.
    pub available_version: Option<String>,
    /// Verified local installer path inside `GNOMCODE_HOME/updates/`.
    pub downloaded_path: Option<String>,
    /// Expected/verified SHA-256 hex digest of `downloaded_path`.
    pub sha256: Option<String>,
    /// Opt-out preference persisted locally (default `true`).
    pub auto_update_enabled: bool,
    /// RFC 3339 timestamp of the last check attempt.
    pub last_checked_at: Option<String>,
    /// Short non-fatal error summary when in `Error` state.
    pub last_error: Option<String>,
}

impl UpdateStatus {
    /// Builds an initial up-to-date status snapshot.
    #[must_use]
    pub fn initial(current_version: &str, auto_update_enabled: bool) -> Self {
        Self {
            state: if auto_update_enabled {
                UpdateStateKind::UpToDate
            } else {
                UpdateStateKind::Disabled
            },
            current_version: current_version.to_owned(),
            available_version: None,
            downloaded_path: None,
            sha256: None,
            auto_update_enabled,
            last_checked_at: None,
            last_error: None,
        }
    }
}

/// Canonical `(major, minor, patch)` SemVer comparison; ignores invalid/prerelease versions.
#[must_use]
pub fn parse_stable_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let trimmed = raw.trim().strip_prefix('v').unwrap_or(raw.trim());
    let mut parts = trimmed.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Returns `true` when `candidate` is strictly newer than `current` (both stable SemVer).
#[must_use]
pub fn is_newer_version(current: &str, candidate: &str) -> bool {
    match (parse_stable_semver(current), parse_stable_semver(candidate)) {
        (Some(cur), Some(cand)) => cand > cur,
        _ => false,
    }
}

/// Maps OS + arch constants into manifest platform target keys.
#[must_use]
pub fn platform_target(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("windows", "x86_64") => Some("win32-x64"),
        ("windows", "aarch64") => Some("win32-arm64"),
        ("macos", "aarch64") => Some("darwin-arm64"),
        ("macos", "x86_64") => Some("darwin-x64"),
        ("linux", "x86_64") => Some("linux-x64"),
        ("linux", "aarch64") => Some("linux-arm64"),
        _ => None,
    }
}

/// Returns `true` when `url` is a clean HTTPS URL on an approved release origin.
#[must_use]
pub fn is_allowed_release_url(url: &str, allowed_prefixes: &[&str]) -> bool {
    if !url.starts_with("https://")
        || url.contains('@')
        || url.contains('?')
        || url.contains('#')
        || url.len() > 1024
    {
        return false;
    }
    allowed_prefixes.iter().any(|prefix| {
        !prefix.is_empty()
            && url
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    })
}

/// Verifies `bytes` SHA-256 against a 64-character hex digest.
#[must_use]
pub fn verify_sha256_hex(bytes: &[u8], expected_hex: &str) -> bool {
    let expected = expected_hex.trim();
    if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
        return false;
    }
    let digest = Sha256::digest(bytes);
    let actual_hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    actual_hex.eq_ignore_ascii_case(expected)
}

/// Verifies an Ed25519 signature (128-char hex) over `bytes`.
#[must_use]
pub fn verify_ed25519_hex(bytes: &[u8], signature_hex: &str, public_key_hex: &str) -> bool {
    let Some(sig_bytes) = decode_fixed_hex::<64>(signature_hex.trim()) else {
        return false;
    };
    let Some(key_bytes) = decode_fixed_hex::<32>(public_key_hex.trim()) else {
        return false;
    };
    let Ok(key) = VerifyingKey::from_bytes(&key_bytes) else {
        return false;
    };
    let sig = Signature::from_bytes(&sig_bytes);
    key.verify(bytes, &sig).is_ok()
}

fn decode_fixed_hex<const N: usize>(hex: &str) -> Option<[u8; N]> {
    if hex.len() != N * 2 || !hex.is_ascii() {
        return None;
    }
    let mut out = [0_u8; N];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Validates and returns a safe single-segment installer file name.
#[must_use]
pub fn safe_asset_filename(url: &str, explicit_name: Option<&str>) -> Option<String> {
    let raw = explicit_name
        .filter(|s| !s.trim().is_empty())
        .or_else(|| url.rsplit('/').next())?;
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.len() > 128
        || trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        return None;
    }
    let allowed_ext = [
        ".exe",
        ".msi",
        ".zip",
        ".dmg",
        ".deb",
        ".rpm",
        ".AppImage",
        ".tar.gz",
    ]
    .iter()
    .any(|ext| trimmed.ends_with(ext));
    allowed_ext.then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn semver_comparison_rejects_downgrades_equal_and_prereleases() {
        assert!(is_newer_version("0.9.4", "0.9.5"));
        assert!(is_newer_version("0.9.4", "v1.0.0"));
        assert!(!is_newer_version("0.9.4", "0.9.4"));
        assert!(!is_newer_version("0.9.5", "0.9.4"));
        assert!(!is_newer_version("0.9.4", "0.9.5-rc.1"));
        assert!(!is_newer_version("0.9.4", "invalid"));
    }

    #[test]
    fn allowed_release_url_blocks_untrusted_origins_http_and_credentials() {
        let allowed = [
            "https://gnomcode.vercel.app/downloads",
            "https://github.com/mwahaj36/gnomcode-releases/releases",
        ];
        assert!(is_allowed_release_url(
            "https://gnomcode.vercel.app/downloads/Gnomcode-Setup.exe",
            &allowed
        ));
        assert!(is_allowed_release_url(
            "https://github.com/mwahaj36/gnomcode-releases/releases/download/v0.9.5/Gnomcode-Setup.exe",
            &allowed
        ));
        assert!(!is_allowed_release_url(
            "http://gnomcode.vercel.app/downloads/Gnomcode-Setup.exe",
            &allowed
        ));
        assert!(!is_allowed_release_url(
            "https://evil.example/downloads/Gnomcode-Setup.exe",
            &allowed
        ));
        assert!(!is_allowed_release_url(
            "https://user@gnomcode.vercel.app/downloads/Gnomcode-Setup.exe",
            &allowed
        ));
        assert!(!is_allowed_release_url(
            "https://gnomcode.vercel.app/downloads/Gnomcode-Setup.exe?redirect=bad",
            &allowed
        ));
    }

    #[test]
    fn verifies_sha256_and_ed25519_signatures_strictly() {
        let payload = b"gnomcode verified installer bytes";
        let digest = Sha256::digest(payload);
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        assert!(verify_sha256_hex(payload, &hex));
        assert!(!verify_sha256_hex(b"tampered bytes", &hex));
        assert!(!verify_sha256_hex(payload, "short"));

        let signing = SigningKey::from_bytes(&[7_u8; 32]);
        let verifying_hex: String = signing
            .verifying_key()
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let sig = signing.sign(payload);
        let sig_hex: String = sig.to_bytes().iter().map(|b| format!("{b:02x}")).collect();

        assert!(verify_ed25519_hex(payload, &sig_hex, &verifying_hex));
        assert!(!verify_ed25519_hex(
            b"tampered bytes",
            &sig_hex,
            &verifying_hex
        ));
    }

    #[test]
    fn rejects_path_traversal_and_unsafe_asset_filenames() {
        assert_eq!(
            safe_asset_filename("https://example.com/Gnomcode-Setup.exe", None).as_deref(),
            Some("Gnomcode-Setup.exe")
        );
        assert!(
            safe_asset_filename("https://example.com/../evil.exe", Some("../evil.exe")).is_none()
        );
        assert!(safe_asset_filename("https://example.com/script.ps1", None).is_none());
    }
}
