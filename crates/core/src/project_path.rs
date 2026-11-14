//! Project identity from a working directory (SPEC §3.6). Pure string rules, no I/O.
//!
//! Agents report the directory a session started in. That is not always a project: it can be a
//! drive root, a home folder, a Codex per-chat scratch folder or a temporary directory, and the
//! same folder arrives spelled several ways (`/` vs `\`, doubled separators, percent-encoding,
//! `wsl.localhost` with or without the leading `//`).

/// Canonical key shared by every session without a usable working directory.
pub const UNASSIGNED_KEY: &str = "/unknown";
/// Canonical key for Codex per-chat scratch folders (`.../Codex/<yyyy-mm-dd>/<slug>`).
pub const CODEX_CHATS_KEY: &str = "scratch/codex-chats";
/// Canonical key for folders under an operating-system or test temporary directory.
pub const TEMPORARY_KEY: &str = "scratch/temporary";

const HOME_FOLDERS: [&str; 3] = ["desktop", "documents", "downloads"];

/// Identity derived from a directory path.
pub(crate) struct PathProject {
    pub key: String,
    pub name: String,
    pub repo_root: String,
}

pub(crate) fn from_path(raw: &str) -> PathProject {
    let slashed = percent_decode(raw.trim()).replace('\\', "/");
    // `\\?\C:\x` and `\\?\UNC\host\share` are the same folders as `C:\x` and `\\host\share`.
    let slashed = match slashed.strip_prefix("//?/") {
        Some(rest) => match rest
            .get(..4)
            .filter(|head| head.eq_ignore_ascii_case("UNC/"))
        {
            Some(_) => format!("//{}", &rest[4..]),
            None => rest.to_owned(),
        },
        None => slashed,
    };
    let leading = slashed.len() - slashed.trim_start_matches('/').len();
    let mut segments: Vec<&str> = slashed.split('/').filter(|s| !s.is_empty()).collect();
    // `wsl$` is the legacy name of the `wsl.localhost` share: one folder, one key.
    if let Some(first) = segments
        .first_mut()
        .filter(|s| s.eq_ignore_ascii_case("wsl$"))
    {
        *first = "wsl.localhost";
    }
    let wsl = segments
        .first()
        .is_some_and(|s| s.eq_ignore_ascii_case("wsl.localhost") || s.eq_ignore_ascii_case("wsl$"));
    let unc = leading >= 2 || wsl;
    let drive = !unc
        && segments
            .first()
            .is_some_and(|s| s.len() == 2 && s.ends_with(':'));

    let body = segments.join("/");
    let cleaned = if unc {
        format!("//{body}")
    } else if leading == 1 {
        format!("/{body}")
    } else {
        body
    };
    let anchor = if unc {
        segments.len().min(2)
    } else {
        usize::from(drive)
    };
    let rest: Vec<String> = segments[anchor..]
        .iter()
        .map(|s| s.to_lowercase())
        .collect();

    let (key, name) = if is_unassigned(&rest) {
        (UNASSIGNED_KEY.to_owned(), "unknown".to_owned())
    } else if is_codex_chat(&rest) {
        (CODEX_CHATS_KEY.to_owned(), "Codex chats".to_owned())
    } else if is_temporary(&rest) {
        (TEMPORARY_KEY.to_owned(), "Temporary folders".to_owned())
    } else {
        (
            cleaned.to_lowercase(),
            segments.last().copied().unwrap_or("unknown").to_owned(),
        )
    };
    let repo_root = if key == UNASSIGNED_KEY {
        let raw = raw.trim();
        if raw.is_empty() { UNASSIGNED_KEY } else { raw }.to_owned()
    } else if raw.contains('\\') {
        cleaned.replace('/', "\\")
    } else {
        cleaned
    };
    PathProject {
        key,
        name,
        repo_root,
    }
}

/// No folder at all: a drive or share root, a home folder, a standard home subfolder or the system directory.
fn is_unassigned(rest: &[String]) -> bool {
    let Some(first) = rest.first() else {
        return true;
    };
    match first.as_str() {
        "unknown" => rest.len() == 1,
        "windows" => true,
        "root" => rest.len() == 1,
        "users" | "home" => {
            rest.len() <= 2 || (rest.len() == 3 && HOME_FOLDERS.contains(&rest[2].as_str()))
        }
        _ => false,
    }
}

fn is_codex_chat(rest: &[String]) -> bool {
    rest.windows(3)
        .any(|w| w[0] == "codex" && is_iso_date(&w[1]))
}

fn is_iso_date(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

fn is_temporary(rest: &[String]) -> bool {
    rest.iter().enumerate().any(|(i, segment)| {
        matches!(segment.as_str(), "temp" | "tmp")
            || (i + 1 < rest.len()
                && ["-temp", "-tmp", "_temp", "_tmp"]
                    .iter()
                    .any(|suffix| segment.ends_with(suffix)))
    })
}

fn percent_decode(text: &str) -> String {
    if !text.contains('%') {
        return text.to_owned();
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(raw: &str) -> String {
        from_path(raw).key
    }

    #[test]
    fn drive_share_and_home_roots_are_unassigned() {
        for raw in [
            "C:/",
            "C:\\",
            "D:/",
            "c:",
            "/",
            "",
            "/unknown",
            "//wsl.localhost/Ubuntu-22.04/",
            "wsl.localhost/Ubuntu-22.04",
            "C:/Users/Wahaj",
            "C:\\Users\\Wahaj\\Desktop",
            "c:/Users/Wahaj/Documents",
            "C:/WINDOWS/System32",
            "/home/dev",
            "/Users/dev/Downloads",
            "//wsl.localhost/Ubuntu-22.04/home/wahaj",
        ] {
            assert_eq!(key(raw), UNASSIGNED_KEY, "{raw}");
        }
        assert_eq!(from_path("C:/").name, "unknown");
    }

    #[test]
    fn codex_chat_folders_share_one_project() {
        let a = from_path("C:\\Users\\Wahaj\\Documents\\Codex\\2026-10-21\\bas");
        let b = from_path("c:/Users/Wahaj/Documents/Codex/2026-11-09/cle");
        assert_eq!(a.key, CODEX_CHATS_KEY);
        assert_eq!(a.key, b.key);
        assert_eq!(a.name, "Codex chats");
        assert_eq!(
            a.repo_root,
            "C:\\Users\\Wahaj\\Documents\\Codex\\2026-10-21\\bas"
        );
        assert_ne!(key("C:/Users/Wahaj/Documents/Codex"), CODEX_CHATS_KEY);
        assert_ne!(
            key("C:/Users/Wahaj/Documents/Codex/notes/x"),
            CODEX_CHATS_KEY
        );
    }

    #[test]
    fn temporary_directories_share_one_project() {
        for raw in [
            "D:\\spawnt-ide-test-temp\\real-codex-QT57Qs\\account-retest-1793248674048",
            "C:/Users/Wahaj/AppData/Local/Temp/gnomcode-live-audit-20261106",
            "//wsl.localhost/Ubuntu-22.04/tmp/pr74/web",
            "/tmp/build",
        ] {
            assert_eq!(key(raw), TEMPORARY_KEY, "{raw}");
        }
        assert_eq!(from_path("/tmp/build").name, "Temporary folders");
        assert_ne!(
            key("C:/code/foo-temp"),
            TEMPORARY_KEY,
            "the folder itself is a real name"
        );
        assert_ne!(key("C:/code/temp-test"), TEMPORARY_KEY);
    }

    #[test]
    fn separator_percent_and_unc_spellings_collapse() {
        let reference = key("C:/Users/Wahaj/Documents/GitHub/fleethud");
        for raw in [
            "c:\\\\Users\\\\Wahaj\\\\Documents\\\\GitHub\\\\fleethud",
            "C:\\Users\\Wahaj\\Documents\\GitHub\\fleethud\\",
            "C://Users//Wahaj//Documents//GitHub//fleethud",
            "\\\\?\\C:\\Users\\Wahaj\\Documents\\GitHub\\fleethud",
        ] {
            assert_eq!(key(raw), reference, "{raw}");
        }
        let verbatim = from_path("\\\\?\\UNC\\wsl.localhost\\Ubuntu-22.04\\home\\wahaj\\RECON");
        assert_eq!(
            verbatim.key,
            key("wsl.localhost/Ubuntu-22.04/home/wahaj/RECON")
        );
        assert_eq!(
            verbatim.repo_root,
            "\\\\wsl.localhost\\Ubuntu-22.04\\home\\wahaj\\RECON"
        );
        assert_eq!(
            key("//wsl.localhost/Ubuntu-22.04/home/wahaj/RECON"),
            key("wsl.localhost/Ubuntu-22.04/home/wahaj/RECON")
        );
        let encoded = from_path("c:/Users/Wahaj/Documents/infosec%20assignment1/a2");
        assert_eq!(
            encoded.key,
            "c:/users/wahaj/documents/infosec assignment1/a2"
        );
        assert_eq!(
            encoded.repo_root,
            "c:/Users/Wahaj/Documents/infosec assignment1/a2"
        );
        assert_eq!(key("C:/x/50%/y"), "c:/x/50%/y");
    }

    #[test]
    fn names_keep_the_folder_spelling_and_repo_root_keeps_the_separator_style() {
        let project = from_path("C:\\\\Users\\\\Wahaj\\\\Documents\\\\GitHub\\\\Subtext");
        assert_eq!(project.name, "Subtext");
        assert_eq!(
            project.repo_root,
            "C:\\Users\\Wahaj\\Documents\\GitHub\\Subtext"
        );
    }
}
