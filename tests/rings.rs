//! `--rings` parsing and validation (FR-12–FR-15, FR-63).

use std::sync::Arc;

use radargen::rings::{RingMatch, RingSet, RingsError, parse_ring_list, parse_rings};
use radargen::settings::Source;

#[test]
fn omitting_the_flag_yields_the_in_house_default() {
    // FR-13, edge-to-centre (FR-12, D-60): the lifecycle as it is read.
    assert_eq!(
        parse_rings(None).expect("default is valid").rings(),
        vec!["Assess", "Trial", "Adopt", "Retire"]
    );
}

/// FR-17 (D-61): the warning names where the rings came from.
#[test]
fn the_set_records_its_source() {
    assert_eq!(
        parse_rings(None).expect("default is valid").source(),
        &Source::Default
    );
    assert_eq!(
        parse_rings(Some("Adopt")).expect("valid").source(),
        &Source::Flag
    );
}

/// FR-15 uses FR-76's trim, so a zero-width space pasted in with a ring
/// name neither shows in its band label nor stops it matching, nor hides
/// a duplicate (D-61).
#[test]
fn entries_lose_invisible_format_characters_at_their_ends() {
    assert_eq!(
        parse_rings(Some("\u{feff}Adopt\u{200b}, Trial"))
            .expect("valid")
            .rings(),
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
            .rings(),
        vec!["Adopt", "In Use", "Retire"]
    );
}

#[test]
fn a_single_ring_is_valid() {
    // FR-63: the list must yield at least one.
    assert_eq!(
        parse_rings(Some("Adopt")).expect("valid").rings(),
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
        parse_rings(Some("ADOPT,trial")).expect("valid").rings(),
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

/// rg-cuj: a run of spaces inside a ring name is one space, as its
/// wrapped band label and legend subheading read it.
#[test]
fn internal_whitespace_runs_collapse_in_ring_names() {
    assert_eq!(
        parse_rings(Some("Early  Adopt, Trial"))
            .expect("valid")
            .rings(),
        vec!["Early Adopt", "Trial"]
    );
}

// ---- FR-107: `--unplotted-rings`, parsed as `--rings` is.

/// `--rings "Adopt,Trial"` with `list` as `--unplotted-rings`.
fn unplotted(list: &str) -> Result<RingSet, Vec<RingsError>> {
    parse_rings(Some("Adopt,Trial"))?.with_unplotted(list)
}

/// FR-16 and FR-107: a value is a declared ring, by its edge-to-centre
/// index, an unplotted one, or neither, matched under FR-76; a set says
/// whether it names any unplotted ring.
#[test]
fn a_ring_set_classifies_a_value() {
    let set = unplotted("In Use").expect("valid");
    assert!(set.names_unplotted());
    assert_eq!(set.classify(" trial "), RingMatch::Declared(1));
    assert_eq!(set.classify("ADOPT"), RingMatch::Declared(0));
    assert_eq!(set.classify("in   use"), RingMatch::Unplotted);
    assert_eq!(set.classify("Adpot"), RingMatch::Unknown);
    let set = parse_rings(None).expect("default");
    assert!(!set.names_unplotted());
    assert_eq!(set.classify("In Use"), RingMatch::Unknown);
    assert_eq!(set.classify("retire"), RingMatch::Declared(3));
}

/// FR-107 and FR-15: entries are tidied as `--rings`' are, so each matches
/// under FR-76; the declared rings are untouched.
#[test]
fn unplotted_rings_are_a_tidied_comma_list() {
    let set = unplotted(" In  Use ,rejected,\u{200b}Gone").expect("valid");
    for value in ["In Use", "REJECTED", "gone"] {
        assert_eq!(set.classify(value), RingMatch::Unplotted, "{value}");
    }
    assert_eq!(set.classify("In"), RingMatch::Unknown);
    assert_eq!(set.rings(), ["Adopt", "Trial"]);
    assert_eq!(set.source(), &Source::Flag);
}

/// FR-63's checks, with `--rings`' errors: an empty entry, a duplicate
/// under FR-76 naming its first, and a list with no rings at all.
#[test]
fn unplotted_rings_get_fr63s_checks() {
    assert_eq!(
        unplotted("In Use,,in use , Rejected"),
        Err(vec![
            RingsError::EmptyEntry { index: 2 },
            RingsError::Duplicate {
                index: 3,
                entry: "in use".into(),
                first_index: 1,
            },
        ])
    );
    for list in ["", " , "] {
        assert_eq!(
            unplotted(list),
            Err(vec![RingsError::NoneNamed]),
            "{list:?}"
        );
    }
}

/// FR-107: a ring cannot be both drawn and left off. The match is FR-76's,
/// and every violation is reported in entry order (NFR-16 tier one).
#[test]
fn a_ring_in_both_lists_is_an_error() {
    assert_eq!(
        unplotted("In Use, ADOPT ,, trial"),
        Err(vec![
            RingsError::Overlap {
                index: 2,
                entry: "ADOPT".into(),
                by: Source::Flag,
            },
            RingsError::EmptyEntry { index: 3 },
            RingsError::Overlap {
                index: 4,
                entry: "trial".into(),
                by: Source::Flag,
            },
        ])
    );
}

/// FR-107: the default set is drawn, so naming a default ring is the same
/// overlap, said as the default set's.
#[test]
fn a_default_ring_cannot_be_unplotted() {
    assert_eq!(
        parse_rings(None)
            .expect("default")
            .with_unplotted("In Use,adopt"),
        Err(vec![RingsError::Overlap {
            index: 2,
            entry: "adopt".into(),
            by: Source::Default,
        }])
    );
}

/// A repeated overlap is reported as the repeat it is, once.
#[test]
fn a_repeated_overlap_is_a_duplicate() {
    assert_eq!(
        unplotted("Adopt,adopt"),
        Err(vec![
            RingsError::Overlap {
                index: 1,
                entry: "Adopt".into(),
                by: Source::Flag,
            },
            RingsError::Duplicate {
                index: 2,
                entry: "adopt".into(),
                first_index: 1,
            },
        ])
    );
}

/// An empty `--unplotted-rings` says the flag's verb, `named`, where
/// `--rings`' says `declared`.
#[test]
fn each_flag_words_its_empty_list() {
    assert_eq!(RingsError::NoneNamed.to_string(), "no rings named");
    assert_eq!(RingsError::Empty.to_string(), "no rings declared");
}

/// FR-107: once `--rings` is given, the default set is not drawn, so a
/// default ring may be unplotted.
#[test]
fn a_default_ring_may_be_unplotted_once_rings_replace_the_defaults() {
    let set = unplotted("Retire").expect("Retire is not drawn");
    assert!(set.names_unplotted());
    assert_eq!(set.classify("retire"), RingMatch::Unplotted);
}

/// FR-91: a file's rings, joined as the flag would hold them, get exactly
/// the flag's checks, and the set remembers the file (FR-17).
#[test]
fn a_config_ring_list_is_parsed_as_the_flag_is() {
    let file = Source::Config(Arc::from("radar.toml"));
    let set = parse_ring_list(" Hold ,Adopt", file.clone()).expect("parses");
    assert_eq!(set.rings(), ["Hold", "Adopt"]);
    assert_eq!(set.source(), &file);
    assert_eq!(
        parse_ring_list("Adopt,adopt", file.clone()),
        Err(vec![RingsError::Duplicate {
            index: 2,
            entry: "adopt".into(),
            first_index: 1
        }])
    );
    // `rings = []`, joined, is FR-63's one no-rings error.
    assert_eq!(parse_ring_list("", file), Err(vec![RingsError::Empty]));
}

/// FR-107's overlap names a file's ring set by its key and file.
#[test]
fn an_overlap_with_a_config_ring_set_names_its_key() {
    let set =
        parse_ring_list("Adopt,Hold", Source::Config(Arc::from("radar.toml"))).expect("parses");
    let errors = set.with_unplotted("Adopt").expect_err("overlaps");
    assert_eq!(
        errors.first().map(ToString::to_string).as_deref(),
        Some(
            "entry 1 'Adopt' is also in the rings key in radar.toml, which draws it; remove it from one list"
        )
    );
}
