//! The diagnostic contract (architecture §10): exact messages, NFR-11 order,
//! NFR-20 grouping. SC-10's golden tests later pin the same text end to end.

use std::sync::Arc;

use radargen::columns::ColumnKind;
use radargen::diagnostics::{
    Binding, DateColumn, Fatal, Position, RenderKind, RowError, RowKind, Separator, Warning,
    Warnings, column_label,
};
use radargen::rings::{RingSource, RingsError};

const fn at(line: usize, column: usize, kind: RowKind) -> Warning {
    Warning::Row { line, column, kind }
}

/// The rings a `--rings` run declared, as the messages list them.
fn declared() -> Arc<[String]> {
    Arc::from(["Adopt".to_string(), "Trial".to_string()])
}

/// The default ring set, as the messages list it.
fn defaults() -> Arc<[String]> {
    Arc::from(["Assess", "Trial", "Adopt", "Retire"].map(String::from))
}

fn ring(line: usize, value: &str) -> Warning {
    at(
        line,
        2,
        RowKind::UndeclaredRing {
            value: value.into(),
            source: RingSource::Flag,
            rings: declared(),
        },
    )
}

#[test]
fn each_row_kind_has_its_per_row_message() {
    let cases = [
        (
            ring(37, "Adpot"),
            "line 37, ring: 'Adpot' is not one of the rings --rings declares (Adopt, Trial); the row is in the table view, not on the radar; to plot it, add it to --rings",
        ),
        (
            at(
                5,
                4,
                RowKind::UnrecognisedQualifier {
                    value: "maybe".into(),
                },
            ),
            "line 5, qualifier: unrecognised qualifier value 'maybe'; drawn solid",
        ),
        (
            at(
                6,
                5,
                RowKind::UnrecognisedStatus {
                    value: "Gone".into(),
                },
            ),
            "line 6, status: unrecognised status value 'Gone'; drawn as No Change",
        ),
        (
            at(
                7,
                6,
                RowKind::UnparsableDate {
                    column: DateColumn::LastAffirmed,
                    value: "June".into(),
                },
            ),
            "line 7, last_affirmed: 'June' is not a YYYY-MM-DD date; shown as written",
        ),
        (
            at(
                8,
                7,
                RowKind::UnparsableDate {
                    column: DateColumn::ExitDate,
                    value: "soon".into(),
                },
            ),
            "line 8, exit_date: 'soon' is not a YYYY-MM-DD date; shown as written",
        ),
        (
            at(9, 3, RowKind::EmptyDescription),
            "line 9, description: empty; the detail panel omits it",
        ),
        (
            at(
                12,
                0,
                RowKind::DuplicateName {
                    value: "Terraform".into(),
                    first_line: 4,
                    first_value: " terraform ".into(),
                },
            ),
            "line 12, name: 'Terraform' repeats line 4's name 'terraform' (names match ignoring case and surrounding spaces); both are drawn, so rename or delete one if they are the same item",
        ),
    ];
    for (warning, expected) in cases {
        assert_eq!(warning.to_string(), expected);
    }
}

/// FR-90 and FR-93: the cells whose own content is the problem.
#[test]
fn undisplayable_and_unnamed_values_have_their_per_row_message() {
    let cases = [
        (
            at(
                14,
                3,
                RowKind::Undisplayable {
                    column: "description".into(),
                    first: '\u{1}',
                    count: 1,
                },
            ),
            "line 14, description: removed U+0001, which the output cannot display",
        ),
        (
            at(
                15,
                0,
                RowKind::Undisplayable {
                    column: "name".into(),
                    first: '\u{1f}',
                    count: 3,
                },
            ),
            "line 15, name: removed 3 characters the output cannot display, the first U+001F",
        ),
        (
            at(
                16,
                5,
                RowKind::UnnamedValue {
                    column: "column 6 (F)".into(),
                },
            ),
            "line 16, column 6 (F): this column has no name in the header row, so its value is not shown; add a header name, or delete the value",
        ),
    ];
    for (warning, expected) in cases {
        assert_eq!(warning.to_string(), expected);
    }
}

/// FR-88 and FR-89 have no line, so no `line N, column:` prefix.
#[test]
fn each_render_level_kind_has_its_message() {
    let cases = [
        (
            Warning::Render(RenderKind::QuadrantCount { count: 7 }),
            "7 quadrants; the radar reads best with 3 to 6, so sectors narrow and labels may crowd",
        ),
        (
            Warning::Render(RenderKind::OverfullBand {
                quadrant: "Tools".into(),
                ring: "Adopt".into(),
                count: 14,
                fits: 10,
            }),
            "quadrant 'Tools', ring 'Adopt': 14 items; about 10 fit comfortably, so some markers overlap",
        ),
    ];
    for (warning, expected) in cases {
        assert_eq!(warning.to_string(), expected);
    }
}

#[test]
fn values_are_shown_trimmed() {
    let w = ring(3, "  Adpot ");
    assert!(w.to_string().contains("ring: 'Adpot' is"), "{w}");
    // FR-76's trim, so an invisible format character is not quoted either.
    let w = ring(3, "\u{200b}Adpot\u{feff}");
    assert!(w.to_string().contains("ring: 'Adpot' is"), "{w}");
}

/// FR-76's trim in NFR-20's grouped lines and FR-89's labels too: the value
/// is shown without its boundary format characters.
#[test]
fn grouped_values_are_shown_with_fr76s_trim() {
    let mut w = Warnings::default();
    w.push(ring(4, "\u{200b}Adpot"));
    w.push(ring(9, "Adpot\u{feff}"));
    assert!(
        w.render()
            .first()
            .is_some_and(|line| line.contains("— Adpot (2, first line 4);")),
        "{:?}",
        w.render()
    );
    let date = |line, value: &str| {
        at(
            line,
            6,
            RowKind::UnparsableDate {
                column: DateColumn::ExitDate,
                value: value.into(),
            },
        )
    };
    let mut w = Warnings::default();
    w.push(date(4, "\u{200b}June"));
    w.push(date(9, "June"));
    assert!(
        w.render()
            .first()
            .is_some_and(|line| line.contains("— June (2, first line 4);")),
        "{:?}",
        w.render()
    );
    let band = Warning::Render(RenderKind::OverfullBand {
        quadrant: "\u{200b}Tool".into(),
        ring: "Adopt\u{feff}".into(),
        count: 14,
        fits: 10,
    });
    assert!(
        band.to_string()
            .starts_with("quadrant 'Tool', ring 'Adopt':"),
        "{band}"
    );
}

/// NFR-10: a quoted field can hold a newline. Printed raw it would forge a
/// second diagnostic line.
#[test]
fn control_characters_in_values_are_shown_escaped() {
    let w = ring(3, "In\nUse\u{1b}[31m");
    let s = w.to_string();
    assert!(!s.contains('\n') && !s.contains('\u{1b}'), "{s:?}");
    assert!(s.contains(r"In\nUse\u{1b}[31m"), "{s}");
    // C0, DEL and C1 controls; bidi marks, embeddings, overrides and isolates,
    // which reorder a line visually; and the Unicode line and paragraph
    // separators, which some terminals and log viewers break a line at.
    for c in [
        '\u{0}', '\u{7f}', '\u{85}', '\u{9b}', '\u{61c}', '\u{200e}', '\u{200f}', '\u{2028}',
        '\u{2029}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}', '\u{2066}',
        '\u{2067}', '\u{2068}', '\u{2069}',
    ] {
        let shown = at(
            3,
            2,
            RowKind::UndeclaredRing {
                value: format!("a{c}b"),
                source: RingSource::Flag,
                rings: declared(),
            },
        )
        .to_string();
        assert!(
            !shown.contains(c),
            "U+{:04X} passed through raw: {shown:?}",
            u32::from(c)
        );
    }
    // Ordinary non-ASCII text is not escaped.
    let plain = ring(3, "Dvořák — 日本").to_string();
    assert!(plain.contains("'Dvořák — 日本'"), "{plain}");
    // An extra column's header is CSV text too (FR-18).
    let header = at(
        3,
        9,
        RowKind::Undisplayable {
            column: "notes\nline 1, name: forged".into(),
            first: '\u{1}',
            count: 1,
        },
    );
    let header = header.to_string();
    assert!(
        !header.contains('\n') && header.contains(r"notes\nline 1"),
        "{header}"
    );
}

#[test]
fn render_level_sorts_before_any_cell() {
    assert!(Position::Render < Position::Cell { line: 1, column: 0 });
    assert!(Position::Cell { line: 5, column: 3 } < Position::Cell { line: 5, column: 4 });
    assert!(Position::Cell { line: 5, column: 9 } < Position::Cell { line: 6, column: 0 });
}

/// SC-24, exactly.
#[test]
fn forty_rows_and_a_typo_make_one_ring_line_and_a_duplicate_keeps_its_own() {
    let mut w = Warnings::default();
    for line in 4..=44 {
        w.push(ring(line, if line == 37 { "Adpot" } else { "In Use" }));
    }
    w.push(at(
        50,
        0,
        RowKind::DuplicateName {
            value: "Kafka".into(),
            first_line: 9,
            first_value: "Kafka".into(),
        },
    ));
    assert_eq!(
        w.render(),
        vec![
            "warning: 41 rows use a ring --rings does not declare (Adopt, Trial) — In Use (40, first line 4), Adpot (1, line 37); all appear in the table view; to plot one, add it to --rings",
            "warning: line 50, name: 'Kafka' repeats line 9's name 'Kafka' (names match ignoring case and surrounding spaces); both are drawn, so rename or delete one if they are the same item",
        ]
    );
}

/// FR-17 (D-61): the message names where the ring set came from, so an
/// author who never passed `--rings` is not told to edit it.
#[test]
fn the_ring_warning_names_the_default_ring_set_when_no_flag_declared_one() {
    let default = |line| {
        at(
            line,
            2,
            RowKind::UndeclaredRing {
                value: "Adpot".into(),
                source: RingSource::Default,
                rings: defaults(),
            },
        )
    };
    assert_eq!(
        default(37).to_string(),
        "line 37, ring: 'Adpot' is not one of the default rings (Assess, Trial, Adopt, Retire); the row is in the table view, not on the radar; to plot it, pass --rings with every ring you want, since --rings replaces the default set"
    );
    let mut w = Warnings::default();
    w.push(default(4));
    w.push(default(9));
    assert_eq!(
        w.render(),
        vec![
            "warning: 2 rows use a ring that is not one of the default rings (Assess, Trial, Adopt, Retire) — Adpot (2, first line 4); all appear in the table view; to plot one, pass --rings with every ring you want, since --rings replaces the default set"
        ]
    );
}

#[test]
fn a_class_with_one_occurrence_prints_the_per_row_message() {
    let mut w = Warnings::default();
    w.push(ring(37, "Adpot"));
    assert_eq!(
        w.render(),
        vec![
            "warning: line 37, ring: 'Adpot' is not one of the rings --rings declares (Adopt, Trial); the row is in the table view, not on the radar; to plot it, add it to --rings",
        ]
    );
}

#[test]
fn values_group_under_fr_76_normalisation() {
    let mut w = Warnings::default();
    w.push(ring(4, "In Use"));
    w.push(ring(5, " in use "));
    w.push(ring(6, "IN USE"));
    assert_eq!(
        w.render(),
        vec![
            "warning: 3 rows use a ring --rings does not declare (Adopt, Trial) — In Use (3, first line 4); all appear in the table view; to plot one, add it to --rings",
        ]
    );
}

#[test]
fn render_level_warnings_come_first_and_never_group() {
    let mut w = Warnings::default();
    w.push(ring(2, "X"));
    w.push(Warning::Render(RenderKind::QuadrantCount { count: 9 }));
    w.push(Warning::Render(RenderKind::OverfullBand {
        quadrant: "A".into(),
        ring: "Adopt".into(),
        count: 12,
        fits: 10,
    }));
    w.push(Warning::Render(RenderKind::OverfullBand {
        quadrant: "B".into(),
        ring: "Adopt".into(),
        count: 11,
        fits: 10,
    }));
    assert_eq!(
        w.render(),
        vec![
            "warning: 9 quadrants; the radar reads best with 3 to 6, so sectors narrow and labels may crowd",
            "warning: quadrant 'A', ring 'Adopt': 12 items; about 10 fit comfortably, so some markers overlap",
            "warning: quadrant 'B', ring 'Adopt': 11 items; about 10 fit comfortably, so some markers overlap",
            "warning: line 2, ring: 'X' is not one of the rings --rings declares (Adopt, Trial); the row is in the table view, not on the radar; to plot it, add it to --rings",
        ]
    );
}

#[test]
fn classes_order_by_first_line_then_column() {
    let mut w = Warnings::default();
    w.push(ring(9, "X"));
    w.push(at(
        5,
        5,
        RowKind::UnrecognisedStatus {
            value: "Gone".into(),
        },
    ));
    w.push(at(
        5,
        4,
        RowKind::UnrecognisedQualifier {
            value: "maybe".into(),
        },
    ));
    let out = w.render();
    let [qualifier, status, ring] = out.as_slice() else {
        panic!("expected 3 lines: {out:#?}")
    };
    assert!(qualifier.contains("line 5, qualifier"), "{out:#?}");
    assert!(status.contains("line 5, status"), "{out:#?}");
    assert!(ring.contains("line 9, ring"), "{out:#?}");
}

#[test]
fn out_of_order_pushes_are_sorted_by_position() {
    let mut w = Warnings::default();
    w.push(ring(40, "X"));
    w.push(at(
        3,
        0,
        RowKind::DuplicateName {
            value: "Kafka".into(),
            first_line: 2,
            first_value: "Kafka".into(),
        },
    ));
    let out = w.render();
    let [name, ..] = out.as_slice() else {
        panic!("expected at least 1 line: {out:#?}")
    };
    assert!(name.contains("line 3, name"), "{out:#?}");
}

#[test]
fn grouped_values_are_never_capped() {
    let mut w = Warnings::default();
    for i in 0..30 {
        w.push(ring(i + 2, &format!("Ring{i:02}")));
    }
    let out = w.render();
    let [grouped] = out.as_slice() else {
        panic!("expected one grouped line: {out:#?}")
    };
    for i in 0..30 {
        assert!(
            grouped.contains(&format!("Ring{i:02} (1, line {})", i + 2)),
            "missing Ring{i:02}"
        );
    }
}

#[test]
fn every_row_class_has_a_grouped_line() {
    let cases = [
        (
            RowKind::UnrecognisedQualifier {
                value: "maybe".into(),
            },
            "warning: 2 rows have an unrecognised qualifier — maybe (2, first line 4); each is drawn solid",
        ),
        (
            RowKind::UnrecognisedStatus {
                value: "Gone".into(),
            },
            "warning: 2 rows have an unrecognised status — Gone (2, first line 4); each is drawn as No Change",
        ),
        (
            RowKind::UnparsableDate {
                column: DateColumn::ExitDate,
                value: "soon".into(),
            },
            "warning: 2 rows have exit_date values that are not YYYY-MM-DD dates — soon (2, first line 4); each is shown as written",
        ),
        (
            RowKind::EmptyDescription,
            "warning: 2 rows have an empty description — lines 4, 6; each detail panel omits it",
        ),
        (
            RowKind::DuplicateName {
                value: "Kafka".into(),
                first_line: 2,
                first_value: "Kafka".into(),
            },
            "warning: 2 rows repeat an earlier name — Kafka (line 2; repeated on 2 rows, first line 4); every row is drawn, so rename or delete the repeats that are the same item",
        ),
    ];
    for (kind, expected) in cases {
        let mut w = Warnings::default();
        w.push(at(4, 1, kind.clone()));
        w.push(at(6, 1, kind));
        assert_eq!(w.render(), vec![expected]);
    }
    // Each member names its own column.
    let mut mixed = Warnings::default();
    mixed.push(at(
        4,
        3,
        RowKind::Undisplayable {
            column: "description".into(),
            first: '\u{1}',
            count: 1,
        },
    ));
    mixed.push(at(
        6,
        0,
        RowKind::Undisplayable {
            column: "name".into(),
            first: '\u{7f}',
            count: 2,
        },
    ));
    assert_eq!(
        mixed.render(),
        vec![
            "warning: 2 values had characters the output cannot display — description on line 4 (U+0001), name on line 6 (2, first U+007F); each was removed",
        ]
    );
}

#[test]
fn the_two_date_columns_are_separate_classes() {
    let mut w = Warnings::default();
    w.push(at(
        4,
        5,
        RowKind::UnparsableDate {
            column: DateColumn::LastAffirmed,
            value: "x".into(),
        },
    ));
    w.push(at(
        4,
        6,
        RowKind::UnparsableDate {
            column: DateColumn::ExitDate,
            value: "y".into(),
        },
    ));
    assert_eq!(w.render().len(), 2);
}

#[test]
fn no_warnings_render_nothing() {
    assert!(Warnings::default().is_empty());
    assert_eq!(Warnings::default().render(), Vec::<String>::new());
}

/// NFR-10 applies to the grouped line too: it prints values through its own
/// path, and a forged line there is just as convincing.
#[test]
fn control_characters_are_escaped_in_grouped_lines_too() {
    let mut w = Warnings::default();
    w.push(ring(4, "In\nUse"));
    w.push(ring(5, "In\nUse"));
    let out = w.render();
    let [line] = out.as_slice() else {
        panic!("expected one grouped line: {out:#?}")
    };
    assert!(!line.contains('\n'), "{line:?}");
    assert!(line.contains(r"In\nUse (2, first line 4)"), "{line}");
}

/// NFR-11: values within a grouped line order by the line they first occur
/// on, not by the order a stage happened to push them.
#[test]
fn grouped_values_order_by_first_line_not_push_order() {
    let mut w = Warnings::default();
    w.push(ring(37, "Adpot"));
    w.push(ring(9, "in use"));
    w.push(ring(4, "In Use"));
    // The shown spelling is the first occurrence's (line 4), not the first pushed.
    assert_eq!(
        w.render(),
        vec![
            "warning: 3 rows use a ring --rings does not declare (Adopt, Trial) — In Use (2, first line 4), Adpot (1, line 37); all appear in the table view; to plot one, add it to --rings",
        ]
    );
}

/// NFR-11: two warnings at one position keep push order, so the output is
/// deterministic. An unstable sort would pass every other test.
#[test]
fn warnings_at_one_position_keep_push_order() {
    let qualifier = || {
        at(
            5,
            4,
            RowKind::UnrecognisedQualifier {
                value: "maybe".into(),
            },
        )
    };
    let removed = || {
        at(
            5,
            4,
            RowKind::Undisplayable {
                column: "qualifier".into(),
                first: '\u{1}',
                count: 1,
            },
        )
    };
    for (first, second, lead) in [
        (qualifier(), removed(), "unrecognised"),
        (removed(), qualifier(), "removed"),
    ] {
        let mut w = Warnings::default();
        w.push(first);
        w.push(second);
        let out = w.render();
        let [first, ..] = out.as_slice() else {
            panic!("expected at least 1 line: {out:#?}")
        };
        assert!(
            first.starts_with(&format!("warning: line 5, qualifier: {lead}")),
            "{out:#?}"
        );
    }
}

/// A grouped line sits where its first member does, not its last.
#[test]
fn a_grouped_line_is_placed_by_its_first_member() {
    let mut w = Warnings::default();
    w.push(ring(4, "X"));
    w.push(ring(40, "X"));
    w.push(at(
        10,
        4,
        RowKind::UnrecognisedQualifier {
            value: "maybe".into(),
        },
    ));
    let out = w.render();
    let [grouped, qualifier] = out.as_slice() else {
        panic!("expected 2 lines: {out:#?}")
    };
    assert!(grouped.contains("rows use a ring"), "{out:#?}");
    assert!(qualifier.contains("line 10, qualifier"), "{out:#?}");
}

/// FR-76 folds case only for the comparisons it lists, and dates are not one:
/// a parsed value is only trimmed. Two spellings are two distinct values.
#[test]
fn date_values_group_by_trimmed_text_not_case() {
    let date = |line, value: &str| {
        at(
            line,
            5,
            RowKind::UnparsableDate {
                column: DateColumn::LastAffirmed,
                value: value.into(),
            },
        )
    };
    let mut w = Warnings::default();
    w.push(date(4, "Jan-1"));
    w.push(date(5, " Jan-1 "));
    w.push(date(6, "JAN-1"));
    assert_eq!(
        w.render(),
        vec![
            "warning: 3 rows have last_affirmed values that are not YYYY-MM-DD dates — Jan-1 (2, first line 4), JAN-1 (1, line 6); each is shown as written",
        ]
    );
}

/// A repeated name within a multi-name group names its one repeat line.
#[test]
fn a_single_repeat_in_a_duplicate_group_names_its_line() {
    let mut w = Warnings::default();
    w.push(at(
        12,
        0,
        RowKind::DuplicateName {
            value: "kafka".into(),
            first_line: 2,
            first_value: "Kafka".into(),
        },
    ));
    w.push(at(
        14,
        0,
        RowKind::DuplicateName {
            value: "Redis".into(),
            first_line: 3,
            first_value: "Redis".into(),
        },
    ));
    assert_eq!(
        w.render(),
        vec![
            "warning: 2 rows repeat an earlier name — Kafka (line 2; repeated on line 12), Redis (line 3; repeated on line 14); every row is drawn, so rename or delete the repeats that are the same item",
        ]
    );
}

/// Stages assert on kinds and positions, not on rendered English.
#[test]
fn the_raw_warnings_are_readable() {
    let mut w = Warnings::default();
    w.push(ring(7, "X"));
    let [only] = w.as_slice() else {
        panic!("expected one warning: {:#?}", w.as_slice())
    };
    assert_eq!(only.position(), Position::Cell { line: 7, column: 2 });
    assert!(matches!(
        only,
        Warning::Row {
            kind: RowKind::UndeclaredRing { .. },
            ..
        }
    ));
}

#[test]
fn fatal_messages() {
    let cases = [
        (
            Fatal::UnrecognisedExtension {
                path: "radar.pdf".into(),
            },
            "error: -o radar.pdf: the output file name must end in .html",
        ),
        (
            Fatal::UnrecognisedExtension {
                path: "radar".into(),
            },
            "error: -o radar: the output file name must end in .html",
        ),
        (
            Fatal::FormatNotInThisVersion {
                path: "radar.png".into(),
                ext: "png".into(),
                arrives_in: "1.1",
            },
            "error: -o radar.png: .png output arrives in radargen 1.1; this version writes .html",
        ),
        (
            Fatal::from(RingsError::EmptyEntry { index: 2 }),
            "error: --rings: entry 2 is empty",
        ),
        (
            Fatal::from(RingsError::Duplicate {
                index: 3,
                entry: "adopt".into(),
                first_index: 1,
            }),
            "error: --rings: entry 3 'adopt' repeats entry 1",
        ),
        (
            Fatal::from(RingsError::Empty),
            "error: --rings: no rings declared",
        ),
        (
            Fatal::from(RingsError::Duplicate {
                index: 2,
                entry: "a\nerror: forged".into(),
                first_index: 1,
            }),
            r"error: --rings: entry 2 'a\nerror: forged' repeats entry 1",
        ),
    ];
    for (fatal, expected) in cases {
        assert_eq!(fatal.render(), expected);
    }
    let row = RowError::FieldCount {
        line: 4,
        found: 2,
        expected: 1,
        unnamed: 0,
        padded: false,
    };
    assert_eq!(
        row.render(),
        "error: line 4: the row has 2 values but the header has 1 column; a row with more values usually has a comma that needs the value in double quotes"
    );
}

/// The CSV stage's file-level (tier-one) messages (plan RG-005), apart
/// from `fatal_messages` so neither passes clippy's `too_many_lines`.
#[test]
fn csv_file_messages() {
    let separated = |line, separator| Fatal::NotCommaSeparated { line, separator };
    let cases = [
        (
            Fatal::Unreadable {
                path: "in.csv".into(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
            },
            "error: in.csv: cannot be read: not found",
        ),
        (
            Fatal::Unwritable {
                path: "out/radar.html".into(),
                source: std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "permission denied",
                ),
            },
            "error: -o out/radar.html: cannot be written: permission denied",
        ),
        (
            Fatal::NotUtf8 {
                path: "in.csv".into(),
                line: 2,
            },
            "error: in.csv, line 2: not UTF-8 text; re-save the file as UTF-8 (in Excel, Save As \"CSV UTF-8\")",
        ),
        (
            Fatal::NoHeader,
            "error: the CSV has no header row (it is empty or blank); the header must name at least name, quadrant, ring and description",
        ),
        (
            separated(1, Separator::Semicolon),
            "error: line 1: the header has semicolons between its names, not commas; radargen reads comma-separated files only, so export the file with commas as the separator",
        ),
        (
            separated(2, Separator::Tab),
            "error: line 2: the header has tabs between its names, not commas; radargen reads comma-separated files only, so export the file with commas as the separator",
        ),
        (
            Fatal::UnterminatedQuote { line: 3, column: 2 },
            "error: line 3, column 3 (C): a quote opens this value and never closes; add the closing quote, and write any quote inside the value as \"\"",
        ),
        (
            Fatal::TextAfterQuote { line: 2, column: 2 },
            "error: line 2, column 3 (C): text follows a closing quote; inside quotes, write a quote as \"\"",
        ),
    ];
    for (fatal, expected) in cases {
        assert_eq!(fatal.render(), expected);
    }
}

/// The CSV stage's FR-4 messages (tier two).
#[test]
fn csv_header_and_row_messages() {
    let repeats = |index, name: &str, first_name: &str| RowError::DuplicateHeader {
        line: 1,
        index,
        name: name.into(),
        first_index: 0,
        first_name: first_name.into(),
    };
    let count = |found| RowError::FieldCount {
        line: 4,
        found,
        expected: 2,
        unnamed: 0,
        padded: false,
    };
    let padded = RowError::FieldCount {
        line: 4,
        found: 3,
        expected: 2,
        unnamed: 0,
        padded: true,
    };
    let unnamed = RowError::FieldCount {
        line: 4,
        found: 1,
        expected: 2,
        unnamed: 1,
        padded: false,
    };
    let cases = [
        (
            repeats(2, "Name", "name"),
            "error: line 1: column 3 (C) 'Name' repeats column 1 (A) 'name'; header names match ignoring case and surrounding spaces, so rename or remove one",
        ),
        // A header is CSV text: a quoted newline in it must not forge a line.
        (
            repeats(1, "a\nerror: forged", "a"),
            r"error: line 1: column 2 (B) 'a\nerror: forged' repeats column 1 (A) 'a'; header names match ignoring case and surrounding spaces, so rename or remove one",
        ),
        (
            count(3),
            "error: line 4: the row has 3 values but the header has 2 columns; a row with more values usually has a comma that needs the value in double quotes",
        ),
        (
            count(1),
            "error: line 4: the row has 1 value but the header has 2 columns; a row with fewer values is usually missing one, or an unquoted line break split it",
        ),
        (
            padded,
            "error: line 4: the row has 3 values but the header has 2 columns; rows with more values end in empty cells: delete them, or give the header matching empty columns",
        ),
        (
            unnamed,
            "error: line 4: the row has 1 value but the header has 2 columns, 1 of them unnamed; a row with fewer values is usually missing one, or an unquoted line break split it",
        ),
    ];
    for (fatal, expected) in cases {
        assert_eq!(fatal.render(), expected);
    }
}

/// FR-5 and FR-6, which `model::build_catalogue` raises (PR 06).
#[test]
fn model_messages() {
    let cases = [
        (
            RowError::MissingColumn {
                line: 1,
                column: ColumnKind::Ring,
            },
            "error: line 1: the header has no 'ring' column; add one, or rename the column that holds it — the header must name at least name, quadrant, ring and description",
        ),
        (
            RowError::EmptyRequired {
                line: 7,
                column: ColumnKind::Quadrant,
            },
            "error: line 7, quadrant: empty; every row needs a quadrant, so fill it in or delete the row",
        ),
    ];
    for (fatal, expected) in cases {
        assert_eq!(fatal.render(), expected);
    }
}

/// FR-93 through NFR-20: values are counted, each column enumerated with
/// its count and first line, in first-occurrence order (NFR-11).
#[test]
fn unnamed_column_values_group_by_column() {
    let unnamed = |line, column: usize| {
        at(
            line,
            column,
            RowKind::UnnamedValue {
                column: column_label(column),
            },
        )
    };
    let mut w = Warnings::default();
    w.push(unnamed(9, 6));
    w.push(unnamed(4, 5));
    w.push(unnamed(5, 5));
    assert_eq!(
        w.render(),
        vec![
            "warning: 3 values are under columns with no name in the header row — column 6 (F) (2, first line 4), column 7 (G) (1, line 9); none is shown, so add a header name, or delete the values"
        ]
    );
}

/// NFR-10: a column is its 1-based number and its spreadsheet letters.
#[test]
fn column_labels_count_like_a_spreadsheet() {
    for (index, expected) in [
        (0, "column 1 (A)"),
        (25, "column 26 (Z)"),
        (26, "column 27 (AA)"),
        (51, "column 52 (AZ)"),
        (52, "column 53 (BA)"),
        (701, "column 702 (ZZ)"),
        (702, "column 703 (AAA)"),
        (18277, "column 18278 (ZZZ)"),
        (18278, "column 18279 (AAAA)"),
    ] {
        assert_eq!(column_label(index), expected);
    }
}

/// D-64: an invisible format character inside a value is escaped, so the
/// author can see it; ZWJ, ZWNJ and tag characters build emoji and flags,
/// so they pass through.
#[test]
fn interior_format_characters_are_escaped_except_joiners_and_tags() {
    let shown = ring(3, "Ad\u{200B}opt").to_string();
    assert!(shown.contains("'Ad\\u{200b}opt'"), "{shown}");
    let family = "\u{1F468}\u{200D}\u{1F469}";
    let england = "\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}";
    let word_joiner = ring(3, "Ad\u{2060}opt").to_string();
    assert!(word_joiner.contains("'Ad\\u{2060}opt'"), "{word_joiner}");
    for kept in [family, england, "a\u{200C}b", "a\u{E0001}b"] {
        let shown = ring(3, kept).to_string();
        assert!(shown.contains(&format!("'{kept}'")), "{shown}");
    }
}

/// The pipeline's own tier-one messages (plan RG-014A, Task 1).
#[test]
fn pipeline_messages() {
    let cases = [
        (
            Fatal::MissingDirectory {
                path: "out/radar.html".into(),
                dir: "out".into(),
            },
            "error: -o out/radar.html: the directory out does not exist",
        ),
        (
            Fatal::OutputIsInput {
                path: "radar.csv".into(),
            },
            "error: -o radar.csv: is the input CSV; choose another output path",
        ),
    ];
    for (fatal, expected) in cases {
        assert_eq!(fatal.render(), expected);
    }
}

/// NFR-15: a face that does not parse is named, so the message says which.
#[test]
fn a_font_failure_names_the_face() {
    let Err(error) = radargen::text::FontMetrics::load(b"x", "Inter SemiBold") else {
        panic!("one byte is not a font");
    };
    let line = Fatal::from(error).render();
    assert!(
        line.starts_with("error: the embedded Inter SemiBold font cannot be read: "),
        "{line}"
    );
}

/// FR-95 names the label that sets the width, quadrant or ring (D-73),
/// trimmed and single-quoted as FR-89's names are.
#[test]
fn fr95_names_a_quadrant_or_a_ring() {
    let cases = [
        (
            RenderKind::NarrowWidened {
                binding: Binding::Quadrant(" Infrastructure Platforms and Tooling ".into()),
                width: 503,
            },
            "the narrow arrangement is 503 units wide, past the 480 its legend rows and marker targets are sized for, so on a 320-pixel screen they fall below 24 pixels; a shorter name for quadrant 'Infrastructure Platforms and Tooling' would fix it",
        ),
        (
            RenderKind::NarrowWidened {
                binding: Binding::Ring {
                    name: "Under evaluation by the architecture board".into(),
                    source: RingSource::Flag,
                },
                width: 750,
            },
            "the narrow arrangement is 750 units wide, past the 480 its legend rows and marker targets are sized for, so on a 320-pixel screen they fall below 24 pixels; a shorter name for ring 'Under evaluation by the architecture board' in --rings would fix it",
        ),
    ];
    for (kind, expected) in cases {
        assert_eq!(Warning::Render(kind).to_string(), expected);
    }
}

/// NFR-12 and SC-25: the output carries `major.minor`, never the patch.
#[test]
fn output_version_has_no_patch() {
    let mut parts = env!("CARGO_PKG_VERSION").split('.');
    let major = parts.next().expect("a major version");
    let minor = parts.next().expect("a minor version");
    assert_eq!(
        radargen::emit::OUTPUT_VERSION,
        format!("radargen {major}.{minor}")
    );
}

/// rg-bsb: an I/O reason reads one way on every path: the OS's own words,
/// without its "(os error N)" suffix or a trailing full stop, lowercase.
#[test]
fn io_reasons_read_one_way() {
    let unix = std::io::Error::other("Is a directory (os error 21)");
    let windows = std::io::Error::other("The system cannot find the path specified. (os error 3)");
    assert_eq!(
        Fatal::Unwritable {
            path: "out.html".into(),
            source: unix,
        }
        .render(),
        "error: -o out.html: cannot be written: is a directory"
    );
    assert_eq!(
        Fatal::Unreadable {
            path: "in.csv".into(),
            source: windows,
        }
        .render(),
        "error: in.csv: cannot be read: the system cannot find the path specified"
    );
}

/// rg-iqm: a ring that sets the narrow width is named with where it came
/// from (D-73), so the fix points at the right place.
#[test]
fn fr95_names_where_a_ring_came_from() {
    let message = |source| {
        Warning::Render(RenderKind::NarrowWidened {
            binding: Binding::Ring {
                name: "Under evaluation".into(),
                source,
            },
            width: 600,
        })
        .to_string()
    };
    assert!(
        message(RingSource::Flag)
            .ends_with("a shorter name for ring 'Under evaluation' in --rings would fix it")
    );
    assert!(message(RingSource::Default).ends_with(
        "a shorter name for ring 'Under evaluation' (a default ring, so pass --rings to rename it) would fix it"
    ));
}
