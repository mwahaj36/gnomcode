//! File preview policy: type detection and size caps for workspace and artifact previews.
//!
//! Pure: callers read bytes and pass them in. The UI only renders the classified result.

use serde::Serialize;

/// Text previews are cut at this many bytes and flagged as truncated.
pub const TEXT_PREVIEW_CAP: u64 = 512 * 1024;
/// Images, PDFs and downloads are sent inline (base64) only up to this size.
pub const INLINE_BYTES_CAP: u64 = 1024 * 1024;
/// Bytes inspected for binary detection.
const SNIFF_BYTES: usize = 8 * 1024;

/// How the console should present a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewKind {
    /// Raster image (png, jpg, gif, webp).
    Image,
    /// SVG image, shown only as an `<img>` (no script, no external loads).
    Svg,
    /// PDF document.
    Pdf,
    /// Markdown rendered with the chat renderer.
    Markdown,
    /// JSON shown pretty-printed.
    Json,
    /// Comma-separated values shown as a table.
    Csv,
    /// Source code with a language label.
    Code,
    /// Plain text.
    Text,
    /// Binary or unknown: name, size and download only.
    Binary,
}

/// Classified preview metadata for a file name and its leading bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreviewPolicy {
    /// Presentation kind.
    pub kind: PreviewKind,
    /// MIME type for inline data and downloads.
    pub mime_type: &'static str,
    /// Highlighter language label for code, empty otherwise.
    pub language: &'static str,
}

impl PreviewPolicy {
    /// Whether the kind is shown from bytes (base64) rather than text.
    #[must_use]
    pub const fn is_inline_binary(&self) -> bool {
        matches!(
            self.kind,
            PreviewKind::Image | PreviewKind::Svg | PreviewKind::Pdf | PreviewKind::Binary
        )
    }
}

fn extension(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    match base.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ext.to_ascii_lowercase(),
        _ => base.to_ascii_lowercase(),
    }
}

const CODE: &[(&str, &str)] = &[
    ("rs", "rust"),
    ("js", "javascript"),
    ("mjs", "javascript"),
    ("cjs", "javascript"),
    ("jsx", "javascript"),
    ("ts", "typescript"),
    ("tsx", "typescript"),
    ("py", "python"),
    ("go", "go"),
    ("java", "java"),
    ("kt", "kotlin"),
    ("swift", "swift"),
    ("c", "c"),
    ("h", "c"),
    ("cpp", "cpp"),
    ("cc", "cpp"),
    ("hpp", "cpp"),
    ("cs", "csharp"),
    ("rb", "ruby"),
    ("php", "php"),
    ("sh", "shell"),
    ("bash", "shell"),
    ("zsh", "shell"),
    ("ps1", "powershell"),
    ("sql", "sql"),
    ("lua", "lua"),
    ("toml", "toml"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("css", "css"),
    ("scss", "css"),
    ("html", "html"),
    ("htm", "html"),
    ("xml", "xml"),
    ("diff", "diff"),
    ("patch", "diff"),
    ("dockerfile", "dockerfile"),
    ("makefile", "makefile"),
];

/// Classifies a file by extension, then by content when the extension is unknown.
#[must_use]
pub fn classify(name: &str, head: &[u8]) -> PreviewPolicy {
    let ext = extension(name);
    let policy = |kind, mime_type, language| PreviewPolicy {
        kind,
        mime_type,
        language,
    };
    match ext.as_str() {
        "png" => return policy(PreviewKind::Image, "image/png", ""),
        "jpg" | "jpeg" => return policy(PreviewKind::Image, "image/jpeg", ""),
        "gif" => return policy(PreviewKind::Image, "image/gif", ""),
        "webp" => return policy(PreviewKind::Image, "image/webp", ""),
        "svg" => return policy(PreviewKind::Svg, "image/svg+xml", ""),
        "pdf" => return policy(PreviewKind::Pdf, "application/pdf", ""),
        _ => {}
    }
    let sniff = &head[..head.len().min(SNIFF_BYTES)];
    if sniff.contains(&0) {
        return policy(PreviewKind::Binary, "application/octet-stream", "");
    }
    match ext.as_str() {
        "md" | "markdown" | "mdx" => policy(PreviewKind::Markdown, "text/markdown", ""),
        "json" | "jsonl" | "geojson" => policy(PreviewKind::Json, "application/json", "json"),
        "csv" => policy(PreviewKind::Csv, "text/csv", ""),
        _ => CODE.iter().find(|(e, _)| *e == ext).map_or_else(
            || {
                // Invalid UTF-8 outside a cut multibyte tail means binary.
                let valid = match std::str::from_utf8(sniff) {
                    Ok(_) => true,
                    Err(err) => err.error_len().is_none(),
                };
                if valid {
                    policy(PreviewKind::Text, "text/plain", "")
                } else {
                    policy(PreviewKind::Binary, "application/octet-stream", "")
                }
            },
            |(_, lang)| policy(PreviewKind::Code, "text/plain", lang),
        ),
    }
}

/// How many bytes to read for a file of `size`, and whether the result is truncated.
#[must_use]
pub fn read_budget(policy: &PreviewPolicy, size: u64) -> (u64, bool) {
    if policy.is_inline_binary() {
        if size > INLINE_BYTES_CAP {
            (0, true)
        } else {
            (size, false)
        }
    } else {
        (size.min(TEXT_PREVIEW_CAP), size > TEXT_PREVIEW_CAP)
    }
}

/// Encodes bytes as standard base64 for `data:` URLs.
#[must_use]
pub fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(TABLE[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_each_preview_type() {
        for (name, kind) in [
            ("a.PNG", PreviewKind::Image),
            ("a.jpeg", PreviewKind::Image),
            ("a.gif", PreviewKind::Image),
            ("a.webp", PreviewKind::Image),
            ("logo.svg", PreviewKind::Svg),
            ("doc.pdf", PreviewKind::Pdf),
            ("README.md", PreviewKind::Markdown),
            ("data.json", PreviewKind::Json),
            ("rows.csv", PreviewKind::Csv),
            ("src/main.rs", PreviewKind::Code),
            ("notes.txt", PreviewKind::Text),
            ("Makefile", PreviewKind::Code),
        ] {
            assert_eq!(classify(name, b"hello").kind, kind, "{name}");
        }
        assert_eq!(classify("main.rs", b"x").language, "rust");
    }

    #[test]
    fn nul_or_invalid_utf8_is_binary() {
        assert_eq!(classify("a.bin", b"ab\0c").kind, PreviewKind::Binary);
        assert_eq!(classify("a.rs", b"ab\0c").kind, PreviewKind::Binary);
        assert_eq!(
            classify("a.dat", &[0xff, 0xfe, 0x41]).kind,
            PreviewKind::Binary
        );
        // A multibyte character cut at the sniff boundary is still text.
        assert_eq!(
            classify("a.txt", &[b'a', 0xe2, 0x82]).kind,
            PreviewKind::Text
        );
    }

    #[test]
    fn size_caps_truncate_text_and_withhold_large_binaries() {
        let text = classify("a.txt", b"x");
        assert_eq!(read_budget(&text, 10), (10, false));
        assert_eq!(
            read_budget(&text, TEXT_PREVIEW_CAP + 1),
            (TEXT_PREVIEW_CAP, true)
        );
        let image = classify("a.png", b"");
        assert_eq!(
            read_budget(&image, INLINE_BYTES_CAP),
            (INLINE_BYTES_CAP, false)
        );
        assert_eq!(read_budget(&image, INLINE_BYTES_CAP + 1), (0, true));
    }

    #[test]
    fn base64_matches_reference() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"f"), "Zg==");
        assert_eq!(encode_base64(b"fo"), "Zm8=");
        assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");
    }
}

/// Local Markdown link destinations referenced by a conversation, independent of provider.
/// Network links and fragment-only links never become file access requests.
#[must_use]
pub fn linked_file_paths(text: &str) -> Vec<String> {
    text.split("](")
        .skip(1)
        .filter_map(|part| {
            let target = if let Some(angle) = part.strip_prefix('<') {
                angle.split_once('>')?.0
            } else {
                part.split_once(')')?.0.split_whitespace().next()?
            };
            let target = target.split('#').next()?.trim();
            if target.is_empty() || target.starts_with("//") {
                return None;
            }
            if let Some((scheme, _)) = target.split_once(':')
                && scheme != "file"
                && !(scheme.len() == 1 && scheme.as_bytes()[0].is_ascii_alphabetic())
            {
                return None;
            }
            Some(target.to_owned())
        })
        .take(100)
        .collect()
}

#[cfg(test)]
mod reference_tests {
    #[test]
    fn linked_files_are_provider_independent_and_ignore_network_links() {
        assert_eq!(
            super::linked_file_paths(
                "[Handoff](localdocs/handoff.md) [web](https://example.org) [File](<docs/a b.md>)"
            ),
            vec!["localdocs/handoff.md", "docs/a b.md"]
        );
    }
}
