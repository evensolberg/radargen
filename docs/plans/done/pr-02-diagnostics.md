---
tags:
  - plan
  - diagnostics
aliases: []
doc_id: RG-002
doc_name: pr-02-diagnostics
crumb_id: rg-xr3
document_title: "PR 02 — `diagnostics.rs`: Implementation Plan"
created_date: 2026-10-09
synopsis: "One diagnostic taxonomy: a Fatal error enum and Warnings printed in NFR-11's order with NFR-20's grouping."
status: complete
type: Plan
revision: "1.7"
review_date: 2026-10-09
reviewed_by:
  - rust-engineer
  - qa-expert
  - architect-reviewer
  - ui-designer
  - ux-researcher
completed_date: 2026-10-10
comments:
---
# PR 02 — `diagnostics.rs`: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-xr3` · **Phase:** v1.0 · **Blocks:** `rg-aqb` (PR 05), `rg-rnc` (PR 06), `rg-tiu` (FR-63 collect-all)

**Goal:** One diagnostic taxonomy for the whole tool: `Fatal` for errors (a `thiserror` enum), and `Warning`/`Warnings` for warnings, printed in NFR-11's order with NFR-20's grouping.

**Architecture:** `Warnings` is a plain `Vec<Warning>`, grouped only when it's printed (architecture §7). A warning's `Position` is either `Render` or `Cell { line, column }`. `Render` is declared first, so the derived `Ord` sorts it first, as FR-88 requires. A stable sort by position then gives NFR-11's order with no map. Row-level warnings group into NFR-20 classes. Render-level ones never group: FR-88 fires once, and FR-89 promises one warning per band. `cli::FatalError` moves into `diagnostics::Fatal`, so there's one error taxonomy rather than two (§1). `RingsError` gets a `Display` and becomes `Fatal::Rings`.

**Tech Stack:** Rust 2024, `thiserror` 2 (already a dependency).

**Spec:** `docs/product/radargen-prd.md` v0.38: NFR-9, NFR-10, NFR-11, NFR-16, NFR-20, FR-7, FR-17, FR-20, FR-24–FR-26, FR-28, FR-52, FR-63, FR-88, FR-89, FR-90, SC-10, SC-24, D-37, D-51, D-55. Design: `docs/design/architecture.md` §1 (error strategy), §6 (no `HashMap`), §7 (threading).

## Global Constraints

- **`thiserror`, not `anyhow`** (architecture §1).
- **No `HashMap` or `HashSet`** anywhere warnings are ordered or grouped (§6). Use `Vec` and explicit sorts.
- **Ordering:** render-level warnings first. Then ascending CSV line, and within a line, header column order (NFR-11, FR-88).
- **Grouping:** one line per class, listing the distinct values in first-occurrence order with no cap. A class with exactly one occurrence prints the ordinary per-row message (NFR-20).
- **No flag** changes any of this (D-51).
- `cargo clippy --all-targets -- -D warnings` is clean. `Cargo.toml`'s `[lints]` denies `clippy::pedantic` and forbids `unsafe_code`, so that one command enforces both.
- Commits: `type(scope): description`, plus the session's `Co-Authored-By` trailer. Run `git-mit es` first if present.

## Review Focus

1. **A value containing a newline or other control character.** A quoted CSV field can hold a newline (NFR-10). Printed raw, it would forge a second line on stderr that reads like a separate diagnostic. *Test: Task 1, `control_characters_in_values_are_shown_escaped`.*
2. **Two spellings of one value.** `In Use` and `in use ` are one value under FR-76. Listing them separately would hide a count behind a whitespace difference. *Test: Task 2, `values_group_under_fr_76_normalisation`.*
3. **Warnings pushed out of line order.** Duplicate-name detection may push a warning for line 3 after one for line 40. Order must come from position, not push order. *Test: Task 2, `out_of_order_pushes_are_sorted_by_position`.*
4. **Several overfull bands.** FR-89 promises one warning per band. A class-grouping rule applied blindly would merge them into one line. *Test: Task 2, `render_level_warnings_come_first_and_never_group`.*
5. **A long tail of distinct values.** NFR-20 forbids a cap, because the rare value is the likely typo. *Test: Task 2, `grouped_values_are_never_capped`.*

## Decisions taken in this plan (flag on review)

The PRD fixes two message texts: NFR-20's grouped ring line and FR-20's qualifier message. **Every other wording below is proposed**. SC-10's golden tests (PR 13 onward) will pin them, so this is the cheap moment to change them.

| Class | Per-row message (after `line N, column:`) | Grouped line |
| --- | --- | --- |
| ring (FR-17) | `ring 'X' is not declared by --rings; the row is in the table view, not on the radar` | `N rows use a ring --rings does not declare — …; all appear in the table view` *(PRD)* |
| qualifier (FR-20) | `unrecognised qualifier value 'X'` *(PRD)* | `N rows have an unrecognised qualifier — …; each is drawn solid` |
| status (FR-26) | `unrecognised status value 'X'; drawn as No Change` | `N rows have an unrecognised status — …; each is drawn as No Change` |
| date (FR-24/25) | `'X' is not a YYYY-MM-DD date; shown as written` | `N rows have last_affirmed values that are not YYYY-MM-DD dates — …; each is shown as written` |
| description (FR-7) | `description is empty; the detail panel omits it` | `N rows have an empty description — lines L1, L2, …; the detail panel omits each` |
| duplicate name (FR-28) | `name 'X' repeats line L; both are drawn` | `N rows repeat an earlier name — X (n, first line F; original line L), …; every row is drawn` |
| undisplayable character (FR-90) | `removed U+0001, which the output cannot display`, or `removed 3 characters the output cannot display, the first U+0001` | `N fields held characters the output cannot display — line L1 C1, line L2 C2, …; each was removed` |
| FR-88 (no line) | `N quadrants; the radar reads best with three to six, so sectors narrow and labels may crowd` | never grouped |
| FR-89 (no line) | `quadrant 'Q', ring 'R': N items; about F fit comfortably, so some markers overlap` | never grouped |

Other choices:
- **Each line starts with `warning: ` or `error: `.**
- **The per-row prefix is `line N, column: `**, using the canonical lower-case column name.
- **The two date columns are separate classes.** Each grouped line then names its column, as NFR-10 requires.
- **The value shown is the first occurrence's, trimmed.** Values group by `normalise()`.
- **For duplicate names, the count is the number of repeat rows,** and "first line" is the first repeat. The per-row message names the original line, and so does each grouped value (`original line L`). Without it, the grouped line names only half of each pair.
- **An empty description lists every line, uncapped.** It has no value to enumerate, so NFR-20's "the rare one is the typo" reasoning applies to the lines instead.
- **FR-90's warning is one per field, not one per character,** with the count and the first code point as `U+XXXX`. Its grouped line lists `line column` pairs, uncapped. `column` carries the header name because `Position::Cell` holds only the index.
- **This PR covers FR-90 for CSV cells only.** `Undisplayable` names a column; the flag-value half (`--title`, `--subtitle`, `--rings`) needs a position that names a flag, and is crumb `rg-akv`.
- **Every stderr line is escaped whole**, in `Warning`'s `Display`, in `render`'s grouped lines and in `Fatal::render`. Values are only trimmed where they're interpolated, so no new kind can forget to escape one.
- **FR-89 quotes its names.** Quadrant names can contain `/` or `&`, so `Q / R` would be ambiguous.
- **Bidi controls are escaped too** (U+200E/F, U+202A–E, U+2066–9). `is_control` misses them, and they can visually reorder a stderr line just as an escape sequence can recolour it.
- **`cli::FatalError` becomes `diagnostics::Fatal`.** The `--rings` messages are `entry N is empty`, `'X' repeats entry N` and `no rings declared`. The `-o` message is `the output file name must end in .html`, which reads correctly for a wrong, missing or `.htm` extension (FR-52 covers the absent case too).
- **`Fatal` derives no `PartialEq`.** PR 13's write failures (NFR-15) carry `std::io::Error`, which isn't comparable, so the derive would break there. Tests compare `render()` output or use `matches!`.

---

### Task 1: `Position`, `Warning`, `WarningKind` and the per-row message

**Files:**
- Create: `src/diagnostics.rs`
- Modify: `src/lib.rs` (add `pub mod diagnostics;`, keeping alphabetical order with the other `pub mod` lines)
- Modify: `docs/design/architecture.md` (§7's last paragraph: a warning's position is `Option<line>` with `None` first becomes `Position::{Render, Cell { line, column }}`, `Render` first — NFR-11 also orders by column within a line, which `Option<line>` cannot express)
- Test: `tests/diagnostics.rs`

**Interfaces:**
- Consumes: `radargen::normalise` (exists).
- Produces, all in `radargen::diagnostics`:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)] pub enum Position { Render, Cell { line: usize, column: usize } }`
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum DateColumn { LastAffirmed, ExitDate }`, which implements `Display` as `last_affirmed` and `exit_date`
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub enum WarningKind`. Its variants are exactly those in the code below.
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct Warning { pub position: Position, pub kind: WarningKind }`, which implements `Display` as the per-row form without the `warning: ` prefix

  PR 06 pushes the row-level kinds, and PR 08/10 push `QuadrantCount` and `OverfullBand`. `column` is the header index of the column concerned, which PR 06 supplies.

- [x] **Step 1: Write the failing tests**

`tests/diagnostics.rs`:

```rust
//! The diagnostic contract (architecture §10): exact messages, NFR-11 order,
//! NFR-20 grouping. SC-10's golden tests later pin the same text end to end.

use radargen::diagnostics::{DateColumn, Position, Warning, WarningKind};

fn at(line: usize, column: usize, kind: WarningKind) -> Warning {
    Warning { position: Position::Cell { line, column }, kind }
}

fn render_level(kind: WarningKind) -> Warning {
    Warning { position: Position::Render, kind }
}

#[test]
fn each_kind_has_its_per_row_message() {
    let cases = [
        (at(37, 2, WarningKind::UndeclaredRing { value: "Adpot".into() }),
         "line 37, ring: ring 'Adpot' is not declared by --rings; the row is in the table view, not on the radar"),
        (at(5, 4, WarningKind::UnrecognisedQualifier { value: "maybe".into() }),
         "line 5, qualifier: unrecognised qualifier value 'maybe'"),
        (at(6, 5, WarningKind::UnrecognisedStatus { value: "Gone".into() }),
         "line 6, status: unrecognised status value 'Gone'; drawn as No Change"),
        (at(7, 6, WarningKind::UnparsableDate { column: DateColumn::LastAffirmed, value: "June".into() }),
         "line 7, last_affirmed: 'June' is not a YYYY-MM-DD date; shown as written"),
        (at(8, 7, WarningKind::UnparsableDate { column: DateColumn::ExitDate, value: "soon".into() }),
         "line 8, exit_date: 'soon' is not a YYYY-MM-DD date; shown as written"),
        (at(9, 3, WarningKind::EmptyDescription),
         "line 9, description: description is empty; the detail panel omits it"),
        (at(12, 0, WarningKind::DuplicateName { value: "Terraform".into(), first_line: 4 }),
         "line 12, name: name 'Terraform' repeats line 4; both are drawn"),
        (at(14, 3, WarningKind::Undisplayable { column: "description".into(), first: '\u{1}', count: 1 }),
         "line 14, description: removed U+0001, which the output cannot display"),
        (at(15, 0, WarningKind::Undisplayable { column: "name".into(), first: '\u{1f}', count: 3 }),
         "line 15, name: removed 3 characters the output cannot display, the first U+001F"),
        (render_level(WarningKind::QuadrantCount { count: 7 }),
         "7 quadrants; the radar reads best with three to six, so sectors narrow and labels may crowd"),
        (render_level(WarningKind::OverfullBand { quadrant: "Tools".into(), ring: "Adopt".into(), count: 14, fits: 10 }),
         "quadrant 'Tools', ring 'Adopt': 14 items; about 10 fit comfortably, so some markers overlap"),
    ];
    for (warning, expected) in cases {
        assert_eq!(warning.to_string(), expected);
    }
}

#[test]
fn values_are_shown_trimmed() {
    let w = at(3, 2, WarningKind::UndeclaredRing { value: "  Adpot ".into() });
    assert!(w.to_string().contains("ring 'Adpot' is"), "{w}");
}

/// NFR-10: a quoted field can hold a newline. Printed raw it would forge a
/// second diagnostic line.
#[test]
fn control_characters_in_values_are_shown_escaped() {
    let w = at(3, 2, WarningKind::UndeclaredRing { value: "In\nUse\u{1b}[31m".into() });
    let s = w.to_string();
    assert!(!s.contains('\n') && !s.contains('\u{1b}'), "{s:?}");
    assert!(s.contains(r"In\nUse\u{1b}[31m"), "{s}");
    let bidi = at(3, 2, WarningKind::UndeclaredRing { value: "a\u{202e}b".into() }).to_string();
    assert!(bidi.contains(r"a\u{202e}b"), "{bidi}");
    // An extra column's header is CSV text too (FR-18).
    let header = at(3, 9, WarningKind::Undisplayable { column: "notes\nline 1, name: forged".into(), first: '\u{1}', count: 1 });
    let header = header.to_string();
    assert!(!header.contains('\n') && header.contains(r"notes\nline 1"), "{header}");
}

#[test]
fn render_level_sorts_before_any_cell() {
    assert!(Position::Render < Position::Cell { line: 1, column: 0 });
    assert!(Position::Cell { line: 5, column: 3 } < Position::Cell { line: 5, column: 4 });
    assert!(Position::Cell { line: 5, column: 9 } < Position::Cell { line: 6, column: 0 });
}
```

- [x] **Step 2: Run them to see them fail**

Run: `cargo test --test diagnostics`
Expected: a compile error, `unresolved import radargen::diagnostics`.

- [x] **Step 3: Implement**

`src/diagnostics.rs`:

```rust
//! The diagnostic taxonomy (architecture §3, §7): what is fatal, what warns,
//! and how warnings reach stderr (NFR-10, NFR-11, NFR-20).

use std::fmt;

/// Where a warning points.
///
/// `Render` concerns the render as a whole (FR-88, FR-89) and has no line.
/// It is declared first so the derived order puts it before every row
/// warning, which is NFR-11's order once FR-88 is placed (architecture §7) —
/// a sort key, not a special case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Position {
    Render,
    /// `line` is the 1-based physical line the record begins on, counting the
    /// header as line 1 (NFR-10, D-37). `column` is the header index, which
    /// orders warnings within a line (NFR-11).
    Cell { line: usize, column: usize },
}

/// FR-24 and FR-25 share a parsing rule but are separate columns, and NFR-10
/// requires the message to name the column — so they are separate classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateColumn {
    LastAffirmed,
    ExitDate,
}

impl fmt::Display for DateColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::LastAffirmed => "last_affirmed",
            Self::ExitDate => "exit_date",
        })
    }
}

/// Every warning NFR-9 lists. Values are carried as the CSV had them; display
/// trims them and escapes control characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarningKind {
    /// FR-17.
    UndeclaredRing { value: String },
    /// FR-20 — non-blank only; a blank qualifier claims nothing (D-35).
    UnrecognisedQualifier { value: String },
    /// FR-26 — non-blank only, as FR-20.
    UnrecognisedStatus { value: String },
    /// FR-24, FR-25.
    UnparsableDate { column: DateColumn, value: String },
    /// FR-7.
    EmptyDescription,
    /// FR-28, reported at the later line naming the earlier (NFR-10).
    DuplicateName { value: String, first_line: usize },
    /// FR-90: one per field, however many characters it lost. `column` is the
    /// canonical header name, since any column can carry one.
    Undisplayable { column: String, first: char, count: usize },
    /// FR-88. Render-level.
    QuadrantCount { count: usize },
    /// FR-89. Render-level, one per overfull band.
    OverfullBand { quadrant: String, ring: String, count: usize, fits: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub position: Position,
    pub kind: WarningKind,
}

impl WarningKind {
    /// The column a row-level warning concerns, as NFR-10 names it.
    fn column(&self) -> String {
        match self {
            Self::UndeclaredRing { .. } => "ring".into(),
            Self::UnrecognisedQualifier { .. } => "qualifier".into(),
            Self::UnrecognisedStatus { .. } => "status".into(),
            Self::UnparsableDate { column, .. } => column.to_string(),
            Self::EmptyDescription => "description".into(),
            Self::DuplicateName { .. } => "name".into(),
            // A header name is CSV text (FR-18's extra columns); `Display`
            // escapes it with the rest of the line.
            Self::Undisplayable { column, .. } => column.clone(),
            Self::QuadrantCount { .. } | Self::OverfullBand { .. } => String::new(),
        }
    }

    fn message(&self) -> String {
        match self {
            Self::UndeclaredRing { value } => format!(
                "ring '{}' is not declared by --rings; the row is in the table view, not on the radar",
                value.trim()
            ),
            Self::UnrecognisedQualifier { value } => {
                format!("unrecognised qualifier value '{}'", value.trim())
            }
            Self::UnrecognisedStatus { value } => format!(
                "unrecognised status value '{}'; drawn as No Change",
                value.trim()
            ),
            Self::UnparsableDate { value, .. } => format!(
                "'{}' is not a YYYY-MM-DD date; shown as written",
                value.trim()
            ),
            Self::EmptyDescription => "description is empty; the detail panel omits it".into(),
            Self::DuplicateName { value, first_line } => format!(
                "name '{}' repeats line {first_line}; both are drawn",
                value.trim()
            ),
            Self::Undisplayable { first, count: 1, .. } => {
                format!("removed U+{:04X}, which the output cannot display", u32::from(*first))
            }
            Self::Undisplayable { first, count, .. } => format!(
                "removed {count} characters the output cannot display, the first U+{:04X}",
                u32::from(*first)
            ),
            Self::QuadrantCount { count } => format!(
                "{count} quadrants; the radar reads best with three to six, so sectors narrow and labels may crowd"
            ),
            Self::OverfullBand { quadrant, ring, count, fits } => format!(
                "quadrant '{}', ring '{}': {count} items; about {fits} fit comfortably, so some markers overlap",
                quadrant.trim(),
                ring.trim()
            ),
        }
    }
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let line = match self.position {
            Position::Render => self.kind.message(),
            Position::Cell { line, .. } => {
                format!("line {line}, {}: {}", self.kind.column(), self.kind.message())
            }
        };
        f.write_str(&escape_controls(&line))
    }
}

/// A CSV value can carry a newline inside quotes (NFR-10), and printed raw it
/// would forge a second diagnostic line; an escape sequence could recolour the
/// terminal, and a bidi override could reorder it. Every stderr line passes
/// through here whole, so no value can be forgotten; values are only trimmed.
fn escape_controls(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_control()
            || matches!(c, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        {
            out.extend(c.escape_default());
        } else {
            out.push(c);
        }
    }
    out
}
```

Add `pub mod diagnostics;` to `src/lib.rs`, before `pub mod rings;`.

- [x] **Step 4: Run the tests**

Run: `cargo test --test diagnostics && cargo clippy --all-targets -- -D warnings`
Expected: 4 passed, clippy clean.

- [x] **Step 5: Commit**

```bash
git add src/diagnostics.rs src/lib.rs tests/diagnostics.rs docs/design/architecture.md
git commit -m "feat(diagnostics): Warning and Position, with per-row messages (NFR-10)"
```

---

### Task 2: `Warnings` — NFR-11 order and NFR-20 grouping

**Files:**
- Modify: `src/diagnostics.rs` (append)
- Test: `tests/diagnostics.rs` (append)

**Interfaces:**
- Consumes: `Warning`, `WarningKind`, `Position` (Task 1), and `radargen::normalise`.
- Produces: `#[derive(Debug, Default)] pub struct Warnings`, with:
  - `pub fn push(&mut self, w: Warning)`
  - `pub fn as_slice(&self) -> &[Warning]` and `pub fn is_empty(&self) -> bool`, so PR 05 and PR 06's stage tests assert on kinds and positions, not wording
  - `pub fn render(&self) -> Vec<String>`, one string per stderr line, each prefixed `warning: `

  Stages that can warn take `&mut Warnings` (§7). `main.rs` (PR 13) prints `render()` to stderr.

- [x] **Step 1: Write the failing tests**

Append to `tests/diagnostics.rs`:

```rust
use radargen::diagnostics::Warnings;

fn ring(line: usize, value: &str) -> Warning {
    at(line, 2, WarningKind::UndeclaredRing { value: value.into() })
}

/// SC-24, exactly.
#[test]
fn forty_rows_and_a_typo_make_one_ring_line_and_a_duplicate_keeps_its_own() {
    let mut w = Warnings::default();
    for line in 4..=44 {
        w.push(ring(line, if line == 37 { "Adpot" } else { "In Use" }));
    }
    w.push(at(50, 0, WarningKind::DuplicateName { value: "Kafka".into(), first_line: 9 }));
    assert_eq!(
        w.render(),
        vec![
            "warning: 41 rows use a ring --rings does not declare — In Use (40, first line 4), Adpot (1, line 37); all appear in the table view",
            "warning: line 50, name: name 'Kafka' repeats line 9; both are drawn",
        ]
    );
}

#[test]
fn a_class_with_one_occurrence_prints_the_per_row_message() {
    let mut w = Warnings::default();
    w.push(ring(37, "Adpot"));
    assert_eq!(w.render(), vec![
        "warning: line 37, ring: ring 'Adpot' is not declared by --rings; the row is in the table view, not on the radar",
    ]);
}

#[test]
fn values_group_under_fr_76_normalisation() {
    let mut w = Warnings::default();
    w.push(ring(4, "In Use"));
    w.push(ring(5, " in use "));
    w.push(ring(6, "IN USE"));
    assert_eq!(w.render(), vec![
        "warning: 3 rows use a ring --rings does not declare — In Use (3, first line 4); all appear in the table view",
    ]);
}

#[test]
fn render_level_warnings_come_first_and_never_group() {
    let mut w = Warnings::default();
    w.push(ring(2, "X"));
    w.push(render_level(WarningKind::QuadrantCount { count: 9 }));
    w.push(render_level(WarningKind::OverfullBand { quadrant: "A".into(), ring: "Adopt".into(), count: 12, fits: 10 }));
    w.push(render_level(WarningKind::OverfullBand { quadrant: "B".into(), ring: "Adopt".into(), count: 11, fits: 10 }));
    assert_eq!(w.render(), vec![
        "warning: 9 quadrants; the radar reads best with three to six, so sectors narrow and labels may crowd",
        "warning: quadrant 'A', ring 'Adopt': 12 items; about 10 fit comfortably, so some markers overlap",
        "warning: quadrant 'B', ring 'Adopt': 11 items; about 10 fit comfortably, so some markers overlap",
        "warning: line 2, ring: ring 'X' is not declared by --rings; the row is in the table view, not on the radar",
    ]);
}

#[test]
fn classes_order_by_first_line_then_column() {
    let mut w = Warnings::default();
    w.push(ring(9, "X"));
    w.push(at(5, 5, WarningKind::UnrecognisedStatus { value: "Gone".into() }));
    w.push(at(5, 4, WarningKind::UnrecognisedQualifier { value: "maybe".into() }));
    let out = w.render();
    assert!(out[0].contains("line 5, qualifier"), "{out:#?}");
    assert!(out[1].contains("line 5, status"), "{out:#?}");
    assert!(out[2].contains("line 9, ring"), "{out:#?}");
}

#[test]
fn out_of_order_pushes_are_sorted_by_position() {
    let mut w = Warnings::default();
    w.push(ring(40, "X"));
    w.push(at(3, 0, WarningKind::DuplicateName { value: "Kafka".into(), first_line: 2 }));
    let out = w.render();
    assert!(out[0].contains("line 3, name"), "{out:#?}");
}

#[test]
fn grouped_values_are_never_capped() {
    let mut w = Warnings::default();
    for i in 0..30 {
        w.push(ring(i + 2, &format!("Ring{i:02}")));
    }
    let out = w.render();
    assert_eq!(out.len(), 1);
    for i in 0..30 {
        assert!(out[0].contains(&format!("Ring{i:02} (1, line {})", i + 2)), "missing Ring{i:02}");
    }
}

#[test]
fn every_row_class_has_a_grouped_line() {
    let two = |kind: WarningKind| {
        let mut w = Warnings::default();
        w.push(at(4, 1, kind.clone()));
        w.push(at(6, 1, kind));
        w.render()
    };
    assert_eq!(two(WarningKind::UnrecognisedQualifier { value: "maybe".into() }), vec![
        "warning: 2 rows have an unrecognised qualifier — maybe (2, first line 4); each is drawn solid",
    ]);
    assert_eq!(two(WarningKind::UnrecognisedStatus { value: "Gone".into() }), vec![
        "warning: 2 rows have an unrecognised status — Gone (2, first line 4); each is drawn as No Change",
    ]);
    assert_eq!(two(WarningKind::UnparsableDate { column: DateColumn::ExitDate, value: "soon".into() }), vec![
        "warning: 2 rows have exit_date values that are not YYYY-MM-DD dates — soon (2, first line 4); each is shown as written",
    ]);
    assert_eq!(two(WarningKind::EmptyDescription), vec![
        "warning: 2 rows have an empty description — lines 4, 6; the detail panel omits each",
    ]);
    assert_eq!(two(WarningKind::DuplicateName { value: "Kafka".into(), first_line: 2 }), vec![
        "warning: 2 rows repeat an earlier name — Kafka (2, first line 4; original line 2); every row is drawn",
    ]);
    // Each member names its own column.
    let mut mixed = Warnings::default();
    mixed.push(at(4, 3, WarningKind::Undisplayable { column: "description".into(), first: '\u{1}', count: 1 }));
    mixed.push(at(6, 0, WarningKind::Undisplayable { column: "name".into(), first: '\u{7f}', count: 2 }));
    assert_eq!(mixed.render(), vec![
        "warning: 2 fields held characters the output cannot display — line 4 description, line 6 name; each was removed",
    ]);
}

#[test]
fn the_two_date_columns_are_separate_classes() {
    let mut w = Warnings::default();
    w.push(at(4, 5, WarningKind::UnparsableDate { column: DateColumn::LastAffirmed, value: "x".into() }));
    w.push(at(4, 6, WarningKind::UnparsableDate { column: DateColumn::ExitDate, value: "y".into() }));
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
    assert_eq!(out.len(), 1, "{out:#?}");
    assert!(!out[0].contains('\n'), "{:?}", out[0]);
    assert!(out[0].contains(r"In\nUse (2, first line 4)"), "{}", out[0]);
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
    assert_eq!(w.render(), vec![
        "warning: 3 rows use a ring --rings does not declare — In Use (2, first line 4), Adpot (1, line 37); all appear in the table view",
    ]);
}
```

- [x] **Step 2: Run them to see them fail**

Run: `cargo test --test diagnostics`
Expected: a compile error, `unresolved import radargen::diagnostics::Warnings`.

- [x] **Step 3: Implement**

Append to `src/diagnostics.rs`, and add `use crate::normalise;` at the top:

```rust
/// Warnings collected over a run (NFR-16 tier three).
///
/// A plain list, grouped only when printed: NFR-20's grouping needs the whole
/// run, and grouping at push time would need a map keyed by value, which §6
/// rules out of output-affecting code (architecture §7).
#[derive(Debug, Default)]
pub struct Warnings(Vec<Warning>);

/// NFR-20's warning classes. Render-level warnings have none: FR-88 fires
/// once and FR-89 promises one warning per band, so neither groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Ring,
    Qualifier,
    Status,
    Date(DateColumn),
    Description,
    DuplicateName,
    Undisplayable,
}

impl WarningKind {
    fn class(&self) -> Option<Class> {
        match self {
            Self::UndeclaredRing { .. } => Some(Class::Ring),
            Self::UnrecognisedQualifier { .. } => Some(Class::Qualifier),
            Self::UnrecognisedStatus { .. } => Some(Class::Status),
            Self::UnparsableDate { column, .. } => Some(Class::Date(*column)),
            Self::EmptyDescription => Some(Class::Description),
            Self::DuplicateName { .. } => Some(Class::DuplicateName),
            Self::Undisplayable { .. } => Some(Class::Undisplayable),
            Self::QuadrantCount { .. } | Self::OverfullBand { .. } => None,
        }
    }

    /// The value NFR-20 enumerates; `None` for a class with no value.
    fn value(&self) -> Option<&str> {
        match self {
            Self::UndeclaredRing { value }
            | Self::UnrecognisedQualifier { value }
            | Self::UnrecognisedStatus { value }
            | Self::UnparsableDate { value, .. }
            | Self::DuplicateName { value, .. } => Some(value),
            Self::EmptyDescription
            | Self::Undisplayable { .. }
            | Self::QuadrantCount { .. }
            | Self::OverfullBand { .. } => None,
        }
    }
}

impl Warnings {
    pub fn push(&mut self, w: Warning) {
        self.0.push(w);
    }

    /// The raw warnings in push order, so a stage's tests can assert on
    /// kinds and positions rather than on rendered English.
    #[must_use]
    pub fn as_slice(&self) -> &[Warning] {
        &self.0
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// One string per stderr line, in NFR-11's order, grouped per NFR-20.
    #[must_use]
    pub fn render(&self) -> Vec<String> {
        // Stable: equal positions keep push order.
        let mut sorted: Vec<&Warning> = self.0.iter().collect();
        sorted.sort_by_key(|w| w.position);

        let mut lines = Vec::new();
        let mut printed: Vec<Class> = Vec::new();
        for w in &sorted {
            let Some(class) = w.kind.class() else {
                lines.push(format!("warning: {w}"));
                continue;
            };
            if printed.contains(&class) {
                continue;
            }
            printed.push(class);
            let members: Vec<&Warning> = sorted
                .iter()
                .copied()
                .filter(|m| m.kind.class() == Some(class))
                .collect();
            if members.len() == 1 {
                lines.push(format!("warning: {w}"));
            } else {
                lines.push(format!("warning: {}", escape_controls(&grouped(class, &members))));
            }
        }
        lines
    }
}

/// A distinct value within a class, in first-occurrence order.
struct Tally {
    key: String,
    shown: String,
    count: usize,
    first_line: usize,
    /// FR-28's earlier line, for a duplicate name; `None` for every other class.
    original: Option<usize>,
}

fn line_of(w: &Warning) -> usize {
    match w.position {
        Position::Cell { line, .. } => line,
        Position::Render => 0, // unreachable for a classed warning; 0 is never a CSV line
    }
}

fn grouped(class: Class, members: &[&Warning]) -> String {
    let n = members.len();
    let list = match class {
        // No value to enumerate, so NFR-20 lists positions, uncapped.
        Class::Description => {
            let lines: Vec<String> = members.iter().map(|m| line_of(m).to_string()).collect();
            format!("lines {}", lines.join(", "))
        }
        Class::Undisplayable => members
            .iter()
            .map(|m| format!("line {} {}", line_of(m), m.kind.column()))
            .collect::<Vec<_>>()
            .join(", "),
        _ => tallied(members),
    };
    match class {
        Class::Ring => format!("{n} rows use a ring --rings does not declare — {list}; all appear in the table view"),
        Class::Qualifier => format!("{n} rows have an unrecognised qualifier — {list}; each is drawn solid"),
        Class::Status => format!("{n} rows have an unrecognised status — {list}; each is drawn as No Change"),
        Class::Date(column) => format!(
            "{n} rows have {column} values that are not YYYY-MM-DD dates — {list}; each is shown as written"
        ),
        Class::Description => format!("{n} rows have an empty description — {list}; the detail panel omits each"),
        Class::DuplicateName => format!("{n} rows repeat an earlier name — {list}; every row is drawn"),
        Class::Undisplayable => {
            format!("{n} fields held characters the output cannot display — {list}; each was removed")
        }
    }
}

/// NFR-20's enumeration: each distinct value, in first-occurrence order, with
/// its count and first line.
fn tallied(members: &[&Warning]) -> String {
    let mut tallies: Vec<Tally> = Vec::new();
    for m in members {
        let value = m.kind.value().unwrap_or_default();
        let key = normalise(value);
        if let Some(t) = tallies.iter_mut().find(|t| t.key == key) {
            t.count += 1;
        } else {
            let original = match m.kind {
                WarningKind::DuplicateName { first_line, .. } => Some(first_line),
                _ => None,
            };
            tallies.push(Tally { key, shown: value.trim().to_string(), count: 1, first_line: line_of(m), original });
        }
    }
    tallies
        .iter()
        .map(|t| {
            let at = match t.count {
                1 => format!("1, line {}", t.first_line),
                c => format!("{c}, first line {}", t.first_line),
            };
            match t.original {
                Some(o) => format!("{} ({at}; original line {o})", t.shown),
                None => format!("{} ({at})", t.shown),
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}
```

`Position::Render => 0` is the one place the types allow a case the logic excludes: a classed warning always has a cell.

- [x] **Step 4: Run the tests**

Run: `cargo test --test diagnostics && cargo clippy --all-targets -- -D warnings`
Expected: 16 passed, clippy clean.

- [x] **Step 5: Commit**

```bash
git add src/diagnostics.rs tests/diagnostics.rs
git commit -m "feat(diagnostics): Warnings — NFR-11 order and NFR-20 grouping"
```

---

### Task 3: `Fatal` — one error taxonomy

**Files:**
- Modify: `src/diagnostics.rs` (append `Fatal`)
- Modify: `src/rings.rs` (derive `thiserror::Error` on `RingsError`)
- Modify: `src/cli.rs` (delete `FatalError`; `format()` returns `Result<OutputFormat, Fatal>`)
- Modify: `tests/cli.rs` (import `radargen::diagnostics::Fatal` instead of `FatalError`)
- Test: `tests/diagnostics.rs` (append)

**Interfaces:**
- Consumes: `rings::RingsError` (exists).
- Produces: `#[derive(Debug, thiserror::Error)] pub enum Fatal` (no `PartialEq`: PR 13's write failures carry `std::io::Error`), with the variants `UnrecognisedExtension { path: String }`, `FormatNotInThisVersion { ext: String, arrives_in: &'static str }` and `Rings(#[from] RingsError)`, plus `impl Fatal { pub fn render(&self) -> String }`, which gives `error: …`.

  PR 05 adds decode, parse and quoting variants; PR 06 adds FR-5/FR-6; PR 13 adds the write failures. Each PR adds its own variants next to the code that raises them. Tier two returns `Vec<Fatal>` (§7), and so does a single flag with several violations (NFR-16 tier one, `rg-tiu`), so `main.rs` renders a slice, not one `Fatal`. `render` escapes the whole message, so no variant has to remember to: argv and CSV text alike can't forge a stderr line.

- [x] **Step 1: Write the failing tests**

Append to `tests/diagnostics.rs`:

```rust
use radargen::diagnostics::Fatal;
use radargen::rings::RingsError;

#[test]
fn fatal_messages() {
    let cases = [
        (Fatal::UnrecognisedExtension { path: "radar.pdf".into() },
         "error: -o radar.pdf: the output file name must end in .html"),
        (Fatal::UnrecognisedExtension { path: "radar".into() },
         "error: -o radar: the output file name must end in .html"),
        (Fatal::FormatNotInThisVersion { ext: "png".into(), arrives_in: "1.1" },
         "error: -o: .png output arrives in radargen 1.1; this version writes .html"),
        (Fatal::from(RingsError::EmptyEntry { index: 2 }),
         "error: --rings: entry 2 is empty"),
        (Fatal::from(RingsError::Duplicate { entry: "adopt".into(), first_index: 1 }),
         "error: --rings: 'adopt' repeats entry 1"),
        (Fatal::from(RingsError::Empty),
         "error: --rings: no rings declared"),
        (Fatal::from(RingsError::Duplicate { entry: "a\nerror: forged".into(), first_index: 1 }),
         r"error: --rings: 'a\nerror: forged' repeats entry 1"),
    ];
    for (fatal, expected) in cases {
        assert_eq!(fatal.render(), expected);
    }
}
```

In `tests/cli.rs`, change line 5 to `use radargen::cli::{Cli, OutputFormat, Theme};`, add `use radargen::diagnostics::Fatal;`, and replace both `FatalError::` occurrences with `Fatal::`.

- [x] **Step 2: Run them to see them fail**

Run: `cargo test --test diagnostics --test cli`
Expected: a compile error, `unresolved import radargen::diagnostics::Fatal`.

- [x] **Step 3: Implement**

In `src/rings.rs`, change the derive and add the messages. FR-63 says "naming the offending entry":

```rust
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RingsError {
    /// An empty entry would draw a nameless band. `index` is 1-based.
    #[error("entry {index} is empty")]
    EmptyEntry { index: usize },
    /// A duplicate would make FR-16's match ambiguous and FR-32's band order
    /// undefined. `entry` is the offending text as written, trimmed.
    #[error("'{entry}' repeats entry {first_index}")]
    Duplicate { entry: String, first_index: usize },
    /// The list yielded no rings at all.
    #[error("no rings declared")]
    Empty,
}
```

Append to `src/diagnostics.rs`:

```rust
use crate::rings::RingsError;

/// Everything the tool refuses to run on: non-zero exit, nothing written
/// (NFR-8, NFR-9). One taxonomy for the whole tool (architecture §1); each
/// later stage adds its variants beside the code that raises them.
#[derive(Debug, thiserror::Error)]
pub enum Fatal {
    /// FR-52: the `-o` extension is absent or names a format we do not write.
    #[error("-o {path}: the output file name must end in .html")]
    UnrecognisedExtension { path: String },
    /// §14.1: the extension is one v1.1 adds, so say that rather than
    /// "unrecognised", which sends the caller hunting for a typo in a path
    /// that is correct.
    #[error("-o: .{ext} output arrives in radargen {arrives_in}; this version writes .html")]
    FormatNotInThisVersion { ext: String, arrives_in: &'static str },
    /// FR-63, NFR-16 tier one.
    #[error("--rings: {0}")]
    Rings(#[from] RingsError),
}

impl Fatal {
    /// The stderr line.
    #[must_use]
    pub fn render(&self) -> String {
        // Escaped as a whole, so no variant can forget to: `-o` and `--rings`
        // values come from argv, and later variants carry CSV text (NFR-10).
        format!("error: {}", escape_controls(&self.to_string()))
    }
}
```

In `src/cli.rs`:
- Delete the `FatalError` enum and its doc comment.
- Add `use crate::diagnostics::Fatal;`.
- Change `format`'s signature to `pub fn format(&self) -> Result<OutputFormat, Fatal>`, and rename `FatalError::` to `Fatal::` in its body and in its `# Errors` doc links.

- [x] **Step 4: Run everything**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: all pass, clippy clean.

- [x] **Step 5: Commit**

```bash
git add src/diagnostics.rs src/rings.rs src/cli.rs tests/diagnostics.rs tests/cli.rs
git commit -m "feat(diagnostics): Fatal — one error taxonomy; RingsError and -o errors join it"
```

---

## Finishing the PR

1. Run `crumbs start rg-xr3` before Task 1. Work on branch `feat/pr-02-diagnostics`, created from `main`.
2. After Task 3: push, then open the PR with `gh pr create` and **no reviewer flag**.
3. Run the code-reviewer skill until clean, then do the Copilot loop until it reports "no new comments" on the head SHA. Then run simplify, then code-reviewer once more.
4. Close `rg-xr3`, changing the status field only. **No version bump:** the binary still exits 1 without printing anything. Once this is merged, `rg-tiu` (FR-63 collect-all) is unblocked.
5. Move this file to `docs/plans/done/` in the same PR. Merge; there's no tag.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-09 | Initial version |
| 1.1 | 2026-10-09 | Five-agent review (rust-engineer, qa-expert, architect-reviewer, ui-designer, ux-researcher) folded in |
| 1.2 | 2026-10-09 | FR-90's warning class (`XmlForbidden`) added for PRD v0.38, resolving crumb `rg-jyx` |
| 1.3 | 2026-10-09 | PR 6 review: `Fatal::render` escapes the whole message; `Warnings::as_slice`/`is_empty`; `XmlForbidden` renamed `Undisplayable`, its column escaped and worded 'the output cannot display'; stronger grouping tests; every stderr line escaped whole, and `grouped` is one exhaustive `match` |
| 1.4 | 2026-10-09 | The pedantic and unsafe-code gate is now live in `Cargo.toml` (crumb `rg-q3w`); `format`'s `# Errors` doc links are renamed along with its body |
| 1.5 | 2026-10-10 | Implemented. Deviations: rustfmt pushed `each_kind_has_its_per_row_message` past pedantic's 100-line limit, so it is split into `each_row_kind_has_its_per_row_message` and `each_render_level_kind_has_its_message` (17 tests after Task 2, not 16). Architecture §7's previous paragraph also said NFR-11's order came from insertion order; it now says a stable sort by position. Task 3's commit subject was shortened to fit 72 characters |
| 1.6 | 2026-10-10 | PR 11 review. `Warning`'s fields are private behind `Warning::cell` and `Warning::render_level`, which keep the position fitting the kind, with `position()` and `kind()` for stage tests. The decisions table above is as planned; the shipped wording differs: per-row messages no longer repeat the column (`line 37, ring: 'Adpot' is not declared …`, `line 9, description: empty; …`, `line 12, name: 'Terraform' repeats line 4; …`); the qualifier message adds `; drawn solid`; FR-88 says `3 to 6`; the grouped duplicate entry leads with the original line (`Kafka (line 2; repeated on 2 rows, first line 4)`); the grouped FR-90 line names each field and code point (`description on line 4 (U+0001)`) and says `had`; the empty-description line ends `each detail panel omits it`; both `-o` errors name the path; `--rings` duplicates name their own entry (`entry 3 'adopt' repeats entry 1`). U+061C, U+2028 and U+2029 are escaped too. Date values group by trimmed text, not case, since FR-76 does not fold parsed values |
| 1.7 | 2026-10-10 | Simplify pass. `WarningKind` splits into `RowKind` and `RenderKind`, and `Warning` becomes `enum Warning { Render(RenderKind), Row { line, column, kind: RowKind } }`, so a row warning always has a line by type; the constructors, their asserts, the `unreachable!` and the empty-column arm go. `Warning::position()` remains the sort key. `render` groups in one pass; `grouped` is one `match`; `tallied` takes the key function (`normalise`, or trim for dates) and keeps a reference to each value's first occurrence instead of copying its fields |
