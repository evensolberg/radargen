//! FR-99, FR-101: the files the tool writes to start from, each given by
//! its own flag alone: a CSV to fill in and a palette to edit (D-83, D-91).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use crate::diagnostics::Fatal;
use crate::palette::SAMPLE;
use crate::template::TEMPLATE;

/// A file to start from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Starter {
    /// `--create-template`: the starter CSV (FR-99).
    Template,
    /// `--create-palette`: the built-in light palette as a `--palette` file
    /// (FR-101).
    Palette,
}

impl Starter {
    /// The flag that asks for it, as a message names it.
    #[must_use]
    pub const fn flag(self) -> &'static str {
        match self {
            Self::Template => "--create-template",
            Self::Palette => "--create-palette",
        }
    }

    /// What it writes.
    #[must_use]
    pub const fn contents(self) -> &'static str {
        match self {
            Self::Template => TEMPLATE,
            Self::Palette => SAMPLE,
        }
    }

    /// Writes [`Self::contents`] at `path`, never replacing anything there.
    ///
    /// The file is created with `create_new` (`O_EXCL`), so a file that
    /// appears between a check and the write is refused too, and a dangling
    /// symlink is refused rather than followed. It is written in place, not
    /// renamed into place as `-o` is (NFR-14): a no-replace rename is
    /// unsupported on some filesystems, exFAT on macOS among them, and with
    /// nothing there before, a rename has nothing to protect. A failed write
    /// removes the file; a process killed mid-write can leave part of it.
    ///
    /// # Errors
    ///
    /// [`Fatal::StarterExists`] when anything is at `path`, and
    /// [`Fatal::StarterUnwritable`] when it cannot be written.
    pub fn create(self, path: &Path) -> Result<(), Fatal> {
        let flag = self.flag();
        let unwritable = |source| Fatal::StarterUnwritable {
            flag,
            path: path.display().to_string(),
            source,
        };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|source: io::Error| match source.kind() {
                io::ErrorKind::AlreadyExists => Fatal::StarterExists {
                    flag,
                    path: path.display().to_string(),
                },
                _ => unwritable(source),
            })?;
        file.write_all(self.contents().as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|source| {
                drop(file);
                let _ = fs::remove_file(path);
                unwritable(source)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOTH: [Starter; 2] = [Starter::Template, Starter::Palette];

    #[test]
    fn create_writes_the_contents() {
        for starter in BOTH {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("start");
            starter.create(&path).expect("writes");
            assert_eq!(
                fs::read_to_string(&path).expect("reads"),
                starter.contents()
            );
        }
    }

    /// FR-99, FR-101: the likeliest file at the path is the author's own.
    #[test]
    fn create_never_replaces_a_file() {
        for starter in BOTH {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("start");
            fs::write(&path, b"mine").expect("seeds");
            let error = starter.create(&path).expect_err("refuses");
            assert!(
                matches!(error, Fatal::StarterExists { flag, .. } if flag == starter.flag()),
                "{error}"
            );
            assert_eq!(fs::read(&path).expect("reads"), b"mine");
        }
    }

    /// A dangling symlink is refused, not followed to create its target.
    #[cfg(unix)]
    #[test]
    fn create_refuses_a_dangling_symlink() {
        for starter in BOTH {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("start");
            let target = dir.path().join("elsewhere");
            std::os::unix::fs::symlink(&target, &path).expect("links");
            let error = starter.create(&path).expect_err("refuses");
            assert!(matches!(error, Fatal::StarterExists { .. }), "{error}");
            assert!(!target.exists());
        }
    }

    #[test]
    fn create_in_a_missing_directory_is_unwritable() {
        for starter in BOTH {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("nope").join("start");
            let error = starter.create(&path).expect_err("refuses");
            assert!(
                matches!(error, Fatal::StarterUnwritable { flag, .. } if flag == starter.flag()),
                "{error}"
            );
        }
    }
}
