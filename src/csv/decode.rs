//! Bytes to text (FR-1, FR-2, FR-61). No transcoding and no encoding flag:
//! a non-UTF-8 file is an error that says how to fix it (D-27).

use std::path::Path;

use crate::diagnostics::Fatal;

/// FR-2: the UTF-8 byte-order mark.
const BOM: &[u8] = b"\xEF\xBB\xBF";

/// The input is not UTF-8. `line` is the 1-based physical line holding the
/// first byte that is not.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct NotUtf8 {
    pub(crate) line: usize,
}

/// FR-2, then FR-1: strip one leading BOM, then require UTF-8.
///
/// # Errors
///
/// [`NotUtf8`] naming the line of the first byte that is not UTF-8.
pub(crate) fn decode(bytes: &[u8]) -> Result<String, NotUtf8> {
    let body = bytes.strip_prefix(BOM).unwrap_or(bytes);
    std::str::from_utf8(body)
        .map(str::to_string)
        .map_err(|e| NotUtf8 {
            line: line_at(body, e.valid_up_to()),
        })
}

/// The 1-based physical line holding byte `offset`, counting LF, CRLF and a
/// lone CR once each, as an editor does (NFR-10). `csv::parse` counts the
/// same way, so the two stages never disagree about a line; its test
/// `decode_and_parse_agree_on_every_small_input` checks that they agree.
fn line_at(bytes: &[u8], offset: usize) -> usize {
    let mut line = 1;
    let mut after_cr = false;
    for &b in &bytes[..offset] {
        // A CR is a break; an LF is one unless it completes a CRLF.
        if b == b'\r' || (b == b'\n' && !after_cr) {
            line += 1;
        }
        after_cr = b == b'\r';
    }
    line
}

/// FR-61: read and decode the input file. The one place the CSV stage
/// touches the file system (Decisions).
///
/// # Errors
///
/// [`Fatal::Unreadable`] when the file cannot be read; [`Fatal::NotUtf8`]
/// when it is not UTF-8.
pub fn read(path: &Path) -> Result<String, Fatal> {
    let shown = path.display().to_string();
    let bytes = std::fs::read(path).map_err(|source| Fatal::Unreadable {
        path: shown.clone(),
        source,
    })?;
    decode(&bytes).map_err(|NotUtf8 { line }| Fatal::NotUtf8 { path: shown, line })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-2: Excel writes a BOM by default; left in, the first header reads
    /// as `\u{feff}name` and FR-5 fails on a file that looks correct.
    #[test]
    fn a_leading_bom_is_stripped_once() {
        assert_eq!(
            decode(b"\xEF\xBB\xBFname,ring\n").as_deref(),
            Ok("name,ring\n")
        );
        assert_eq!(
            decode(b"\xEF\xBB\xBF\xEF\xBB\xBFname").as_deref(),
            Ok("\u{feff}name"),
            "only one BOM is the encoding's; a second is content"
        );
        assert_eq!(decode(b"name").as_deref(), Ok("name"));
        assert_eq!(decode(b"").as_deref(), Ok(""));
        assert_eq!(
            decode(BOM).as_deref(),
            Ok(""),
            "a BOM-only file is empty, for parse's NoHeader"
        );
    }

    /// FR-61, D-27: the likely cause is a Windows-1252 export, and the line
    /// is what lets the author find the offending cell.
    #[test]
    fn a_legacy_code_page_names_its_line() {
        assert_eq!(decode(b"name\nKafka\nCaf\xE9\n"), Err(NotUtf8 { line: 3 }));
        assert_eq!(decode(b"\xE9"), Err(NotUtf8 { line: 1 }));
        assert_eq!(decode(b"\xEF\xBB\xBFa\n\xE9"), Err(NotUtf8 { line: 2 }));
        // Excel's "Unicode Text" is UTF-16: refused at line 1, never transcoded (D-27).
        assert_eq!(decode(b"\xFF\xFEn\x00a\x00"), Err(NotUtf8 { line: 1 }));
        assert_eq!(decode(b"\xFE\xFF\x00n\x00a"), Err(NotUtf8 { line: 1 }));
        // A sequence cut off by the end of the file.
        assert_eq!(decode(b"name\n\xC3"), Err(NotUtf8 { line: 2 }));
        // FR-61 names the byte's line, even inside a multi-line quoted field.
        assert_eq!(decode(b"a\n\"x\ny\xE9\"\n"), Err(NotUtf8 { line: 3 }));
    }

    /// NFR-10: CRLF is one break, and a lone CR is one too, as editors count.
    #[test]
    fn every_line_break_style_counts_once() {
        assert_eq!(decode(b"a\r\nb\r\n\xE9"), Err(NotUtf8 { line: 3 }));
        assert_eq!(decode(b"a\rb\r\xE9"), Err(NotUtf8 { line: 3 }));
        assert_eq!(decode(b"a\r\n\r\n\xE9"), Err(NotUtf8 { line: 3 }));
    }

    #[test]
    fn a_missing_file_is_unreadable_and_names_its_path() {
        let path =
            std::env::temp_dir().join(format!("radargen-no-such-file-{}.csv", std::process::id()));
        let err = read(&path).expect_err("missing");
        assert!(matches!(err, Fatal::Unreadable { .. }), "{err:?}");
        let shown = err.render();
        assert!(
            shown.contains(&format!("{}: cannot be read", path.display())),
            "{shown}"
        );
    }

    /// FR-61 names a directory as unreadable too. On Unix the open succeeds
    /// and the read fails; on Windows (NFR-13) the open fails. Both land here.
    #[test]
    fn a_directory_is_unreadable() {
        let err = read(&std::env::temp_dir()).expect_err("a directory");
        assert!(matches!(err, Fatal::Unreadable { .. }), "{err:?}");
    }

    /// FR-61: a file the user may not read. Unix only: Windows has no mode
    /// bits to clear.
    #[cfg(unix)]
    #[test]
    fn a_file_without_read_permission_is_unreadable() {
        use std::os::unix::fs::PermissionsExt;
        let path =
            std::env::temp_dir().join(format!("radargen-no-read-{}.csv", std::process::id()));
        std::fs::write(&path, "name\n").expect("write fixture");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("chmod");
        let result = read(&path);
        std::fs::remove_file(&path).expect("remove fixture");
        assert!(
            matches!(result, Err(Fatal::Unreadable { .. })),
            "{result:?}"
        );
    }

    /// `read` of a temporary file holding `bytes`, removed before returning.
    fn read_fixture(name: &str, bytes: &[u8]) -> Result<String, Fatal> {
        let path = write_fixture(name, bytes);
        let text = read(&path);
        remove_fixture(&path);
        text
    }

    /// Write `bytes` to a per-process temporary CSV and return its path.
    fn write_fixture(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("radargen-{name}-{}.csv", std::process::id()));
        std::fs::write(&path, bytes).expect("the temp directory is writable");
        path
    }

    /// Remove a fixture written by `write_fixture`.
    fn remove_fixture(path: &std::path::Path) {
        std::fs::remove_file(path).expect("the fixture was just written");
    }

    #[test]
    fn a_non_utf8_file_says_how_to_fix_it() {
        let err = read_fixture("cp1252", b"name\nCaf\xE9\n").expect_err("not UTF-8");
        let shown = err.render();
        assert!(
            shown.contains(", line 2: not UTF-8 text; re-save the file as UTF-8"),
            "{shown}"
        );
    }

    #[test]
    fn a_utf8_file_reads_back_without_its_bom() {
        let text = read_fixture("utf8", "\u{feff}name\nÆrø\n".as_bytes());
        assert_eq!(text.expect("reads"), "name\nÆrø\n");
    }
}
