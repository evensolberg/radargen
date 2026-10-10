//! `--rings` parsing and validation (FR-12–FR-15, FR-63).

use radargen::rings::{RingsError, parse_rings};

#[test]
fn omitting_the_flag_yields_the_in_house_default() {
    // FR-13, centre-to-edge (FR-12).
    assert_eq!(
        parse_rings(None).expect("default is valid"),
        vec!["Retire", "Adopt", "Trial", "Assess"]
    );
}

#[test]
fn entries_are_trimmed_but_keep_internal_spaces() {
    // FR-15: `"Adopt, Trial"` and `"Adopt,Trial"` parse identically, while a
    // multi-word ring name survives intact.
    assert_eq!(
        parse_rings(Some("Retire, In Use , Adopt")).expect("valid"),
        vec!["Retire", "In Use", "Adopt"]
    );
}

#[test]
fn a_single_ring_is_valid() {
    // FR-63: the list must yield at least one.
    assert_eq!(parse_rings(Some("Adopt")).expect("valid"), vec!["Adopt"]);
}

#[test]
fn an_empty_entry_is_rejected_and_names_its_position() {
    // FR-63: an empty entry would draw a nameless band.
    match parse_rings(Some("Adopt,,Trial")) {
        Err(RingsError::EmptyEntry { index }) => assert_eq!(index, 2),
        other => panic!("expected EmptyEntry, got {other:?}"),
    }
}

#[test]
fn an_all_empty_list_is_rejected() {
    assert!(parse_rings(Some("")).is_err());
    assert!(parse_rings(Some("   ")).is_err());
}

#[test]
fn a_duplicate_is_rejected_under_the_matching_rule() {
    // FR-63 + FR-76: duplicate after the same trim-and-fold used for matching,
    // so `Adopt` and `adopt ` collide. A duplicate would make FR-16's match
    // ambiguous and FR-32's band order undefined.
    match parse_rings(Some("Adopt,Trial,adopt ")) {
        Err(RingsError::Duplicate { entry, .. }) => assert_eq!(entry, "adopt"),
        other => panic!("expected Duplicate, got {other:?}"),
    }
}

#[test]
fn the_declared_casing_is_what_gets_displayed() {
    // FR-76: normalisation never edits displayed text beyond FR-15's trim —
    // the declared name is what appears as a band label and legend subheading.
    assert_eq!(
        parse_rings(Some("ADOPT,trial")).expect("valid"),
        vec!["ADOPT", "trial"]
    );
}
