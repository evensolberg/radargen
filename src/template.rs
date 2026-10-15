//! FR-99: the starter CSV `--create-template` writes, for an author to fill
//! in rather than guess the columns from the PRD (D-83).
//!
//! The text is a file beside this one, so it reads as the CSV it is. The
//! tests below hold it to the parser: every column in order, every kind of
//! value shown, and no warning when it is read; `tests/run.rs` renders it.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use crate::diagnostics::Fatal;

/// The starter CSV: a header naming every column the tool reads, then
/// example rows to replace or delete.
pub const TEMPLATE: &str = include_str!("template.csv");

/// FR-99: [`TEMPLATE`] at `path`, never replacing anything there.
///
/// The file is created with `create_new` (`O_EXCL`), so a file that appears
/// between a check and the write is refused too, and a dangling symlink is
/// refused rather than followed. It is written in place, not renamed into
/// place as `-o` is (NFR-14): a no-replace rename is unsupported on some
/// filesystems, exFAT on macOS among them, and with nothing there before,
/// a rename has nothing to protect. A failed write removes the file; a
/// process killed mid-write can leave part of it.
///
/// # Errors
///
/// [`Fatal::TemplateExists`] when anything is at `path`, and
/// [`Fatal::TemplateUnwritable`] when it cannot be written.
pub fn create(path: &Path) -> Result<(), Fatal> {
    let shown = || path.display().to_string();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| match source.kind() {
            io::ErrorKind::AlreadyExists => Fatal::TemplateExists { path: shown() },
            _ => Fatal::TemplateUnwritable {
                path: shown(),
                source,
            },
        })?;
    file.write_all(TEMPLATE.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|source| {
            drop(file);
            let _ = fs::remove_file(path);
            Fatal::TemplateUnwritable {
                path: shown(),
                source,
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::columns::ColumnKind;
    use crate::csv::parse::parse;
    use crate::diagnostics::Warnings;
    use crate::model::{Catalogue, Qualifier, Status, build_catalogue};
    use crate::rings::{DEFAULT_RINGS, parse_rings};

    /// The template as the default rings read it, and what that warned.
    fn catalogue() -> (Catalogue, Warnings) {
        let mut warnings = Warnings::default();
        let (table, errors) = parse(TEMPLATE, &mut warnings).expect("parses");
        assert!(errors.is_empty(), "{errors:?}");
        let rings = parse_rings(None).expect("the default rings");
        let catalogue = build_catalogue(&table, &rings, &mut warnings).expect("builds");
        (catalogue, warnings)
    }

    #[test]
    fn the_header_names_every_column_in_order() {
        let header = TEMPLATE.lines().next().unwrap_or_default();
        let known: Vec<&str> = ColumnKind::KNOWN.iter().map(|k| k.as_str()).collect();
        assert_eq!(header, known.join(","));
    }

    #[test]
    fn it_reads_with_no_warning() {
        let (catalogue, warnings) = catalogue();
        assert!(warnings.is_empty(), "{:?}", warnings.render());
        assert!(catalogue.items.iter().all(|item| item.ring.is_some()));
    }

    #[test]
    fn it_shows_every_kind_of_value() {
        let (catalogue, _) = catalogue();
        let items = &catalogue.items;
        for ring in 0..DEFAULT_RINGS.len() {
            assert!(items.iter().any(|i| i.ring == Some(ring)), "ring {ring}");
        }
        for qualifier in [Some(Qualifier::Default), Some(Qualifier::Conditional), None] {
            assert!(
                items.iter().any(|i| i.qualifier == qualifier),
                "{qualifier:?}"
            );
        }
        for status in Status::ALL.map(Some).into_iter().chain([None]) {
            assert!(items.iter().any(|i| i.status == status), "{status:?}");
        }
        assert!(items.iter().any(|i| i.link.is_some()), "a link");
        let blank = |kind| {
            let column = catalogue.column(kind).expect("a column");
            items.iter().any(|i| i.cell(column).is_none())
        };
        for kind in [
            ColumnKind::Domain,
            ColumnKind::Rationale,
            ColumnKind::LastAffirmed,
            ColumnKind::ExitDate,
            ColumnKind::Link,
        ] {
            assert!(blank(kind), "a blank {kind:?}");
        }
        let dated = |kind| {
            let column = catalogue.column(kind).expect("a column");
            items.iter().any(|i| i.cell(column).is_some())
        };
        assert!(dated(ColumnKind::LastAffirmed) && dated(ColumnKind::ExitDate));
    }

    #[test]
    fn create_writes_the_template() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.csv");
        create(&path).expect("writes");
        assert_eq!(fs::read_to_string(&path).expect("reads"), TEMPLATE);
    }

    /// FR-99: the likeliest file at the path is the author's radar.
    #[test]
    fn create_never_replaces_a_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.csv");
        fs::write(&path, b"mine").expect("seeds");
        let error = create(&path).expect_err("refuses");
        assert!(matches!(error, Fatal::TemplateExists { .. }), "{error}");
        assert_eq!(fs::read(&path).expect("reads"), b"mine");
    }

    /// A dangling symlink is refused, not followed to create its target.
    #[cfg(unix)]
    #[test]
    fn create_refuses_a_dangling_symlink() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("radar.csv");
        let target = dir.path().join("elsewhere.csv");
        std::os::unix::fs::symlink(&target, &path).expect("links");
        let error = create(&path).expect_err("refuses");
        assert!(matches!(error, Fatal::TemplateExists { .. }), "{error}");
        assert!(!target.exists());
    }

    #[test]
    fn create_in_a_missing_directory_is_unwritable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nope").join("radar.csv");
        let error = create(&path).expect_err("refuses");
        assert!(matches!(error, Fatal::TemplateUnwritable { .. }), "{error}");
    }

    /// NFR-17: LF, no byte-order mark, and ASCII, so Excel opens it
    /// correctly without one (D-83).
    #[test]
    fn it_is_plain_ascii_with_lf_endings() {
        assert!(TEMPLATE.is_ascii());
        assert!(!TEMPLATE.contains('\r'));
        assert!(TEMPLATE.ends_with('\n'));
    }
}
