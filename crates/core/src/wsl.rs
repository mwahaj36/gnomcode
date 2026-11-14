//! Pure WSL path helpers (ADR 0060): distro tags and Linux-to-UNC folder mapping.

fn parts(path: &str) -> impl Iterator<Item = &str> {
    path.split(['\\', '/'])
        .filter(|s| !s.is_empty() && *s != "?" && !s.eq_ignore_ascii_case("UNC"))
}

/// Returns the distro named by a `\\wsl.localhost\<distro>\...` or `\\wsl$\<distro>\...`
/// path (either separator, with or without a `\\?\UNC\` prefix).
#[must_use]
pub fn distro_of(path: &str) -> Option<&str> {
    if !(path.starts_with("\\\\") || path.starts_with("//")) {
        return None;
    }
    let mut it = parts(path);
    let host = it.next()?;
    if !(host.eq_ignore_ascii_case("wsl.localhost") || host.eq_ignore_ascii_case("wsl$")) {
        return None;
    }
    it.next()
}

/// Display tag for a WSL source, e.g. `WSL: Ubuntu`.
#[must_use]
pub fn distro_label(distro: &str) -> String {
    format!("WSL: {distro}")
}

/// Maps an absolute Linux folder seen inside `distro` to its Windows UNC spelling.
/// Returns `None` for anything that is not an absolute Linux path.
#[must_use]
pub fn linux_to_unc(distro: &str, linux: &str) -> Option<String> {
    if !linux.starts_with('/') || linux.starts_with("//") || distro.is_empty() {
        return None;
    }
    let rest = linux.trim_end_matches('/').replace('/', "\\");
    Some(format!("\\\\wsl.localhost\\{distro}{rest}"))
}

/// Maps a `\\wsl.localhost\<distro>\...` folder back to `(distro, linux path)`.
#[must_use]
pub fn unc_to_linux(path: &str) -> Option<(String, String)> {
    let distro = distro_of(path)?.to_owned();
    let mut out = String::new();
    for part in parts(path).skip(2) {
        out.push('/');
        out.push_str(part);
    }
    if out.is_empty() {
        out.push('/');
    }
    Some((distro, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distro_from_unc_spellings() {
        assert_eq!(distro_of(r"\\wsl.localhost\Ubuntu\home\a"), Some("Ubuntu"));
        assert_eq!(distro_of("//wsl$/Debian/x"), Some("Debian"));
        assert_eq!(
            distro_of(r"\\?\UNC\wsl.localhost\Ubuntu-22.04\h"),
            Some("Ubuntu-22.04")
        );
        assert_eq!(distro_of(r"C:\Users\a"), None);
        assert_eq!(distro_of(r"\\server\share"), None);
        assert_eq!(distro_of("/home/wsl.localhost/x"), None);
        assert_eq!(distro_label("Ubuntu"), "WSL: Ubuntu");
    }

    #[test]
    fn linux_and_unc_round_trip() {
        let unc = linux_to_unc("Ubuntu", "/home/a/proj/").expect("abs");
        assert_eq!(unc, r"\\wsl.localhost\Ubuntu\home\a\proj");
        assert_eq!(
            unc_to_linux(&unc),
            Some(("Ubuntu".into(), "/home/a/proj".into()))
        );
        assert_eq!(
            unc_to_linux(r"\\wsl$\Ubuntu"),
            Some(("Ubuntu".into(), "/".into()))
        );
        assert_eq!(linux_to_unc("Ubuntu", "relative"), None);
        assert_eq!(linux_to_unc("Ubuntu", r"C:\x"), None);
    }
}
