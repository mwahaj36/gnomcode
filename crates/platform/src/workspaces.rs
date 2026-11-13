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
    for drive in b'A'..=b'Z' {
        let root = PathBuf::from(format!(
            "{}:{}",
            char::from(drive),
            std::path::MAIN_SEPARATOR
        ));
        if root.is_dir() {
            roots.push(root);
        }
    }
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

/// Formats a canonical local drive path without the Windows extended-path prefix.
#[must_use]
pub fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    #[cfg(windows)]
    if let Some(plain) = value.strip_prefix(r"\\?\")
        && plain.as_bytes().get(1) == Some(&b':')
    {
        return plain.to_owned();
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
}
