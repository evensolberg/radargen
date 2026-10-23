//! The diagnostic contract (architecture §10): exact messages, NFR-11 order,
//! NFR-20 grouping. SC-10's golden tests later pin the same text end to end.

#[path = "common/source.rs"]
mod source;

use std::path::Path;
use std::sync::Arc;

use radargen::columns::ColumnKind;
use radargen::diagnostics::{
    Binding, DateColumn, Fatal, Lacking, OffRadar, Position, ReadFile, RenderKind, RowError,
    RowKind, Separator, Warning, Warnings, column_label,
};
use radargen::options::OutputFormat;
use radargen::rings::RingsError;
use radargen::settings::file::KeyProblem;
use radargen::settings::{NamedPath, Origin, Setting, Source};

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

/// FR-17 on `line`'s ring `value`, declared by `source`, the row going
/// where `off_radar` says, in a run with no `--unplotted-rings`.
fn undeclared(line: usize, value: &str, source: Source, off_radar: OffRadar) -> Warning {
    quiet_hint(line, value, source, off_radar, true)
}

/// As [`undeclared`], saying whether the run lacks `--unplotted-rings`,
/// so the warning suggests it (D-96).
fn quiet_hint(
    line: usize,
    value: &str,
    source: Source,
    off_radar: OffRadar,
    suggest_unplotted: bool,
) -> Warning {
    let rings = match source {
        Source::Flag | Source::Config(_) => declared(),
        Source::Default => defaults(),
    };
    at(
        line,
        2,
        RowKind::UndeclaredRing {
            value: value.into(),
            origin: Origin::rings(source),
            rings,
            off_radar,
            suggest_unplotted,
        },
    )
}

/// FR-17 under `--rings`, on a `--table all` page.
fn ring(line: usize, value: &str) -> Warning {
    undeclared(line, value, Source::Flag, OffRadar::Listed)
}

/// FR-17 under the default rings, on a `--table all` page: `Adpot`.
fn default_ring(line: usize) -> Warning {
    undeclared(line, "Adpot", Source::Default, OffRadar::Listed)
}

#[test]
fn each_row_kind_has_its_per_row_message() {
    let cases = [
        (
            ring(37, "Adpot"),
            "line 37, ring: 'Adpot' is not in --rings (Adopt, Trial); the row is in the table view, not on the radar; to plot it, add it to --rings; to leave it off without this warning, add it to --unplotted-rings",
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
            at(
                4,
                4,
                RowKind::RefusedLink {
                    value: "javascript:alert(1)".into(),
                },
            ),
            "line 4, link: 'javascript:alert(1)' is not an http or https address; shown as text, not a link",
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
            "line 12, name: 'Terraform' repeats line 4's name 'terraform' (names match ignoring case and extra spaces); both are drawn, so rename or delete one if they are the same item",
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

/// FR-104's text and what it lacks, as the variants carry it.
fn lacking(text: &str, first: char, count: usize, format: OutputFormat) -> Lacking {
    Lacking {
        text: text.into(),
        first,
        count,
        format,
    }
}

/// FR-104 on a cell under `column`.
fn cell(
    column: ColumnKind,
    text: &str,
    first: char,
    count: usize,
    format: OutputFormat,
) -> RowKind {
    RowKind::MissingGlyphs {
        column: column.as_str().to_owned(),
        lacking: lacking(text, first, count, format),
    }
}

/// FR-104's ending for one missing character in a PNG.
const PNG_ONE: &str = "the PNG draws it as a box — an HTML page shows it, or transliterate it";
/// The same for several.
const PNG_MANY: &str =
    "the PNG draws them as boxes — an HTML page shows them, or transliterate them";

/// FR-104: a drawn CSV text with characters the bundled font lacks, named
/// with its text, its first such character, how many there are, and the
/// format that draws the box.
#[test]
fn missing_glyphs_have_their_per_row_message() {
    let cases = [
        (
            at(
                3,
                0,
                cell(ColumnKind::Name, "Kafka 中", '中', 1, OutputFormat::Png),
            ),
            format!("line 3, name: 'Kafka 中' has U+4E2D 中, which the bundled font lacks; {PNG_ONE}"),
        ),
        (
            at(
                4,
                1,
                cell(ColumnKind::Sector, "عربي", 'ع', 4, OutputFormat::Png),
            ),
            format!(
                "line 4, sector: 'عربي' has 4 characters the bundled font lacks, the first U+0639 ع; {PNG_MANY}"
            ),
        ),
        // A lone combining mark would join the space before it, so only its
        // code point is shown.
        (
            at(
                5,
                0,
                cell(ColumnKind::Name, "q\u{1ab0}", '\u{1ab0}', 1, OutputFormat::Svg),
            ),
            "line 5, name: 'q\u{1ab0}' has U+1AB0, which the bundled font lacks; the SVG draws it as a box — an HTML page shows it, or transliterate it".to_owned(),
        ),
        // A PDF's box carries its character (FR-113), so it still reads and
        // copies as it.
        (
            at(
                6,
                0,
                cell(ColumnKind::Name, "Kafka 中", '中', 1, OutputFormat::Pdf),
            ),
            "line 6, name: 'Kafka 中' has U+4E2D 中, which the bundled font lacks; the PDF draws it as a box, though it still reads and copies as itself — an HTML page shows it, or transliterate it".to_owned(),
        ),
        (
            at(
                7,
                0,
                cell(ColumnKind::Name, "中文", '中', 2, OutputFormat::Pdf),
            ),
            "line 7, name: '中文' has 2 characters the bundled font lacks, the first U+4E2D 中; the PDF draws them as boxes, though they still read and copy as themselves — an HTML page shows them, or transliterate them".to_owned(),
        ),
    ];
    for (warning, expected) in cases {
        assert_eq!(warning.to_string(), expected);
    }
}

/// FR-104 on a flag value: the flag is named, as FR-90 names it.
#[test]
fn missing_glyphs_in_a_flag_name_the_flag() {
    let cases = [
        (
            RenderKind::SettingMissingGlyphs {
                origin: Origin::flag(Setting::Title),
                lacking: lacking("雷达 2026", '雷', 2, OutputFormat::Svg),
            },
            "--title: '雷达 2026' has 2 characters the bundled font lacks, the first U+96F7 雷; the SVG draws them as boxes — an HTML page shows them, or transliterate them".to_owned(),
        ),
        (
            RenderKind::SettingMissingGlyphs {
                origin: Origin::flag(Setting::Rings),
                lacking: lacking("採用", '採', 2, OutputFormat::Png),
            },
            format!(
                "--rings: '採用' has 2 characters the bundled font lacks, the first U+63A1 採; {PNG_MANY}"
            ),
        ),
        (
            RenderKind::SettingMissingGlyphs {
                origin: Origin::flag(Setting::Subtitle),
                lacking: lacking("Q3 🙂", '🙂', 1, OutputFormat::Png),
            },
            format!("--subtitle: 'Q3 🙂' has U+1F642 🙂, which the bundled font lacks; {PNG_ONE}"),
        ),
    ];
    for (kind, expected) in cases {
        assert_eq!(Warning::Render(kind).to_string(), expected);
    }
}

/// FR-104 under NFR-20: one class across columns, listing each text's
/// position, its text, and how many characters it lacks, the first named.
#[test]
fn missing_glyphs_group_into_one_line() {
    let mut w = Warnings::default();
    w.push(at(
        4,
        1,
        cell(ColumnKind::Sector, "عربي", 'ع', 4, OutputFormat::Png),
    ));
    w.push(at(
        3,
        0,
        cell(ColumnKind::Name, "Kafka 中", '中', 1, OutputFormat::Png),
    ));
    assert_eq!(
        w.render(),
        vec![format!(
            "warning: 2 values have characters the bundled font lacks — name 'Kafka 中' on line 3 (U+4E2D 中), sector 'عربي' on line 4 (4 characters, first U+0639 ع); {PNG_MANY}"
        )]
    );
}

/// FR-88 and FR-89 have no line, so no `line N, column:` prefix.
#[test]
fn each_render_level_kind_has_its_message() {
    let cases = [
        (
            Warning::Render(RenderKind::SectorCount { count: 7 }),
            "7 sectors; the radar reads best with 3 to 6, so sectors narrow and labels may crowd",
        ),
        (
            Warning::Render(RenderKind::OverfullBand {
                sector: "Tools".into(),
                ring: "Adopt".into(),
                count: 14,
                fits: 10,
            }),
            "sector 'Tools', ring 'Adopt': 14 items; about 10 fit comfortably, so some markers overlap",
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
        sector: "\u{200b}Tool".into(),
        ring: "Adopt\u{feff}".into(),
        count: 14,
        fits: 10,
    });
    assert!(
        band.to_string().starts_with("sector 'Tool', ring 'Adopt':"),
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
        let shown = ring(3, &format!("a{c}b")).to_string();
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
            "warning: 41 rows use a ring not in --rings (Adopt, Trial) — In Use (40, first line 4), Adpot (1, line 37); all appear in the table view; to plot one, add it to --rings; to leave one off without this warning, add it to --unplotted-rings",
            "warning: line 50, name: 'Kafka' repeats line 9's name 'Kafka' (names match ignoring case and extra spaces); both are drawn, so rename or delete one if they are the same item",
        ]
    );
}

/// FR-17 (D-61): the message names where the ring set came from, so an
/// author who never passed `--rings` is not told to edit it.
#[test]
fn the_ring_warning_names_the_default_ring_set_when_no_flag_declared_one() {
    assert_eq!(
        default_ring(37).to_string(),
        "line 37, ring: 'Adpot' is not one of the default rings (Assess, Trial, Adopt, Retire); the row is in the table view, not on the radar; to plot it, pass --rings with every ring to draw (it replaces the defaults); to leave it off without this warning, add it to --unplotted-rings"
    );
    let mut w = Warnings::default();
    w.push(default_ring(4));
    w.push(default_ring(9));
    assert_eq!(
        w.render(),
        vec![
            "warning: 2 rows use a ring that is not one of the default rings (Assess, Trial, Adopt, Retire) — Adpot (2, first line 4); all appear in the table view; to plot one, pass --rings with every ring to draw (it replaces the defaults); to leave one off without this warning, add it to --unplotted-rings"
        ]
    );
}

/// FR-17 under FR-106: the message says where the row went, which depends
/// on the output and on `--table`. An HTML page under the default leaves it
/// out and says what `--table all` would cost; a PNG, SVG or PDF has no table to
/// name. The grouped line follows suit.
#[test]
fn the_ring_warning_says_where_the_row_went() {
    // Every warning of one run carries the same `off_radar`, so a group
    // speaks for them all.
    let rendered = |off_radar: OffRadar, lines: &[usize], flag: bool| {
        let source = if flag { Source::Flag } else { Source::Default };
        let mut w = Warnings::default();
        for &line in lines {
            w.push(undeclared(line, "Adpot", source.clone(), off_radar));
        }
        w.render()
    };
    assert_eq!(
        rendered(OffRadar::Unlisted, &[37], false),
        [
            "warning: line 37, ring: 'Adpot' is not one of the default rings (Assess, Trial, Adopt, Retire), so the row is left off the page; to plot it, pass --rings with every ring to draw (it replaces the defaults); to leave it off without this warning, add it to --unplotted-rings; --table all lists it, but also publishes every other column"
        ]
    );
    assert_eq!(
        rendered(OffRadar::Unlisted, &[37], true),
        [
            "warning: line 37, ring: 'Adpot' is not in --rings (Adopt, Trial), so the row is left off the page; to plot it, add it to --rings; to leave it off without this warning, add it to --unplotted-rings; --table all lists it, but also publishes every other column"
        ]
    );
    assert_eq!(
        rendered(OffRadar::Undrawn, &[37], false),
        [
            "warning: line 37, ring: 'Adpot' is not one of the default rings (Assess, Trial, Adopt, Retire); the row is not on the radar; to plot it, pass --rings with every ring to draw (it replaces the defaults); to leave it off without this warning, add it to --unplotted-rings"
        ]
    );
    assert_eq!(
        rendered(OffRadar::Undrawn, &[37], true),
        [
            "warning: line 37, ring: 'Adpot' is not in --rings (Adopt, Trial); the row is not on the radar; to plot it, add it to --rings; to leave it off without this warning, add it to --unplotted-rings"
        ]
    );
    assert_eq!(
        rendered(OffRadar::Unlisted, &[4, 9], false),
        [
            "warning: 2 rows use a ring that is not one of the default rings (Assess, Trial, Adopt, Retire) — Adpot (2, first line 4); they are left off the page; to plot one, pass --rings with every ring to draw (it replaces the defaults); to leave one off without this warning, add it to --unplotted-rings; --table all lists them, but also publishes every other column"
        ]
    );
    assert_eq!(
        rendered(OffRadar::Undrawn, &[4, 9], false),
        [
            "warning: 2 rows use a ring that is not one of the default rings (Assess, Trial, Adopt, Retire) — Adpot (2, first line 4); none is on the radar; to plot one, pass --rings with every ring to draw (it replaces the defaults); to leave one off without this warning, add it to --unplotted-rings"
        ]
    );
    assert_eq!(
        rendered(OffRadar::Undrawn, &[4, 9], true),
        [
            "warning: 2 rows use a ring not in --rings (Adopt, Trial) — Adpot (2, first line 4); none is on the radar; to plot one, add it to --rings; to leave one off without this warning, add it to --unplotted-rings"
        ]
    );
    assert_eq!(
        rendered(OffRadar::Listed, &[4, 9], true),
        [
            "warning: 2 rows use a ring not in --rings (Adopt, Trial) — Adpot (2, first line 4); all appear in the table view; to plot one, add it to --rings; to leave one off without this warning, add it to --unplotted-rings"
        ]
    );
}

/// FR-106's warning: `--table all` on a PNG, SVG or PDF, which have no table.
/// FR-112's cramped table: under `--table radar` there is no column to
/// leave out, so the remedy is the wider paper or the HTML page.
#[test]
fn a_cramped_radar_table_names_a4_and_the_page() {
    let table = radargen::settings::Sourced {
        value: radargen::options::Table::Radar,
        origin: Origin::new(Setting::Table, Source::Default),
    };
    assert_eq!(
        Warning::Render(RenderKind::TableCramped { columns: 9, table }).to_string(),
        "--pdf full: the table has 9 columns, so its prose columns are narrower than about 21 characters; --paper a4 is a little wider, and the HTML page shows the table in full"
    );
}

#[test]
fn table_all_on_a_snapshot_says_it_has_no_table() {
    assert_eq!(
        Warning::Render(RenderKind::TableStatic {
            format: radargen::options::OutputFormat::Png,
        })
        .to_string(),
        "--table all applies to HTML pages and the full PDF (--pdf full); PNG and SVG have no table"
    );
    assert_eq!(
        Warning::Render(RenderKind::TableStatic {
            format: radargen::options::OutputFormat::Pdf,
        })
        .to_string(),
        "--table all applies to HTML pages and the full PDF; the radar PDF has no table, so add --pdf full for one"
    );
}

#[test]
fn a_class_with_one_occurrence_prints_the_per_row_message() {
    let mut w = Warnings::default();
    w.push(ring(37, "Adpot"));
    assert_eq!(
        w.render(),
        vec![
            "warning: line 37, ring: 'Adpot' is not in --rings (Adopt, Trial); the row is in the table view, not on the radar; to plot it, add it to --rings; to leave it off without this warning, add it to --unplotted-rings",
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
            "warning: 3 rows use a ring not in --rings (Adopt, Trial) — In Use (3, first line 4); all appear in the table view; to plot one, add it to --rings; to leave one off without this warning, add it to --unplotted-rings",
        ]
    );
}

#[test]
fn render_level_warnings_come_first_and_never_group() {
    let mut w = Warnings::default();
    w.push(ring(2, "X"));
    w.push(Warning::Render(RenderKind::SectorCount { count: 9 }));
    w.push(Warning::Render(RenderKind::OverfullBand {
        sector: "A".into(),
        ring: "Adopt".into(),
        count: 12,
        fits: 10,
    }));
    w.push(Warning::Render(RenderKind::OverfullBand {
        sector: "B".into(),
        ring: "Adopt".into(),
        count: 11,
        fits: 10,
    }));
    assert_eq!(
        w.render(),
        vec![
            "warning: 9 sectors; the radar reads best with 3 to 6, so sectors narrow and labels may crowd",
            "warning: sector 'A', ring 'Adopt': 12 items; about 10 fit comfortably, so some markers overlap",
            "warning: sector 'B', ring 'Adopt': 11 items; about 10 fit comfortably, so some markers overlap",
            "warning: line 2, ring: 'X' is not in --rings (Adopt, Trial); the row is in the table view, not on the radar; to plot it, add it to --rings; to leave it off without this warning, add it to --unplotted-rings",
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
            cell(ColumnKind::Name, "中", '中', 1, OutputFormat::Png),
            "warning: 2 values have characters the bundled font lacks — name '中' on line 4 (U+4E2D 中), name '中' on line 6 (U+4E2D 中); the PNG draws them as boxes — an HTML page shows them, or transliterate them",
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
            "warning: 2 values had characters the output cannot display — description on line 4 (U+0001), name on line 6 (2 characters, first U+007F); each was removed",
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
            "warning: 3 rows use a ring not in --rings (Adopt, Trial) — In Use (2, first line 4), Adpot (1, line 37); all appear in the table view; to plot one, add it to --rings; to leave one off without this warning, add it to --unplotted-rings",
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
                output: NamedPath::new(Origin::flag(Setting::Output), Path::new("radar.txt")),
            },
            "error: -o radar.txt: the output file name must end in .html, .png, .svg or .pdf",
        ),
        (
            Fatal::UnrecognisedExtension {
                output: NamedPath::new(Origin::flag(Setting::Output), Path::new("radar")),
            },
            "error: -o radar: the output file name must end in .html, .png, .svg or .pdf",
        ),
        (
            Fatal::Rasterise {
                reason: "an encoder failure".into(),
            },
            "error: the PNG cannot be drawn: an encoder failure",
        ),
        (
            Fatal::ImageTooLarge {
                width: 10_000,
                height: 12_000,
                width_origin: Origin::new(Setting::Width, Source::Flag),
            },
            "error: a 10000 by 12000 pixel PNG is too large to draw; pass a smaller --width",
        ),
        (
            Fatal::ImageTooLarge {
                width: 10_000,
                height: 12_000,
                width_origin: Origin::new(Setting::Width, radar_toml()),
            },
            "error: a 10000 by 12000 pixel PNG is too large to draw; set a smaller width in radar.toml",
        ),
        (
            Fatal::Rings {
                origin: Origin::flag(Setting::Rings),
                error: RingsError::EmptyEntry { index: 2 },
            },
            "error: --rings: entry 2 is empty",
        ),
        (
            Fatal::Rings {
                origin: Origin::flag(Setting::Rings),
                error: RingsError::Duplicate {
                    index: 3,
                    entry: "adopt".into(),
                    first_index: 1,
                },
            },
            "error: --rings: entry 3 'adopt' repeats entry 1",
        ),
        (
            Fatal::Rings {
                origin: Origin::flag(Setting::Rings),
                error: RingsError::Empty,
            },
            "error: --rings: no rings declared",
        ),
        (
            Fatal::Rings {
                origin: Origin::flag(Setting::Rings),
                error: RingsError::Duplicate {
                    index: 2,
                    entry: "a\nerror: forged".into(),
                    first_index: 1,
                },
            },
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
            Fatal::CsvTooLarge {
                path: "in.csv".into(),
            },
            "error: in.csv: larger than 64 MiB; a radar's CSV is usually a few kilobytes, so check the path",
        ),
        (
            Fatal::Unwritable {
                output: NamedPath::new(Origin::flag(Setting::Output), Path::new("out/radar.html")),
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
            "error: the CSV has no header row (it is empty or blank); the header must name at least name, sector, ring and description",
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
            "error: line 1: column 3 (C) 'Name' repeats column 1 (A) 'name'; header names match ignoring case and extra spaces, so rename or remove one",
        ),
        // A header is CSV text: a quoted newline in it must not forge a line.
        (
            repeats(1, "a\nerror: forged", "a"),
            r"error: line 1: column 2 (B) 'a\nerror: forged' repeats column 1 (A) 'a'; header names match ignoring case and extra spaces, so rename or remove one",
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
            "error: line 1: the header has no 'ring' column; add one, or rename the column that holds it — the header must name at least name, sector, ring and description",
        ),
        (
            RowError::EmptyRequired {
                line: 7,
                column: ColumnKind::Sector,
            },
            "error: line 7, sector: empty; every row needs a sector, so fill it in or delete the row",
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
                output: NamedPath::new(Origin::flag(Setting::Output), Path::new("out/radar.html")),
                dir: "out".into(),
            },
            "error: -o out/radar.html: the directory out does not exist",
        ),
        (
            Fatal::OutputIsInput {
                output: NamedPath::new(Origin::flag(Setting::Output), Path::new("radar.csv")),
                input: ReadFile::Csv(None),
            },
            "error: -o radar.csv: is the input CSV; choose another output path",
        ),
        (
            Fatal::MissingDirectory {
                output: NamedPath {
                    origin: Origin::new(Setting::Output, source::config("conf/radar.toml")),
                    shown: "out/radar.html".into(),
                },
                dir: "conf/out".into(),
            },
            "error: the output key in conf/radar.toml (out/radar.html): the directory conf/out does not exist",
        ),
        (
            Fatal::UnrecognisedExtension {
                output: NamedPath {
                    origin: Origin::new(Setting::Output, source::config("radar.toml")),
                    shown: "radar.txt".into(),
                },
            },
            "error: the output key in radar.toml (radar.txt): the output file name must end in .html, .png, .svg or .pdf",
        ),
        (
            Fatal::PaletteTooLarge {
                palette: NamedPath {
                    origin: Origin::new(Setting::Palette, source::config("radar.toml")),
                    shown: "brand.css".into(),
                },
            },
            "error: the palette key in radar.toml (brand.css): larger than 1 MiB; a palette is a few hundred bytes, so check the path",
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

/// D-87: the clamp names both widths; the small-text warning names the
/// width that fixes it, or says none within FR-64 does.
#[test]
fn d87_warnings_name_their_widths() {
    let cases = [
        (
            RenderKind::WidthClamped {
                natural: 18_356,
                drawn: 10_000,
            },
            "the radar is 18356 pixels wide, past --width's limit; the PNG is drawn 10000 wide, or pass --width to choose",
        ),
        (
            RenderKind::PngTextSmall {
                width: 320,
                legible_from: Some(821),
                width_origin: Origin::new(Setting::Width, Source::Flag),
            },
            "at 320 pixels wide the PNG's text is too small to read; --width 821 or more keeps it legible",
        ),
        (
            RenderKind::PngTextSmall {
                width: 10_000,
                legible_from: None,
                width_origin: Origin::new(Setting::Width, Source::Default),
            },
            "at 10000 pixels wide the PNG's text is too small to read, at any --width; fewer or shorter names would help",
        ),
    ];
    for (kind, expected) in cases {
        assert_eq!(Warning::Render(kind).to_string(), expected);
    }
}

/// RG-102: --legend hidden on a static output says what happened.
#[test]
fn legend_static_says_png_and_svg_keep_the_legend() {
    assert_eq!(
        Warning::Render(RenderKind::LegendStatic).to_string(),
        "--legend hidden applies to HTML pages; PNG, SVG and PDF always draw the legend"
    );
}

/// FR-95 names the label that sets the width, sector or ring (D-73),
/// trimmed and single-quoted as FR-89's names are.
#[test]
fn fr95_names_a_sector_or_a_ring() {
    let cases = [
        (
            RenderKind::NarrowWidened {
                binding: Binding::Sector(" Infrastructure Platforms and Tooling ".into()),
                width: 503,
            },
            "the narrow layout is 503 units wide, past the 480 its legend rows and marker targets are sized for, so on a 320-pixel screen they fall below 24 pixels; a shorter name for sector 'Infrastructure Platforms and Tooling' would fix it",
        ),
        (
            RenderKind::NarrowWidened {
                binding: Binding::Ring {
                    name: "Under evaluation by the architecture board".into(),
                    origin: Origin::rings(Source::Flag),
                },
                width: 750,
            },
            "the narrow layout is 750 units wide, past the 480 its legend rows and marker targets are sized for, so on a 320-pixel screen they fall below 24 pixels; a shorter name for ring 'Under evaluation by the architecture board' in --rings would fix it",
        ),
    ];
    for (kind, expected) in cases {
        assert_eq!(Warning::Render(kind).to_string(), expected);
    }
}

/// FR-105 gives the breakpoint and names the label that sets the wide
/// radar's width as FR-95 names its own (D-94); the ring phrases are
/// FR-95's tests'.
#[test]
fn fr105_gives_the_breakpoint_and_names_the_label() {
    assert_eq!(
        Warning::Render(RenderKind::WideBeyondLaptops {
            binding: Binding::Sector(
                " Platforms and Infrastructure Shared Across Every Product Team ".into(),
            ),
            breakpoint: 1477,
        })
        .to_string(),
        "the wide layout needs a screen at least 1477 pixels wide, so 1440-pixel laptops get the narrow layout; a shorter name for sector 'Platforms and Infrastructure Shared Across Every Product Team' would fix it"
    );
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
            output: NamedPath::new(Origin::flag(Setting::Output), Path::new("out.html")),
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
/// from (D-73), so the fix points at the right place, and trimmed, either
/// way.
#[test]
fn fr95_names_where_a_ring_came_from() {
    let message = |source| {
        Warning::Render(RenderKind::NarrowWidened {
            binding: Binding::Ring {
                name: " Under evaluation ".into(),
                origin: Origin::rings(source),
            },
            width: 600,
        })
        .to_string()
    };
    assert!(
        message(Source::Flag)
            .ends_with("a shorter name for ring 'Under evaluation' in --rings would fix it")
    );
    assert!(message(Source::Default).ends_with(
        "a shorter name for ring 'Under evaluation' (a default ring, so pass --rings to rename it) would fix it"
    ));
}

/// FR-96 under NFR-20: refused links group by value as written, trimmed and
/// not folded, since a URL's path is case-sensitive.
#[test]
fn refused_links_group_by_value() {
    let link = |line, value: &str| {
        at(
            line,
            4,
            RowKind::RefusedLink {
                value: value.into(),
            },
        )
    };
    let mut w = Warnings::default();
    w.push(link(2, "ftp://x"));
    w.push(link(3, " ftp://x "));
    w.push(link(4, "mailto:y"));
    w.push(link(5, "FTP://x"));
    assert_eq!(
        w.render(),
        [
            "warning: 4 rows have a link that is not an http or https address — ftp://x (2, first line 2), mailto:y (1, line 4), FTP://x (1, line 5); shown as text, not links"
        ],
        "one line, values in first-occurrence order"
    );
}

/// D-96: a run already given `--unplotted-rings` is not pointed at it again,
/// per row or grouped, in every place a row can go; the rest of the
/// message is unchanged and `--table all`'s clause stays last.
#[test]
fn the_unplotted_rings_hint_is_left_out_once_the_flag_is_given() {
    let quiet = "to leave it off without this warning, add it to --unplotted-rings";
    let quiet_one = "to leave one off without this warning, add it to --unplotted-rings";
    for off_radar in [OffRadar::Listed, OffRadar::Unlisted, OffRadar::Undrawn] {
        for source in [Source::Flag, Source::Default] {
            let row =
                |suggest| quiet_hint(37, "Adpot", source.clone(), off_radar, suggest).to_string();
            let with = row(true);
            assert!(with.contains(quiet), "{with}");
            assert_eq!(row(false), with.replacen(&format!("; {quiet}"), "", 1));
            let grouped = |suggest| {
                let mut w = Warnings::default();
                w.push(quiet_hint(4, "Adpot", source.clone(), off_radar, suggest));
                w.push(quiet_hint(9, "Adpot", source.clone(), off_radar, suggest));
                w.render()
            };
            let [with] = grouped(true).try_into().expect("one line");
            assert!(with.contains(quiet_one), "{with}");
            assert_eq!(
                grouped(false),
                [with.replacen(&format!("; {quiet_one}"), "", 1)]
            );
        }
    }
    assert_eq!(
        quiet_hint(37, "Adpot", Source::Flag, OffRadar::Unlisted, false).to_string(),
        "line 37, ring: 'Adpot' is not in --rings (Adopt, Trial), so the row is left off the page; to plot it, add it to --rings; --table all lists it, but also publishes every other column"
    );
}

/// FR-108 on the header cell at `column`, written `heading`.
fn near_miss(column: usize, heading: &str, known: ColumnKind) -> Warning {
    at(
        1,
        column,
        RowKind::NearMiss {
            heading: heading.into(),
            known,
        },
    )
}

/// FR-108: the header as written, trimmed, and the column it nearly names.
#[test]
fn a_near_miss_header_names_the_column_it_nearly_is() {
    assert_eq!(
        near_miss(4, " Rationalle ", ColumnKind::Rationale).to_string(),
        "line 1, Rationalle: not a column radargen knows; did you mean 'rationale'?"
    );
}

/// FR-108 groups as NFR-20 does: one line at the first, each header with
/// its suggestion in column order, and a row warning after it (NFR-11).
#[test]
fn near_misses_group_into_one_line_before_the_rows() {
    let mut w = Warnings::default();
    w.push(at(2, 3, RowKind::EmptyDescription));
    w.push(near_miss(6, "exit date", ColumnKind::ExitDate));
    w.push(near_miss(4, "Rationalle", ColumnKind::Rationale));
    assert_eq!(
        w.render(),
        vec![
            "warning: 2 header names are not columns radargen knows — Rationalle (did you mean rationale?), exit date (did you mean exit_date?)",
            "warning: line 2, description: empty; the detail panel omits it",
        ]
    );
}

fn radar_toml() -> Source {
    source::config("radar.toml")
}

/// FR-107's two cross-source examples, word for word.
#[test]
fn an_overlap_across_sources_names_both_sides() {
    let key_vs_flag = Fatal::UnplottedRings {
        origin: Origin::new(Setting::UnplottedRings, radar_toml()),
        error: RingsError::Overlap {
            index: 2,
            entry: "Adopt".into(),
            by: Origin::rings(Source::Flag),
        },
    };
    assert_eq!(
        key_vs_flag.render(),
        "error: the unplotted_rings key in radar.toml: entry 2 'Adopt' is also in --rings, which draws it; remove it from one list"
    );
    let flag_vs_key = Fatal::UnplottedRings {
        origin: Origin::flag(Setting::UnplottedRings),
        error: RingsError::Overlap {
            index: 1,
            entry: "Adopt".into(),
            by: Origin::rings(radar_toml()),
        },
    };
    assert_eq!(
        flag_vs_key.render(),
        "error: --unplotted-rings: entry 1 'Adopt' is also in the rings key in radar.toml, which draws it; remove it from one list"
    );
    let rings = Fatal::Rings {
        origin: Origin::new(Setting::Rings, radar_toml()),
        error: RingsError::Empty,
    };
    assert_eq!(
        rings.render(),
        "error: the rings key in radar.toml: no rings declared"
    );
}

/// FR-17 names a file's ring set by its key and file, alone and grouped,
/// and says to edit that list (D-61).
#[test]
fn an_undeclared_ring_names_the_rings_key() {
    let one = quiet_hint(4, "Adpot", radar_toml(), OffRadar::Undrawn, true);
    assert_eq!(
        one.to_string(),
        "line 4, ring: 'Adpot' is not in the rings key in radar.toml (Adopt, Trial); the row is not on the radar; to plot it, add it to that list; to leave it off without this warning, add it to the unplotted_rings key in radar.toml"
    );
    let mut warnings = Warnings::default();
    warnings.push(quiet_hint(
        4,
        "Adpot",
        radar_toml(),
        OffRadar::Undrawn,
        false,
    ));
    warnings.push(quiet_hint(
        9,
        "Adpot",
        radar_toml(),
        OffRadar::Undrawn,
        false,
    ));
    assert_eq!(
        warnings.render(),
        [
            "warning: 2 rows use a ring not in the rings key in radar.toml (Adopt, Trial) — Adpot (2, first line 4); none is on the radar; to plot one, add it to that list"
        ]
    );
}

/// UX review 4: a ring set from the file names the file's keys in every
/// remedy, and its rings key only once.
#[test]
fn an_undeclared_ring_from_a_file_names_the_file_s_keys() {
    let row = quiet_hint(4, "Adpot", radar_toml(), OffRadar::Unlisted, true);
    assert_eq!(
        row.to_string(),
        "line 4, ring: 'Adpot' is not in the rings key in radar.toml (Adopt, Trial), so the row is left off the page; to plot it, add it to that list; to leave it off without this warning, add it to the unplotted_rings key in radar.toml; setting table = \"all\" in radar.toml would list it, but also publishes every other column"
    );
}

/// FR-95 and FR-105 name a ring from a file by its key and file.
#[test]
fn a_binding_ring_from_a_file_names_its_key() {
    let binding = Binding::Ring {
        name: "Very long ring".into(),
        origin: Origin::rings(radar_toml()),
    };
    assert_eq!(
        binding.to_string(),
        "ring 'Very long ring' in the rings key in radar.toml"
    );
}

/// D-87's remedy names where the width came from (plan RG-019A, Ruling 9).
#[test]
fn a_small_png_names_the_width_key() {
    let warning = Warning::Render(RenderKind::PngTextSmall {
        width: 800,
        legible_from: Some(1200),
        width_origin: Origin::new(Setting::Width, radar_toml()),
    });
    assert_eq!(
        warning.to_string(),
        "at 800 pixels wide the PNG's text is too small to read; setting width = 1200 or more in radar.toml keeps it legible"
    );
    let unreadable = Warning::Render(RenderKind::PngTextSmall {
        width: 800,
        legible_from: None,
        width_origin: Origin::new(Setting::Width, radar_toml()),
    });
    assert_eq!(
        unreadable.to_string(),
        "at 800 pixels wide the PNG's text is too small to read, at any width; fewer or shorter names would help"
    );
}

/// FR-91's key messages, each naming the key and the file; clap's words
/// where clap would have checked the flag (plan RG-019A, Ruling 4).
#[test]
fn config_messages() {
    let file: std::sync::Arc<str> = std::sync::Arc::from("radar.toml");
    let key = |problem| Fatal::ConfigKey {
        file: file.clone(),
        problem,
    };
    let cases = [
        (
            key(KeyProblem::Unknown {
                key: "ring".into(),
                near: Some(Setting::Rings),
                table: false,
            }),
            "error: --config radar.toml: unknown key 'ring'; did you mean 'rings'? The keys are input, output, title, subtitle, rings, unplotted_rings, theme, width, legend_columns, legend, table, paper, pdf, palette and edition",
        ),
        (
            key(KeyProblem::WrongType {
                setting: Setting::Rings,
                found: "string",
            }),
            "error: the rings key in radar.toml: must be an array of strings, such as [\"Adopt\", \"Trial\"], not a string",
        ),
        (
            key(KeyProblem::WrongType {
                setting: Setting::Width,
                found: "float",
            }),
            "error: the width key in radar.toml: must be an integer, not a float",
        ),
        (
            key(KeyProblem::EntryType {
                setting: Setting::Rings,
                index: 2,
                found: "integer",
            }),
            "error: the rings key in radar.toml: entry 2 must be a string, not an integer",
        ),
        (
            key(KeyProblem::Comma {
                setting: Setting::Rings,
                index: 1,
                entry: "Adopt,Trial".into(),
            }),
            "error: the rings key in radar.toml: entry 1 'Adopt,Trial' holds a comma; give each ring as its own entry",
        ),
        (
            key(KeyProblem::NotAValue {
                setting: Setting::Theme,
                value: "sepia".into(),
                possible: vec!["light".into(), "dark".into()],
            }),
            "error: invalid value 'sepia' for the theme key in radar.toml [possible values: light, dark]",
        ),
        (
            key(KeyProblem::OutOfRange {
                setting: Setting::Width,
                value: 5,
                min: 320,
                max: 10_000,
            }),
            "error: the width key in radar.toml: 5 is out of range; use 320 to 10000",
        ),
    ];
    for (fatal, expected) in cases {
        assert_eq!(fatal.render(), expected);
    }
    assert!(
        key(KeyProblem::Unknown {
            key: "x".into(),
            near: None,
            table: false
        })
        .is_usage()
    );
}

/// UX review 3 and 11, Rust review M6: the hints a key problem adds.
#[test]
fn config_hint_messages() {
    let file: std::sync::Arc<str> = std::sync::Arc::from("radar.toml");
    let key = |problem| Fatal::ConfigKey {
        file: file.clone(),
        problem,
    };
    let cases = [
        (
            key(KeyProblem::Unknown {
                key: "radar".into(),
                near: None,
                table: true,
            }),
            "error: --config radar.toml: unknown key 'radar'; the keys are input, output, title, subtitle, rings, unplotted_rings, theme, width, legend_columns, legend, table, paper, pdf, palette and edition; keys go at the top of the file, not under [radar]",
        ),
        (
            key(KeyProblem::WrongType {
                setting: Setting::Title,
                found: "integer",
            }),
            "error: the title key in radar.toml: must be a string, not an integer; put the value in quotes",
        ),
        (
            key(KeyProblem::WrongType {
                setting: Setting::LegendColumns,
                found: "string",
            }),
            "error: the legend_columns key in radar.toml: must be an integer, not a string; remove the quotes",
        ),
        (
            key(KeyProblem::Empty {
                setting: Setting::Output,
            }),
            "error: the output key in radar.toml: is empty; give a file name, or remove the key",
        ),
    ];
    for (fatal, expected) in cases {
        assert_eq!(fatal.render(), expected);
    }
}

/// FR-59 across sources, a missing output, and the file's runtime
/// failures (NFR-15), each naming the file.
#[test]
fn config_file_messages() {
    let file: std::sync::Arc<str> = std::sync::Arc::from("radar.toml");
    let cases = [
        (
            Fatal::Conflict { file: file.clone() },
            "error: --config radar.toml: the theme and palette keys cannot be used together; a palette replaces the theme, so remove one",
        ),
        (
            Fatal::NoOutput { file: file.clone() },
            "error: no output: pass -o <OUTPUT>, or add an output key to radar.toml, such as output = \"radar.html\"",
        ),
        (
            Fatal::ConfigTooLarge { file: file.clone() },
            "error: --config radar.toml: larger than 1 MiB; a config file is a few hundred bytes, so check the path",
        ),
        (
            Fatal::ConfigNotUtf8 {
                file: file.clone(),
                line: 4,
            },
            "error: --config radar.toml, line 4: not UTF-8 text; re-save the file as UTF-8",
        ),
        (
            Fatal::ConfigSyntax {
                file: file.clone(),
                line: Some(3),
                message: "duplicate key".into(),
            },
            "error: --config radar.toml, line 3: duplicate key",
        ),
    ];
    for (fatal, expected) in cases {
        assert_eq!(fatal.render(), expected);
    }
    assert!(Fatal::NoOutput { file: file.clone() }.is_usage());
    assert!(
        !Fatal::ConfigTooLarge { file }.is_usage(),
        "NFR-15: a runtime failure"
    );
}
