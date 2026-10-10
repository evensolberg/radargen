---
tags:
  - plan
  - csv
aliases: []
doc_id: RG-005
doc_name: pr-05-csv-decode-parse
crumb_id: rg-aqb
document_title: "PR 05 — `csv/decode` and `csv/parse`: Implementation Plan"
created_date: 2026-10-10
synopsis: "Bytes to text (BOM, UTF-8, FR-61's line-numbered error) and text to records (a hand-written RFC 4180 scanner with physical line numbers, header and field-count checks returned beside the table, and FR-90's removal of undisplayable characters)."
status: draft
type: Plan
revision: "1.2"
review_date: 2026-10-10
reviewed_by:
  - architect-reviewer
  - rust-engineer
  - qa-expert
  - ux-researcher
  - ui-designer
completed_date:
comments:
---
# PR 05 — `csv/decode` and `csv/parse`: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-aqb` · **Phase:** v1.0 · **Blocks:** `rg-rnc` (PR 06)

**Goal:** Turn the input file into a header and a list of records, each carrying the physical line it begins on. Every structural problem is reported as NFR-16 requires, and every undisplayable character is removed and warned about before anything else reads a field. A real Excel export (BOM, CRLF, padded columns, cleared rows, a decimal-comma locale's `;`) either parses or fails with one message that says what to do.

**Architecture:** Two stages under `src/csv/` (architecture §3).
- `decode.rs` is bytes → `String`: strip one BOM (FR-2), require UTF-8 (FR-1), and on failure name the physical line and say to re-save as UTF-8 (FR-61, D-27).
- `parse.rs` is `String` → `Table`. A small hand-written RFC 4180 scanner tracks the physical line each record begins on (NFR-10, D-37). FR-90's removal then runs on every field, with a warning. Finally FR-3/FR-4's duplicate-header and field-count checks are collected across the file and returned **beside** the table (NFR-16 tier two), so PR 06's FR-5/FR-6 errors join them in the same run.

Malformed quoting, an empty file and a non-comma separator abort at once (tier one), because nothing after them parses. Errors are new `diagnostics::Fatal` variants, so there is still one taxonomy (§1); warnings go through `&mut Warnings` (§7).

**Tech Stack:** Rust 2024, standard library only. No new dependencies — see Decisions for why not the `csv` crate.

**Spec:** `docs/product/radargen-prd.md` v0.40 — FR-1, FR-2, FR-3, FR-4, FR-5, FR-61, FR-76, FR-90, NFR-9, NFR-10, NFR-15, NFR-16, NFR-20, D-27, D-37, D-55, SC-10. Design: `docs/design/architecture.md` §3 (the `csv/decode.rs` and `csv/parse.rs` rows), §7 (diagnostics threading), §10 (`row-errors.csv`).

## Global Constraints

- **Physical line numbers** (NFR-10, D-37): every row diagnostic (FR-4, FR-90) names the 1-based physical line a record **begins** on, header counting as line 1. A quoted newline makes later records' lines run ahead of their ordinals, permanently. **The two quoting errors are the exception, by design:** they are file-level (tier one), and they name the line holding the fault — where the unclosed quote opened, or where text follows the closing quote. In a multi-line record that is later than the record's first line, and it is where Go To Line must land (Decisions).
- **A line break is LF, CRLF or a lone CR**, counted once each, in both `decode` and `parse` — the way an editor's Go To Line counts.
- **FR-90 removal comes first** (D-55): a field's undisplayable characters (`!escape::displayable`) are removed before any check reads it, with one `RowKind::Undisplayable` warning per affected field. Use `escape::displayable`; never restate the set.
- **NFR-16:** file-level errors (unreadable, not UTF-8, malformed quoting, no header, not comma-separated) return one `Fatal` and stop. Duplicate headers and field-count mismatches are **all** collected, in line order, and returned **beside** the table rather than instead of it, so PR 06's row errors join them in the same run.
- **No trimming here.** Fields keep their whitespace; FR-76 trims at comparison time (PR 06), and displayed text is never edited beyond FR-15/FR-76. (Blank-record detection *reads* whitespace but changes nothing.)
- **One taxonomy:** new errors are `diagnostics::Fatal` variants (architecture §1). An I/O variant carries the path and marks the error `#[source]`, not `#[from]` (the `Fatal` doc comment). Every new variant is added to the pinned message catalogue in `tests/diagnostics.rs` (`csv_fatal_messages`, beside `fatal_messages`).
- **Messages say what to do** (FR-61's precedent, D-27): each new `Fatal` names the place and, where there is one, the likely fix, and uses "column", never "field", for a position — the word every other diagnostic uses.
- `cargo fmt` before every commit; CI runs `cargo fmt --check` (`.github/workflows/ci.yml`). The snippets below are not all in rustfmt's layout — format them, do not hand-wrap.
- `cargo clippy --all-targets -- -D warnings` and `just lint` (pedantic + nursery + `unwrap_used`) are both clean **at every commit**. No `#[allow]`/`#[expect]` without an inline reason.
- Commits: `type(scope): description` (subject ≤ 72 characters, body lines ≤ 72), plus the session's `Co-Authored-By` trailer. Run `git-mit es` first.

## Review Focus

1. **An Excel export with CRLF line endings.** Every diagnostic must name the line the author's editor shows. (The `csv` crate reports line 1 for line 2 here — see Decisions.) *Tests: Task 2, `crlf_lines_are_counted_like_an_editor_counts_them`; Task 3, `tests/csv.rs::an_excel_export_reads_and_parses_with_editor_line_numbers`.*
2. **A quoted description holding a newline.** Records after it must report their physical line, not their ordinal. *Test: Task 2, `a_quoted_newline_advances_later_line_numbers`.*
3. **An unterminated quote.** It swallows the rest of the file into one field. It must be a hard error naming the line where the quote opened — not a silently truncated radar (NFR-15). *Test: Task 2, `an_unterminated_quote_names_the_line_it_opened_on`.*
4. **A legacy-code-page export** (Windows-1252 `é` as byte `0xE9`). The error must name the line holding the bad byte and say to re-save as UTF-8, not just "invalid utf-8" (FR-61, D-27). *Test: Task 1, `a_legacy_code_page_names_its_line`.*
5. **An invisible control character in a header or a cell.** Removal comes first, so `name` + U+0001 duplicates `name` (FR-4), and a cell holding only U+0001 is empty (SC-10). *Tests: Task 3, `removal_runs_before_the_duplicate_header_check` and `a_cell_of_only_undisplayable_characters_becomes_empty`.*
6. **Excel's padding and locale quirks.** These are a padded used range (trailing `,,` on every line, `,,,` rows at the end or above the header) and a decimal-comma locale's `;`-separated "CSV". Each must give one actionable message, or none — not a duplicate-header or field-count error per line. *Tests: Task 2, `blank_records_are_skipped_but_counted`; Task 3, `empty_header_cells_are_not_duplicates` and `a_semicolon_export_is_named_as_such`.*
7. **A bad row next to good ones.** The good rows must still reach PR 06, so FR-4, FR-5 and FR-6 errors all arrive in one run (NFR-16, architecture §10's `row-errors.csv`). *Test: Task 3, `a_bad_row_does_not_hide_the_good_ones`.*

## Decisions taken in this plan (flag on review)

- **A hand-written RFC 4180 scanner, not the `csv` crate.** Library-first was the default, so `csv` 1.4.0 was measured against this PR's requirements first (2026-10-10):
  - On a CRLF file it reported record two as **line 1** — NFR-10 broken for every Excel export.
  - An unterminated quote produced **no error**: the rest of the file became one field — NFR-15 broken.
  - `"x"y` silently became `xy`.

  Working around all three means tracking quotes and line breaks independently of the library, which is the scanner itself. RFC 4180 is small (quoted fields, doubled quotes, comma, line break); the scanner is about 80 lines of code, is tested directly, and is checked exhaustively over every input of up to six characters from the alphabet `a , " \n \r`.
- **`parse` returns `Result<(Table, Vec<Fatal>), Fatal>`.** `Err` is tier one: nothing parsed. `Ok` carries the table of well-formed records **and** every FR-4 error. A row that fails FR-4 is left out of `records`, which keeps the `header.len()` invariant, but its error travels beside the table. So a short row never hides another row's empty `name` (FR-6), and `main.rs` (PR 13) merges parse's errors with `build_catalogue`'s. The merged order (ascending line, header-level first, is the NFR-11 reading) is PR 13's to fix before SC-10 pins it. Returning `Err(Vec<Fatal>)` instead would make `row-errors.csv` (architecture §10) unpassable.
- **Lenient where Excel never writes, strict where data would be lost.** A `"` inside an *unquoted* field is kept literally (`5" screen` is plausibly what the author meant, and "content is not fatal"). Text *after* a closing quote (`"x"y`) is a hard error: the author's intent is ambiguous, and silently dropping or keeping the quote would both be guesses. **Spaces around quotes are not special-cased.** RFC 4180 counts them as part of the field: `x, "y"` is the unquoted ` "y"`, and `"x" ,y` is text after a quote. Both outcomes are pinned by a test. Dropping them would need a lookahead probe for a hand-typing convenience Excel never needs.
- **Quoting errors name the faulty line, not the record's first line.** In a record spanning lines 2–4:
  - a quote opening on line 3 that never closes reports line 3;
  - text after a quote that closes on line 4 reports line 4.

  These are tier-one file errors (NFR-16), not row diagnostics, and the record's first line would send the author to a line with nothing wrong on it. Both name their column too (NFR-10).
- **A blank record is skipped, but its line is still counted.** A blank record is a line with no characters, a whitespace-only line, or a row of empty fields such as `,,,`. Excel writes the last for cleared rows inside its used range — at the end, between records, and above the header when the sheet starts lower down. Each looks blank to the author, and skipping it loses nothing.
  - "Blank" means every field is empty or only spaces and tabs. The test runs **before** FR-90's removal, so a cell holding only U+0001 is not blank here and still reaches FR-6 (SC-10).
  - The header is the first record that is not blank, so its line is its physical line: 1, unless the file opens with blank lines.
- **An empty header cell is not a duplicate.** Excel pads every line, header included, to its used range, so a stray cell to the right of the data exports as `name,quadrant,ring,description,,`. Two empty cells colliding under FR-4 would report `column 6 '' repeats column 5` on a file that looks right, naming nothing the author can find. An empty cell names nothing, so FR-4 skips it. Its data cells keep the field count true and are named `column N` in any warning. Whether an empty-named column is acceptable is PR 06's call (FR-5, FR-78).
- **A semicolon- or tab-separated file is refused at its header** (FR-1). Excel's CSV formats use the system list separator, which is `;` wherever a comma is the decimal mark, so a continental-European export arrives as one column. Left alone, it reports a field-count error for every row containing a comma, then FR-5's "missing `name`" for a header that visibly has one. A one-column header can never satisfy FR-5, so refusing it early loses nothing. There is still no `--delimiter` flag, for D-27's reason: the message is the mitigation.
- **FR-90 applies to header cells too**, so a header `name` + U+0001 is reported and then collides with `name` under FR-4. A field-count error does not stop that row's FR-90 warnings: both are true, and the run exits non-zero anyway.
- **The warning's column is the header name under FR-76** (`normalise`, so `Name ` reads `name`). That is PR 02's "canonical lower-case column name" and `RowKind::Undisplayable`'s doc, and it is how every other row warning names its column. When the header cell is empty, or the row is longer than the header, the warning says `column N` instead. `DuplicateHeader` alone shows names as written (trimmed), because it points at cells the author must find.
- **`csv::decode::read` does the one file read**, though architecture §2 calls stages pure. It is the only place holding both the path and the bytes, so `Unreadable` and `NotUtf8` both name the path (NFR-10) without `main.rs` re-wrapping `decode`'s error. `decode` itself stays pure, and that is what the line-number tests exercise.
- **`Table { header, records }` rather than bare records.** PR 06 needs the header to map columns (FR-5) and keep the declared order for the table view (FR-78). Architecture §7's `build_catalogue(records: &[Record], …)` becomes `build_catalogue(table: &Table, …)`.
- **A header-only file parses** to zero records; whether that is an error is PR 06's call (it is not structural).
- **Field-count errors are not grouped.** One systematic cause, such as an extra trailing comma on the header only, prints one error per row. NFR-20 groups warnings only, so grouping errors needs a PRD change. This is captured as crumb `rg-pcd`, not decided here.
- **New `Fatal` messages are proposed.** SC-10's golden tests (PR 13 onward) will pin them, so this is the cheap moment to change them:

| Variant | Message (after `error: `) |
| --- | --- |
| `Unreadable` | `{path}: cannot be read: {io error}` |
| `NotUtf8` | `{path}, line {line}: not UTF-8 text; re-save the file as UTF-8 (in Excel, Save As "CSV UTF-8")` |
| `NoHeader` | `the CSV has no header row (it is empty or blank); the header must name at least name, quadrant, ring and description` |
| `NotCommaSeparated` | `line {line}: the header is one column holding {semicolons\|tabs}; radargen reads comma-separated files only, so export the file with commas as the separator` |
| `UnterminatedQuote` | `line {line}, column {column}: a quote opens this value and never closes; add the closing quote, and write any quote inside the value as ""` |
| `TextAfterQuote` | `line {line}, column {column}: text follows a closing quote; inside quotes, write a quote as ""` |
| `DuplicateHeader` | `line {line}: column {index} '{name}' repeats column {first_index} '{first_name}'; header names match ignoring case and surrounding spaces, so rename or remove one` |
| `FieldCount` | `line {line}: the row's field count is {found} but the header's is {expected}; ` then, when `found > expected`, `a comma inside a value needs the value in double quotes`, else `a value is missing, or an unquoted line break split the row` |

- **PRD wording.** Three of these decisions depart from the PRD's literal text: skipping blank records (FR-3, FR-4, FR-6, FR-78), quoting errors naming the fault's line (NFR-10), and `NotCommaSeparated` as a tier-one error (NFR-9, NFR-16). Crumb `rg-lfd` writes them into the PRD. The handoff to PR 06 is appended to `rg-rnc`.
- **No version bump.** The binary still exits 1 without reading a file.

---

### Task 1: `csv/decode.rs` — bytes to text

**Files:**
- Create: `src/csv/mod.rs`, `src/csv/decode.rs`
- Modify: `src/lib.rs` (add `pub mod csv;` after `pub mod cli;`)
- Modify: `src/diagnostics.rs` (add `Unreadable` and `NotUtf8` to `Fatal`)
- Modify: `tests/diagnostics.rs` (add both to a new `csv_fatal_messages` test)

**Interfaces:**
- Consumes: `diagnostics::Fatal` (exists).
- Produces, in `radargen::csv::decode`:
  - `#[derive(Debug, PartialEq, Eq)] pub struct NotUtf8 { pub line: usize }`
  - `pub fn decode(bytes: &[u8]) -> Result<String, NotUtf8>`
  - `pub fn read(path: &std::path::Path) -> Result<String, Fatal>`
- And in `radargen::diagnostics::Fatal`: `Unreadable { path: String, #[source] source: std::io::Error }`, `NotUtf8 { path: String, line: usize }`.

  `main.rs` (PR 13) calls `read`; tests call `decode` directly.

- [ ] **Step 1: Write the failing tests**

`src/csv/mod.rs`:

```rust
//! The CSV stage (architecture §3): bytes to text, then text to records.
//! Nothing here interprets a column, except that `NoHeader` names FR-5's
//! required ones so its message is actionable; meaning is `model.rs` (PR 06).

pub mod decode;
```

`src/csv/decode.rs`, the test module only:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// FR-2: Excel writes a BOM by default; left in, the first header reads
    /// as `\u{feff}name` and FR-5 fails on a file that looks correct.
    #[test]
    fn a_leading_bom_is_stripped_once() {
        assert_eq!(decode(b"\xEF\xBB\xBFname,ring\n").as_deref(), Ok("name,ring\n"));
        assert_eq!(
            decode(b"\xEF\xBB\xBF\xEF\xBB\xBFname").as_deref(),
            Ok("\u{feff}name"),
            "only one BOM is the encoding's; a second is content"
        );
        assert_eq!(decode(b"name").as_deref(), Ok("name"));
        assert_eq!(decode(b"").as_deref(), Ok(""));
        assert_eq!(decode(BOM).as_deref(), Ok(""), "a BOM-only file is empty, for parse's NoHeader");
    }

    /// FR-61, D-27: the likely cause is a Windows-1252 export, and the line
    /// is what lets the author find the offending cell.
    #[test]
    fn a_legacy_code_page_names_its_line() {
        assert_eq!(decode(b"name\nKafka\nCaf\xE9\n"), Err(NotUtf8 { line: 3 }));
        assert_eq!(decode(b"\xE9"), Err(NotUtf8 { line: 1 }));
        assert_eq!(decode(b"\xEF\xBB\xBFa\n\xE9"), Err(NotUtf8 { line: 2 }));
        // Excel's "Unicode Text" is UTF-16: refused at line 1, never transcoded (D-27).
        assert_eq!(decode(b"\xFF\xFEn\x00a\x00"), Err(NotUtf8 { line: 1 }));
        assert_eq!(decode(b"\xFE\xFF\x00n\x00a"), Err(NotUtf8 { line: 1 }));
        // A sequence cut off by the end of the file.
        assert_eq!(decode(b"name\n\xC3"), Err(NotUtf8 { line: 2 }));
        // FR-61 names the byte's line, even inside a multi-line quoted field.
        assert_eq!(decode(b"a\n\"x\ny\xE9\"\n"), Err(NotUtf8 { line: 3 }));
    }

    /// NFR-10: CRLF is one break, and a lone CR is one too, as editors count.
    #[test]
    fn every_line_break_style_counts_once() {
        assert_eq!(decode(b"a\r\nb\r\n\xE9"), Err(NotUtf8 { line: 3 }));
        assert_eq!(decode(b"a\rb\r\xE9"), Err(NotUtf8 { line: 3 }));
        assert_eq!(decode(b"a\r\n\r\n\xE9"), Err(NotUtf8 { line: 3 }));
    }

    #[test]
    fn a_missing_file_is_unreadable_and_names_its_path() {
        let path = std::env::temp_dir().join(format!("radargen-no-such-file-{}.csv", std::process::id()));
        let err = read(&path).expect_err("missing");
        assert!(matches!(err, Fatal::Unreadable { .. }), "{err:?}");
        let shown = err.render();
        assert!(shown.contains(&format!("{}: cannot be read", path.display())), "{shown}");
    }

    /// FR-61 names a directory as unreadable too. On Unix the open succeeds
    /// and the read fails; on Windows (NFR-13) the open fails. Both land here.
    #[test]
    fn a_directory_is_unreadable() {
        let err = read(&std::env::temp_dir()).expect_err("a directory");
        assert!(matches!(err, Fatal::Unreadable { .. }), "{err:?}");
    }

    #[test]
    fn a_non_utf8_file_says_how_to_fix_it() {
        let path = std::env::temp_dir().join(format!("radargen-cp1252-{}.csv", std::process::id()));
        std::fs::write(&path, b"name\nCaf\xE9\n").expect("write fixture");
        let err = read(&path).expect_err("not UTF-8");
        std::fs::remove_file(&path).expect("remove fixture");
        let shown = err.render();
        assert!(shown.contains(", line 2: not UTF-8 text; re-save the file as UTF-8"), "{shown}");
    }

    #[test]
    fn a_utf8_file_reads_back_without_its_bom() {
        let path = std::env::temp_dir().join(format!("radargen-utf8-{}.csv", std::process::id()));
        std::fs::write(&path, "\u{feff}name\nÆrø\n").expect("write fixture");
        let text = read(&path);
        std::fs::remove_file(&path).expect("remove fixture");
        assert_eq!(text.expect("reads"), "name\nÆrø\n");
    }
}
```

Add a new test after `fatal_messages` in `tests/diagnostics.rs`. Keeping it apart means neither function trips pedantic's `too_many_lines` (100 lines):

```rust
/// The CSV stage's messages (plan RG-005), apart from `fatal_messages` so
/// neither passes clippy's `too_many_lines`.
#[test]
fn csv_fatal_messages() {
    let cases = [
        (
            Fatal::Unreadable {
                path: "in.csv".into(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
            },
            "error: in.csv: cannot be read: not found",
        ),
        (
            Fatal::NotUtf8 {
                path: "in.csv".into(),
                line: 2,
            },
            "error: in.csv, line 2: not UTF-8 text; re-save the file as UTF-8 (in Excel, Save As \"CSV UTF-8\")",
        ),
    ];
    for (fatal, expected) in cases {
        assert_eq!(fatal.render(), expected);
    }
}
```

Add `pub mod csv;` to `src/lib.rs` after `pub mod cli;`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib csv::decode`
Expected: a compile error, `cannot find function decode`.

- [ ] **Step 3: Implement**

Add to `Fatal` in `src/diagnostics.rs`, after `Rings`:

```rust
    /// FR-61: the input does not exist, cannot be read, or is a directory.
    #[error("{path}: cannot be read: {source}")]
    Unreadable {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// FR-61, D-27: say what to do — the likely cause is an Excel export in a
    /// legacy code page, and "invalid utf-8" alone does not tell the author
    /// that. `line` is the physical line holding the first bad byte.
    #[error(
        "{path}, line {line}: not UTF-8 text; re-save the file as UTF-8 (in Excel, Save As \"CSV UTF-8\")"
    )]
    NotUtf8 { path: String, line: usize },
```

Put this above the test module in `src/csv/decode.rs`:

```rust
//! Bytes to text (FR-1, FR-2, FR-61). No transcoding and no encoding flag:
//! a non-UTF-8 file is an error that says how to fix it (D-27).

use std::path::Path;

use crate::diagnostics::Fatal;

/// FR-2: the UTF-8 byte-order mark.
const BOM: &[u8] = b"\xEF\xBB\xBF";

/// The input is not UTF-8. `line` is the 1-based physical line holding the
/// first byte that is not.
#[derive(Debug, PartialEq, Eq)]
pub struct NotUtf8 {
    pub line: usize,
}

/// FR-2, then FR-1: strip one leading BOM, then require UTF-8.
///
/// # Errors
///
/// [`NotUtf8`] naming the line of the first byte that is not UTF-8.
pub fn decode(bytes: &[u8]) -> Result<String, NotUtf8> {
    let body = bytes.strip_prefix(BOM).unwrap_or(bytes);
    std::str::from_utf8(body)
        .map(str::to_string)
        .map_err(|e| NotUtf8 { line: line_at(body, e.valid_up_to()) })
}

/// The 1-based physical line holding byte `offset`, counting LF, CRLF and a
/// lone CR once each, as an editor does (NFR-10). `csv::parse` counts the
/// same way, so the two stages never disagree about a line; `tests/csv.rs`
/// checks that they agree.
fn line_at(bytes: &[u8], offset: usize) -> usize {
    let mut line = 1;
    let mut after_cr = false;
    for &b in &bytes[..offset] {
        // A CR is a break; an LF is one unless it completes a CRLF.
        if b == b'\r' || (b == b'\n' && !after_cr) {
            line += 1;
        }
        after_cr = b == b'\r';
    }
    line
}

/// FR-61: read and decode the input file. The one place the CSV stage
/// touches the file system (Decisions).
///
/// # Errors
///
/// [`Fatal::Unreadable`] when the file cannot be read; [`Fatal::NotUtf8`]
/// when it is not UTF-8.
pub fn read(path: &Path) -> Result<String, Fatal> {
    let shown = path.display().to_string();
    let bytes = std::fs::read(path).map_err(|source| Fatal::Unreadable {
        path: shown.clone(),
        source,
    })?;
    decode(&bytes).map_err(|NotUtf8 { line }| Fatal::NotUtf8 { path: shown, line })
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo fmt && cargo test --lib csv::decode && cargo test --test diagnostics && cargo clippy --all-targets -- -D warnings && just lint`
Expected: 7 passed in `csv::decode`; `fatal_messages` and `csv_fatal_messages` pass; both lints clean.

- [ ] **Step 5: Commit**

```bash
git add src/csv/mod.rs src/csv/decode.rs src/lib.rs src/diagnostics.rs tests/diagnostics.rs
git commit -m "feat(csv): decode — BOM, UTF-8, and FR-61's line-numbered error"
```

---

### Task 2: `csv/parse.rs` — the RFC 4180 scanner

Task 2 ends **without a commit**. `scan` is private and only its tests call it until Task 3, so a non-test build reports it, `is_blank`, `Scanner` (and its methods) and `End` as `dead_code`, and `-D warnings` fails. Do not add an `#[allow]`: Task 3 makes them live, and Task 3's commit carries `parse.rs` whole.

**Files:**
- Create: `src/csv/parse.rs`
- Modify: `src/csv/mod.rs` (add `pub mod parse;`)
- Modify: `src/diagnostics.rs` (add `UnterminatedQuote` and `TextAfterQuote` to `Fatal`)

**Interfaces:**
- Consumes: `diagnostics::Fatal`.
- Produces, in `radargen::csv::parse`:
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct Record { pub line: usize, pub fields: Vec<String> }`
  - private `fn scan(text: &str) -> Result<Vec<Record>, Fatal>` (Task 3 builds on it)
- And in `Fatal`: `UnterminatedQuote { line: usize, column: usize }`, `TextAfterQuote { line: usize, column: usize }`.

- [ ] **Step 1: Write the failing tests**

Add `pub mod parse;` to `src/csv/mod.rs`. Create `src/csv/parse.rs` with the test module only:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// `(line, fields)` for every record, so a test reads like the file.
    fn lines(text: &str) -> Vec<(usize, Vec<String>)> {
        scan(text)
            .expect("scans")
            .into_iter()
            .map(|r| (r.line, r.fields))
            .collect()
    }

    fn rec(line: usize, fields: &[&str]) -> (usize, Vec<String>) {
        (line, fields.iter().map(|f| (*f).to_string()).collect())
    }

    #[test]
    fn plain_records_with_their_lines() {
        assert_eq!(
            lines("name,ring\nKafka,Adopt\n"),
            [rec(1, &["name", "ring"]), rec(2, &["Kafka", "Adopt"])]
        );
    }

    #[test]
    fn quoted_fields_hold_commas_and_doubled_quotes() {
        assert_eq!(
            lines("a,b\n\"x, y\",\"say \"\"hi\"\"\"\n"),
            [rec(1, &["a", "b"]), rec(2, &["x, y", "say \"hi\""])]
        );
    }

    /// NFR-10, D-37: the physical line, not the ordinal.
    #[test]
    fn a_quoted_newline_advances_later_line_numbers() {
        assert_eq!(
            lines("a,b\n\"one\ntwo\nthree\",x\nnext,y\n"),
            [rec(1, &["a", "b"]), rec(2, &["one\ntwo\nthree", "x"]), rec(5, &["next", "y"])]
        );
    }

    /// Review focus 1: Excel writes CRLF. A lone CR (classic Mac) counts too.
    #[test]
    fn crlf_lines_are_counted_like_an_editor_counts_them() {
        assert_eq!(
            lines("a,b\r\n1,2\r\n3,4\r\n"),
            [rec(1, &["a", "b"]), rec(2, &["1", "2"]), rec(3, &["3", "4"])]
        );
        assert_eq!(lines("a\r1\r2"), [rec(1, &["a"]), rec(2, &["1"]), rec(3, &["2"])]);
        // A CRLF inside quotes is content, and still one physical break.
        assert_eq!(
            lines("a,b\r\n\"x\r\ny\",z\r\nq,r\r\n"),
            [rec(1, &["a", "b"]), rec(2, &["x\r\ny", "z"]), rec(4, &["q", "r"])]
        );
        // Excel's commonest row: a quoted last field, then CRLF.
        assert_eq!(
            lines("a,b\r\nKafka,\"desc\"\r\n"),
            [rec(1, &["a", "b"]), rec(2, &["Kafka", "desc"])]
        );
        // A lone CR inside quotes is one break too.
        assert_eq!(lines("a\n\"x\ry\"\nq\n"), [rec(1, &["a"]), rec(2, &["x\ry"]), rec(4, &["q"])]);
    }

    /// Decisions: blank lines, whitespace-only lines and Excel's cleared
    /// `,,,` rows are skipped, wherever they sit, but their lines count.
    #[test]
    fn blank_records_are_skipped_but_counted() {
        assert_eq!(
            lines("a\n\n1\n\n\n2\n\n"),
            [rec(1, &["a"]), rec(3, &["1"]), rec(6, &["2"])]
        );
        assert_eq!(lines("a,b\r\n\r\n1,2\r\n"), [rec(1, &["a", "b"]), rec(3, &["1", "2"])]);
        assert_eq!(
            lines(",,\na,b,c\n,,\n  \n1,2,3\n\t,\"\"\n"),
            [rec(2, &["a", "b", "c"]), rec(5, &["1", "2", "3"])]
        );
        // Tested before FR-90: a cell of only U+0001 is not blank (SC-10).
        assert_eq!(lines("a\n\u{1}\n"), [rec(1, &["a"]), rec(2, &["\u{1}"])]);
    }

    #[test]
    fn edge_fields_are_kept() {
        assert_eq!(lines("a,b,\n"), [rec(1, &["a", "b", ""])]);
        assert_eq!(lines("a,b"), [rec(1, &["a", "b"])], "no final newline");
        assert_eq!(lines("a\n\"x\""), [rec(1, &["a"]), rec(2, &["x"])], "quoted, no final newline");
        assert_eq!(lines(" a , b \n"), [rec(1, &[" a ", " b "])], "not trimmed here");
        assert_eq!(lines("\"\",x\n"), [rec(1, &["", "x"])]);
    }

    /// Lenient where Excel never writes: a quote inside an unquoted field.
    #[test]
    fn a_quote_inside_an_unquoted_field_is_literal() {
        assert_eq!(lines("5\" screen,x\n"), [rec(1, &["5\" screen", "x"])]);
    }

    /// Decisions: RFC 4180 counts spaces around quotes as field content, so
    /// a space before the opening quote makes the field unquoted, and one
    /// after the closing quote is text after it.
    #[test]
    fn spaces_around_quotes_are_not_ignored() {
        assert_eq!(lines("x, \"y\"\n"), [rec(1, &["x", " \"y\""])]);
        let err = scan("a,b\n\"x\" ,y\n").expect_err("text after quote");
        assert!(matches!(err, Fatal::TextAfterQuote { line: 2, column: 1 }), "{err:?}");
    }

    /// Review focus 3, NFR-15: it would swallow the rest of the file.
    #[test]
    fn an_unterminated_quote_names_the_line_it_opened_on() {
        let err = scan("a,b\nx,y\n\"open,2\n3,4\n").expect_err("unterminated");
        assert!(matches!(err, Fatal::UnterminatedQuote { line: 3, column: 1 }), "{err:?}");
        assert_eq!(
            err.render(),
            "error: line 3, column 1: a quote opens this value and never closes; add the closing quote, and write any quote inside the value as \"\""
        );
        // The opening quote's line, even when its record began earlier.
        let err = scan("a,b,c\n1,\"two\nlines\",\"open\n").expect_err("unterminated");
        assert!(matches!(err, Fatal::UnterminatedQuote { line: 3, column: 3 }), "{err:?}");
    }

    #[test]
    fn text_after_a_closing_quote_is_an_error() {
        let err = scan("a,b\n1,\"x\"y\n").expect_err("text after quote");
        assert!(matches!(err, Fatal::TextAfterQuote { line: 2, column: 2 }), "{err:?}");
        assert_eq!(
            err.render(),
            "error: line 2, column 2: text follows a closing quote; inside quotes, write a quote as \"\""
        );
        // The closing quote's line, not the record's (Global Constraints).
        let err = scan("a,b\n\"x\ny\"z,1\n").expect_err("text after quote");
        assert!(matches!(err, Fatal::TextAfterQuote { line: 3, column: 1 }), "{err:?}");
    }

    /// Every string of up to `max_len` characters over `alphabet`, shortest
    /// first. Exhaustive rather than random: no dependency, no flaky seed.
    fn all_texts(alphabet: &[char], max_len: u32) -> impl Iterator<Item = String> + '_ {
        (0..=max_len).flat_map(move |len| {
            (0..alphabet.len().pow(len)).map(move |mut i| {
                (0..len)
                    .map(|_| {
                        let c = alphabet[i % alphabet.len()];
                        i /= alphabet.len();
                        c
                    })
                    .collect()
            })
        })
    }

    /// About 19,500 inputs: scan never panics and fails only on quoting;
    /// lines only rise; without quotes, lines are an editor's split on LF,
    /// CRLF and CR, with blank records (here, those without an `a`) skipped.
    #[test]
    fn small_inputs_scan_cleanly_and_lines_only_rise() {
        for text in all_texts(&['a', ',', '"', '\n', '\r'], 6) {
            match scan(&text) {
                Ok(records) => {
                    assert!(records.windows(2).all(|p| p[0].line < p[1].line), "{text:?}");
                    if !text.contains('"') {
                        let norm = text.replace("\r\n", "\n").replace('\r', "\n");
                        let expected: Vec<usize> = norm
                            .split('\n')
                            .enumerate()
                            .filter(|(_, l)| l.contains('a'))
                            .map(|(i, _)| i + 1)
                            .collect();
                        let got: Vec<usize> = records.iter().map(|r| r.line).collect();
                        assert_eq!(got, expected, "{text:?}");
                    }
                }
                Err(e) => assert!(
                    matches!(e, Fatal::UnterminatedQuote { .. } | Fatal::TextAfterQuote { .. }),
                    "{text:?}: {e:?}"
                ),
            }
        }
    }

    /// RFC 4180 round trip: any field, quoted with its quotes doubled as
    /// Excel writes it, scans back unchanged.
    #[test]
    fn excel_quoted_fields_round_trip() {
        for field in all_texts(&['a', ',', '"', '\n', '\r', ' '], 4) {
            let written = format!("\"{}\",x\r\n", field.replace('"', "\"\""));
            let records = scan(&written).expect("scans");
            assert_eq!(records.len(), 1, "{written:?}");
            assert_eq!(records[0].fields, [field.clone(), "x".to_string()], "{written:?}");
        }
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib csv::parse`
Expected: a compile error, `cannot find function scan`.

- [ ] **Step 3: Implement**

Add to `Fatal` in `src/diagnostics.rs`, after `NotUtf8`:

```rust
    /// NFR-15: an unterminated quote swallows the rest of the file into one
    /// field, so nothing after it parses. `line` is where the quote opened,
    /// which may be after the record's first line (plan RG-005, Decisions).
    #[error(
        "line {line}, column {column}: a quote opens this value and never closes; add the closing quote, and write any quote inside the value as \"\""
    )]
    UnterminatedQuote { line: usize, column: usize },
    /// NFR-15: text after a closing quote. RFC 4180 doubles a quote inside a
    /// quoted field; keeping or dropping the stray text would both guess.
    /// `line` is the closing quote's.
    #[error(
        "line {line}, column {column}: text follows a closing quote; inside quotes, write a quote as \"\""
    )]
    TextAfterQuote { line: usize, column: usize },
```

Put this above the test module in `src/csv/parse.rs`:

```rust
//! Text to records (FR-1, FR-3, FR-4, FR-90).
//!
//! RFC 4180 by hand rather than through the `csv` crate, which numbered CRLF
//! lines wrongly and accepted an unterminated quote silently — NFR-10 and
//! NFR-15 both turn on exactly those (plan RG-005, Decisions).

use std::iter::Peekable;
use std::str::Chars;

use crate::diagnostics::Fatal;

/// One record and where it begins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The 1-based physical line the record begins on, the header's line
    /// counting as 1 (NFR-10, D-37). A quoted newline makes this run ahead
    /// of the record's ordinal, which is the point: an editor can go to it.
    pub line: usize,
    /// The fields as written: unquoted, unescaped, untrimmed.
    pub fields: Vec<String>,
}

/// What ended a field.
enum End {
    Comma,
    Line,
    Eof,
}

/// Characters with a running physical line count.
struct Scanner<'a> {
    chars: Peekable<Chars<'a>>,
    line: usize,
}

impl Scanner<'_> {
    /// Consume one line break — LF, CRLF or a lone CR — if one is next,
    /// and count it.
    fn line_break(&mut self) -> bool {
        match self.chars.peek() {
            Some('\n') => {
                self.chars.next();
            }
            Some('\r') => {
                self.chars.next();
                self.chars.next_if_eq(&'\n');
            }
            _ => return false,
        }
        self.line += 1;
        true
    }

    /// Consume whatever ends a field, if a field may end here.
    fn end(&mut self) -> Option<End> {
        if self.chars.next_if_eq(&',').is_some() {
            Some(End::Comma)
        } else if self.line_break() {
            Some(End::Line)
        } else if self.chars.peek().is_none() {
            Some(End::Eof)
        } else {
            None
        }
    }

    /// One field and what ended it. `column` is its 1-based position, for
    /// the error message.
    fn field(&mut self, column: usize) -> Result<(String, End), Fatal> {
        let mut text = String::new();
        if self.chars.next_if_eq(&'"').is_some() {
            let opened = self.line;
            loop {
                match self.chars.next() {
                    None => return Err(Fatal::UnterminatedQuote { line: opened, column }),
                    Some('"') if self.chars.next_if_eq(&'"').is_some() => text.push('"'),
                    Some('"') => break,
                    Some(c) => {
                        // A break inside quotes is content, and still a
                        // physical line; CRLF is counted at its LF.
                        if c == '\n' || (c == '\r' && self.chars.peek() != Some(&'\n')) {
                            self.line += 1;
                        }
                        text.push(c);
                    }
                }
            }
            return self.end().map(|end| (text, end)).ok_or(Fatal::TextAfterQuote {
                line: self.line,
                column,
            });
        }
        loop {
            if let Some(end) = self.end() {
                return Ok((text, end));
            }
            // `end` returned `None`, so a character follows. A quote inside
            // an unquoted field is kept literally (Decisions).
            if let Some(c) = self.chars.next() {
                text.push(c);
            }
        }
    }
}

/// Decisions: a record whose every field is empty or only spaces and tabs
/// looks blank to the author — an empty line, or Excel's `,,,` for a
/// cleared row. Read before FR-90's removal, so U+0001 is not blank.
fn is_blank(fields: &[String]) -> bool {
    fields
        .iter()
        .all(|f| f.chars().all(|c| c == ' ' || c == '\t'))
}

/// RFC 4180 records, each with the physical line it begins on. A blank
/// record is skipped, though its lines are still counted.
///
/// # Errors
///
/// [`Fatal::UnterminatedQuote`] or [`Fatal::TextAfterQuote`]: malformed
/// quoting, after which nothing parses (NFR-15, NFR-16 tier one).
fn scan(text: &str) -> Result<Vec<Record>, Fatal> {
    let mut s = Scanner {
        chars: text.chars().peekable(),
        line: 1,
    };
    let mut records = Vec::new();
    while s.chars.peek().is_some() {
        let line = s.line;
        let mut fields = Vec::new();
        loop {
            let (field, end) = s.field(fields.len() + 1)?;
            fields.push(field);
            if !matches!(end, End::Comma) {
                break;
            }
        }
        if !is_blank(&fields) {
            records.push(Record { line, fields });
        }
    }
    Ok(records)
}
```

An empty line needs no special case: it scans as one empty field, which `is_blank` skips, and `line_break` has already counted it.

- [ ] **Step 4: Run the tests**

Run: `cargo fmt && cargo test --lib csv::parse`
Expected: 12 passed. Clippy is not run here: it fails on dead code until Task 3 (see the note at the top of this task).

- [ ] **Step 5: No commit**

Leave the work uncommitted; Task 3 Step 5 commits it.

---

### Task 3: `parse` — FR-90 removal, the header and field counts

**Files:**
- Modify: `src/csv/parse.rs` (append the public `parse` and `Table`, extend the tests)
- Modify: `src/diagnostics.rs` (add `NoHeader`, `NotCommaSeparated`, `DuplicateHeader` and `FieldCount` to `Fatal`)
- Modify: `tests/diagnostics.rs` (add Tasks 2–3's six variants to `csv_fatal_messages`)
- Create: `tests/csv.rs`
- Modify: `docs/design/architecture.md` (§2 diagram and purity note, §3 rows, §7 signatures)

**Interfaces:**
- Consumes: `scan` and `Record` (Task 2); `escape::displayable` (exists); `normalise` (exists); `diagnostics::{Warning, RowKind, Warnings}` (exist).
- Produces, in `radargen::csv::parse`:
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct Table { pub header: Vec<String>, pub header_line: usize, pub records: Vec<Record> }`
  - `pub fn parse(text: &str, w: &mut Warnings) -> Result<(Table, Vec<Fatal>), Fatal>` — `Err` is tier one (malformed quoting, no header, not comma-separated). `Ok` carries the table of well-formed records **and** every FR-4 error (tier two).
- And in `Fatal`: `NoHeader`, `NotCommaSeparated { line: usize, separator: &'static str }`, `DuplicateHeader { line: usize, index: usize, name: String, first_index: usize, first_name: String }`, `FieldCount { line: usize, found: usize, expected: usize }`.

  PR 06's `build_catalogue(table: &Table, w: &mut Warnings) -> Result<Catalogue, Vec<Fatal>>` consumes `Table`. It maps header names to columns under FR-76 (FR-5) and keeps `header`'s order for the table view (FR-78). Every `Record` has exactly `header.len()` fields. A row that failed FR-4 is absent from `records`, but its error is in parse's `Vec<Fatal>`. `main.rs` (PR 13) merges parse's and `build_catalogue`'s errors; their order is PR 13's to fix (NFR-11) before SC-10 pins it.

- [ ] **Step 1: Write the failing tests**

Append inside `mod tests` (`use super::*` already brings in `Warnings` and `Warning` once Step 3's imports exist):

```rust
    /// `parse`'s outputs, rendered: the table or its tier-one error, the
    /// tier-two errors, and the warnings.
    fn parsed(text: &str) -> (Result<Table, Fatal>, Vec<String>, Vec<String>) {
        let mut w = Warnings::default();
        match parse(text, &mut w) {
            Ok((table, errs)) => (Ok(table), errs.iter().map(Fatal::render).collect(), w.render()),
            Err(e) => (Err(e), Vec::new(), w.render()),
        }
    }

    /// Every error, of either tier.
    fn errors(text: &str) -> Vec<String> {
        match parsed(text) {
            (Err(e), _, _) => vec![e.render()],
            (Ok(_), errs, _) => errs,
        }
    }

    /// A table that parsed with no error of either tier.
    fn table(text: &str) -> Table {
        let (table, errs, _) = parsed(text);
        assert!(errs.is_empty(), "{errs:?}");
        table.expect("parses")
    }

    const MATCH: &str =
        "; header names match ignoring case and surrounding spaces, so rename or remove one";
    const FEWER: &str = "; a value is missing, or an unquoted line break split the row";
    const MORE: &str = "; a comma inside a value needs the value in double quotes";

    #[test]
    fn a_table_keeps_the_header_and_every_record() {
        let (table, errs, warnings) = parsed("name,ring\nKafka,Adopt\nRedis,Trial\n");
        let table = table.expect("parses");
        assert_eq!(table.header, ["name", "ring"]);
        assert_eq!(table.header_line, 1);
        assert_eq!(table.records.len(), 2);
        assert_eq!(table.records[1].line, 3);
        assert!(errs.is_empty() && warnings.is_empty(), "{errs:?} {warnings:?}");
    }

    #[test]
    fn a_header_only_file_parses_to_no_records() {
        assert_eq!(table("name,ring\n").records.len(), 0);
    }

    /// Decisions: the header is the first non-blank record, on its physical line.
    #[test]
    fn leading_blank_lines_move_the_header_line() {
        let t = table("\n,,\nname,ring\nKafka,Adopt\n");
        assert_eq!((t.header_line, t.records[0].line), (3, 4));
        assert_eq!(errors("\n\na,a\n"), [format!("error: line 3: column 2 'a' repeats column 1 'a'{MATCH}")]);
    }

    #[test]
    fn an_empty_file_has_no_header() {
        for text in ["", "\n\n", "\r\n", ",,,\n", "  \n"] {
            assert_eq!(
                errors(text),
                ["error: the CSV has no header row (it is empty or blank); the header must name at least name, quadrant, ring and description"],
                "{text:?}"
            );
        }
    }

    /// FR-1, Review focus 6: a decimal-comma locale's Excel writes `;`.
    #[test]
    fn a_semicolon_export_is_named_as_such() {
        assert_eq!(
            errors("name;ring\nKafka;Adopt, mostly\n"),
            ["error: line 1: the header is one column holding semicolons; radargen reads comma-separated files only, so export the file with commas as the separator"]
        );
        assert_eq!(
            errors("name\tring\nKafka\tAdopt\n"),
            ["error: line 1: the header is one column holding tabs; radargen reads comma-separated files only, so export the file with commas as the separator"]
        );
        assert!(errors("name\nKafka\n").is_empty(), "one plain column is PR 06's FR-5 to judge");
    }

    /// FR-4 under FR-76: `Name` and `name ` collide. Every one is reported.
    #[test]
    fn every_duplicate_header_is_reported() {
        assert_eq!(
            errors("name,ring,Name ,RING,x\n"),
            [
                format!("error: line 1: column 3 'Name' repeats column 1 'name'{MATCH}"),
                format!("error: line 1: column 4 'RING' repeats column 2 'ring'{MATCH}"),
            ]
        );
    }

    /// Decisions, Review focus 6: Excel pads the header to its used range;
    /// empty cells name nothing.
    #[test]
    fn empty_header_cells_are_not_duplicates() {
        assert_eq!(table("name,ring,,\nKafka,Adopt,,\n").header, ["name", "ring", "", ""]);
    }

    /// FR-4, NFR-16 tier two: every bad row at once, in line order.
    #[test]
    fn every_field_count_mismatch_is_reported() {
        assert_eq!(
            errors("a,b\n1,2\n1\n1,2,3\n\"multi\nline\",2,3\n"),
            [
                format!("error: line 3: the row's field count is 1 but the header's is 2{FEWER}"),
                format!("error: line 4: the row's field count is 3 but the header's is 2{MORE}"),
                format!("error: line 5: the row's field count is 3 but the header's is 2{MORE}"),
            ]
        );
    }

    /// Review focus 7: a bad row is reported, and the good rows still reach
    /// PR 06, so its FR-5/FR-6 errors arrive in the same run.
    #[test]
    fn a_bad_row_does_not_hide_the_good_ones() {
        let (table, errs, _) = parsed("a,b\n1\n2,3\n");
        assert_eq!(errs, [format!("error: line 2: the row's field count is 1 but the header's is 2{FEWER}")]);
        let table = table.expect("structural");
        assert_eq!(table.records.len(), 1);
        assert_eq!(table.records[0].line, 3);
    }

    #[test]
    fn duplicate_headers_and_bad_rows_are_reported_together() {
        assert_eq!(
            errors("a,A\n1\n"),
            [
                format!("error: line 1: column 2 'A' repeats column 1 'a'{MATCH}"),
                format!("error: line 2: the row's field count is 1 but the header's is 2{FEWER}"),
            ],
            "a duplicate header does not hide a bad row, and comes first"
        );
    }

    /// Malformed quoting stops the run: nothing after it can be trusted.
    #[test]
    fn malformed_quoting_is_the_only_error_reported() {
        assert_eq!(
            errors("a,a\n1\n\"open\n"),
            ["error: line 3, column 1: a quote opens this value and never closes; add the closing quote, and write any quote inside the value as \"\""]
        );
    }

    /// FR-90, D-55: removed, with one warning per field naming line and column.
    #[test]
    fn undisplayable_characters_are_removed_and_warned() {
        let (table, _, warnings) = parsed("name,description\nKaf\u{1}ka,ok\n");
        assert_eq!(table.expect("parses").records[0].fields, ["Kafka", "ok"]);
        assert_eq!(
            warnings,
            ["warning: line 2, name: removed U+0001, which the output cannot display"]
        );
        // Several fields group into one line (NFR-20), each naming its column.
        let (table, _, warnings) = parsed("name,description\nKaf\u{1}ka,a\u{7f}b\u{85}c\n");
        assert_eq!(table.expect("parses").records[0].fields, ["Kafka", "abc"]);
        assert_eq!(
            warnings,
            [
                "warning: 2 fields had characters the output cannot display — name on line 2 (U+0001), description on line 2 (2, first U+007F); each was removed"
            ]
        );
    }

    /// PR 02's prefix rule: a row warning names its column in canonical
    /// form, so ` Name ` reads `name`, as FR-6 and FR-28 will.
    #[test]
    fn the_warning_names_the_column_in_canonical_form() {
        let (_, _, warnings) = parsed(" Name ,ring\nKaf\u{1}ka,Adopt\n");
        assert_eq!(
            warnings,
            ["warning: line 2, name: removed U+0001, which the output cannot display"]
        );
    }

    /// FR-90's whole set: NUL and the two noncharacters go too.
    #[test]
    fn nul_and_noncharacters_are_removed() {
        let (table, _, warnings) = parsed("a\n\u{0}x\u{fffe}\u{ffff}y\n");
        assert_eq!(table.expect("parses").records[0].fields, ["xy"]);
        assert_eq!(
            warnings,
            ["warning: line 2, a: removed 3 characters the output cannot display, the first U+0000"]
        );
    }

    /// NFR-11 orders a line's warnings by column, so the index must be the
    /// field's, 0-based as `Position::Cell` documents.
    #[test]
    fn a_warning_carries_its_line_and_column_index() {
        let mut w = Warnings::default();
        parse("a,b,c\n1,2\u{1},3\n", &mut w).expect("parses");
        assert!(
            matches!(w.as_slice(), [Warning::Row { line: 2, column: 1, .. }]),
            "{:?}",
            w.as_slice()
        );
    }

    /// SC-10: a cell of only such characters is empty, for FR-6 to reject.
    #[test]
    fn a_cell_of_only_undisplayable_characters_becomes_empty() {
        let (table, _, _) = parsed("name,ring\n\u{1}\u{2},Adopt\n");
        assert_eq!(table.expect("parses").records[0].fields, ["", "Adopt"]);
    }

    /// Review focus 5: removal comes first, so the duplicate is the one the
    /// author sees in any editor that hides the character.
    #[test]
    fn removal_runs_before_the_duplicate_header_check() {
        let (_, errs, warnings) = parsed("name,name\u{1}\nx,y\n");
        assert_eq!(errs, [format!("error: line 1: column 2 'name' repeats column 1 'name'{MATCH}")]);
        assert_eq!(
            warnings,
            ["warning: line 1, name: removed U+0001, which the output cannot display"]
        );
    }

    /// Tab, LF and CR are displayable; they survive.
    #[test]
    fn tab_and_line_breaks_survive_removal() {
        let (table, _, warnings) = parsed("a\n\"x\ty\r\nz\"\n");
        assert_eq!(table.expect("parses").records[0].fields, ["x\ty\r\nz"]);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// A field past the header's end, or under an empty header cell, has no
    /// name to give; it gets its position instead.
    #[test]
    fn an_unnamed_column_is_named_by_position() {
        let (_, _, warnings) = parsed("name,\nx,\u{1}\n");
        assert_eq!(
            warnings,
            ["warning: line 2, column 2: removed U+0001, which the output cannot display"]
        );
    }

    /// Decisions: a field-count error does not hide the row's FR-90
    /// warnings, and a field past the header's end is named by position.
    #[test]
    fn a_short_or_long_row_still_warns_about_removal() {
        let (_, errs, warnings) = parsed("a,b\n\u{1}\nx,\u{2},\u{3}\n");
        assert_eq!(
            errs,
            [
                format!("error: line 2: the row's field count is 1 but the header's is 2{FEWER}"),
                format!("error: line 3: the row's field count is 3 but the header's is 2{MORE}"),
            ]
        );
        assert_eq!(
            warnings,
            ["warning: 3 fields had characters the output cannot display — a on line 2 (U+0001), b on line 3 (U+0002), column 3 on line 3 (U+0003); each was removed"]
        );
    }
```

Create `tests/csv.rs`:

```rust
//! `csv::decode` and `csv::parse` through the public API (FR-1, FR-2, NFR-10).

use radargen::csv::decode::{decode, read};
use radargen::csv::parse::parse;
use radargen::diagnostics::Warnings;

/// Review focus 1, end to end: BOM, CRLF, a cell with Alt+Enter, a blank
/// trailing line, and a cleared `,` row.
#[test]
fn an_excel_export_reads_and_parses_with_editor_line_numbers() {
    let path = std::env::temp_dir().join(format!("radargen-excel-{}.csv", std::process::id()));
    std::fs::write(
        &path,
        b"\xEF\xBB\xBFname,description\r\nKafka,\"Streams,\nlogs\"\r\nRedis,\"Cache\"\r\n,\r\n\r\n",
    )
    .expect("write fixture");
    let text = read(&path);
    std::fs::remove_file(&path).expect("remove fixture");
    let mut w = Warnings::default();
    let (table, errors) = parse(&text.expect("reads"), &mut w).expect("parses");
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(table.header, ["name", "description"]);
    let got: Vec<(usize, Vec<String>)> = table.records.into_iter().map(|r| (r.line, r.fields)).collect();
    assert_eq!(
        got,
        [
            (2, vec!["Kafka".to_string(), "Streams,\nlogs".to_string()]),
            (4, vec!["Redis".to_string(), "Cache".to_string()]),
        ]
    );
    assert!(w.is_empty());
}

/// `line_at`'s promise: decode and parse count LF, CRLF and lone CR alike,
/// so a bad byte on a record's first line is reported at that record's line.
#[test]
fn decode_and_parse_agree_on_line_numbers() {
    let mut bytes = b"h\r\n\r\n\"x\ry\nz\"\r\nq\r".to_vec();
    let mut text = String::from_utf8(bytes.clone()).expect("ascii");
    text.push('w');
    let mut w = Warnings::default();
    let (table, _) = parse(&text, &mut w).expect("parses");
    let last = table.records.last().expect("a record").line;
    bytes.push(0xE9);
    assert_eq!(last, 7);
    assert_eq!(decode(&bytes).map_err(|e| e.line), Err(last));
}

/// An empty sheet saved from Excel is a BOM and nothing else.
#[test]
fn a_bom_only_file_has_no_header() {
    let text = decode(b"\xEF\xBB\xBF").expect("decodes");
    let mut w = Warnings::default();
    let err = parse(&text, &mut w).expect_err("no header");
    assert_eq!(
        err.render(),
        "error: the CSV has no header row (it is empty or blank); the header must name at least name, quadrant, ring and description"
    );
}
```

Append to the `cases` array in `tests/diagnostics.rs::csv_fatal_messages`:

```rust
        (
            Fatal::NoHeader,
            "error: the CSV has no header row (it is empty or blank); the header must name at least name, quadrant, ring and description",
        ),
        (
            Fatal::NotCommaSeparated {
                line: 1,
                separator: "semicolons",
            },
            "error: line 1: the header is one column holding semicolons; radargen reads comma-separated files only, so export the file with commas as the separator",
        ),
        (
            Fatal::UnterminatedQuote { line: 3, column: 2 },
            "error: line 3, column 2: a quote opens this value and never closes; add the closing quote, and write any quote inside the value as \"\"",
        ),
        (
            Fatal::TextAfterQuote { line: 2, column: 2 },
            "error: line 2, column 2: text follows a closing quote; inside quotes, write a quote as \"\"",
        ),
        (
            Fatal::DuplicateHeader {
                line: 1,
                index: 3,
                name: "Name".into(),
                first_index: 1,
                first_name: "name".into(),
            },
            "error: line 1: column 3 'Name' repeats column 1 'name'; header names match ignoring case and surrounding spaces, so rename or remove one",
        ),
        // A header is CSV text: a quoted newline in it must not forge a line.
        (
            Fatal::DuplicateHeader {
                line: 1,
                index: 2,
                name: "a\nerror: forged".into(),
                first_index: 1,
                first_name: "a".into(),
            },
            r"error: line 1: column 2 'a\nerror: forged' repeats column 1 'a'; header names match ignoring case and surrounding spaces, so rename or remove one",
        ),
        (
            Fatal::FieldCount {
                line: 4,
                found: 3,
                expected: 2,
            },
            "error: line 4: the row's field count is 3 but the header's is 2; a comma inside a value needs the value in double quotes",
        ),
        (
            Fatal::FieldCount {
                line: 4,
                found: 1,
                expected: 2,
            },
            "error: line 4: the row's field count is 1 but the header's is 2; a value is missing, or an unquoted line break split the row",
        ),
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib csv::parse`
Expected: a compile error, `cannot find function parse`.

- [ ] **Step 3: Implement**

Add to `Fatal` in `src/diagnostics.rs`, after `TextAfterQuote`:

```rust
    /// FR-3: the file has no non-blank records, so no header names a column.
    /// The required columns are FR-5's, named so the message is actionable.
    #[error(
        "the CSV has no header row (it is empty or blank); the header must name at least name, quadrant, ring and description"
    )]
    NoHeader,
    /// FR-1: a one-column header holding `;` or a tab is a file with another
    /// separator — Excel writes `;` where a comma is the decimal mark.
    /// Without this, every row would mis-report its field count.
    #[error(
        "line {line}: the header is one column holding {separator}; radargen reads comma-separated files only, so export the file with commas as the separator"
    )]
    NotCommaSeparated {
        line: usize,
        separator: &'static str,
    },
    /// FR-4: a header name repeated under FR-76's normalisation, so
    /// `Name` and `name ` collide. Indices are 1-based; names are as
    /// written, trimmed, so the author can find both cells.
    #[error(
        "line {line}: column {index} '{name}' repeats column {first_index} '{first_name}'; header names match ignoring case and surrounding spaces, so rename or remove one"
    )]
    DuplicateHeader {
        line: usize,
        index: usize,
        name: String,
        first_index: usize,
        first_name: String,
    },
    /// FR-4: a record whose field count is not the header's, with the usual
    /// cause for each direction (FR-61's precedent: say what to do).
    #[error(
        "line {line}: the row's field count is {found} but the header's is {expected}; {}",
        if found > expected {
            "a comma inside a value needs the value in double quotes"
        } else {
            "a value is missing, or an unquoted line break split the row"
        }
    )]
    FieldCount {
        line: usize,
        found: usize,
        expected: usize,
    },
```

(`thiserror` 2 binds every field by reference inside the format arguments, so `found > expected` compares `&usize` with `&usize`. If the compiler disagrees, write the hint as a private `fn field_count_hint(found: usize, expected: usize) -> &'static str` and pass `field_count_hint(*found, *expected)`.)

In `src/csv/parse.rs`, extend the imports to:

```rust
use crate::diagnostics::{Fatal, RowKind, Warning, Warnings};
use crate::escape::displayable;
use crate::normalise;
```

and append above the test module:

```rust
/// The parsed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// Column names as written, after FR-90's removal, in declared order.
    pub header: Vec<String>,
    /// The header's physical line: 1, unless the file opens with blank lines.
    pub header_line: usize,
    /// Every well-formed data record; each has exactly `header.len()` fields.
    pub records: Vec<Record>,
}

/// FR-3, FR-4 and FR-90: the header and records, with undisplayable
/// characters removed — and warned about — before anything reads a field.
///
/// The `Vec<Fatal>` beside the table is NFR-16 tier two: every duplicate
/// header and every field-count mismatch, in line order. A row that fails
/// its field count is left out of `records`; the rest still reach PR 06, so
/// its row errors join these in the same run.
///
/// # Errors
///
/// Tier one, alone, since nothing after it parses: malformed quoting, an
/// empty file, or a header that shows another separator was used.
pub fn parse(text: &str, w: &mut Warnings) -> Result<(Table, Vec<Fatal>), Fatal> {
    let mut rows = scan(text)?.into_iter();
    let Some(head) = rows.next() else {
        return Err(Fatal::NoHeader);
    };
    let (head, removed) = cleaned(head);
    let header = head.fields;
    warn(w, head.line, &removed, &header);
    if let Some(separator) = other_separator(&header) {
        return Err(Fatal::NotCommaSeparated {
            line: head.line,
            separator,
        });
    }

    let mut errors = duplicate_headers(&header, head.line);
    let mut records = Vec::new();
    for row in rows {
        let (row, removed) = cleaned(row);
        warn(w, row.line, &removed, &header);
        if row.fields.len() == header.len() {
            records.push(row);
        } else {
            errors.push(Fatal::FieldCount {
                line: row.line,
                found: row.fields.len(),
                expected: header.len(),
            });
        }
    }
    let table = Table {
        header,
        header_line: head.line,
        records,
    };
    Ok((table, errors))
}

/// FR-1: the separator a one-column header shows was used instead of a
/// comma, if any. A one-column header can never satisfy FR-5, so naming
/// the real cause loses nothing (Decisions).
fn other_separator(header: &[String]) -> Option<&'static str> {
    let [only] = header else {
        return None;
    };
    if only.contains(';') {
        Some("semicolons")
    } else if only.contains('\t') {
        Some("tabs")
    } else {
        None
    }
}

/// FR-90: `record` with every undisplayable character removed, and for each
/// field that lost any, its index, the first removed character and how many.
fn cleaned(record: Record) -> (Record, Vec<(usize, char, usize)>) {
    let mut removed = Vec::new();
    let fields = record
        .fields
        .into_iter()
        .enumerate()
        .map(|(i, field)| {
            let Some(first) = field.chars().find(|&c| !displayable(c)) else {
                return field;
            };
            let count = field.chars().filter(|&c| !displayable(c)).count();
            removed.push((i, first, count));
            field.chars().filter(|&c| displayable(c)).collect()
        })
        .collect();
    (
        Record {
            line: record.line,
            fields,
        },
        removed,
    )
}

/// One FR-90 warning per field that lost characters. The column is named
/// in canonical form, as every row warning names it (PR 02), or by
/// position when the header has no name for it.
fn warn(w: &mut Warnings, line: usize, removed: &[(usize, char, usize)], header: &[String]) {
    for &(column, first, count) in removed {
        let name = header
            .get(column)
            .map(String::as_str)
            .map(normalise)
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("column {}", column + 1));
        w.push(Warning::Row {
            line,
            column,
            kind: RowKind::Undisplayable {
                column: name,
                first,
                count,
            },
        });
    }
}

/// FR-4: every header name repeated under FR-76, naming the column it
/// repeats. An empty cell names nothing, so it cannot repeat a name; Excel
/// pads the header with these (Decisions).
fn duplicate_headers(header: &[String], line: usize) -> Vec<Fatal> {
    let mut seen: Vec<(String, usize, &str)> = Vec::new();
    let mut errors = Vec::new();
    for (i, name) in header.iter().enumerate() {
        let key = normalise(name);
        if key.is_empty() {
            continue;
        }
        if let Some((_, first, first_name)) = seen.iter().find(|(k, ..)| *k == key) {
            errors.push(Fatal::DuplicateHeader {
                line,
                index: i + 1,
                name: name.trim().to_string(),
                first_index: *first,
                first_name: (*first_name).to_string(),
            });
        } else {
            seen.push((key, i + 1, name.trim()));
        }
    }
    errors
}
```

In `docs/design/architecture.md`:
- **§2:** replace the diagram's first two stages (from `bytes` down to, not including, the `Catalogue` line) with:

  ```text
    bytes                                                       §6.1
      │  decode       strip BOM, require UTF-8                  FR-1, FR-2, FR-61
      ▼
    String
      │  parse        FR-90 removal, header, field counts       FR-3, FR-4, FR-90
      ▼
    Table             header and records; FR-4 errors beside it
      │  model        map columns, check required values        FR-5, FR-6
      ▼
  ```

  After "Each stage is a pure function of its input plus the diagnostics it may add." add: "The one exception is `csv::decode::read`, which reads the input file so its errors can name the path (NFR-10); `decode` itself is pure."
- **§3:** in the `src/csv/decode.rs` row, append "; `read` is the stage's one file read". In the `src/csv/parse.rs` row, change "String → records" to "String → `Table` (header and records)".
- **§7:** change the first signature in the code block to `pub fn build_catalogue(table: &Table, w: &mut Warnings) -> Result<Catalogue, Vec<Fatal>>;`. After the paragraph on tier two, add: "`csv::parse` returns `Result<(Table, Vec<Fatal>), Fatal>`. Tier one is the `Err`. Its tier-two FR-4 errors travel beside the table rather than instead of it, so they join `build_catalogue`'s in the same run."

- [ ] **Step 4: Run everything**

Run: `cargo fmt && cargo fmt --check && cargo test && cargo clippy --all-targets -- -D warnings && just lint`
Expected: all pass, including `tests/csv.rs` and `csv_fatal_messages`; both lints clean (Task 2's dead-code warnings are gone).

- [ ] **Step 5: Commit**

Tasks 2 and 3 land together:

```bash
git add src/csv/mod.rs src/csv/parse.rs src/diagnostics.rs tests/diagnostics.rs tests/csv.rs docs/design/architecture.md
git commit -m "feat(csv): parse — RFC 4180 scanner, FR-90, headers, field counts"
```

---

## Finishing the PR

Follow the house workflow in `~/.claude/CLAUDE.md`. For this PR:

1. Run `crumbs start rg-aqb` before Task 1. Work on branch `feat/pr-05-csv`, created from `main`.
2. After Task 3: push, then open the PR with `gh pr create` and **no reviewer flag**.
3. Run the code-reviewer skill until clean, then the Copilot step (this repo has no `copilot_code_review` ruleset, so a local multi-agent review substitutes, and the PR body says so). Then run simplify, then code-reviewer once more.
4. Close `rg-aqb` with `crumbs close rg-aqb`, which also stops the timer `crumbs start` began; do not hand-edit the file. **No version bump:** nothing a user runs changes.
5. Move this file to `docs/plans/done/` in the same PR. Merge; there's no tag.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-10 | Initial version. The `csv` crate was measured and rejected (CRLF line numbers, silent unterminated quote). All three tasks were applied to `main` in a scratch worktree before review: 142 tests pass, and `cargo clippy --all-targets -- -D warnings` and `just lint` are clean |
| 1.1 | 2026-10-10 | Reviewed by architect, Rust, QA, UX and UI agents. **`parse` returns FR-4 errors beside the table**, so PR 06's FR-5/FR-6 errors arrive in the same run (NFR-16, `row-errors.csv`). Excel quirks handled: blank and `,,,` rows skipped, empty header cells are not duplicates, a `;`- or tab-separated file is refused at its header. Quoting errors name the faulty line by design, and their column. Every new message says what to do, and says "column", not "field". FR-90 warnings name columns canonically (PR 02). Tasks 2 and 3 share one commit, so no commit fails clippy, and `cargo fmt` is in every run step. Added a `csv_fatal_messages` catalogue test (kept apart from `fatal_messages`, which would exceed pedantic's 100-line `too_many_lines`), `tests/csv.rs`, exhaustive scanner checks, and decode edge cases (UTF-16, truncated sequence, directory). Spaces around quotes are declined (RFC 4180). Error grouping is deferred to `rg-pcd`. Applied to `main` in a scratch worktree: 158 tests pass; `cargo fmt --check`, clippy and `just lint` are clean |
| 1.2 | 2026-10-10 | PR #17 review: the PRD departures (blank records, quoting-error lines, the separator check) recorded as crumb `rg-lfd`, and the PR 06 handoff appended to `rg-rnc`. The error order across parse and `build_catalogue` is left to PR 13 (NFR-11). The architecture §2 edit is spelled out against the actual diagram, with `read`'s exception to stage purity. Review metadata filled in; minor wording fixes |
