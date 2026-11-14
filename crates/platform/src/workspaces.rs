//! Read-only directory navigation for choosing an agent workspace.

use std::io;
use std::path::{Path, PathBuf};

/// A bounded directory-only listing. File contents are never opened.
#[derive(Debug)]
pub struct FolderListing {
    /// Current canonical directory.
    pub path: PathBuf,
    /// Parent directory, when present.
    pub parent: Option<PathBuf>,
    /// Home and local volume roots available for navigation.
    pub roots: Vec<PathBuf>,
    /// Sorted child directories; symlinks and files are omitted.
    pub folders: Vec<PathBuf>,
    /// Whether the listing exceeds the displayed directory limit.
    pub truncated: bool,
}

/// Lists directories for a confirmed Remote client, defaulting to the desktop home.
///
/// # Errors
/// Returns an I/O error for missing, inaccessible or non-directory paths.
pub fn browse(path: Option<&str>) -> io::Result<FolderListing> {
    let current = path
        .map_or_else(crate::user_home_dir, PathBuf::from)
        .canonicalize()?;
    if !current.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Choose a directory",
        ));
    }
    let entries = std::fs::read_dir(&current)?.take(4097).collect::<Vec<_>>();
    let scan_truncated = entries.len() > 4096;
    let mut folders = entries
        .into_iter()
        .take(4096)
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    folders.sort_by_key(|path| {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase()
    });
    let truncated = scan_truncated || folders.len() > 256;
    folders.truncate(256);
    let mut roots = vec![crate::user_home_dir()];
    #[cfg(windows)]
    roots.extend(win_drives::local_drive_roots());
    #[cfg(not(windows))]
    roots.push(PathBuf::from(std::path::MAIN_SEPARATOR_STR));
    Ok(FolderListing {
        parent: current.parent().map(Path::to_path_buf),
        path: current,
        roots,
        folders,
        truncated,
    })
}

#[cfg(windows)]
mod win_drives {
    #![allow(unsafe_code)]

    use std::path::PathBuf;

    const DRIVE_FIXED: u32 = 3;
    const DRIVE_RAMDISK: u32 = 6;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLogicalDrives() -> u32;
        fn GetDriveTypeW(root_path_name: *const u16) -> u32;
    }

    pub(super) fn local_drive_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();
        // SAFETY: `GetLogicalDrives` takes no arguments and returns a bitmask of drive letters.
        let mask = unsafe { GetLogicalDrives() };
        for offset in 0u8..26 {
            if mask & (1u32 << offset) == 0 {
                continue;
            }
            let letter = b'A' + offset;
            let wide = [u16::from(letter), u16::from(b':'), u16::from(b'\\'), 0];
            // SAFETY: `wide` is a valid NUL-terminated UTF-16 drive root string (`X:\`).
            let kind = unsafe { GetDriveTypeW(wide.as_ptr()) };
            if matches!(kind, DRIVE_FIXED | DRIVE_RAMDISK) {
                roots.push(PathBuf::from(format!(
                    "{}:{}",
                    char::from(letter),
                    std::path::MAIN_SEPARATOR
                )));
            }
        }
        roots
    }
}

/// Formats a canonical local drive path without the Windows extended-path prefix.
#[must_use]
pub fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    #[cfg(windows)]
    if let Some(plain) = value.strip_prefix(r"\\?\") {
        if plain.as_bytes().get(1) == Some(&b':') {
            return plain.to_owned();
        }
        if let Some(unc) = plain.strip_prefix(r"UNC\") {
            return format!(r"\\{unc}");
        }
    }
    value.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_returns_only_sorted_directories_and_a_parent() {
        let root = tempfile::tempdir().expect("directory");
        std::fs::create_dir(root.path().join("beta")).expect("folder");
        std::fs::create_dir(root.path().join("Alpha")).expect("folder");
        std::fs::write(root.path().join("secret.txt"), "not returned").expect("file");
        let listing = browse(root.path().to_str()).expect("listing");
        assert_eq!(
            listing
                .folders
                .iter()
                .map(|path| path
                    .file_name()
                    .expect("name")
                    .to_string_lossy()
                    .into_owned())
                .collect::<Vec<_>>(),
            ["Alpha", "beta"]
        );
        assert_eq!(listing.parent, listing.path.parent().map(Path::to_path_buf));
        assert!(!listing.truncated);
        assert!(browse(root.path().join("secret.txt").to_str()).is_err());
    }

    #[test]
    fn large_directory_lists_are_bounded() {
        let root = tempfile::tempdir().expect("directory");
        for i in 0..260 {
            std::fs::create_dir(root.path().join(format!("folder-{i:03}"))).expect("folder");
        }
        let listing = browse(root.path().to_str()).expect("listing");
        assert_eq!(listing.folders.len(), 256);
        assert!(listing.truncated);
    }

    #[cfg(windows)]
    #[test]
    fn display_path_strips_extended_prefixes() {
        assert_eq!(display_path(Path::new(r"\\?\C:\repo")), r"C:\repo");
        assert_eq!(
            display_path(Path::new(r"\\?\UNC\wsl.localhost\Ubuntu-22.04\tmp\repo")),
            r"\\wsl.localhost\Ubuntu-22.04\tmp\repo"
        );
    }
}
