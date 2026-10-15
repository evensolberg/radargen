//! NFR-14's atomic write: the bytes reach `-o` whole or not at all.
//!
//! The temporary file is `tempfile`'s: a random name opened with `O_EXCL`,
//! so no other process can predict or pre-create it, and in a sticky
//! world-writable directory a rename over another user's file fails closed.
//! Checking for a read-only target and then renaming is a race, but a benign
//! one: it can only let the write happen or refuse it, never send it
//! elsewhere. The directory is not fsynced after the rename, so a power loss
//! may undo the rename; atomicity holds either way (the old file or the new
//! one), and durability across power loss is out of scope.

use std::io::{self, Write};
use std::path::Path;

use crate::diagnostics::Fatal;

/// The mode a new output file is created with on unix, before the umask:
/// what `fs::write` uses, so 0644 under the usual umask of 022.
#[cfg(unix)]
const OUTPUT_MODE: u32 = 0o666;

/// `bytes` at `path`, atomically (NFR-14): written to a temporary file
/// beside it with the permissions `fs::write` would give, flushed to disk
/// and renamed into place.
///
/// On an error return any file already at `path` is byte-unchanged and the
/// temporary file is gone; a process killed mid-write can leave a
/// `.radargen-*.tmp` behind. A symlink at `path` is replaced, not followed,
/// and a replaced file takes the new file's mode and owner: rename
/// semantics. On Windows, replacing a file another process holds open, or
/// one being replaced at the same moment, fails with `Unwritable` and
/// leaves the existing file whole.
///
/// # Errors
///
/// [`Fatal::Unwritable`] when the directory is missing or unwritable, the
/// path names a directory or a read-only file, or the disk is full.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Fatal> {
    let unwritable = |source| Fatal::Unwritable {
        path: path.display().to_string(),
        source,
    };
    refuse_read_only(path).map_err(unwritable)?;
    let file = staged(path, bytes).map_err(unwritable)?;
    file.persist(path).map_err(|e| unwritable(e.error))?;
    Ok(())
}

/// `bytes` at `path` as [`write_atomic`] writes them, except that a file
/// already at `path` is never replaced (FR-99): the rename refuses one, so a
/// file appearing between a check and the write is refused too.
///
/// # Errors
///
/// [`Fatal::TemplateExists`] when anything is at `path`, a dangling symlink
/// included, and [`Fatal::TemplateUnwritable`] for every reason
/// [`write_atomic`] gives [`Fatal::Unwritable`].
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<(), Fatal> {
    let failed = |source: io::Error| {
        let path = path.display().to_string();
        if source.kind() == io::ErrorKind::AlreadyExists {
            Fatal::TemplateExists { path }
        } else {
            Fatal::TemplateUnwritable { path, source }
        }
    };
    let file = staged(path, bytes).map_err(failed)?;
    file.persist_noclobber(path).map_err(|e| failed(e.error))?;
    Ok(())
}

/// `bytes` in a temporary file beside `path`, with the permissions
/// `fs::write` would give, flushed to disk and ready to rename into place.
fn staged(path: &Path, bytes: &[u8]) -> io::Result<tempfile::NamedTempFile> {
    let mut builder = tempfile::Builder::new();
    builder.prefix(".radargen-").suffix(".tmp");
    // tempfile creates 0600; give the output what fs::write would, 0666
    // less the umask, so a published edition stays readable.
    #[cfg(unix)]
    builder.permissions(std::os::unix::fs::PermissionsExt::from_mode(OUTPUT_MODE));
    // tempfile's own errors name the random temporary path, which the user
    // never gave and which changes every run, and expose the OS error only
    // inside that message; keep its kind.
    let mut file = builder
        .tempfile_in(parent_dir(path))
        .map_err(|e| io::Error::from(e.kind()))?;
    file.as_file_mut().write_all(bytes)?;
    file.as_file().sync_all()?;
    Ok(file)
}

/// A read-only file at `path` is refused, as `fs::write` refuses it: a
/// rename would replace it on unix but not on Windows. A symlink is judged
/// as itself, since it is the link that is replaced.
fn refuse_read_only(path: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && meta.permissions().readonly() => {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        }
        _ => Ok(()),
    }
}

/// The directory `path` is in, `.` for a bare file name: where the temporary
/// file goes, so the rename stays on one filesystem (NFR-14), and what FR-62
/// checks exists.
pub(crate) fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// The names in `dir`, sorted.
    fn listing(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .expect("lists")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn writes_a_new_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.html");
        let bytes = b"<svg>\n</svg>\n";
        write_atomic(&path, bytes).expect("writes");
        assert_eq!(fs::read(&path).expect("reads"), bytes);
        assert_eq!(listing(dir.path()), ["radar.html"]);
    }

    #[test]
    fn replaces_an_existing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.html");
        fs::write(&path, b"old").expect("seeds");
        write_atomic(&path, b"new").expect("writes");
        assert_eq!(fs::read(&path).expect("reads"), b"new");
        assert_eq!(listing(dir.path()), ["radar.html"]);
    }

    #[test]
    fn parent_dir_names_the_parent() {
        assert_eq!(parent_dir(Path::new("out.html")), Path::new("."));
        assert_eq!(parent_dir(Path::new("a/out.html")), Path::new("a"));
        assert_eq!(parent_dir(Path::new("/out.html")), Path::new("/"));
    }

    /// A temp file is created and then `persist` fails: the drop guard's case.
    #[test]
    fn a_directory_at_the_path_is_unwritable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.html");
        fs::create_dir(&path).expect("makes the directory");
        let before = listing(dir.path());
        let error = write_atomic(&path, b"new").expect_err("refuses");
        assert_eq!(listing(dir.path()), before, "no temp file left");
        assert!(std::error::Error::source(&error).is_some());
        let text = error.to_string();
        let want = format!("-o {}: cannot be written: ", path.display());
        assert!(text.starts_with(&want), "{text}");
        assert!(!text.contains(".radargen-"), "{text}");
    }

    #[test]
    fn write_new_writes_a_new_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.csv");
        write_new(&path, b"name\n").expect("writes");
        assert_eq!(fs::read(&path).expect("reads"), b"name\n");
        assert_eq!(listing(dir.path()), ["radar.csv"]);
    }

    /// FR-99: the likeliest file at the path is the author's radar.
    #[test]
    fn write_new_never_replaces_a_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.csv");
        fs::write(&path, b"mine").expect("seeds");
        let error = write_new(&path, b"template").expect_err("refuses");
        assert!(matches!(error, Fatal::TemplateExists { .. }), "{error}");
        assert_eq!(fs::read(&path).expect("reads"), b"mine");
        assert_eq!(listing(dir.path()), ["radar.csv"], "no temp file left");
    }

    #[test]
    fn write_new_into_a_missing_directory_is_unwritable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nope").join("radar.csv");
        let error = write_new(&path, b"template").expect_err("refuses");
        assert!(matches!(error, Fatal::TemplateUnwritable { .. }), "{error}");
        assert!(!dir.path().join("nope").exists());
    }

    /// Clears a file's read-only flag when dropped, so the tempdir can go.
    struct Writable(PathBuf);

    impl Drop for Writable {
        fn drop(&mut self) {
            if let Ok(meta) = fs::metadata(&self.0) {
                let mut permissions = meta.permissions();
                #[expect(
                    clippy::permissions_set_readonly_false,
                    reason = "restoring a test file so its directory can be removed"
                )]
                permissions.set_readonly(false);
                let _ = fs::set_permissions(&self.0, permissions);
            }
        }
    }

    #[test]
    fn a_read_only_file_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.html");
        fs::write(&path, b"old").expect("seeds");
        let mut permissions = fs::metadata(&path).expect("meta").permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).expect("read-only");
        let _guard = Writable(path.clone());
        let error = write_atomic(&path, b"new").expect_err("refuses");
        assert!(matches!(error, Fatal::Unwritable { .. }), "{error}");
        assert_eq!(fs::read(&path).expect("reads"), b"old");
        assert_eq!(listing(dir.path()), ["radar.html"]);
    }

    #[test]
    fn a_missing_directory_is_unwritable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nope").join("radar.html");
        let error = write_atomic(&path, b"new").expect_err("refuses");
        assert!(matches!(error, Fatal::Unwritable { .. }), "{error}");
        assert!(!dir.path().join("nope").exists());
        assert!(!error.to_string().contains(".radargen-"), "{error}");
    }

    /// Last writer wins, never an interleaving. On Windows a rename onto a
    /// file another rename is replacing is refused (`Unwritable`); the file
    /// is whole either way, which is the guarantee.
    #[test]
    fn concurrent_writes_leave_one_whole_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.html");
        let payloads = [vec![b'a'; 1 << 20], vec![b'b'; 1 << 20]];
        std::thread::scope(|scope| {
            for payload in &payloads {
                let path = &path;
                scope.spawn(move || {
                    let wrote = (0..20)
                        .filter(|_| match write_atomic(path, payload) {
                            Ok(()) => true,
                            // Only Windows refuses a rename onto a file another
                            // rename is replacing; anywhere else it is a bug.
                            Err(error) if cfg!(windows) => {
                                assert!(matches!(error, Fatal::Unwritable { .. }), "{error}");
                                false
                            }
                            Err(error) => panic!("{error}"),
                        })
                        .count();
                    assert!(wrote > 0, "every write was refused");
                });
            }
        });
        let written = fs::read(&path).expect("reads");
        assert!(payloads.contains(&written), "interleaved");
        assert_eq!(listing(dir.path()), ["radar.html"]);
    }

    #[cfg(unix)]
    mod unix {
        use super::*;
        use std::os::unix::fs::PermissionsExt;

        fn mode(path: &Path) -> u32 {
            fs::metadata(path).expect("meta").permissions().mode() & 0o777
        }

        #[test]
        fn the_output_gets_the_mode_fs_write_gives() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("radar.html");
            write_atomic(&path, b"x").expect("writes");
            let plain = dir.path().join("plain.html");
            fs::write(&plain, b"x").expect("writes");
            assert_eq!(mode(&path), mode(&plain));
        }

        #[test]
        fn a_symlink_at_the_path_is_replaced_not_followed() {
            let dir = tempfile::tempdir().expect("tempdir");
            let target = dir.path().join("edition.html");
            fs::write(&target, b"edition").expect("seeds");
            let link = dir.path().join("latest.html");
            std::os::unix::fs::symlink(&target, &link).expect("links");
            write_atomic(&link, b"new").expect("writes");
            assert_eq!(fs::read(&target).expect("reads"), b"edition");
            assert!(fs::symlink_metadata(&link).expect("meta").is_file());
            assert_eq!(fs::read(&link).expect("reads"), b"new");
        }

        /// Restores a directory's mode when dropped.
        struct Restore(PathBuf, u32);

        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(self.1));
            }
        }

        #[test]
        fn a_read_only_directory_is_unwritable() {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("radar.html");
            fs::write(&path, b"old").expect("seeds");
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).expect("locks");
            let _restore = Restore(dir.path().to_path_buf(), 0o755);
            assert!(
                fs::write(dir.path().join("probe"), b"").is_err(),
                "the directory is still writable: running as root?"
            );
            let error = write_atomic(&path, b"new").expect_err("refuses");
            assert!(matches!(error, Fatal::Unwritable { .. }), "{error}");
            assert_eq!(fs::read(&path).expect("reads"), b"old");
            assert_eq!(listing(dir.path()), ["radar.html"]);
        }
    }
}
