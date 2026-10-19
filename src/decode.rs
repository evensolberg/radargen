//! Bytes to text (FR-2, FR-61), for every file the tool reads.
//!
//! The CSV, a `--palette` file (FR-102) and a `--config` file (FR-91). No
//! transcoding and no encoding flag: a non-UTF-8 file is an error that says
//! how to fix it (D-27).

use std::fs::File;
use std::io::{self, Read as _};
use std::path::Path;

/// FR-2: the UTF-8 byte-order mark.
const BOM: &[u8] = b"\xEF\xBB\xBF";

/// The input is not UTF-8, or holds a NUL byte (FR-61, D-65). `line` is the
/// 1-based physical line holding the first offending byte.
#[derive(Debug, PartialEq, Eq)]
pub struct NotUtf8 {
    /// The 1-based physical line holding the first offending byte.
    pub line: usize,
}

/// FR-2, then FR-1: strip one leading BOM, then require UTF-8.
///
/// # Errors
///
/// [`NotUtf8`] naming the line of the first byte that is not UTF-8, or of
/// the first NUL (FR-61).
pub fn decode(bytes: &[u8]) -> Result<String, NotUtf8> {
    let body = bytes.strip_prefix(BOM).unwrap_or(bytes);
    let text = std::str::from_utf8(body).map_err(|e| NotUtf8 {
        line: line_at(body, e.valid_up_to()),
    })?;
    // UTF-16 without a BOM is valid UTF-8 full of NULs, and no text file
    // holds one, so a NUL gets the same error and advice (re-save as UTF-8).
    if let Some(nul) = text.find('\0') {
        return Err(NotUtf8 {
            line: line_at(body, nul),
        });
    }
    Ok(text.to_string())
}

/// The bytes of the file at `path`, read through a cap so a wrong path such
/// as `/dev/zero` ends (NFR-15): `None` when it is larger than `max` bytes.
///
/// At most one byte past `max` is read. The file is not checked to be a
/// regular file, so a process substitution works.
///
/// # Errors
///
/// The I/O error when the file cannot be opened or read, a directory
/// included.
pub fn read_capped(path: &Path, max: usize) -> io::Result<Option<Vec<u8>>> {
    let cap = u64::try_from(max).map_or(u64::MAX, |max| max.saturating_add(1));
    let mut bytes = Vec::new();
    File::open(path)?.take(cap).read_to_end(&mut bytes)?;
    Ok((bytes.len() <= max).then_some(bytes))
}

/// The 1-based physical line holding byte `offset`, counting LF, CRLF and a
/// lone CR once each, as an editor does (NFR-10).
///
/// `csv::parse` counts the same way, so the two stages never disagree about a line; its test
/// `decode_and_parse_agree_on_every_small_input` checks that they agree.
#[must_use]
pub fn line_at(bytes: &[u8], offset: usize) -> usize {
    let mut line = 1;
    let mut after_cr = false;
    for &b in bytes.iter().take(offset) {
        // A CR is a break; an LF is one unless it completes a CRLF.
        if b == b'\r' || (b == b'\n' && !after_cr) {
            line += 1;
        }
        after_cr = b == b'\r';
    }
    line
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

    /// FR-61: UTF-16 without a BOM is valid UTF-8 full of NULs, and no text
    /// file holds one; the same error and advice apply.
    #[test]
    fn a_nul_byte_names_its_line() {
        assert_eq!(decode(b"n\0a\0m\0e\0\n\0"), Err(NotUtf8 { line: 1 }));
        assert_eq!(decode(b"name\nKa\0fka\n"), Err(NotUtf8 { line: 2 }));
        assert_eq!(decode(b"name\nKafka\n").as_deref(), Ok("name\nKafka\n"));
        // The NUL's own line, not the next one: the offset is not shifted.
        assert_eq!(decode(b"a\0\nb\n"), Err(NotUtf8 { line: 1 }));
        // Inside a quoted multi-line value the physical line still counts.
        assert_eq!(decode(b"a\n\"x\r\ny\0\"\n"), Err(NotUtf8 { line: 3 }));
    }

    /// NFR-10: CRLF is one break, and a lone CR is one too, as editors count.
    #[test]
    fn every_line_break_style_counts_once() {
        assert_eq!(decode(b"a\r\nb\r\n\xE9"), Err(NotUtf8 { line: 3 }));
        assert_eq!(decode(b"a\rb\r\xE9"), Err(NotUtf8 { line: 3 }));
        assert_eq!(decode(b"a\r\n\r\n\xE9"), Err(NotUtf8 { line: 3 }));
    }

    /// A file at the cap reads whole, and one byte past it is told apart,
    /// without reading more than one byte beyond (NFR-15).
    #[test]
    fn a_capped_read_tells_a_file_at_the_cap_from_one_past_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("f");
        std::fs::write(&path, b"abcd").expect("writes");
        assert_eq!(
            read_capped(&path, 4).expect("reads"),
            Some(b"abcd".to_vec())
        );
        assert_eq!(read_capped(&path, 3).expect("reads"), None);
        assert!(read_capped(dir.path(), 4).is_err(), "a directory");
    }
}
