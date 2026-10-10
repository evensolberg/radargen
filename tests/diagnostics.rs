//! The diagnostic contract (architecture §10): exact messages, NFR-11 order,
//! NFR-20 grouping. SC-10's golden tests later pin the same text end to end.

use radargen::diagnostics::{DateColumn, Position, Warning, WarningKind};

fn at(line: usize, column: usize, kind: WarningKind) -> Warning {
    Warning {
        position: Position::Cell { line, column },
        kind,
    }
}

fn render_level(kind: WarningKind) -> Warning {
    Warning {
        position: Position::Render,
        kind,
    }
}

#[test]
fn each_row_kind_has_its_per_row_message() {
    let cases = [
        (
            at(
                37,
                2,
                WarningKind::UndeclaredRing {
                    value: "Adpot".into(),
                },
            ),
            "line 37, ring: ring 'Adpot' is not declared by --rings; the row is in the table view, not on the radar",
        ),
        (
            at(
                5,
                4,
                WarningKind::UnrecognisedQualifier {
                    value: "maybe".into(),
                },
            ),
            "line 5, qualifier: unrecognised qualifier value 'maybe'",
        ),
        (
            at(
                6,
                5,
                WarningKind::UnrecognisedStatus {
                    value: "Gone".into(),
                },
            ),
            "line 6, status: unrecognised status value 'Gone'; drawn as No Change",
        ),
        (
            at(
                7,
                6,
                WarningKind::UnparsableDate {
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
                WarningKind::UnparsableDate {
                    column: DateColumn::ExitDate,
                    value: "soon".into(),
                },
            ),
            "line 8, exit_date: 'soon' is not a YYYY-MM-DD date; shown as written",
        ),
        (
            at(9, 3, WarningKind::EmptyDescription),
            "line 9, description: description is empty; the detail panel omits it",
        ),
        (
            at(
                12,
                0,
                WarningKind::DuplicateName {
                    value: "Terraform".into(),
                    first_line: 4,
                },
            ),
            "line 12, name: name 'Terraform' repeats line 4; both are drawn",
        ),
        (
            at(
                14,
                3,
                WarningKind::Undisplayable {
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
                WarningKind::Undisplayable {
                    column: "name".into(),
                    first: '\u{1f}',
                    count: 3,
                },
            ),
            "line 15, name: removed 3 characters the output cannot display, the first U+001F",
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
            render_level(WarningKind::QuadrantCount { count: 7 }),
            "7 quadrants; the radar reads best with three to six, so sectors narrow and labels may crowd",
        ),
        (
            render_level(WarningKind::OverfullBand {
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
    let w = at(
        3,
        2,
        WarningKind::UndeclaredRing {
            value: "  Adpot ".into(),
        },
    );
    assert!(w.to_string().contains("ring 'Adpot' is"), "{w}");
}

/// NFR-10: a quoted field can hold a newline. Printed raw it would forge a
/// second diagnostic line.
#[test]
fn control_characters_in_values_are_shown_escaped() {
    let w = at(
        3,
        2,
        WarningKind::UndeclaredRing {
            value: "In\nUse\u{1b}[31m".into(),
        },
    );
    let s = w.to_string();
    assert!(!s.contains('\n') && !s.contains('\u{1b}'), "{s:?}");
    assert!(s.contains(r"In\nUse\u{1b}[31m"), "{s}");
    let bidi = at(
        3,
        2,
        WarningKind::UndeclaredRing {
            value: "a\u{202e}b".into(),
        },
    )
    .to_string();
    assert!(bidi.contains(r"a\u{202e}b"), "{bidi}");
    // An extra column's header is CSV text too (FR-18).
    let header = at(
        3,
        9,
        WarningKind::Undisplayable {
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
