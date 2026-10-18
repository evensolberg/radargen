//! CSV in, catalogue out (architecture §2): `csv::parse` and
//! `model::build_catalogue` together, on whole files.

use radargen::csv::parse::parse;
use radargen::diagnostics::{OffRadar, RowError, Warnings};
use radargen::model::{ColumnKind, Link, Qualifier, Status, build_catalogue};
use radargen::rings::parse_rings;

/// PRD §6.1.6's example, architecture §10's `three-quadrant.csv` (SC-2):
/// three quadrants, every optional column empty at least once, and one row
/// whose ring the default set does not declare.
#[test]
fn the_prd_example_builds_with_one_warning() {
    let mut w = Warnings::default();
    let (table, errs) = parse(include_str!("fixtures/three-quadrant.csv"), &mut w).expect("parses");
    assert!(errs.is_empty(), "{errs:?}");
    let rings = parse_rings(None).expect("default");
    let cat = build_catalogue(&table, &rings, OffRadar::Listed, &mut w).expect("builds");

    let quadrants: Vec<_> = cat.quadrants.iter().map(|q| q.label.as_str()).collect();
    assert_eq!(quadrants, ["Tool", "Platform", "Practice"]);
    assert_eq!(cat.items.len(), 7);
    let unplotted: Vec<_> = cat
        .items
        .iter()
        .filter(|i| i.ring.is_none())
        .map(|i| i.name.as_str())
        .collect();
    assert_eq!(unplotted, ["Legacy Ticketing-Driven Change"]);

    // FR-20 and FR-26 on real rows.
    let qualifiers: Vec<_> = cat.items.iter().map(|i| i.qualifier).collect();
    let (d, c) = (Some(Qualifier::Default), Some(Qualifier::Conditional));
    assert_eq!(qualifiers, [d, c, None, None, d, d, None]);
    let statuses: Vec<_> = cat.items.iter().map(|i| i.status).collect();
    assert_eq!(
        statuses,
        [
            Some(Status::NoChange),
            Some(Status::New),
            Some(Status::MovedIn),
            None,
            None,
            None,
            None
        ]
    );
    // FR-23: free text is kept verbatim.
    let [first, _, _, fourth, _, _, seventh] = cat.items.as_slice() else {
        panic!("expected seven items: {:#?}", cat.items)
    };
    let rationale = cat.column(ColumnKind::Rationale).expect("rationale");
    assert_eq!(
        seventh.cell(rationale),
        Some("Declined during the 2026 review — see the domain ADR.")
    );
    // FR-22: the domain verbatim; FR-25: an exit date whatever the ring.
    let domain = cat.column(ColumnKind::Domain).expect("domain");
    assert_eq!(first.cell(domain), Some("Infrastructure as Code"));
    let exit = cat.column(ColumnKind::ExitDate).expect("exit_date");
    assert_eq!(fourth.cell(exit), Some("2027-01-31"));

    assert_eq!(
        w.render(),
        [
            "warning: line 8, ring: 'Rejected' is not one of the default rings (Assess, Trial, Adopt, Retire); the row is in the table view, not on the radar; to plot it, pass --rings with every ring you want, since --rings replaces the default set"
        ]
    );
}

/// NFR-16 tier two and architecture §10's `row-errors.csv`: FR-4's errors
/// from `parse` and FR-5's and FR-6's from `build_catalogue` all arrive
/// in one run, and the rows `parse` rejected do not hide the others. Each
/// list is in line order; `main.rs` merges the two (PR 14).
#[test]
fn row_errors_from_both_stages_arrive_in_one_run() {
    let mut w = Warnings::default();
    let (table, parse_errors) =
        parse(include_str!("fixtures/row-errors.csv"), &mut w).expect("parses");
    let rings = parse_rings(None).expect("default");
    let model_errors =
        build_catalogue(&table, &rings, OffRadar::Listed, &mut w).expect_err("fails");
    assert!(w.render().is_empty(), "{:?}", w.render());
    let rendered = |errs: &[RowError]| errs.iter().map(RowError::render).collect::<Vec<_>>();
    assert_eq!(
        rendered(&parse_errors),
        [
            "error: line 4: the row has 3 values but the header has 4 columns; a row with fewer values is usually missing one, or an unquoted line break split it",
            "error: line 7: the row has 5 values but the header has 4 columns; a row with more values usually has a comma that needs the value in double quotes",
        ]
    );
    assert_eq!(
        rendered(&model_errors),
        [
            "error: line 1: the header has no 'description' column; add one, or rename the column that holds it — the header must name at least name, quadrant, ring and description",
            "error: line 3, name: empty; every row needs a name, so fill it in or delete the row",
            "error: line 5, ring: empty; every row needs a ring, so fill it in or delete the row",
            "error: line 8, quadrant: empty; every row needs a quadrant, so fill it in or delete the row",
        ]
    );
}

/// A repeated required header is FR-4's error in `parse`, which still
/// returns the table. The model reads the first `name` and does not panic.
#[test]
fn a_repeated_required_header_reads_the_first() {
    let mut w = Warnings::default();
    let (table, parse_errors) = parse(
        "name,quadrant,ring,description,Name\n,T,Adopt,d,K\n",
        &mut w,
    )
    .expect("parses");
    assert_eq!(parse_errors.len(), 1);
    let rings = parse_rings(None).expect("default");
    let errs = build_catalogue(&table, &rings, OffRadar::Listed, &mut w).expect_err("fails");
    assert_eq!(
        errs.iter().map(RowError::render).collect::<Vec<_>>(),
        ["error: line 2, name: empty; every row needs a name, so fill it in or delete the row"]
    );
}

/// A row FR-4 rejected does not stop the model warning for the rest; what
/// a failed run prints is NFR-11's rule, applied by `main.rs` (PR 14).
#[test]
fn parse_errors_do_not_suppress_model_warnings() {
    let mut w = Warnings::default();
    let (table, errs) =
        parse("name,quadrant,ring,description\nK,T\nL,T,Adpot,d\n", &mut w).expect("parses");
    assert_eq!(errs.len(), 1);
    let rings = parse_rings(None).expect("default");
    build_catalogue(&table, &rings, OffRadar::Listed, &mut w).expect("builds");
    assert_eq!(
        w.render(),
        [
            "warning: line 3, ring: 'Adpot' is not one of the default rings (Assess, Trial, Adopt, Retire); the row is in the table view, not on the radar; to plot it, pass --rings with every ring you want, since --rings replaces the default set"
        ]
    );
}

/// FR-96: the `link` header matches under FR-76, an `http` or `https` value
/// is an address and any other non-blank value is text, with one warning per
/// refused row; a line break in the cell is a space (FR-97).
#[test]
fn the_link_column_is_read_once_per_row() {
    for header in ["link", " Link ", "LINK"] {
        let csv = format!(
            "name,quadrant,ring,description,{header}\n\
             A,T,Adopt,d,https://a.example\n\
             B,T,Adopt,d,javascript:alert(1)\n\
             C,T,Adopt,d,\"https://c.example/\r\nmore\"\n\
             D,T,Adopt,d,\n"
        );
        let mut w = Warnings::default();
        let (table, errs) = parse(&csv, &mut w).expect("parses");
        assert!(errs.is_empty(), "{errs:?}");
        let rings = parse_rings(None).expect("default");
        let cat = build_catalogue(&table, &rings, OffRadar::Listed, &mut w).expect("builds");
        let links: Vec<Option<Link>> = cat.items.iter().map(|i| i.link.clone()).collect();
        assert_eq!(
            links,
            [
                Some(Link::Address("https://a.example".into())),
                Some(Link::Text("javascript:alert(1)".into())),
                Some(Link::Address("https://c.example/ more".into())),
                None,
            ],
            "{header:?}"
        );
        assert_eq!(
            w.render(),
            [
                "warning: line 3, link: 'javascript:alert(1)' is not an http or https address; shown as text, not a link"
            ],
            "{header:?}: one warning, for the refused row"
        );
    }
}

/// Two `link` columns are FR-4's duplicate header, so the model never has to
/// choose between them.
#[test]
fn two_link_columns_are_a_duplicate_header() {
    let mut w = Warnings::default();
    let (_, errs) = parse(
        "name,quadrant,ring,description,link,Link\nA,T,Adopt,d,x,y\n",
        &mut w,
    )
    .expect("parses");
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs.iter()
            .map(RowError::render)
            .all(|e| e.contains("link")),
        "{errs:?}"
    );
}
