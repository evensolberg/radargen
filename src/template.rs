//! FR-99: the starter CSV `--create-template` writes, for an author to fill
//! in rather than guess the columns from the PRD (D-83).
//!
//! The text is a file beside this one, so it reads as the CSV it is. The
//! tests below hold it to the parser: every column in order, every kind of
//! value shown, and no warning when it is read; `tests/run.rs` renders it.

/// The starter CSV: a header naming every column the tool reads, then
/// example rows to replace or delete.
pub const TEMPLATE: &str = include_str!("template.csv");

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
        let catalogue = build_catalogue(
            &table,
            &rings,
            crate::diagnostics::OffRadar::Listed,
            &mut warnings,
        )
        .expect("builds");
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
        // Each optional free-text and date column, both blank and filled.
        for kind in [
            ColumnKind::Domain,
            ColumnKind::Rationale,
            ColumnKind::LastAffirmed,
            ColumnKind::ExitDate,
            ColumnKind::Link,
        ] {
            let column = catalogue.column(kind).expect("a column");
            let filled: Vec<bool> = items.iter().map(|i| i.cell(column).is_some()).collect();
            assert!(
                filled.contains(&true) && filled.contains(&false),
                "{kind:?}"
            );
        }
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
