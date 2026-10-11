---
tags:
  - plan
  - model
  - csv
aliases: []
doc_id: RG-006
doc_name: pr-06-model
crumb_id: rg-rnc
document_title: "PR 06 — `model.rs`: Item, Catalogue, column validation: Implementation Plan"
created_date: 2026-10-10
synopsis: "Records to meaning: FR-76's trim widened to invisible format characters, the ring set's source carried into FR-17's warning, and model.rs turning a parsed Table into a Catalogue — required columns, empty values, quadrants, ring matching, optional columns, unnamed columns and accessible names."
status: draft
type: Plan
revision: "1.1"
review_date: 2026-10-10
reviewed_by:
  - architect-reviewer
  - rust-engineer
  - qa-expert
  - accessibility-tester
completed_date:
comments:
---
# PR 06 — `model.rs`: Item, Catalogue, column validation: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-rnc` · **Phase:** v1.0 · **Blocks:** `rg-efx` (PR 07), `rg-48z` (PR 08), `rg-f27` (PR 14)

**Goal:** Turn PR 05's `Table` into a `Catalogue`: what each column means, what each row claims, and every structural error in one run. A real export with a stray byte-order mark, a zero-width space, padded columns or an undeclared ring either builds or fails with messages that say what to do.

**Architecture:** Three layers, built in dependency order (architecture §2, §7).
- `lib::trim` is FR-76's trim, widened in PRD v0.43 to strip leading and trailing Unicode format characters except the tag characters (D-61). `normalise` and every display trim go through it.
- `rings::RingSet` carries the declared rings **and their source**, so FR-17's warning names `--rings` or the default ring set (D-61).
- `model::build_catalogue(&Table, &RingSet, &mut Warnings) -> Result<Catalogue, Vec<Fatal>>` maps the header once and checks FR-5 and FR-6 across the whole file (NFR-16 tier two). It then builds one `Item` per row: quadrant grouping (FR-9), ring matching (FR-16, FR-17), duplicate names (FR-28), the optional columns (FR-20–FR-26) and unnamed columns (FR-93). A `Catalogue` keeps every named column's cell in header order for the table view (FR-78), and `accessible_name` serves FR-50 and FR-75.

Ordering (FR-10, FR-34) and numbering (FR-18) are PR 07's. A `Catalogue` keeps quadrants and items in CSV order.

**Tech Stack:** Rust 2024. One new direct dependency, `unicode-properties` 0.1.4 with only its `general-category` feature. It is already in the tree through `rustybuzz`, so `Cargo.lock` gains one dependency edge and no new crate.

**Spec:** `docs/product/radargen-prd.md` v0.43 — FR-5–FR-29, FR-50, FR-75, FR-76, FR-78, FR-93, NFR-9, NFR-10, NFR-11, NFR-16, NFR-20, D-12, D-29, D-35, D-59, D-61, SC-10, SC-19. Design: `docs/design/architecture.md` §2 (the `model` stage and the `Catalogue` side-channel), §3 (the `model.rs` row), §6 (pre-computed normalised keys), §7 (diagnostics threading), §10 (`row-errors.csv`).

## Global Constraints

- **One trim, one fold** (FR-76, D-61). Every comparison goes through `normalise`, and every display trim goes through `trim`. Never call `str::trim` on CSV or `--rings` text, and never restate the trimmable set.
- **Displayed text is never edited** beyond FR-90's removal and the two label trims, FR-9's quadrant label and FR-15's ring name. An `Item` keeps `name` and `cells` exactly as `parse` left them.
- **NFR-16 tier two:** `build_catalogue` returns **every** FR-5 and FR-6 error together, in line order and then **header** column order. It checks every row before building any, so an `Err` comes with no warning from this stage.
- **Two index spaces, never mixed.** Warning positions are **header** indices (`Position::Cell { column }`, 0-based), the space `parse` uses for FR-90, which keeps NFR-11's within-line order total across both stages. `Item::cells`, `Item::cell` and `Catalogue::column` index the **named** columns (`Catalogue::columns`), which skip FR-93's unnamed ones.
- **No `HashMap` iteration** in output-affecting code (architecture §6). Look-ups here are `Vec` scans in first-occurrence order; FR-28's is O(rows²), which is fine at A-2's tens of rows and took 1.4 s for 20,000 rows in a debug build.
- **Normalised keys are computed once**, when the `Item` or `Quadrant` is built (architecture §6). PR 07's comparators read `name_key` and `Quadrant::key`; they never fold.
- **One taxonomy:** new errors and warnings are `diagnostics` variants (architecture §1). Every new message is pinned in `tests/diagnostics.rs`.
- **Messages say what to do** and say "column", never "field" (PR 05's rule).
- `cargo fmt` before every commit; CI runs `cargo fmt --check`. The snippets are in rustfmt's layout, but run it anyway.
- `cargo clippy --all-targets -- -D warnings` and `just lint` (pedantic + nursery + `unwrap_used`) are clean **at every commit**. No `#[allow]`/`#[expect]` without an inline reason.
- `just ci` passes before the PR opens. It is GitHub CI's gates plus `cargo deny check licenses` and rustqual against `rustqual-baseline.json`. rustqual fails on a lower quality score, so keep integration functions free of logic (IOSP) and name numeric constants.
- Commits: `type(scope): description` (subject ≤ 72 characters, body lines ≤ 72), plus the session's `Co-Authored-By` trailer. Run `git-mit es` first.

## Review Focus

1. **A header cell `name` carrying an invisible character** — a second byte-order mark from concatenated exports, or a zero-width space pasted from a web page. FR-5 must find `name`, not report it missing from a header that visibly has it (`rg-lme`). *Tests: Task 1, `trims_a_stray_bom_and_zero_width_space`; Task 4, `required_columns_match_under_fr76`.*
2. **An undeclared ring when the author never passed `--rings`.** The warning must say `the default ring set`, not send them to a flag they never used (`rg-dy8`). *Tests: Task 2, `the_ring_warning_names_the_default_ring_set_when_no_flag_declared_one`; Task 4, `an_undeclared_ring_is_unplotted_and_names_the_ring_source`; Task 6, `the_prd_example_builds_with_one_warning`.*
3. **A 200-row export with several kinds of error.** FR-4's errors from `parse` and FR-5's and FR-6's from the model must all arrive in one run, and a cleared `,,,` row must not shift a line number (`rg-rnc`). *Tests: Task 4, `every_empty_required_value_is_reported`, `missing_columns_and_empty_values_arrive_together` and `a_cleared_row_does_not_shift_line_numbers`; Task 6, `row_errors_from_both_stages_arrive_in_one_run`.*
4. **Excel's padded used range, and a value typed under no heading.** Padding must pass silently. A real value must warn, naming its column by position, and so must FR-90's removal in that column (`rg-rnc`). *Tests: Task 5, `unnamed_columns_warn_only_when_they_hold_a_value` and `a_dropped_column_still_gets_fr90s_warning`.*
5. **Dates that look right but are not:** `2026-6-1`, `2026-04-31`, `2023-02-29`, full-width digits, or `202é06-01` (ten bytes, a multi-byte character across a boundary). Each must warn and render as written, and none may panic. *Tests: Task 5, `iso_dates_are_exactly_the_shape_and_real` and `unparsable_dates_warn_and_keep_their_text`.*
6. **Subdivision-flag emoji in a name or quadrant.** England's and Scotland's flags must stay distinct after trimming, and every tag character must survive, not only CANCEL TAG (D-61). *Test: Task 1, `keeps_the_tag_characters_that_end_a_flag`.*
7. **An invisible control character in a name** (SC-10). `Kafka` + U+0001 repeats `Kafka`, and a `name` of only U+0001 is the empty-`name` error. *Tests: Task 4, `invisible_characters_do_not_hide_a_repeated_name` and `a_name_of_only_undisplayable_characters_is_empty`.*
8. **A header whose columns are in an unusual order, or with an unnamed column in the middle.** Errors and warnings on one line must follow the header's column order across both stages (NFR-11), and cell look-ups must not shift. *Tests: Task 4, `empty_required_values_follow_header_column_order`; Task 5, `warnings_from_both_stages_sort_by_header_column` and `cells_index_named_columns_not_header_positions`; Task 6, `an_unplotted_ring_is_found_past_an_unnamed_column`.*

## Crumbs this plan incorporates

| Crumb | What it asked | Where |
| --- | --- | --- |
| `rg-rnc` | FR-5–FR-29; FR-27/FR-78 header order and unrecognised cells; `accessible_name()`; `Result<_, Vec<Fatal>>`; normalised keys once | Tasks 3–6 |
| `rg-rnc` (PR 05 note) | `build_catalogue` takes `&Table`; FR-4's rows are absent but their errors travel beside; a `,,,` row must not shift FR-6's line | Task 4, `a_cleared_row_does_not_shift_line_numbers`; Task 6, `row_errors_from_both_stages_arrive_in_one_run` |
| `rg-rnc` (v0.43) | FR-93 drop and warning; FR-76 Cf trim; ring source in FR-17's warning | Tasks 1, 2, 4, 5 |
| `rg-rnc` (v0.43 review) | Template `N rows use a ring not declared by <source>`; FR-93's grouped form counts values; tag-character exemption; FR-90 still warns in a dropped column | Tasks 1, 2, 3, 5 |
| `rg-rnc` (efficiency) | `unicode-properties` for Cf; one borrowing trim used at every display site; `RingSource` a small `Copy` enum; header names normalised once | Tasks 1, 2 (Decisions) |
| `rg-rnc` (ownership) | Does FR-93 live in `csv::parse` or `model.rs`? | `model.rs` (Decisions) |
| `rg-lme` *(closed by PRD v0.43)* | Invisible format characters in header names | Task 1 |
| `rg-dy8` *(closed by PRD v0.43)* | The ring warning names its source | Task 2 |
| `rg-dsa` | FR-24: does `' 2026-06-01'` display with its space? | Task 5 keeps the cell as written and pins it; settled by FR-76 (Finishing step 4 notes it on the crumb) |
| `rg-u6u` (PR 19) | `--config` | Not here. `RingSource` gains its config variant then (Finishing step 4) |
| `rg-efx` (PR 07) | Comparators | Not here. It reads `Item::name_key`, `Quadrant::key` and `Item::line` (Finishing step 4) |
| `rg-akv`, `rg-s71` | FR-90 on flag values; one provenance rule | Out of scope: both concern flags, not the CSV model |
| `rg-6ny`, `rg-owp`, `rg-bvl`, `rg-fju`, `rg-85c`, `rg-40l`, `rg-q2b` *(new, from this plan's review)* | Table rows' role; one phrase for unplotted rows; the default-ring message; column letters; 'field' in old messages; FR-9 vs FR-76 'raw'; visible Cf marks | Each needs a PRD change or belongs to a later PR, so each is a crumb |

## Decisions taken in this plan (flag on review)

- **FR-76's trim is `lib::trim`, and every trim goes through it.** It removes any leading or trailing run of `char::is_whitespace` (the White_Space property `str::trim` uses) and general category Cf, except the tag characters U+E0001 and U+E0020–U+E007F. `normalise` is `trim` then `to_lowercase`. The display sites switch from `str::trim` to it too: `parse`'s duplicate-header names, the values diagnostics quote, the FR-89 labels, and FR-15's ring entries. Otherwise a ring declared as `Adopt` plus U+200B would match `Adopt` and still draw the zero-width space in its band label.
- **`unicode-properties` supplies the category**, not a hand-copied table. It is already compiled into the binary through `rustybuzz`, it tracks Unicode 17, and D-61 accepts that Cf membership follows the pinned crate's Unicode version. The `general-category` feature alone is enabled.
- **The ring set carries its source.** `parse_rings` returns `RingSet { rings, source }` with `RingSource::{Flag, Default}`. `RingSource` is `Copy` and rides in `RowKind::UndeclaredRing`, so a 40-row warning clones no string. The grouping class becomes `Class::Ring(RingSource)`, so the grouped line names the source without a fallback; one run has one ring set, so the source never splits a class. `--config`'s variant arrives with PR 19. Naming a file stops `RingSource`, and with it `Class`, being `Copy`, and `RowKind::class` stops being `const fn`; holding the path as `Arc<str>` keeps every clone O(1) (noted on `rg-u6u`).
- **`build_catalogue` takes the ring set:** `build_catalogue(&Table, &RingSet, &mut Warnings)`. Matching a row's ring (FR-16, FR-17) and grouping its quadrant (FR-9) are per-row column meaning, and FR-17's warning needs the source. Ordering (FR-10, FR-34) and numbering (FR-18) stay with PR 07, which shrinks the pipeline's `resolve` stage to "order, number". `Config` holds the `RingSet`, and only `build_catalogue` reads it: everything downstream reads `Catalogue::rings`, which `Item::ring` indexes. Architecture §2 and §7 are updated to match (Task 6).
- **FR-93 lives in `model.rs`, not `csv::parse`** — `rg-rnc`'s open ownership question. Both stages name a warning's column by its **header index** (NFR-11's within-line order). Dropping the column in `parse` would renumber every later column, so FR-90's warnings (parse) and FR-17's (model) on one line could sort out of column order. The model already sees the header and every record's line. `parse` is unchanged, so FR-90 still warns for a cell in a dropped column, and FR-93 warns beside it.
- **A `Catalogue` keeps the CSV's shape** (architecture §2). `columns` is the named columns in header order, and each `Item::cells` holds exactly those cells, as written. Typed fields exist only where behaviour depends on them: `quadrant`, `ring`, `qualifier`, `status` and `name_key`. The table view (FR-78) and detail panel (FR-38) read `cells` through `Item::cell`, which returns `None` for an empty cell (FR-29), and find a column with `Catalogue::column(kind)`. Dates, `domain` and `rationale` therefore render verbatim by construction (FR-22–FR-24). For `status`, the panel shows `Status::label()`, not the cell: an unrecognised value is `No Change` (FR-26). Architecture §2 and §3 are rewritten to describe this shape (Task 6).
- **`Item::quadrant` indexes first-appearance order**, not FR-10's, and is never a palette index (architecture §5). PR 07 returns FR-10's order as a list of indices into `Catalogue::quadrants`, plus a rank, and does not reorder the `Catalogue` (noted on `rg-efx`).
- **A short record reads as empty, not as a panic.** `parse` keeps only records as long as the header, but `Table` and `Record` are public, so `field(record, p)` uses `.get` and FR-6 reports a missing cell instead of indexing past the end.
- **FR-6 still runs when FR-5 fails**, for the required columns that are present. NFR-16 asks for every row-level error in one run, and a missing `description` should not hide an empty `name`.
- **No warning from the model when there are errors.** It checks every row before building any, so warnings about rendering are never produced for a run that cannot render. This is a property of the stage, not a run-wide rule: `parse`'s FR-90 warnings are already pushed, and when `parse` returns FR-4 errors but the model succeeds, the model warns for the valid rows. What a failed run prints is `main.rs`'s rule (PR 13), as RG-005 recorded. `errors_push_no_model_warnings` pins both halves.
- **FR-9's label is the minimum of the trimmed variants by `str::cmp`**, computed as each row arrives, so it does not depend on row order (D-29). That is FR-76's code-point order (D-53), which is a plain `<` on `&str`; the site cites D-53, and architecture §8 lists it beside `order::compare_names` (it cannot call it: `order.rs` depends on `model.rs`). FR-76 elsewhere says the FR-9 tiebreak compares 'raw' values; the two readings differ for ` tool` against `Tool`. This plan follows FR-9's own text, and `rg-40l` asks the PRD to settle it.
- **FR-28 warns at every later occurrence, naming the first.** The grouped line then reads `kafka (line 2; repeated on 2 rows, first line 3)` (PR 02's format).
- **FR-24's check is hand-written, about 25 lines.** `chrono`, `jiff` and `time` were considered. FR-24 wants exactly `YYYY-MM-DD`: zero-padded, no week dates or times, and a real Gregorian date. A library would need a format description plus a separate padding check, or would accept forms FR-24 rejects. The check matches a `YYYY-MM-DD` template byte by byte and returns early if it fails, so what is split is only ASCII digits and hyphens and nothing is sliced. Rustqual's magic-number rule is why the month lengths and leap rule are named constants.
- **`accessible_name(item, number)` starts with the legend row's visible text** (WCAG 2.5.3). FR-74's row shows number, name, and whichever of qualifier and status are present, so the name is `"3. Kafka, Conditional, Moved In, Tool, Adopt"`: the row's text, then the quadrant's FR-9 label and the ring, which a screen reader reaching one row by Tab cannot get from the grouping (FR-75). Qualifier and status use the canonical labels the legend shows (`Qualifier::label`, `Status::label`); a blank or unrecognised qualifier claims nothing and shows nothing. An unplotted row has no number (FR-18) and ends `, not on the radar`. `number` must be `Some` exactly for plotted items (a `debug_assert`). This serves markers and legend rows. Table rows should keep native table semantics with a button in the name cell, which is a PRD change to FR-50 (`rg-6ny`).
- **`Qualifier` is an enum, not a `hollow` flag.** FR-74 needs the label, and FR-20's fill is `Item::hollow()`.
- **Fixtures start now.** `tests/fixtures/three-quadrant.csv` is PRD §6.1.6's example, which architecture §10 already lists under that name (SC-2). `tests/fixtures/row-errors.csv` is §10's tier-two fixture. PR 14 adds their golden output.
- **rustqual.** `build_catalogue` stays an integration function (calls only): the error check is `none_or_all`, and the row loop is `Builder::build`. `model.rs` reports `SRP_MODULE` at about 470 lines, as `diagnostics.rs` already does. Splitting it would separate the column checks from the builder that calls them, so it stays one file, as architecture §3 lists it. Its public functions show as test-only `DEAD_CODE` until `main.rs` calls them in PR 13, as `parse` does today. With all of this, the score rises from 59.0% to 66.8% in a trial run.
- **New messages.** SC-10's golden tests (PR 13 onward) will pin these, so this is the cheap moment to change them:

| Source | Message (after `error: ` or `warning: `) |
| --- | --- |
| `Fatal::MissingColumn` (FR-5) | `line {line}: the header has no '{column}' column; the header must include at least the columns 'name', 'quadrant', 'ring' and 'description'` |
| `Fatal::EmptyRequired` (FR-6) | `line {line}, {column}: empty; every row needs a name, a quadrant and a ring, so fill it in or delete the row` |
| `RowKind::UnnamedValue` (FR-93), one | `line {line}, column {n}: this column has no name in the header row, so its value is not shown; add a header name, or delete the value` |
| `RowKind::UnnamedValue`, grouped | `{n} values are in columns with no name in the header row — column 6 (2, first line 4), column 7 (1, line 9); none is shown, so add a header name to show them` |
| `RowKind::UndeclaredRing` (FR-17), one | `line {line}, ring: '{value}' is not declared by {--rings \| the default ring set}; the row is in the table view, not on the radar` |
| `RowKind::UndeclaredRing`, grouped | `{n} rows use a ring not declared by {source} — {values}; all appear in the table view` |

The FR-17 wording is the PRD's (D-61). A clearer default-set message that lists the rings is `rg-bvl`; column letters (`column 6 (F)`) are `rg-fju`.

- **No version bump.** The binary still exits 1 without reading a file.

---

### Task 1: FR-76's trim (`lib::trim`)

**Files:**
- Modify: `Cargo.toml` (add `unicode-properties`)
- Modify: `src/lib.rs` (add `trim`, rebuild `normalise` on it, add unit tests)
- Modify: `src/rings.rs` (FR-15 uses `trim`)
- Modify: `src/csv/parse.rs` (`duplicate_headers` uses `trim` and normalises once; two tests)
- Modify: `src/diagnostics.rs` (four display trims use `trim`)
- Test: `tests/diagnostics.rs` (one assertion)

**Interfaces:**
- Consumes: nothing new.
- Produces: `radargen::trim(&str) -> &str` (FR-76's trim). `radargen::normalise(&str) -> String` is unchanged in signature and is now `trim(value).to_lowercase()`.

- [ ] **Step 1: Add the dependency**

In `Cargo.toml`, under `[dependencies]`, after `ttf-parser = "0.25"`:

```toml
unicode-properties = { version = "0.1.4", default-features = false, features = ["general-category"] }
```

- [ ] **Step 2: Write the failing tests**

In `src/lib.rs`, below `normalise`, add the test module. (`trim` does not exist yet, so this fails to compile.)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// England's flag: U+1F3F4, the tags g b e n g, then CANCEL TAG.
    const ENGLAND: &str = "\u{1f3f4}\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}";
    /// Scotland's: the same black flag, tags g b s c t.
    const SCOTLAND: &str = "\u{1f3f4}\u{e0067}\u{e0062}\u{e0073}\u{e0063}\u{e0074}\u{e007f}";

    #[test]
    fn trims_a_stray_bom_and_zero_width_space() {
        assert_eq!(trim("\u{feff}name\u{200b}"), "name");
    }

    #[test]
    fn trims_a_mixed_run_of_spaces_and_format_characters() {
        assert_eq!(trim("\u{200b} \u{feff}name \u{200b}\t"), "name");
    }

    #[test]
    fn keeps_format_characters_inside_a_value() {
        // A zero-width joiner inside an emoji sequence is content (D-61).
        let family = "\u{1f469}\u{200d}\u{1f467}";
        assert_eq!(trim(family), family);
    }

    #[test]
    fn keeps_the_tag_characters_that_end_a_flag() {
        let england = format!("Team {ENGLAND}");
        let scotland = format!("Team {SCOTLAND}");
        assert_eq!(trim(&england), england);
        assert_ne!(normalise(&england), normalise(&scotland));
        // The whole tag range, not only CANCEL TAG (D-61).
        for tag in ['\u{e0001}', '\u{e0020}', '\u{e0067}', '\u{e007f}'] {
            let value = format!("x{tag}");
            assert_eq!(trim(&value), value);
        }
    }

    #[test]
    fn a_value_of_only_format_characters_is_empty() {
        assert_eq!(trim("\u{200b}\u{feff}\u{2060}"), "");
    }

    #[test]
    fn normalise_folds_after_trimming() {
        assert_eq!(normalise("\u{feff} Name\u{200b}"), "name");
    }
}
```

In `src/csv/parse.rs`'s test module, immediately above `empty_header_cells_are_not_duplicates`:

```rust
    /// FR-76 (D-61): a zero-width space or second byte-order mark pasted
    /// in with a name does not hide the name, so it collides, and the
    /// error shows the name without it.
    #[test]
    fn invisible_format_characters_do_not_hide_a_duplicate_header() {
        assert_eq!(
            errors("name,\u{200b}Name\u{feff},ring\n"),
            [repeats(1, 2, "Name", 1, "name")]
        );
    }

    /// A header cell of only format characters names nothing either.
    #[test]
    fn a_header_cell_of_only_format_characters_is_not_a_duplicate() {
        let errs = errors("name,\u{200b},\u{feff}\nK,a,b\n");
        assert!(errs.is_empty(), "{errs:?}");
    }
```

In `tests/diagnostics.rs`, extend `values_are_shown_trimmed` so it reads:

```rust
#[test]
fn values_are_shown_trimmed() {
    let w = ring(3, "  Adpot ");
    assert!(w.to_string().contains("ring: 'Adpot' is"), "{w}");
    // FR-76's trim, so an invisible format character is not quoted either.
    let w = ring(3, "\u{200b}Adpot\u{feff}");
    assert!(w.to_string().contains("ring: 'Adpot' is"), "{w}");
}
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -q`
Expected: compile error, `cannot find function 'trim'` in `src/lib.rs`.

- [ ] **Step 4: Implement**

In `src/lib.rs`, replace the whole `normalise` function and its doc comment with:

```rust
use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};

/// FR-76's trim: any leading or trailing run of whitespace and Unicode
/// format characters (Cf), in any mix.
///
/// A stray byte-order mark or zero-width space is invisible in an editor, so
/// it must not stop `name` matching `name`.
///
/// The tag characters are kept: they end a subdivision flag such as
/// England's, and trimming them would turn every such flag into the same bare
/// black flag (D-61). Format characters inside a value are content and kept.
///
/// Every display trim uses this too — FR-9's quadrant label, FR-15's ring
/// name, and the values diagnostics quote — so no site trims differently.
#[must_use]
pub fn trim(value: &str) -> &str {
    value.trim_matches(trimmable)
}

/// Whitespace, or a format character that is not a tag character.
fn trimmable(c: char) -> bool {
    c.is_whitespace()
        || (c.general_category() == GeneralCategory::Format
            && !matches!(c, '\u{e0001}' | '\u{e0020}'..='\u{e007f}'))
}

/// FR-76: **one normalisation rule**, applied to every value read from the CSV
/// or from `--rings`.
///
/// Both operands of a comparison get [`trim`] and are then folded
/// case-insensitively. Folding is Unicode's default lowercase mapping —
/// `str::to_lowercase`, not full case folding and not locale-aware (D-53).
///
/// This never edits displayed text — callers compare on the result and render
/// the original.
#[must_use]
pub fn normalise(value: &str) -> String {
    trim(value).to_lowercase()
}
```

In `src/rings.rs`:
- change `use crate::normalise;` to `use crate::{normalise, trim};`;
- in `parse_rings`'s doc comment, change "Entries are trimmed of leading/trailing whitespace only (FR-15), so a" to "Entries get FR-76's trim at their ends only (FR-15), so a";
- change `let entry = raw.trim();` to `let entry = trim(raw);`.

In `src/csv/parse.rs`, change `use crate::normalise;` to `use crate::{normalise, trim};`, then replace `duplicate_headers` with:

```rust
/// FR-4: every header name repeated under FR-76, naming the column it
/// repeats. An empty cell names nothing, so it cannot repeat a name; Excel
/// pads the header with these (Decisions).
fn duplicate_headers(header: &[String], line: usize) -> Vec<Fatal> {
    // Normalised once, not once per pair.
    let keys: Vec<String> = header.iter().map(|n| normalise(n)).collect();
    let mut errors = Vec::new();
    for (i, key) in keys.iter().enumerate() {
        if key.is_empty() {
            continue;
        }
        // The first earlier match is the name's first occurrence.
        if let Some(first) = keys[..i].iter().position(|k| k == key) {
            errors.push(Fatal::DuplicateHeader {
                line,
                index: i + 1,
                name: trim(&header[i]).to_string(),
                first_index: first + 1,
                first_name: trim(&header[first]).to_string(),
            });
        }
    }
    errors
}
```

In `src/diagnostics.rs`:
- change `use crate::normalise;` to `use crate::{normalise, trim};`;
- in `RowKind`'s doc comment, change "display trims them and escapes control characters" to "display trims them (FR-76) and escapes control characters";
- in `RowKind::message`, change `self.value().map(str::trim)` to `self.value().map(trim)`;
- in `RenderKind::message`, change `quadrant.trim()` and `ring.trim()` to `trim(quadrant)` and `trim(ring)`;
- in `grouped`'s `Class::Date` arm, change `|v| v.trim().to_string()` to `|v| trim(v).to_string()`;
- in `tallied`, change `first.kind.value().unwrap_or_default().trim()` to `trim(first.kind.value().unwrap_or_default())`.

- [ ] **Step 5: Run everything**

Run: `cargo fmt && cargo test -q && cargo clippy --all-targets -- -D warnings && just lint`
Expected: all pass; both lints clean.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/rings.rs src/csv/parse.rs src/diagnostics.rs tests/diagnostics.rs
git commit -m "feat(text): FR-76 trims invisible format characters (D-61)"
```

---

### Task 2: The ring set's source (`rings::RingSet`)

**Files:**
- Modify: `src/rings.rs` (`RingSource`, `RingSet`; `parse_rings` returns `RingSet`)
- Modify: `src/diagnostics.rs` (`UndeclaredRing` carries the source; `Class::Ring(RingSource)`)
- Test: `tests/rings.rs`, `tests/diagnostics.rs`

**Interfaces:**
- Consumes: `trim` (Task 1).
- Produces:
  - `radargen::rings::RingSource { Flag, Default }`: `Copy`, with `Display` giving `--rings` and `the default ring set`.
  - `radargen::rings::RingSet { pub rings: Vec<String>, pub source: RingSource }`.
  - `parse_rings(Option<&str>) -> Result<RingSet, Vec<RingsError>>`.
  - `RowKind::UndeclaredRing { value: String, source: RingSource }`.

- [ ] **Step 1: Write the failing tests**

In `tests/rings.rs`, change the import to `use radargen::rings::{RingSource, RingsError, parse_rings};`. In every test that compares a successful `parse_rings(…).expect(…)` with a list, append `.rings` after the `.expect(…)`. There are four: `omitting_the_flag_yields_the_in_house_default`, `entries_are_trimmed_but_keep_internal_spaces`, `a_single_ring_is_valid` and `the_declared_casing_is_what_gets_displayed`. Then add, above `entries_are_trimmed_but_keep_internal_spaces`:

```rust
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

/// FR-15 uses FR-76's trim, so a zero-width space pasted in with a ring
/// name neither shows in its band label nor stops it matching (D-61).
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
}
```

In `tests/diagnostics.rs`:
- change the import to `use radargen::rings::{RingSource, RingsError};`;
- add `source: RingSource::Flag,` to the two `RowKind::UndeclaredRing { … }` literals, one in the `ring` helper and one in `control_characters_in_values_are_shown_escaped`;
- replace each of the three expected strings containing `use a ring --rings does not declare` with `use a ring not declared by --rings`;
- add, above `a_class_with_one_occurrence_prints_the_per_row_message`:

```rust
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
            },
        )
    };
    assert_eq!(
        default(37).to_string(),
        "line 37, ring: 'Adpot' is not declared by the default ring set; the row is in the table view, not on the radar"
    );
    let mut w = Warnings::default();
    w.push(default(4));
    w.push(default(9));
    assert_eq!(
        w.render(),
        vec![
            "warning: 2 rows use a ring not declared by the default ring set — Adpot (2, first line 4); all appear in the table view"
        ]
    );
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -q`
Expected: compile errors: no `RingSource` in `radargen::rings`, and no field `source` on `UndeclaredRing`.

- [ ] **Step 3: Implement**

In `src/rings.rs`, change the imports to:

```rust
use std::fmt;

use crate::{normalise, trim};
```

Add after `DEFAULT_RINGS`:

```rust
/// Where the ring set came from. FR-17's warning names it, so an author
/// told a ring is undeclared knows what to edit (D-61).
///
/// `--config`'s `rings` key (FR-91) adds a variant when PR 19 adds the
/// file; that variant names the file as given on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingSource {
    /// `--rings` on the command line.
    Flag,
    /// FR-13's default, when nothing declared a ring set.
    Default,
}

/// How FR-17's warning names it: `not declared by {source}`.
// qual:allow(dry, boilerplate) reason: "deriving Display needs a crate (strum, derive_more) for one two-variant enum"
impl fmt::Display for RingSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Flag => "--rings",
            Self::Default => "the default ring set",
        })
    }
}

/// The declared rings, edge-to-centre (FR-12), and where they came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RingSet {
    /// As declared, after FR-15's trim: the band labels and legend
    /// subheadings.
    pub rings: Vec<String>,
    pub source: RingSource,
}
```

Change `parse_rings`'s signature to return `Result<RingSet, Vec<RingsError>>`. Return the default as:

```rust
        return Ok(RingSet {
            rings: DEFAULT_RINGS.iter().map(|r| (*r).to_string()).collect(),
            source: RingSource::Default,
        });
```

and the flag's rings as:

```rust
        Ok(RingSet {
            rings,
            source: RingSource::Flag,
        })
```

In `src/diagnostics.rs`:
- change `use crate::rings::RingsError;` to `use crate::rings::{RingSource, RingsError};`;
- replace the `UndeclaredRing` variant and its doc comment with:

  ```rust
      /// FR-17. `source` is what the message says to edit (D-61).
      UndeclaredRing { value: String, source: RingSource },
  ```

- replace `Class`'s `Ring,` with:

  ```rust
      /// One run declares one ring set, so the source never splits a class;
      /// it rides here so the grouped line can name it.
      Ring(RingSource),
  ```

- in `RowKind::class`, map `Self::UndeclaredRing { source, .. } => Class::Ring(*source)`;
- in `RowKind::value`, change the pattern `Self::UndeclaredRing { value }` to `Self::UndeclaredRing { value, .. }`;
- in `RowKind::message`, use the source:

  ```rust
              Self::UndeclaredRing { source, .. } => format!(
                  "'{v}' is not declared by {source}; the row is in the table view, not on the radar"
              ),
  ```

- in `grouped`, replace the `Class::Ring` arm with:

  ```rust
          Class::Ring(source) => format!(
              "{n} rows use a ring not declared by {source} — {}; all appear in the table view",
              tallied(members, normalise)
          ),
  ```

- [ ] **Step 4: Run everything**

Run: `cargo fmt && cargo test -q && cargo clippy --all-targets -- -D warnings && just lint`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/rings.rs src/diagnostics.rs tests/rings.rs tests/diagnostics.rs
git commit -m "feat(rings): FR-17's warning names the ring set's source (D-61)"
```

---

### Task 3: The model's diagnostics

**Files:**
- Modify: `src/diagnostics.rs` (`Fatal::MissingColumn`, `Fatal::EmptyRequired`, `RowKind::UnnamedValue`, `Class::Unnamed`)
- Test: `tests/diagnostics.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `Fatal::MissingColumn { line: usize, column: &'static str }` (FR-5).
  - `Fatal::EmptyRequired { line: usize, column: &'static str }` (FR-6).
  - `RowKind::UnnamedValue { column: String }` (FR-93). `column` is `"column N"`, 1-based.

- [ ] **Step 1: Write the failing tests**

In `tests/diagnostics.rs`, add a case at the end of the `cases` array in `each_row_kind_has_its_per_row_message`:

```rust
        (
            at(
                16,
                5,
                RowKind::UnnamedValue {
                    column: "column 6".into(),
                },
            ),
            "line 16, column 6: this column has no name in the header row, so its value is not shown; add a header name, or delete the value",
        ),
```

Append to the file:

```rust
/// FR-5 and FR-6, which `model::build_catalogue` raises (PR 06).
#[test]
fn model_messages() {
    let cases = [
        (
            Fatal::MissingColumn {
                line: 1,
                column: "ring",
            },
            "error: line 1: the header has no 'ring' column; the header must include at least the columns 'name', 'quadrant', 'ring' and 'description'",
        ),
        (
            Fatal::EmptyRequired {
                line: 7,
                column: "quadrant",
            },
            "error: line 7, quadrant: empty; every row needs a name, a quadrant and a ring, so fill it in or delete the row",
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
                column: format!("column {}", column + 1),
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
            "warning: 3 values are in columns with no name in the header row — column 6 (2, first line 4), column 7 (1, line 9); none is shown, so add a header name to show them"
        ]
    );
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -q --test diagnostics`
Expected: compile errors: no variant `UnnamedValue`, `MissingColumn` or `EmptyRequired`.

- [ ] **Step 3: Implement**

In `src/diagnostics.rs`, add to `RowKind`, after `Undisplayable`:

```rust
    /// FR-93: a value under a column whose header cell is empty. `column`
    /// is `column N`, the 1-based position, since there is no name.
    UnnamedValue { column: String },
```

Add `Unnamed,` as the last `Class` variant, and in `RowKind::class` map `Self::UnnamedValue { .. } => Class::Unnamed,`.

In `RowKind::column_name`, merge the new variant into `Undisplayable`'s arm:

```rust
            Self::Undisplayable { column, .. } | Self::UnnamedValue { column } => column,
```

Replace `RowKind::value`'s doc comment and its first arm with:

```rust
    /// The value NFR-20 enumerates; `None` for a class that lists positions.
    /// FR-93's class enumerates its columns, so its value is the column.
    fn value(&self) -> Option<&str> {
        match self {
            Self::UndeclaredRing { value, .. }
            | Self::UnrecognisedQualifier { value }
            | Self::UnrecognisedStatus { value }
            | Self::UnparsableDate { value, .. }
            | Self::DuplicateName { value, .. }
            | Self::UnnamedValue { column: value } => Some(value),
```

Add a last arm to `RowKind::message`:

```rust
            Self::UnnamedValue { .. } => {
                "this column has no name in the header row, so its value is not shown; add a header name, or delete the value".into()
            }
```

Add a last arm to `grouped`:

```rust
        // FR-93 counts values, each column with its count and first line.
        Class::Unnamed => format!(
            "{n} values are in columns with no name in the header row — {}; none is shown, so add a header name to show them",
            tallied(members, str::to_string)
        ),
```

Add to `Fatal`, after `FieldCount`:

```rust
    /// FR-5: the header lacks a required column. Every missing one is
    /// reported (NFR-16 tier two). `line` is the header's.
    #[error(
        "line {line}: the header has no '{column}' column; the header must include at least the columns 'name', 'quadrant', 'ring' and 'description'"
    )]
    MissingColumn { line: usize, column: &'static str },
    /// FR-6: an empty `name`, `quadrant` or `ring`, empty under FR-76's
    /// trim, so a cell of spaces or a lone zero-width space counts.
    #[error(
        "line {line}, {column}: empty; every row needs a name, a quadrant and a ring, so fill it in or delete the row"
    )]
    EmptyRequired { line: usize, column: &'static str },
```

- [ ] **Step 4: Run everything**

Run: `cargo fmt && cargo test -q && cargo clippy --all-targets -- -D warnings && just lint`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src/diagnostics.rs tests/diagnostics.rs
git commit -m "feat(diagnostics): FR-5, FR-6 and FR-93 messages"
```

---

### Task 4: `model.rs` — header, required values, quadrants, rings, names

**Files:**
- Create: `src/model.rs`
- Modify: `src/lib.rs` (add `pub mod model;` after `pub mod escape;`)

**Interfaces:**
- Consumes: `csv::parse::{parse, Record, Table}` (PR 05); `RingSet`, `RingSource`, `parse_rings` (Task 2); `trim`, `normalise` (Task 1); the Task 3 variants.
- Produces, in `radargen::model`:
  - `ColumnKind` (`Name`, `Quadrant`, `Ring`, `Description`, `Qualifier`, `Domain`, `Rationale`, `LastAffirmed`, `ExitDate`, `Status`, `Other`), with `as_str()`.
  - `Column { heading: String, kind: ColumnKind }`.
  - `Quadrant { key: String, label: String }`.
  - `Item { line, name, name_key, quadrant: usize, ring: Option<usize>, cells: Vec<String> }`. Task 5 adds `qualifier` and `status`.
  - `Catalogue { columns, quadrants, rings: Vec<String>, items }`, all fields public.
  - `build_catalogue(&Table, &RingSet, &mut Warnings) -> Result<Catalogue, Vec<Fatal>>`.

The listing in Step 3 is the whole file at the end of this task, with its tests at the bottom.

- [ ] **Step 1: Write the failing tests**

Add `pub mod model;` to `src/lib.rs`. Create `src/model.rs` holding only its module doc comment, the `use` lines and the `#[cfg(test)] mod tests { … }` block from the listing in Step 3.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -q --lib model`
Expected: compile errors: `build_catalogue`, `Catalogue`, `ColumnKind` and `Quadrant` not found.

- [ ] **Step 3: Implement**

`src/model.rs`, complete:

```rust
//! Records to meaning (architecture §2): which column is which, what each
//! row claims, and what is wrong with it (FR-5–FR-29, FR-93).
//!
//! Ordering and numbering are not here: FR-10 and FR-34 are `order.rs`'s
//! (PR 07), so a `Catalogue` keeps quadrants and items in CSV order.

use crate::csv::parse::{Record, Table};
use crate::diagnostics::{Fatal, RowKind, Warning, Warnings};
use crate::rings::{RingSet, RingSource};
use crate::{normalise, trim};

/// FR-5's required columns, in the order its error lists them.
const REQUIRED: [ColumnKind; 4] = [
    ColumnKind::Name,
    ColumnKind::Quadrant,
    ColumnKind::Ring,
    ColumnKind::Description,
];

/// FR-6's columns: required, and never empty. `description` may be empty
/// (FR-7).
const NON_EMPTY: [ColumnKind; 3] = [ColumnKind::Name, ColumnKind::Quadrant, ColumnKind::Ring];

/// What a named column means. A header name is matched under FR-76, so
/// `Name ` is `name`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnKind {
    Name,
    Quadrant,
    Ring,
    Description,
    Qualifier,
    Domain,
    Rationale,
    LastAffirmed,
    ExitDate,
    Status,
    /// FR-27: a column the tool does not know. Kept for the table (FR-78).
    Other,
}

impl ColumnKind {
    /// The kind a header name under FR-76 names.
    fn of(key: &str) -> Self {
        match key {
            "name" => Self::Name,
            "quadrant" => Self::Quadrant,
            "ring" => Self::Ring,
            "description" => Self::Description,
            "qualifier" => Self::Qualifier,
            "domain" => Self::Domain,
            "rationale" => Self::Rationale,
            "last_affirmed" => Self::LastAffirmed,
            "exit_date" => Self::ExitDate,
            "status" => Self::Status,
            _ => Self::Other,
        }
    }

    /// The canonical header name; empty for [`ColumnKind::Other`].
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Quadrant => "quadrant",
            Self::Ring => "ring",
            Self::Description => "description",
            Self::Qualifier => "qualifier",
            Self::Domain => "domain",
            Self::Rationale => "rationale",
            Self::LastAffirmed => "last_affirmed",
            Self::ExitDate => "exit_date",
            Self::Status => "status",
            Self::Other => "",
        }
    }
}

/// One named column, in header order (FR-27, FR-78).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// The header cell under FR-76's trim, as the table heads it.
    pub heading: String,
    pub kind: ColumnKind,
}

/// One quadrant (FR-8, FR-9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quadrant {
    /// FR-76's normalised value, which groups the rows.
    pub key: String,
    /// FR-9: the ordinally first variant, trimmed.
    pub label: String,
}

/// One CSV data row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The physical line the record begins on (NFR-10). Lines ascend with
    /// CSV order, so this is also FR-34's last tiebreak.
    pub line: usize,
    /// As written, after FR-90's removal.
    pub name: String,
    /// FR-76's key, computed once so no comparator folds (architecture §6).
    pub name_key: String,
    /// Index into [`Catalogue::quadrants`], which is in first-appearance
    /// order — **not** FR-10's, and never a palette index (architecture
    /// §5). `order.rs` (PR 07) gives the FR-10 rank.
    pub quadrant: usize,
    /// Index into [`Catalogue::rings`]; `None` when the ring is not
    /// declared (FR-17), which leaves the row unplotted.
    pub ring: Option<usize>,
    /// Every named column's cell, aligned with [`Catalogue::columns`] (not
    /// with the header, which may hold unnamed columns), as written after
    /// FR-90's removal (FR-78).
    pub cells: Vec<String>,
}

/// The rows and what they mean, in CSV order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalogue {
    /// The named columns in header order. An unnamed column is not here:
    /// FR-93 drops it.
    pub columns: Vec<Column>,
    /// In order of first appearance; FR-10 orders them (PR 07).
    pub quadrants: Vec<Quadrant>,
    /// The declared rings, edge-to-centre (FR-12). [`Item::ring`] indexes
    /// this copy; nothing below the model reads the `RingSet`.
    pub rings: Vec<String>,
    pub items: Vec<Item>,
}

/// FR-5–FR-29 and FR-93: the catalogue, or every row-level error.
///
/// NFR-16 tier two: every missing required column (FR-5) and every empty
/// `name`, `quadrant` or `ring` (FR-6) is returned together, in line order
/// then column order. Rows are checked before any is built, so an `Err`
/// comes with no warning from this stage. `csv::parse`'s FR-4 errors and
/// FR-90 warnings are separate: `main.rs` decides what a failed run prints
/// (PR 13).
///
/// # Errors
///
/// [`Fatal::MissingColumn`] and [`Fatal::EmptyRequired`].
pub fn build_catalogue(
    table: &Table,
    rings: &RingSet,
    w: &mut Warnings,
) -> Result<Catalogue, Vec<Fatal>> {
    let header = Header::of(&table.header);
    none_or_all(
        [
            header.missing(table.header_line),
            empty_required(&header, &table.records),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )?;
    Ok(Builder::new(header, rings).build(&table.records, w))
}

/// `Ok` when there are no errors, else every one of them.
fn none_or_all(errors: Vec<Fatal>) -> Result<(), Vec<Fatal>> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The cell at header position `p`. `parse` keeps only records as long as
/// the header, so a missing cell reads as empty rather than panicking on a
/// hand-built `Table`.
fn field(record: &Record, p: usize) -> &str {
    record.fields.get(p).map_or("", String::as_str)
}

/// The header, read once: which position holds which column.
struct Header {
    /// Per header position, the index of its named column in `columns`, or
    /// `None` for an unnamed one (FR-93).
    slots: Vec<Option<usize>>,
    columns: Vec<Column>,
}

impl Header {
    fn of(names: &[String]) -> Self {
        let mut slots = Vec::with_capacity(names.len());
        let mut columns = Vec::new();
        for name in names {
            let key = normalise(name);
            if key.is_empty() {
                slots.push(None);
                continue;
            }
            slots.push(Some(columns.len()));
            columns.push(Column {
                heading: trim(name).to_string(),
                kind: ColumnKind::of(&key),
            });
        }
        Self { slots, columns }
    }

    /// The header position of the first column of `kind`. A repeat is
    /// already FR-4's error, so the first is the one that counts.
    fn position(&self, kind: ColumnKind) -> Option<usize> {
        self.slots
            .iter()
            .position(|s| s.is_some_and(|c| self.columns[c].kind == kind))
    }

    /// FR-5: every required column the header lacks.
    fn missing(&self, line: usize) -> Vec<Fatal> {
        REQUIRED
            .into_iter()
            .filter(|&kind| self.position(kind).is_none())
            .map(|kind| Fatal::MissingColumn {
                line,
                column: kind.as_str(),
            })
            .collect()
    }
}

/// FR-6: every empty `name`, `quadrant` or `ring` the header has, in line
/// order, then header column order (NFR-11).
fn empty_required(header: &Header, records: &[Record]) -> Vec<Fatal> {
    let mut present: Vec<(usize, ColumnKind)> = NON_EMPTY
        .into_iter()
        .filter_map(|kind| header.position(kind).map(|p| (p, kind)))
        .collect();
    present.sort_by_key(|&(p, _)| p);
    records
        .iter()
        .flat_map(|record| {
            present
                .iter()
                .filter(|&&(p, _)| trim(field(record, p)).is_empty())
                .map(|&(_, kind)| Fatal::EmptyRequired {
                    line: record.line,
                    column: kind.as_str(),
                })
        })
        .collect()
}

/// Where each column the rows are read from sits in the header.
struct Positions {
    name: usize,
    quadrant: usize,
    ring: usize,
    description: usize,
}

impl Positions {
    fn of(header: &Header) -> Self {
        // FR-5 has passed before a `Positions` is built, so the default is
        // never taken.
        let required = |kind| header.position(kind).unwrap_or_default();
        Self {
            name: required(ColumnKind::Name),
            quadrant: required(ColumnKind::Quadrant),
            ring: required(ColumnKind::Ring),
            description: required(ColumnKind::Description),
        }
    }
}

/// Builds the catalogue a row at a time.
struct Builder {
    header: Header,
    at: Positions,
    /// The declared rings under FR-76, for FR-16's match.
    ring_keys: Vec<String>,
    source: RingSource,
    rings: Vec<String>,
    quadrants: Vec<Quadrant>,
    /// Each name's key and the line it first appears on, for FR-28.
    names: Vec<(String, usize)>,
    items: Vec<Item>,
}

impl Builder {
    fn new(header: Header, rings: &RingSet) -> Self {
        Self {
            at: Positions::of(&header),
            header,
            ring_keys: rings.rings.iter().map(|r| normalise(r)).collect(),
            source: rings.source,
            rings: rings.rings.clone(),
            quadrants: Vec::new(),
            names: Vec::new(),
            items: Vec::new(),
        }
    }

    fn build(mut self, records: &[Record], w: &mut Warnings) -> Catalogue {
        for record in records {
            self.add(record, w);
        }
        Catalogue {
            columns: self.header.columns,
            quadrants: self.quadrants,
            rings: self.rings,
            items: self.items,
        }
    }

    fn add(&mut self, record: &Record, w: &mut Warnings) {
        let at = |p: usize| field(record, p);
        let mut warn = |column: usize, kind: RowKind| {
            w.push(Warning::Row {
                line: record.line,
                column,
                kind,
            });
        };
        let name = at(self.at.name);
        let name_key = self.name(name, record.line, &mut warn);
        let quadrant = self.quadrant(at(self.at.quadrant));
        let ring = self.ring(at(self.at.ring), &mut warn);
        if trim(at(self.at.description)).is_empty() {
            warn(self.at.description, RowKind::EmptyDescription);
        }
        let cells = record
            .fields
            .iter()
            .zip(&self.header.slots)
            .filter(|(_, slot)| slot.is_some())
            .map(|(f, _)| f.clone())
            .collect();
        self.items.push(Item {
            line: record.line,
            name: name.to_string(),
            name_key,
            quadrant,
            ring,
            cells,
        });
    }

    /// FR-28: every later occurrence of a name warns, naming the first.
    fn name(&mut self, name: &str, line: usize, warn: &mut impl FnMut(usize, RowKind)) -> String {
        let key = normalise(name);
        match self.names.iter().find(|(k, _)| *k == key) {
            Some(&(_, first_line)) => warn(
                self.at.name,
                RowKind::DuplicateName {
                    value: name.to_string(),
                    first_line,
                },
            ),
            None => self.names.push((key.clone(), line)),
        }
        key
    }

    /// FR-8, FR-9: the row's quadrant, added on first sight. The label is
    /// the ordinally first variant, trimmed, whatever order rows come in.
    fn quadrant(&mut self, value: &str) -> usize {
        let key = normalise(value);
        let label = trim(value);
        if let Some(i) = self.quadrants.iter().position(|q| q.key == key) {
            // D-53: "ordinally" is code-point order, which is `str`'s `<`.
            // `order::compare_names` (PR 07) is the same rule; this site
            // cannot call it, since `order.rs` depends on this module.
            if label < self.quadrants[i].label.as_str() {
                self.quadrants[i].label = label.to_string();
            }
            return i;
        }
        self.quadrants.push(Quadrant {
            key,
            label: label.to_string(),
        });
        self.quadrants.len() - 1
    }

    /// FR-16, FR-17: the declared ring the value matches, or a warning
    /// naming where the ring set came from (D-61).
    fn ring(&self, value: &str, warn: &mut impl FnMut(usize, RowKind)) -> Option<usize> {
        let key = normalise(value);
        let found = self.ring_keys.iter().position(|k| *k == key);
        if found.is_none() {
            warn(
                self.at.ring,
                RowKind::UndeclaredRing {
                    value: value.to_string(),
                    source: self.source,
                },
            );
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csv::parse::parse;
    use crate::rings::parse_rings;

    const HEAD: &str = "name,quadrant,ring,description\n";

    /// `csv` through `parse` and `build_catalogue` under the default rings,
    /// with the rendered warnings. Parse must succeed with no error.
    fn build(csv: &str) -> (Result<Catalogue, Vec<Fatal>>, Vec<String>) {
        build_with(csv, None)
    }

    fn build_with(csv: &str, rings: Option<&str>) -> (Result<Catalogue, Vec<Fatal>>, Vec<String>) {
        let mut w = Warnings::default();
        let (table, errs) = parse(csv, &mut w).expect("parses");
        assert!(errs.is_empty(), "{errs:?}");
        let rings = parse_rings(rings).expect("valid rings");
        let result = build_catalogue(&table, &rings, &mut w);
        (result, w.render())
    }

    fn catalogue(csv: &str) -> Catalogue {
        build(csv).0.expect("builds")
    }

    fn errors(csv: &str) -> Vec<String> {
        build(csv)
            .0
            .expect_err("fails")
            .iter()
            .map(Fatal::render)
            .collect()
    }

    fn warnings(csv: &str) -> Vec<String> {
        let (result, warnings) = build(csv);
        result.expect("builds");
        warnings
    }

    fn missing(column: &'static str) -> String {
        Fatal::MissingColumn { line: 1, column }.render()
    }

    fn empty(line: usize, column: &'static str) -> String {
        Fatal::EmptyRequired { line, column }.render()
    }

    // --- The header: FR-5, FR-27, FR-78, FR-93 ---

    /// FR-78's columns in header order; FR-93 drops the unnamed one.
    #[test]
    fn columns_keep_header_order_and_drop_unnamed_ones() {
        let cat = catalogue("name,quadrant,,ring,description, Notes ,\nK,T,,Adopt,d,n,\n");
        let headings: Vec<_> = cat.columns.iter().map(|c| c.heading.as_str()).collect();
        assert_eq!(
            headings,
            ["name", "quadrant", "ring", "description", "Notes"]
        );
        assert_eq!(cat.columns[4].kind, ColumnKind::Other);
        assert_eq!(cat.items[0].cells, ["K", "T", "Adopt", "d", "n"]);
    }

    /// FR-76 (D-61): case, spaces and invisible format characters do not
    /// hide a required column — `rg-lme`'s case.
    #[test]
    fn required_columns_match_under_fr76() {
        let cat = catalogue("\u{feff}Name\u{200b}, QUADRANT ,Ring,description\nK,T,Adopt,d\n");
        let kinds: Vec<_> = cat.columns.iter().map(|c| c.kind).collect();
        assert_eq!(
            kinds,
            [
                ColumnKind::Name,
                ColumnKind::Quadrant,
                ColumnKind::Ring,
                ColumnKind::Description
            ]
        );
    }

    /// FR-5, NFR-16: every missing column at once, in FR-5's order.
    #[test]
    fn every_missing_required_column_is_reported() {
        assert_eq!(
            errors("ring,name\nAdopt,K\n"),
            [missing("quadrant"), missing("description")]
        );
    }

    /// FR-5's line is the header's, which blank lines above it move (FR-3).
    #[test]
    fn a_missing_column_names_the_header_line() {
        assert_eq!(
            errors("\n,,\nname,quadrant,ring\n"),
            [Fatal::MissingColumn {
                line: 3,
                column: "description"
            }
            .render()]
        );
    }

    /// FR-27: an unrecognised column is no error and no warning.
    #[test]
    fn an_unrecognised_column_is_kept_silently() {
        let csv = "name,quadrant,ring,description,ticket\nK,T,Adopt,d,OPS-1\n";
        let quiet = warnings(csv);
        assert!(quiet.is_empty(), "{quiet:?}");
        assert_eq!(catalogue(csv).items[0].cells[4], "OPS-1");
    }

    // --- FR-6 ---

    /// FR-6 under FR-76: spaces or a lone zero-width space are empty.
    /// Every one is reported, in line order then column order (NFR-16).
    #[test]
    fn every_empty_required_value_is_reported() {
        let csv = format!("{HEAD} ,T,Adopt,d\nK,\u{200b},  ,d\nK,T,Adopt,d\n");
        assert_eq!(
            errors(&csv),
            [empty(2, "name"), empty(3, "quadrant"), empty(3, "ring")]
        );
    }

    /// NFR-11: within a line, errors follow the header's column order, not
    /// FR-6's list.
    #[test]
    fn empty_required_values_follow_header_column_order() {
        assert_eq!(
            errors("ring,name,quadrant,description\n ,,T,d\n"),
            [empty(2, "ring"), empty(2, "name")]
        );
    }

    /// SC-10: FR-90's removal runs first, so a `name` of only U+0001 is
    /// the empty-`name` hard error.
    #[test]
    fn a_name_of_only_undisplayable_characters_is_empty() {
        assert_eq!(
            errors(&format!("{HEAD}\u{1},T,Adopt,d\n")),
            [empty(2, "name")]
        );
    }

    /// NFR-16: FR-5 and FR-6 arrive in one run. The present required
    /// columns are still checked.
    #[test]
    fn missing_columns_and_empty_values_arrive_together() {
        assert_eq!(
            errors("name,quadrant,ring\n,T,Adopt\n"),
            [missing("description"), empty(2, "name")]
        );
    }

    /// `rg-rnc`: a cleared row (`,,,`) between data rows is skipped, but
    /// its line still counts (FR-3), so later lines stay the editor's.
    #[test]
    fn a_cleared_row_does_not_shift_line_numbers() {
        let csv = format!("{HEAD}K,T,Adopt,d\n,,,\n,T,Adopt,d\n");
        assert_eq!(errors(&csv), [empty(4, "name")]);
    }

    /// No warning from the model when it fails; `parse`'s FR-90 warnings
    /// are already pushed and stay.
    #[test]
    fn errors_push_no_model_warnings() {
        let csv = format!("{HEAD},T,Nope,\nK\u{1},T,Adopt,d\n");
        let (result, warnings) = build(&csv);
        assert!(result.is_err());
        assert_eq!(
            warnings,
            ["warning: line 3, name: removed U+0001, which the output cannot display"]
        );
    }

    /// A header-only file is FR-42's empty state, not an error.
    #[test]
    fn a_header_only_file_builds_an_empty_catalogue() {
        let cat = catalogue(HEAD);
        assert!(cat.items.is_empty() && cat.quadrants.is_empty());
        assert_eq!(cat.rings.len(), 4);
    }

    // --- Quadrants, rings and names: FR-8, FR-9, FR-11, FR-16, FR-17, FR-28 ---

    /// FR-9: one quadrant per normalised value, labelled by the ordinally
    /// first variant, trimmed — the same label whatever the row order.
    #[test]
    fn quadrants_group_and_take_the_ordinally_first_label() {
        for rows in [
            "a,tool ,Adopt,d\nb,Tool,Adopt,d\nc,TOOL,Adopt,d\n",
            "c,TOOL,Adopt,d\nb,Tool,Adopt,d\na,tool ,Adopt,d\n",
        ] {
            let cat = catalogue(&format!("{HEAD}{rows}"));
            assert_eq!(
                cat.quadrants,
                [Quadrant {
                    key: "tool".into(),
                    label: "TOOL".into()
                }]
            );
            assert!(cat.items.iter().all(|i| i.quadrant == 0));
        }
    }

    /// FR-9 compares the variants **trimmed**: untrimmed, ` tool` would
    /// sort first (a space is below `T`). A boundary format character goes
    /// too (D-61).
    #[test]
    fn quadrant_labels_compare_trimmed_variants() {
        let cat = catalogue(&format!(
            "{HEAD}a, tool,Adopt,d\nb,Tool,Adopt,d\nc,\u{200b}Tool,Adopt,d\n"
        ));
        assert_eq!(cat.quadrants[0].label, "Tool");
    }

    /// FR-8: quadrants in order of first appearance; FR-10 orders them later.
    #[test]
    fn quadrants_keep_first_appearance_order() {
        let cat = catalogue(&format!(
            "{HEAD}a,Tool,Adopt,d\nb,Platform,Adopt,d\nc,tool,Adopt,d\n"
        ));
        let labels: Vec<_> = cat.quadrants.iter().map(|q| q.label.as_str()).collect();
        assert_eq!(labels, ["Tool", "Platform"]);
        let indices: Vec<_> = cat.items.iter().map(|i| i.quadrant).collect();
        assert_eq!(indices, [0, 1, 0]);
    }

    /// FR-11: near-duplicates are neither merged nor flagged.
    #[test]
    fn near_duplicates_stay_apart_silently() {
        let csv = format!("{HEAD}Kafka,Tool,Adopt,d\nKafkas,Tools,Adopt,d\n");
        let (result, warnings) = build(&csv);
        assert_eq!(result.expect("builds").quadrants.len(), 2);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// FR-16: matched under FR-76, both sides normalised.
    #[test]
    fn rings_match_under_fr76() {
        let cat = catalogue(&format!("{HEAD}a,T, adopt\u{200b},d\nb,T,RETIRE,d\n"));
        let rings: Vec<_> = cat.items.iter().map(|i| i.ring).collect();
        assert_eq!(rings, [Some(2), Some(3)]);
        assert_eq!(cat.rings, ["Assess", "Trial", "Adopt", "Retire"]);
    }

    /// FR-17 (D-61): unplotted, kept, and the warning names the source.
    #[test]
    fn an_undeclared_ring_is_unplotted_and_names_the_ring_source() {
        let csv = format!("{HEAD}a,T,In Use,d\n");
        let (result, warnings) = build(&csv);
        assert_eq!(result.expect("builds").items[0].ring, None);
        assert_eq!(
            warnings,
            [
                "warning: line 2, ring: 'In Use' is not declared by the default ring set; the row is in the table view, not on the radar"
            ]
        );
        let (_, warnings) = build_with(&csv, Some("Adopt,Hold"));
        assert_eq!(
            warnings,
            [
                "warning: line 2, ring: 'In Use' is not declared by --rings; the row is in the table view, not on the radar"
            ]
        );
    }

    /// FR-28: every row is kept, and each later occurrence names the
    /// **first** line, not the previous one.
    #[test]
    fn a_repeated_name_names_its_first_line() {
        let csv = format!("{HEAD}Kafka,T,Adopt,d\nkafka ,T,Trial,d\nKAFKA,T,Assess,d\n");
        let mut w = Warnings::default();
        let (table, _) = parse(&csv, &mut w).expect("parses");
        let rings = parse_rings(None).expect("default");
        let cat = build_catalogue(&table, &rings, &mut w).expect("builds");
        assert_eq!(cat.items.len(), 3);
        let firsts: Vec<_> = w
            .as_slice()
            .iter()
            .filter_map(|warning| match warning {
                Warning::Row {
                    kind: RowKind::DuplicateName { first_line, .. },
                    ..
                } => Some(*first_line),
                _ => None,
            })
            .collect();
        assert_eq!(firsts, [2, 2]);
    }

    /// SC-10: FR-90's removal runs first, so `Kafka` + U+0001 repeats
    /// `Kafka`; so does `Kafka` behind a zero-width space (FR-76, D-61).
    #[test]
    fn invisible_characters_do_not_hide_a_repeated_name() {
        assert_eq!(
            warnings(&format!("{HEAD}Kafka,T,Adopt,d\nKafka\u{1},T,Adopt,d\n")),
            [
                "warning: line 3, name: removed U+0001, which the output cannot display",
                "warning: line 3, name: 'Kafka' repeats line 2; both are drawn",
            ]
        );
        assert_eq!(
            warnings(&format!("{HEAD}Kafka,T,Adopt,d\n\u{200b}Kafka,T,Adopt,d\n")),
            ["warning: line 3, name: 'Kafka' repeats line 2; both are drawn"]
        );
    }

    /// The name is kept as written; its key is FR-76's, computed once.
    #[test]
    fn names_keep_their_text_and_carry_a_key() {
        let item = &catalogue(&format!("{HEAD} Kafka ,T,Adopt,d\n")).items[0];
        assert_eq!(
            (item.name.as_str(), item.name_key.as_str()),
            (" Kafka ", "kafka")
        );
    }

    /// FR-7: an empty description renders, with a warning. Spaces or a
    /// lone zero-width space are empty.
    #[test]
    fn an_empty_description_warns() {
        assert_eq!(
            warnings(&format!("{HEAD}K,T,Adopt, \nL,T,Adopt,\u{200b}\n")),
            ["warning: 2 rows have an empty description — lines 2, 3; each detail panel omits it"]
        );
    }
}
```

- [ ] **Step 4: Run everything**

Run: `cargo fmt && cargo test -q && cargo clippy --all-targets -- -D warnings && just lint`
Expected: all pass, with 22 tests in `model::tests`.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/model.rs
git commit -m "feat(model): header, FR-5/FR-6, quadrants, rings and names"
```

---

### Task 5: Optional columns and unnamed columns

**Files:**
- Modify: `src/model.rs`

**Interfaces:**
- Consumes: Task 4's `model.rs`; `DateColumn` and the Task 3 variants.
- Produces:
  - `Qualifier { Default, Conditional }` and `Status { New, MovedIn, MovedOut, NoChange }`, each with `label() -> &'static str`: the canonical text a legend row and the detail panel show.
  - `Item::qualifier: Option<Qualifier>` (FR-20; `None` when blank or unrecognised) and `Item::status: Option<Status>` (FR-26).
  - `Item::hollow(&self) -> bool` (FR-20).
  - `Item::cell(&self, column: usize) -> Option<&str>` (FR-29: `None` when empty under FR-76's trim). `column` indexes `Catalogue::columns`.

- [ ] **Step 1: Write the failing tests**

In `model::tests`, add after `const HEAD`:

```rust
    const FULL: &str = "name,quadrant,ring,description,qualifier,last_affirmed,exit_date,status\n";
```

Add after the `warnings` helper:

```rust
    /// Each item's `f`, in CSV order, with the rendered warnings.
    fn per_item<T>(csv: &str, f: impl Fn(&Item) -> T) -> (Vec<T>, Vec<String>) {
        let (result, warnings) = build(csv);
        (
            result.expect("builds").items.iter().map(f).collect(),
            warnings,
        )
    }
```

Append to the test module:

```rust
    // --- Optional columns: FR-20, FR-21, FR-24–FR-26, FR-29, FR-93 ---

    /// FR-20: `conditional` is hollow and `default` is recognised; blank —
    /// spaces and format characters included (FR-76) — claims nothing,
    /// silently; anything else claims nothing, with a warning.
    #[test]
    fn qualifiers_are_read_under_fr76() {
        let csv = format!(
            "{FULL}a,T,Adopt,d, Conditional,,,\nb,T,Adopt,d,default,,,\nc,T,Adopt,d,,,,\nd,T,Adopt,d,  ,,,\ne,T,Adopt,d,\u{200b},,,\nf,T,Adopt,d,\u{200b}conditional,,,\ng,T,Adopt,d,maybe,,,\n"
        );
        let (qualifiers, warnings) = per_item(&csv, |i| (i.qualifier, i.hollow()));
        assert_eq!(
            qualifiers,
            [
                (Some(Qualifier::Conditional), true),
                (Some(Qualifier::Default), false),
                (None, false),
                (None, false),
                (None, false),
                (Some(Qualifier::Conditional), true),
                (None, false),
            ]
        );
        assert_eq!(
            warnings,
            ["warning: line 8, qualifier: unrecognised qualifier value 'maybe'; drawn solid"]
        );
    }

    /// FR-21: the qualifier ignores the ring, under any ring set.
    #[test]
    fn the_qualifier_ignores_the_ring() {
        let csv = format!("{FULL}a,T,Use,d,conditional,,,\nb,T,Gone,d,conditional,,,\n");
        let (result, _) = build_with(&csv, Some("Use,Approved"));
        let hollow: Vec<_> = result
            .expect("builds")
            .items
            .iter()
            .map(Item::hollow)
            .collect();
        assert_eq!(hollow, [true, true]);
    }

    /// FR-26: four values under FR-76; blank (spaces and format characters
    /// included) is absent; anything else is `No Change` with a warning.
    #[test]
    fn statuses_are_read_under_fr76() {
        let csv = format!(
            "{FULL}a,T,Adopt,d,,,,new\nb,T,Adopt,d,,,,Moved In\nc,T,Adopt,d,,,,MOVED OUT\nd,T,Adopt,d,,,,\u{feff}no change\ne,T,Adopt,d,,,,\nf,T,Adopt,d,,,,  \ng,T,Adopt,d,,,,\u{200b}\nh,T,Adopt,d,,,,Gone\n"
        );
        let (statuses, warnings) = per_item(&csv, |i| i.status);
        assert_eq!(
            statuses,
            [
                Some(Status::New),
                Some(Status::MovedIn),
                Some(Status::MovedOut),
                Some(Status::NoChange),
                None,
                None,
                None,
                Some(Status::NoChange)
            ]
        );
        assert_eq!(
            warnings,
            ["warning: line 9, status: unrecognised status value 'Gone'; drawn as No Change"]
        );
    }

    /// FR-24, FR-25: an unparsable date warns, naming its column; a blank
    /// one does not, and nor does one inside spaces or format characters.
    /// Either way the cell is kept as written.
    #[test]
    fn unparsable_dates_warn_and_keep_their_text() {
        let csv = format!(
            "{FULL}a,T,Adopt,d,, 2026-06-01 ,June 2027,\nb,T,Adopt,d,,\u{200b}2026-06-01\u{feff},,\nc,T,Adopt,d,,,,\n"
        );
        let (result, warnings) = build(&csv);
        let items = result.expect("builds").items;
        assert_eq!(items[0].cell(5), Some(" 2026-06-01 "));
        assert_eq!(items[1].cell(5), Some("\u{200b}2026-06-01\u{feff}"));
        assert_eq!(
            warnings,
            ["warning: line 2, exit_date: 'June 2027' is not a YYYY-MM-DD date; shown as written"]
        );
    }

    /// FR-24's shape exactly, and a real calendar date. A multi-byte
    /// character at a field boundary is rejected, not sliced.
    #[test]
    fn iso_dates_are_exactly_the_shape_and_real() {
        for ok in [
            "2026-06-01",
            "2026-04-30",
            "2024-02-29",
            "2000-02-29",
            "1999-12-31",
        ] {
            assert!(is_iso_date(ok), "{ok}");
        }
        for bad in [
            "2026-6-1",
            "2026-04-31",
            "2026-02-30",
            "2023-02-29",
            "1900-02-29",
            "2026-13-01",
            "2026-00-10",
            "2026-01-00",
            "2026/06/01",
            "20260601",
            "2026-06-01T00:00",
            "２０２６-06-01",
            "202é06-01",
            "",
        ] {
            assert!(!is_iso_date(bad), "{bad}");
        }
    }

    /// FR-29: an absent optional column claims nothing and warns nothing.
    #[test]
    fn absent_optional_columns_claim_nothing() {
        let csv = format!("{HEAD}K,T,Adopt,d\n");
        let (result, warnings) = build(&csv);
        let item = &result.expect("builds").items[0];
        assert_eq!((item.qualifier, item.status), (None, None));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// FR-29: an empty cell is omitted, not shown as an empty label.
    #[test]
    fn an_empty_cell_is_omitted() {
        let item = &catalogue(&format!("{FULL}K,T,Adopt,d,,  ,,\n")).items[0];
        assert_eq!(
            (item.cell(4), item.cell(5), item.cell(99)),
            (None, None, None)
        );
        assert_eq!(item.cell(0), Some("K"));
    }

    /// `Item::cell` indexes the named columns, not the header: an unnamed
    /// column before `exit_date` does not shift it.
    #[test]
    fn cells_index_named_columns_not_header_positions() {
        let cat = catalogue(
            "name,,quadrant,ring,description,exit_date\nOld,,Tool,Rejected,d,2027-01-01\n",
        );
        assert_eq!(cat.items[0].cell(4), Some("2027-01-01"));
    }

    /// FR-93: Excel's trailing padding passes silently; a value under no
    /// name warns, naming the column's position — wherever the column is,
    /// and whether the header cell is empty or only a format character.
    #[test]
    fn unnamed_columns_warn_only_when_they_hold_a_value() {
        let padding = warnings("name,quadrant,ring,description,,\nK,T,Adopt,d,,\n");
        assert!(padding.is_empty(), "{padding:?}");
        assert_eq!(
            warnings("name,quadrant,ring,description,,\nK,T,Adopt,d,, \nL,T,Adopt,d,,x\n"),
            [
                "warning: line 3, column 6: this column has no name in the header row, so its value is not shown; add a header name, or delete the value"
            ]
        );
        assert_eq!(
            warnings("name,\u{200b},quadrant,ring,description\nK,x,T,Adopt,d\n"),
            [
                "warning: line 2, column 2: this column has no name in the header row, so its value is not shown; add a header name, or delete the value"
            ]
        );
    }

    /// `rg-rnc`: FR-90 still warns for a cell in a column FR-93 drops, and
    /// FR-93 warns as well, since the cell still holds a value.
    #[test]
    fn a_dropped_column_still_gets_fr90s_warning() {
        assert_eq!(
            warnings("name,quadrant,ring,description,\nK,T,Adopt,d,a\u{1}\n"),
            [
                "warning: line 2, column 5: removed U+0001, which the output cannot display",
                "warning: line 2, column 5: this column has no name in the header row, so its value is not shown; add a header name, or delete the value",
            ]
        );
    }

    /// NFR-11 across the two stages: parse's and the model's warnings on
    /// one line sort by header column, whichever stage pushed first.
    #[test]
    fn warnings_from_both_stages_sort_by_header_column() {
        let csv = "name,,quadrant,ring,description,\nK,x,T,Adpot,d\u{1},\nL,,T,Adopt,,\n";
        assert_eq!(
            warnings(csv),
            [
                "warning: line 2, column 2: this column has no name in the header row, so its value is not shown; add a header name, or delete the value",
                "warning: line 2, ring: 'Adpot' is not declared by the default ring set; the row is in the table view, not on the radar",
                "warning: line 2, description: removed U+0001, which the output cannot display",
                "warning: line 3, description: empty; the detail panel omits it",
            ]
        );
    }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -q --lib model`
Expected: compile errors: no field `qualifier` or `status`, no methods `hollow` or `cell`, and no `Qualifier`, `Status` or `is_iso_date`.

- [ ] **Step 3: Implement**

Change the diagnostics import to:

```rust
use crate::diagnostics::{DateColumn, Fatal, RowKind, Warning, Warnings};
```

After `Column`, add:

```rust
/// FR-20's recognised values. Blank and unrecognised values claim nothing,
/// so they have no variant: the marker is solid and the legend row shows
/// no qualifier (FR-74).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qualifier {
    Default,
    Conditional,
}

impl Qualifier {
    /// What a legend row shows (FR-74), so an accessible name can contain
    /// it (WCAG 2.5.3).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Conditional => "Conditional",
        }
    }
}

/// FR-26's values. An unrecognised one is `NoChange`, with a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    New,
    MovedIn,
    MovedOut,
    NoChange,
}

impl Status {
    /// Every value, so the vocabulary is written once, in [`Status::label`].
    const ALL: [Self; 4] = [Self::New, Self::MovedIn, Self::MovedOut, Self::NoChange];

    /// The canonical spelling: what a legend row and the detail panel show
    /// (FR-26, FR-74), and what a cell matches under FR-76.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::New => "New",
            Self::MovedIn => "Moved In",
            Self::MovedOut => "Moved Out",
            Self::NoChange => "No Change",
        }
    }
}
```

Replace `Item` with:

```rust
/// One CSV data row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The physical line the record begins on (NFR-10). Lines ascend with
    /// CSV order, so this is also FR-34's last tiebreak.
    pub line: usize,
    /// As written, after FR-90's removal.
    pub name: String,
    /// FR-76's key, computed once so no comparator folds (architecture §6).
    pub name_key: String,
    /// Index into [`Catalogue::quadrants`], which is in first-appearance
    /// order — **not** FR-10's, and never a palette index (architecture
    /// §5). `order.rs` (PR 07) gives the FR-10 rank.
    pub quadrant: usize,
    /// Index into [`Catalogue::rings`]; `None` when the ring is not
    /// declared (FR-17), which leaves the row unplotted.
    pub ring: Option<usize>,
    /// FR-20; `None` when blank or unrecognised.
    pub qualifier: Option<Qualifier>,
    /// FR-26: `None` when blank or absent.
    pub status: Option<Status>,
    /// Every named column's cell, aligned with [`Catalogue::columns`] (not
    /// with the header, which may hold unnamed columns), as written after
    /// FR-90's removal (FR-78).
    pub cells: Vec<String>,
}
```

After `Item`, add:

```rust
impl Item {
    /// FR-20: drawn hollow.
    #[must_use]
    pub fn hollow(&self) -> bool {
        self.qualifier == Some(Qualifier::Conditional)
    }

    /// The cell under `column`, an index into [`Catalogue::columns`], or
    /// `None` when it is empty under FR-76's trim, which FR-29 omits rather
    /// than showing an empty label.
    #[must_use]
    pub fn cell(&self, column: usize) -> Option<&str> {
        self.cells
            .get(column)
            .map(String::as_str)
            .filter(|c| !trim(c).is_empty())
    }
}
```

Replace `Positions` and its `impl` with:

```rust
/// Where each column the rows are read from sits in the header.
struct Positions {
    name: usize,
    quadrant: usize,
    ring: usize,
    description: usize,
    qualifier: Option<usize>,
    status: Option<usize>,
    last_affirmed: Option<usize>,
    exit_date: Option<usize>,
    /// FR-93's unnamed columns.
    unnamed: Vec<usize>,
}

impl Positions {
    fn of(header: &Header) -> Self {
        // FR-5 has passed before a `Positions` is built, so the default is
        // never taken.
        let required = |kind| header.position(kind).unwrap_or_default();
        Self {
            name: required(ColumnKind::Name),
            quadrant: required(ColumnKind::Quadrant),
            ring: required(ColumnKind::Ring),
            description: required(ColumnKind::Description),
            qualifier: header.position(ColumnKind::Qualifier),
            status: header.position(ColumnKind::Status),
            last_affirmed: header.position(ColumnKind::LastAffirmed),
            exit_date: header.position(ColumnKind::ExitDate),
            unnamed: (0..header.slots.len())
                .filter(|&p| header.slots[p].is_none())
                .collect(),
        }
    }
}
```

Replace `Builder::add` with:

```rust
    fn add(&mut self, record: &Record, w: &mut Warnings) {
        let at = |p: usize| field(record, p);
        let mut warn = |column: usize, kind: RowKind| {
            w.push(Warning::Row {
                line: record.line,
                column,
                kind,
            });
        };
        let name = at(self.at.name);
        let name_key = self.name(name, record.line, &mut warn);
        let quadrant = self.quadrant(at(self.at.quadrant));
        let ring = self.ring(at(self.at.ring), &mut warn);
        if trim(at(self.at.description)).is_empty() {
            warn(self.at.description, RowKind::EmptyDescription);
        }
        let qualifier = self
            .at
            .qualifier
            .and_then(|p| read_qualifier(at(p), p, &mut warn));
        let status = self
            .at
            .status
            .and_then(|p| read_status(at(p), p, &mut warn));
        for (column, position) in [
            (DateColumn::LastAffirmed, self.at.last_affirmed),
            (DateColumn::ExitDate, self.at.exit_date),
        ] {
            if let Some(p) = position {
                check_date(at(p), p, column, &mut warn);
            }
        }
        for &p in &self.at.unnamed {
            check_unnamed(at(p), p, &mut warn);
        }
        let cells = record
            .fields
            .iter()
            .zip(&self.header.slots)
            .filter(|(_, slot)| slot.is_some())
            .map(|(f, _)| f.clone())
            .collect();
        self.items.push(Item {
            line: record.line,
            name: name.to_string(),
            name_key,
            quadrant,
            ring,
            qualifier,
            status,
            cells,
        });
    }
```

Add before `#[cfg(test)]`:

```rust
/// FR-20: `conditional` and `default` are recognised; blank claims nothing;
/// anything else claims nothing too, with a warning (D-6, D-35).
fn read_qualifier(
    value: &str,
    column: usize,
    warn: &mut impl FnMut(usize, RowKind),
) -> Option<Qualifier> {
    match normalise(value).as_str() {
        "conditional" => Some(Qualifier::Conditional),
        "default" => Some(Qualifier::Default),
        "" => None,
        _ => {
            warn(
                column,
                RowKind::UnrecognisedQualifier {
                    value: value.to_string(),
                },
            );
            None
        }
    }
}

/// FR-26: blank is absent; an unrecognised value is `NoChange`, with a
/// warning (D-35).
fn read_status(
    value: &str,
    column: usize,
    warn: &mut impl FnMut(usize, RowKind),
) -> Option<Status> {
    let key = normalise(value);
    if key.is_empty() {
        return None;
    }
    let known = Status::ALL
        .into_iter()
        .find(|s| normalise(s.label()) == key);
    if known.is_none() {
        warn(
            column,
            RowKind::UnrecognisedStatus {
                value: value.to_string(),
            },
        );
    }
    Some(known.unwrap_or(Status::NoChange))
}

/// FR-24, FR-25: a non-blank value that is not `YYYY-MM-DD` warns. Either
/// way it renders as written (FR-24), so nothing is returned.
fn check_date(value: &str, at: usize, column: DateColumn, warn: &mut impl FnMut(usize, RowKind)) {
    let t = trim(value);
    if !t.is_empty() && !is_iso_date(t) {
        warn(
            at,
            RowKind::UnparsableDate {
                column,
                value: value.to_string(),
            },
        );
    }
}

/// FR-93: a value under a column with no name is not shown, so say so.
fn check_unnamed(value: &str, at: usize, warn: &mut impl FnMut(usize, RowKind)) {
    if !trim(value).is_empty() {
        warn(
            at,
            RowKind::UnnamedValue {
                column: format!("column {}", at + 1),
            },
        );
    }
}

/// FR-24's shape: a digit wherever the pattern has a letter.
const ISO_DATE: &str = "YYYY-MM-DD";
/// Days per month in a common year.
const MONTH_DAYS: [usize; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const FEBRUARY: usize = 2;
/// The Gregorian leap rule: every fourth year, except centuries, except
/// every fourth century.
const LEAP_YEAR: usize = 4;
const CENTURY: usize = 100;
const LEAP_CENTURY: usize = 400;

/// `YYYY-MM-DD`, a real calendar date. Exactly that shape: no week dates,
/// no times, no unpadded fields (FR-24). The shape is checked first, so
/// what is split is ASCII digits and hyphens only.
fn is_iso_date(s: &str) -> bool {
    let shaped = s.len() == ISO_DATE.len()
        && s.bytes().zip(ISO_DATE.bytes()).all(|(c, p)| {
            if p == b'-' {
                c == b'-'
            } else {
                c.is_ascii_digit()
            }
        });
    if !shaped {
        return false;
    }
    let mut fields = s.split('-').filter_map(|f| f.parse::<usize>().ok());
    let (Some(year), Some(month), Some(day)) = (fields.next(), fields.next(), fields.next()) else {
        return false;
    };
    let leap = year.is_multiple_of(LEAP_YEAR)
        && (!year.is_multiple_of(CENTURY) || year.is_multiple_of(LEAP_CENTURY));
    month
        .checked_sub(1)
        .and_then(|m| MONTH_DAYS.get(m))
        .map(|&d| d + usize::from(leap && month == FEBRUARY))
        .is_some_and(|d| (1..=d).contains(&day))
}
```

- [ ] **Step 4: Run everything**

Run: `cargo fmt && cargo test -q && cargo clippy --all-targets -- -D warnings && just lint`
Expected: all pass, with 33 tests in `model::tests`.

- [ ] **Step 5: Commit**

```bash
git add src/model.rs
git commit -m "feat(model): optional columns, dates and unnamed columns"
```

---

### Task 6: Accessible names, end-to-end tests, architecture

**Files:**
- Modify: `src/model.rs` (`Catalogue::column`, `Catalogue::accessible_name`)
- Create: `tests/fixtures/three-quadrant.csv`, `tests/fixtures/row-errors.csv`, `tests/model.rs`
- Modify: `docs/design/architecture.md` (header, §2, §3, §5, §7, §8, §10)

**Interfaces:**
- Consumes: Tasks 1–5.
- Produces:
  - `Catalogue::column(&self, kind: ColumnKind) -> Option<usize>`: an index into `Catalogue::columns`, for `Item::cell`.
  - `Catalogue::accessible_name(&self, item: &Item, number: Option<usize>) -> String` (FR-50, FR-75), for markers and legend rows. PR 07 supplies `number`; PR 14 calls it.

- [ ] **Step 1: Write the failing tests**

Append to `model::tests`:

```rust
    // --- FR-50, FR-75 ---

    /// The legend row's text first — number, name, qualifier, status —
    /// then the quadrant and ring; an unplotted row says so instead of a
    /// number (FR-18).
    #[test]
    fn accessible_names_carry_the_row_text_and_its_position() {
        let cat = catalogue(&format!(
            "{FULL} Kafka ,tool,adopt,d,conditional,,,moved in\nOld,Tool,Rejected,d,,,,\n"
        ));
        assert_eq!(
            cat.accessible_name(&cat.items[0], Some(3)),
            "3. Kafka, Conditional, Moved In, Tool, Adopt"
        );
        assert_eq!(
            cat.accessible_name(&cat.items[1], None),
            "Old, Tool, Rejected, not on the radar"
        );
    }

    /// The unplotted ring is read from the named columns, so an unnamed
    /// column before it does not shift it onto another cell.
    #[test]
    fn an_unplotted_ring_is_found_past_an_unnamed_column() {
        let cat = catalogue("name,,quadrant,ring,description\nOld,,Tool,Rejected,d\n");
        assert_eq!(
            cat.accessible_name(&cat.items[0], None),
            "Old, Tool, Rejected, not on the radar"
        );
        assert_eq!(cat.column(ColumnKind::Ring), Some(2));
    }
```

Create `tests/fixtures/three-quadrant.csv` with PRD §6.1.6's example, verbatim:

```csv
name,quadrant,ring,description,qualifier,domain,rationale,last_affirmed,exit_date,status
Terraform,Tool,Adopt,"Declarative infrastructure-as-code tool for provisioning cloud and on-prem resources.",default,Infrastructure as Code,"Default IaC tool across all managed environments.",2026-06-01,,No Change
Pulumi,Tool,Adopt,"Infrastructure-as-code authored in a general-purpose language instead of a domain-specific syntax.",conditional,Infrastructure as Code,"Acceptable where a client mandates a general-purpose language over HCL.",2026-06-01,,New
Ansible,Tool,Trial,"Agentless configuration management and orchestration over SSH.",,Configuration Management,"Bounded trial on one managed estate; was Assess last edition.",2026-05-12,,Moved In
Chef,Tool,Retire,"Agent-based configuration management using a Ruby DSL.",,Configuration Management,"Superseded by the Ansible trial outcome; migration owned and scheduled.",2026-03-15,2027-01-31,
Kubernetes,Platform,Adopt,"Container orchestration platform for deploying, scaling, and managing containerised workloads.",default,Container Orchestration,"Default for all new containerised workloads.",2026-06-01,,
Trunk-Based Development,Practice,Adopt,"Developers integrate small, frequent changes into a shared trunk.",default,Delivery Practice,"Default branching model for managed delivery teams.",2026-06-01,,
Legacy Ticketing-Driven Change,Practice,Rejected,"Manual, ticket-driven change approval replacing automated policy gates.",,Operations Practice,"Declined during the 2026 review — see the domain ADR.",2026-01-10,,
```

Create `tests/fixtures/row-errors.csv`:

```csv
name,quadrant,ring,domain
Kafka,Tool,Adopt,Streaming
,Tool,Adopt,Streaming
Flink,Tool,Trial
Spark,Tool, ,Batch
,,,
Beam,,Assess,Batch,extra
Storm,,Retire,Streaming
```

Create `tests/model.rs`:

```rust
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
        cat.items[6].cell(rationale),
        Some("Declined during the 2026 review — see the domain ADR.")
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
/// in one run, and the rows `parse` rejected do not hide the others.
#[test]
fn row_errors_from_both_stages_arrive_in_one_run() {
    let mut w = Warnings::default();
    let (table, parse_errors) =
        parse(include_str!("fixtures/row-errors.csv"), &mut w).expect("parses");
    let rings = parse_rings(None).expect("default");
    let model_errors = build_catalogue(&table, &rings, &mut w).expect_err("fails");
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
            "error: line 1: the header has no 'description' column; the header must include at least the columns 'name', 'quadrant', 'ring' and 'description'",
            "error: line 3, name: empty; every row needs a name, a quadrant and a ring, so fill it in or delete the row",
            "error: line 5, ring: empty; every row needs a name, a quadrant and a ring, so fill it in or delete the row",
            "error: line 8, quadrant: empty; every row needs a name, a quadrant and a ring, so fill it in or delete the row",
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
        [
            "error: line 2, name: empty; every row needs a name, a quadrant and a ring, so fill it in or delete the row"
        ]
    );
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -q`
Expected: compile errors: no methods `accessible_name` or `column` on `Catalogue`.

- [ ] **Step 3: Implement**

After `Catalogue`, add:

```rust
impl Catalogue {
    /// The index in [`Catalogue::columns`] of the first column of `kind`.
    #[must_use]
    pub fn column(&self, kind: ColumnKind) -> Option<usize> {
        self.columns.iter().position(|c| c.kind == kind)
    }

    /// FR-50, FR-75: an accessible name for a marker or legend row.
    ///
    /// It starts with what the legend row shows — number, name, qualifier,
    /// status (FR-74) — so the visible text is in the name (WCAG 2.5.3).
    /// The quadrant and ring follow even where the legend's grouping shows
    /// them, since a screen reader reaching one row by Tab sees no grouping.
    /// An unplotted row has no number (FR-18) and says it is not on the
    /// radar. The result is plain text; the emitter escapes it (NFR-6).
    ///
    /// `number` is FR-34's, for a plotted item, and `None` for an unplotted
    /// one.
    ///
    /// # Panics
    ///
    /// If `item` is not one of `self.items`.
    #[must_use]
    pub fn accessible_name(&self, item: &Item, number: Option<usize>) -> String {
        debug_assert_eq!(number.is_some(), item.ring.is_some(), "FR-18");
        let ring = item.ring.map_or_else(
            || {
                let column = self.column(ColumnKind::Ring).expect("FR-5 passed");
                trim(&item.cells[column])
            },
            |r| self.rings[r].as_str(),
        );
        let parts = [
            Some(trim(&item.name)),
            item.qualifier.map(Qualifier::label),
            item.status.map(Status::label),
            Some(self.quadrants[item.quadrant].label.as_str()),
            Some(ring),
        ];
        let text = parts.into_iter().flatten().collect::<Vec<_>>().join(", ");
        number.map_or_else(
            || format!("{text}, not on the radar"),
            |n| format!("{n}. {text}"),
        )
    }
}
```

- [ ] **Step 4: Update the architecture**

In `docs/design/architecture.md`:
- **Header:** where it names the PRD version it implements, change v0.42 to v0.43.
- **§2 diagram:** change the `model` line to `    │  model        map columns, check values, match rings        FR-5, FR-6`, keeping the column alignment; the `Catalogue` line keeps `FR-5–FR-29`. Change the `resolve` line to `    │  resolve      order, number                  │          FR-10, FR-18, FR-34`.
- **§2, the paragraph beginning "`Catalogue` therefore keeps the CSV's shape":** replace its sentence beginning "`Catalogue` carries the declared header as an ordered list" with: "`Catalogue` carries the named columns in header order and, per row, every named cell as written (`Item::cells`, read through `Item::cell`, which applies FR-29). Typed fields exist only where behaviour depends on them: quadrant, ring, qualifier, status and `name_key`. That makes FR-78's \"every optional column the header declared\" a lookup rather than a re-parse." Keep the sentence about unnamed columns that follows.
- **§3, the `src/lib.rs` row:** after "`normalise()` — FR-76's single rule, stated once", add "; `trim()` — FR-76's trim, also the display trim (D-61)".
- **§3, the `src/model.rs` row:** replace "column validation (FR-5–FR-29); the declared header order and each row's unrecognised cells (FR-27, FR-78)" with "column validation, quadrant grouping and ring matching (FR-5–FR-29); the named columns in header order and every row's cells (FR-27, FR-78); unnamed columns dropped, with a warning when they hold a value (FR-93)".
- **§5:** after the paragraph's sentence "**No node carries a hex value.** … and the *emitter* binds it.", add: "A quadrant's series entry is indexed by its FR-10 rank from `order.rs` (FR-67), never by `Item::quadrant`, which is first-appearance order."
- **§7:** change the signature to `pub fn build_catalogue(table: &Table, rings: &RingSet, w: &mut Warnings) -> Result<Catalogue, Vec<Fatal>>;`. After the paragraph beginning "`csv::parse` returns", add: "`build_catalogue` takes the ring set because matching a row's ring is part of what the row means (FR-16), and FR-17's warning names the set's source. It checks every row before building any, so its `Err` comes with no warning of its own; what a failed run prints is `main.rs`'s rule." In the `Config` paragraph, change "the ring list and its source (FR-17)" to "the `RingSet` — the ring list and its source (FR-17), read only by `build_catalogue`; stages below read `Catalogue::rings`".
- **§8, the OQ-9 *(b)* row:** add the model as a second site: "`order::compare_names`; also FR-9's label choice in `model.rs`, which cannot call `order.rs` (a cycle) and cites D-53".
- **§10, the testing levels table:** add `tests/model.rs` to the Contract row's "Where". The fixture table already lists `three-quadrant.csv` and `row-errors.csv`.

- [ ] **Step 5: Run everything**

Run: `cargo fmt && just ci`
Expected: all pass, with 218 tests in total and 35 in `model::tests`. rustqual reports no regression against `rustqual-baseline.json`; the trial run went from 59.0% to 66.8%.

- [ ] **Step 6: Commit**

```bash
git add src/model.rs tests/fixtures tests/model.rs docs/design/architecture.md
git commit -m "feat(model): accessible names, fixtures and end-to-end tests"
```

---

## Finishing the PR

Follow the house workflow in `~/.claude/CLAUDE.md`. For this PR:

1. Run `crumbs start rg-rnc` before Task 1. Work on branch `feat/pr-06-model`, created from `main`.
2. After Task 6: push, then open the PR with `gh pr create` and **no reviewer flag**.
3. Run the code-reviewer skill until clean. Copilot is out of credits until November 2026, so a local multi-agent review substitutes, and the PR body says so. Then run simplify, then code-reviewer once more.
4. Update the crumbs:
   - `crumbs append rg-dsa` noting that FR-24's leading-space question is settled: FR-76 trims for parsing only, and the cell renders as written, pinned by `unparsable_dates_warn_and_keep_their_text`.
   - `crumbs append rg-u6u`: `RingSource` needs a variant for `--config`'s `rings` key, naming the file as given (FR-17, FR-91). Hold the path as `Arc<str>`. `RingSource` and `diagnostics::Class` then become `Clone` rather than `Copy`, and `RowKind::class` stops being `const fn`.
   - `crumbs append rg-efx`: PR 07 reads `Item::name_key`, `Quadrant::key` and `Item::line` (FR-34's last tiebreak). It returns FR-10's order as a list of indices into `Catalogue::quadrants`, plus a rank, and does not reorder or mutate the `Catalogue`. `Item::quadrant` is first-appearance order and never a palette index. Settle whether FR-34's ordinal name tiebreak compares raw or trimmed names (`rg-40l`).
   - `crumbs append rg-f27`: the detail panel's status line reads `Item::status` through `Status::label()`, not the raw cell. Legend rows show `Qualifier::label()` and `Status::label()`, which `accessible_name` contains (WCAG 2.5.3). Table rows: see `rg-6ny`.
   - Close `rg-rnc` with `crumbs close rg-rnc`, which also stops the timer `crumbs start` began. Do not hand-edit the file.
5. **No version bump:** nothing a user runs changes. Run `just quality-baseline` to lock in rustqual's higher score, and commit `rustqual-baseline.json`.
6. Move this file to `docs/plans/done/` in the same PR. Merge; there's no tag.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-10 | Initial version, from crumb `rg-rnc` and PRD v0.43 (D-61). Every task was applied to `main` in a scratch worktree before review, and each task's end state compiled and passed on its own: 207 tests pass, and `cargo fmt --check`, clippy, `just lint` and `just ci` (cargo-deny and rustqual) are clean |
| 1.1 | 2026-10-10 | Reviewed by architecture, Rust, QA and accessibility agents. **Fixed:** FR-6 errors now follow header column order, not FR-6's list (NFR-11). **Accessible name** now starts with the legend row's visible text (number, name, qualifier, status; WCAG 2.5.3), so `Qualifier` is an enum with `label()`, `Status` gains `label()`, and `Item::hollow()` replaces the flag. `Catalogue::column(kind)` replaces a private field, so `Catalogue` is fully public. Short records read as empty, not a panic. `Builder` owns its `Header`. Free functions are `read_*`/`check_*`. `is_iso_date` returns early on a bad shape. The FR-5, FR-6 and FR-93 messages now say what to do. The fixture is `three-quadrant.csv`, as architecture §10 names it. **Added tests:** SC-10 through the model, the blank/unrecognised split with spaces and format characters, FR-9's trimmed comparison, FR-11, FR-21, FR-23, the two index spaces, cross-stage NFR-11 order, FR-28 naming the first line, the full tag range, a header-only file, a repeated required header, and more date shapes. Architecture edits now cover §2's `Catalogue` description, §3, §5, §7's `Config`, §8 and §10. Seven findings needing a PRD change or a later PR are crumbs (`rg-6ny`, `rg-owp`, `rg-bvl`, `rg-fju`, `rg-85c`, `rg-40l`, `rg-q2b`). Re-applied in the scratch worktree: Task 4 and Task 5 end states pass alone (22 and 33 model tests); the final state passes `just ci` with 218 tests, and rustqual goes from 59.0% to 66.8% |
