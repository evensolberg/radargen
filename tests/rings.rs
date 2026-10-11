//! `--rings` parsing and validation (FR-12–FR-15, FR-63).

use radargen::rings::{RingSource, RingsError, parse_rings};

#[test]
fn omitting_the_flag_yields_the_in_house_default() {
    // FR-13, edge-to-centre (FR-12, D-60): the lifecycle as it is read.
    assert_eq!(
        parse_rings(None).expect("default is valid").rings,
        vec!["Assess", "Trial", "Adopt", "Retire"]
    );
}

/// FR-15 uses FR-76's trim, so a zero-width space pasted in with a ring
/// name neither shows in its band label nor stops it matching, nor hides
/// a duplicate (D-61).
/// FR-17 (D-61): the warning names where the rings came from.
#[test]
fn the_set_records_its_source() {
    assert_eq!(
        parse_rings(None).expect("default is valid").source,
        RingSource::Default
    );
    assert_eq!(
        parse_rings(Some("Adopt")).expect("valid").source,
        RingSource::Flag
    );
}

#[test]
fn entries_lose_invisible_format_characters_at_their_ends() {
    assert_eq!(
        parse_rings(Some("\u{feff}Adopt\u{200b}, Trial"))
            .expect("valid")
            .rings,
        vec!["Adopt", "Trial"]
    );
    assert!(matches!(
        parse_rings(Some("Adopt,\u{200b}")).expect_err("empty entry")[..],
        [RingsError::EmptyEntry { index: 2 }]
    ));
    assert!(matches!(
        parse_rings(Some("Adopt,\u{200b}adopt")).expect_err("duplicate")[..],
        [RingsError::Duplicate {
            index: 2,
            first_index: 1,
            ..
        }]
    ));
}

#[test]
fn entries_are_trimmed_but_keep_internal_spaces() {
    // FR-15: `"Adopt, Trial"` and `"Adopt,Trial"` parse identically, while a
    // multi-word ring name survives intact.
    assert_eq!(
        parse_rings(Some("Adopt, In Use , Retire"))
            .expect("valid")
            .rings,
        vec!["Adopt", "In Use", "Retire"]
    );
}

#[test]
fn a_single_ring_is_valid() {
    // FR-63: the list must yield at least one.
    assert_eq!(
        parse_rings(Some("Adopt")).expect("valid").rings,
        vec!["Adopt"]
    );
}

#[test]
fn an_empty_entry_is_rejected_and_names_its_position() {
    // FR-63: an empty entry would draw a nameless band.
    assert_eq!(
        parse_rings(Some("Adopt,,Trial")),
        Err(vec![RingsError::EmptyEntry { index: 2 }])
    );
}

#[test]
fn an_all_empty_list_is_one_error_not_one_per_comma() {
    for list in ["", "   ", ",", " , ,"] {
        assert_eq!(
            parse_rings(Some(list)),
            Err(vec![RingsError::Empty]),
            "{list:?}"
        );
    }
}

#[test]
fn a_duplicate_is_rejected_under_the_matching_rule() {
    // FR-63 + FR-76: duplicate after the same trim-and-fold used for matching,
    // so `Adopt` and `adopt ` collide. A duplicate would make FR-16's match
    // ambiguous and FR-32's band order undefined.
    assert_eq!(
        parse_rings(Some("Adopt,Trial,adopt ")),
        Err(vec![RingsError::Duplicate {
            index: 3,
            entry: "adopt".into(),
            first_index: 1,
        }])
    );
}

/// NFR-16 tier one: every violation within a flag is reported together, in
/// entry order, so one run shows the author everything to fix.
#[test]
fn every_violation_is_reported_in_entry_order() {
    assert_eq!(
        parse_rings(Some("Adopt,,Trial,adopt, ,TRIAL")),
        Err(vec![
            RingsError::EmptyEntry { index: 2 },
            RingsError::Duplicate {
                index: 4,
                entry: "adopt".into(),
                first_index: 1,
            },
            RingsError::EmptyEntry { index: 5 },
            RingsError::Duplicate {
                index: 6,
                entry: "TRIAL".into(),
                first_index: 3,
            },
        ])
    );
}

#[test]
fn the_declared_casing_is_what_gets_displayed() {
    // FR-76: normalisation never edits displayed text beyond FR-15's trim —
    // the declared name is what appears as a band label and legend subheading.
    assert_eq!(
        parse_rings(Some("ADOPT,trial")).expect("valid").rings,
        vec!["ADOPT", "trial"]
    );
}

/// A repeat of a repeat names where the ring was first declared, not the
/// earlier repeat.
#[test]
fn every_repeat_names_the_first_declaration() {
    assert_eq!(
        parse_rings(Some("a,A,a")),
        Err(vec![
            RingsError::Duplicate {
                index: 2,
                entry: "A".into(),
                first_index: 1,
            },
            RingsError::Duplicate {
                index: 3,
                entry: "a".into(),
                first_index: 1,
            },
        ])
    );
}

/// Entry numbers count every entry, empty ones included, so a duplicate
/// names the entry the author typed, not a position among the valid rings.
#[test]
fn entry_numbers_count_empty_entries() {
    assert_eq!(
        parse_rings(Some(",a,A")),
        Err(vec![
            RingsError::EmptyEntry { index: 1 },
            RingsError::Duplicate {
                index: 3,
                entry: "A".into(),
                first_index: 2,
            },
        ])
    );
}
