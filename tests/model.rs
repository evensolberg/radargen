//! CSV in, catalogue out (architecture §2): `csv::parse` and
//! `model::build_catalogue` together, on whole files.

use radargen::csv::parse::parse;
use radargen::diagnostics::{Fatal, Warnings};
use radargen::model::{ColumnKind, Qualifier, Status, build_catalogue};
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
    let cat = build_catalogue(&table, &rings, &mut w).expect("builds");

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
    let rationale = cat.column(ColumnKind::Rationale).expect("rationale");
    assert_eq!(
        cat.items.get(6).expect("items[6] exists").cell(rationale),
        Some("Declined during the 2026 review — see the domain ADR.")
    );
    // FR-22: the domain verbatim; FR-25: an exit date whatever the ring.
    let domain = cat.column(ColumnKind::Domain).expect("domain");
    assert_eq!(
        cat.items.first().expect("items[0] exists").cell(domain),
        Some("Infrastructure as Code")
    );
    let exit = cat.column(ColumnKind::ExitDate).expect("exit_date");
    assert_eq!(
        cat.items.get(3).expect("items[3] exists").cell(exit),
        Some("2027-01-31")
    );

    assert_eq!(
        w.render(),
        [
            "warning: line 8, ring: 'Rejected' is not declared by the default ring set; the row is in the table view, not on the radar"
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
    let model_errors = build_catalogue(&table, &rings, &mut w).expect_err("fails");
    assert!(w.render().is_empty(), "{:?}", w.render());
    let rendered = |errs: &[Fatal]| errs.iter().map(Fatal::render).collect::<Vec<_>>();
    assert_eq!(
        rendered(&parse_errors),
        [
            "error: line 4: the row's field count is 3 but the header's is 4; a value is missing, or an unquoted line break split the row",
            "error: line 7: the row's field count is 5 but the header's is 4; a comma inside a value needs the value in double quotes",
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
    let errs = build_catalogue(&table, &rings, &mut w).expect_err("fails");
    assert_eq!(
        errs.iter().map(Fatal::render).collect::<Vec<_>>(),
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
    build_catalogue(&table, &rings, &mut w).expect("builds");
    assert_eq!(
        w.render(),
        [
            "warning: line 3, ring: 'Adpot' is not declared by the default ring set; the row is in the table view, not on the radar"
        ]
    );
}
