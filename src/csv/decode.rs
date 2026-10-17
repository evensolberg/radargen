//! The CSV's file read (FR-1, FR-61): the one place the CSV stage touches
//! the file system. Decoding is [`crate::decode`]'s, shared with the
//! `--palette` file.

use std::fs::File;
use std::io::Read as _;
use std::path::Path;

use crate::decode::{NotUtf8, decode};
use crate::diagnostics::Fatal;

/// The largest input read (rg-w5k).
///
/// A radar's CSV is a few kilobytes, and ten thousand rows with paragraph-long descriptions are about 20 MB; far
/// past anything a page can show. The cap is there so a wrong path —
/// `/dev/zero`, a disk image, a log — fails with a message rather than
/// exhausting memory, since the text is held, decoded and parsed whole.
pub const MAX_BYTES: usize = 64 << 20;
/// What [`read`] reads at most: one byte past the cap, to tell a file at
/// the cap from one beyond it.
const READ_CAP: u64 = (64 << 20) + 1;

/// FR-61: read and decode the input file. The one place the CSV stage
/// touches the file system (Decisions).
///
/// Read through a cap rather than whole, so `radargen /dev/zero` ends; not
/// checked to be a regular file, so a process substitution still works.
///
/// # Errors
///
/// [`Fatal::Unreadable`] when the file cannot be read; [`Fatal::CsvTooLarge`]
/// when it is larger than [`MAX_BYTES`]; [`Fatal::NotUtf8`] when it is not
/// UTF-8, or holds a NUL byte (FR-61, D-65).
pub fn read(path: &Path) -> Result<String, Fatal> {
    let shown = path.display().to_string();
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(READ_CAP).read_to_end(&mut bytes))
        .map_err(|source| Fatal::Unreadable {
            path: shown.clone(),
            source,
        })?;
    if bytes.len() > MAX_BYTES {
        return Err(Fatal::CsvTooLarge { path: shown });
    }
    decode(&bytes).map_err(|NotUtf8 { line }| Fatal::NotUtf8 { path: shown, line })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    #[expect(
        clippy::unwrap_in_result,
        reason = "the Result is the value under test; a fixture IO failure is a test bug and should panic"
    )]
    fn read_fixture(name: &str, bytes: &[u8]) -> Result<String, Fatal> {
        let path = std::env::temp_dir().join(format!("radargen-{name}-{}.csv", std::process::id()));
        std::fs::write(&path, bytes).expect("the temp directory is writable");
        let text = read(&path);
        std::fs::remove_file(&path).expect("the fixture was just written");
        text
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

    /// rg-w5k: a file past the cap is refused before it is decoded, naming
    /// its path. Sparse files (`set_len`), so no test writes 64 MiB: zeros
    /// at the cap pass the cap and fail decoding on their NUL instead.
    #[test]
    fn a_file_past_the_cap_is_too_large_and_names_its_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("big.csv");
        let file = std::fs::File::create(&path).expect("creates");
        file.set_len(READ_CAP).expect("grows");
        let err = read(&path).expect_err("past the cap");
        assert!(matches!(err, Fatal::CsvTooLarge { .. }), "{err:?}");
        let shown = err.render();
        assert!(
            shown.contains(&format!("{}: larger than 64 MiB", path.display())),
            "{shown}"
        );
        file.set_len(READ_CAP - 1).expect("shrinks");
        assert!(
            matches!(read(&path), Err(Fatal::NotUtf8 { line: 1, .. })),
            "at the cap, the file is read whole and decoded"
        );
    }

    /// rg-w5k: `/dev/zero` never ends; the cap ends the read.
    #[cfg(unix)]
    #[test]
    fn a_file_that_never_ends_is_too_large() {
        let err = read(Path::new("/dev/zero")).expect_err("never ends");
        assert!(matches!(err, Fatal::CsvTooLarge { .. }), "{err:?}");
    }

    /// `READ_CAP` is one byte past `MAX_BYTES`, so a file at the cap reads
    /// whole and one past it is told apart.
    #[test]
    fn the_read_cap_is_one_past_the_limit() {
        assert_eq!(u64::try_from(MAX_BYTES + 1), Ok(READ_CAP));
    }

    #[test]
    fn a_utf8_file_reads_back_without_its_bom() {
        let text = read_fixture("utf8", "\u{feff}name\nÆrø\n".as_bytes());
        assert_eq!(text.expect("reads"), "name\nÆrø\n");
    }
}
