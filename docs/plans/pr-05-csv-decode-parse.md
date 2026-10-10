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
synopsis: "Bytes to text (BOM, UTF-8, FR-61's line-numbered error) and text to records (a hand-written RFC 4180 scanner with physical line numbers, header and field-count checks, and FR-90's removal of undisplayable characters)."
status: draft
type: Plan
revision: "1.0"
review_date:
reviewed_by: []
completed_date:
comments:
---
# PR 05 — `csv/decode` and `csv/parse`: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-aqb` · **Phase:** v1.0 · **Blocks:** `rg-rnc` (PR 06)

**Goal:** Turn the input file into a header and a list of records, each carrying the physical line it begins on, with every structural problem reported as NFR-16 requires and every undisplayable character removed and warned about before anything else reads a field.

**Architecture:** Two stages under `src/csv/` (architecture §3). `decode.rs` is bytes → `String`: strip one BOM (FR-2), require UTF-8 (FR-1), and on failure name the physical line and say to re-save as UTF-8 (FR-61, D-27). `parse.rs` is `String` → `Table`: a small hand-written RFC 4180 scanner that tracks the physical line each record begins on (NFR-10, D-37), then FR-90's removal on every field with a warning, then FR-3/FR-4's duplicate-header and field-count checks, collected across the file (NFR-16 tier two). Quoting errors abort at once, because nothing after them parses. Errors are new `diagnostics::Fatal` variants, so there is still one taxonomy (§1); warnings go through `&mut Warnings` (§7).

**Tech Stack:** Rust 2024, standard library only. No new dependencies — see Decisions for why not the `csv` crate.

**Spec:** `docs/product/radargen-prd.md` v0.40 — FR-1, FR-2, FR-3, FR-4, FR-61, FR-76, FR-90, NFR-9, NFR-10, NFR-15, NFR-16, D-27, D-37, D-55, SC-10. Design: `docs/design/architecture.md` §3 (the `csv/decode.rs` and `csv/parse.rs` rows), §7 (diagnostics threading).

## Global Constraints

- **Physical line numbers** (NFR-10, D-37): every diagnostic names the 1-based physical line a record **begins** on, header counting as line 1. A quoted newline makes later records' lines run ahead of their ordinals, permanently.
- **A line break is LF, CRLF or a lone CR**, counted once each, in both `decode` and `parse` — the way an editor's Go To Line counts.
- **FR-90 removal comes first** (D-55): a field's undisplayable characters (`!escape::displayable`) are removed before any check reads it, with one `RowKind::Undisplayable` warning per affected field. Use `escape::displayable`; never restate the set.
- **NFR-16:** file-level errors (unreadable, not UTF-8, malformed quoting, no header) return one `Fatal` and stop; duplicate headers and field-count mismatches are **all** collected and returned together, in line order.
- **No trimming here.** Fields keep their whitespace; FR-76 trims at comparison time (PR 06), and displayed text is never edited beyond FR-15/FR-76.
- **One taxonomy:** new errors are `diagnostics::Fatal` variants (architecture §1). An I/O variant carries the path and marks the error `#[source]`, not `#[from]` (the `Fatal` doc comment).
- `cargo clippy --all-targets -- -D warnings` and `just lint` (pedantic + nursery + `unwrap_used`) are both clean. No `#[allow]`/`#[expect]` without an inline reason.
- Commits: `type(scope): description` (subject ≤ 72 characters, body lines ≤ 72), plus the session's `Co-Authored-By` trailer. Run `git-mit es` first.

## Review Focus

1. **An Excel export with CRLF line endings.** Every diagnostic must name the line the author's editor shows. (The `csv` crate reports line 1 for line 2 here — see Decisions.) *Test: Task 2, `crlf_lines_are_counted_like_an_editor_counts_them`.*
2. **A quoted description holding a newline.** Records after it must report their physical line, not their ordinal. *Test: Task 2, `a_quoted_newline_advances_later_line_numbers`.*
3. **An unterminated quote.** It swallows the rest of the file into one field; it must be a hard error naming the line where the quote opened, not a silently truncated radar (NFR-15). *Test: Task 2, `an_unterminated_quote_names_the_line_it_opened_on`.*
4. **A legacy-code-page export** (Windows-1252 `é` as byte `0xE9`) — the error must name the line holding the bad byte and say to re-save as UTF-8, not just "invalid utf-8" (FR-61, D-27). *Test: Task 1, `a_legacy_code_page_names_its_line`.*
5. **An invisible control character in a header or a cell.** Removal comes first, so `name` + U+0001 duplicates `name` (FR-4), and a cell holding only U+0001 is empty (SC-10). *Test: Task 3, `removal_runs_before_the_duplicate_header_check` and `a_cell_of_only_undisplayable_characters_becomes_empty`.*

## Decisions taken in this plan (flag on review)

- **A hand-written RFC 4180 scanner, not the `csv` crate.** Library-first was the default, so `csv` 1.4.0 was measured against this PR's requirements first (2026-10-10):
  - On a CRLF file it reported record two as **line 1** — NFR-10 broken for every Excel export.
  - An unterminated quote produced **no error**: the rest of the file became one field — NFR-15 broken.
  - `"x"y` silently became `xy`.

  Working around all three means tracking quotes and line breaks independently of the library, which is the scanner itself. RFC 4180 is small (quoted fields, doubled quotes, comma, line break); the scanner is about 60 lines and is tested directly.
- **Lenient where Excel never writes, strict where data would be lost.** A `"` inside an *unquoted* field is kept literally (`5" screen` is plausibly what the author meant, and "content is not fatal"). Text *after* a closing quote (`"x"y`) is a hard error: the author's intent is ambiguous and silently dropping or keeping the quote both guess.
- **A line with no characters at all is skipped**, but still counted, so a trailing blank line or a blank line between records is harmless. A whitespace-only line is a record (one field), and will fail the field count — it is not obviously blank to the author either.
- **The header's line is its physical line** (1 unless the file opens with blank lines).
- **FR-90 applies to header cells too**, so a header `name` + U+0001 is reported and then collides with `name` under FR-4. A field-count error does not stop that row's FR-90 warnings: both are true, and the run exits non-zero anyway.
- **The warning's column is the header name as written, trimmed**, or `column N` when the header cell is empty or the row is longer than the header.
- **`Table { header, records }` rather than bare records.** PR 06 needs the header to map columns (FR-5) and keep the declared order for the table view (FR-78). Architecture §7's `build_catalogue(records: &[Record], …)` becomes `build_catalogue(table: &Table, …)`.
- **A header-only file parses** to zero records; whether that is an error is PR 06's call (it is not structural).
- **New `Fatal` messages are proposed.** SC-10's golden tests (PR 13 onward) will pin them, so this is the cheap moment to change them:

  | Variant | Message (after `error: `) |
  | --- | --- |
  | `Unreadable` | `{path}: cannot be read: {io error}` |
  | `NotUtf8` | `{path}, line {line}: not UTF-8 text; re-save the file as UTF-8 (in Excel, Save As "CSV UTF-8")` |
  | `NoHeader` | `the CSV has no header row; its first line must name the columns` |
  | `UnterminatedQuote` | `line {line}: a quoted field opens here and never closes` |
  | `TextAfterQuote` | `line {line}, field {field}: text follows a closing quote; inside quotes, write a quote as ""` |
  | `DuplicateHeader` | `line {line}: column {index} '{name}' repeats column {first_index}` |
  | `FieldCount` | `line {line}: {found} fields, but the header has {expected}` |
- **No version bump.** The binary still exits 1 without reading a file.

---

### Task 1: `csv/decode.rs` — bytes to text

**Files:**
- Create: `src/csv/mod.rs`, `src/csv/decode.rs`
- Modify: `src/lib.rs` (add `pub mod csv;` after `pub mod cli;`)
- Modify: `src/diagnostics.rs` (add `Unreadable` and `NotUtf8` to `Fatal`)

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
//! Nothing here knows what a column means; that is `model.rs` (PR 06).

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
    }

    /// FR-61, D-27: the likely cause is a Windows-1252 export, and the line
    /// is what lets the author find the offending cell.
    #[test]
    fn a_legacy_code_page_names_its_line() {
        assert_eq!(decode(b"name\nKafka\nCaf\xE9\n"), Err(NotUtf8 { line: 3 }));
        assert_eq!(decode(b"\xE9"), Err(NotUtf8 { line: 1 }));
        assert_eq!(decode(b"\xEF\xBB\xBFa\n\xE9"), Err(NotUtf8 { line: 2 }));
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
        let path = std::env::temp_dir().join("radargen-no-such-file.csv");
        let err = read(&path).expect_err("missing");
        assert!(matches!(err, Fatal::Unreadable { .. }), "{err:?}");
        let shown = err.render();
        assert!(shown.contains("radargen-no-such-file.csv: cannot be read"), "{shown}");
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
/// same way, so the two stages never disagree about a line.
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

/// FR-61: read and decode the input file.
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

Run: `cargo test --lib csv::decode && cargo clippy --all-targets -- -D warnings && just lint`
Expected: 6 passed; both lints clean.

- [ ] **Step 5: Commit**

```bash
git add src/csv/mod.rs src/csv/decode.rs src/lib.rs src/diagnostics.rs
git commit -m "feat(csv): decode — BOM, UTF-8, and FR-61's line-numbered error"
```

---

### Task 2: `csv/parse.rs` — the RFC 4180 scanner

**Files:**
- Create: `src/csv/parse.rs`
- Modify: `src/csv/mod.rs` (add `pub mod parse;`)
- Modify: `src/diagnostics.rs` (add `UnterminatedQuote` and `TextAfterQuote` to `Fatal`)

**Interfaces:**
- Consumes: `diagnostics::Fatal`.
- Produces, in `radargen::csv::parse`:
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct Record { pub line: usize, pub fields: Vec<String> }`
  - private `fn scan(text: &str) -> Result<Vec<Record>, Fatal>` (Task 3 builds on it)
- And in `Fatal`: `UnterminatedQuote { line: usize }`, `TextAfterQuote { line: usize, field: usize }`.

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
    }

    #[test]
    fn blank_lines_are_skipped_but_counted() {
        assert_eq!(
            lines("a\n\n1\n\n\n2\n\n"),
            [rec(1, &["a"]), rec(3, &["1"]), rec(6, &["2"])]
        );
    }

    #[test]
    fn edge_fields_are_kept() {
        assert_eq!(lines("a,b,\n"), [rec(1, &["a", "b", ""])]);
        assert_eq!(lines(",\n"), [rec(1, &["", ""])]);
        assert_eq!(lines("a,b"), [rec(1, &["a", "b"])], "no final newline");
        assert_eq!(lines(" a , b \n"), [rec(1, &[" a ", " b "])], "not trimmed here");
        assert_eq!(lines("\"\",x\n"), [rec(1, &["", "x"])]);
    }

    /// Lenient where Excel never writes: a quote inside an unquoted field.
    #[test]
    fn a_quote_inside_an_unquoted_field_is_literal() {
        assert_eq!(lines("5\" screen,x\n"), [rec(1, &["5\" screen", "x"])]);
    }

    /// Review focus 3, NFR-15: it would swallow the rest of the file.
    #[test]
    fn an_unterminated_quote_names_the_line_it_opened_on() {
        let err = scan("a,b\nx,y\n\"open,2\n3,4\n").expect_err("unterminated");
        assert!(matches!(err, Fatal::UnterminatedQuote { line: 3 }), "{err:?}");
        assert_eq!(err.render(), "error: line 3: a quoted field opens here and never closes");
    }

    #[test]
    fn text_after_a_closing_quote_is_an_error() {
        let err = scan("a,b\n1,\"x\"y\n").expect_err("text after quote");
        assert!(matches!(err, Fatal::TextAfterQuote { line: 2, field: 2 }), "{err:?}");
        assert_eq!(
            err.render(),
            "error: line 2, field 2: text follows a closing quote; inside quotes, write a quote as \"\""
        );
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
    /// field, so nothing after it parses. `line` is where the quote opened.
    #[error("line {line}: a quoted field opens here and never closes")]
    UnterminatedQuote { line: usize },
    /// NFR-15: text after a closing quote. RFC 4180 doubles a quote inside a
    /// quoted field; keeping or dropping the stray text would both guess.
    #[error(
        "line {line}, field {field}: text follows a closing quote; inside quotes, write a quote as \"\""
    )]
    TextAfterQuote { line: usize, field: usize },
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

    /// One field and what ended it. `number` is its 1-based position, for
    /// the error message.
    fn field(&mut self, number: usize) -> Result<(String, End), Fatal> {
        let mut text = String::new();
        if self.chars.next_if_eq(&'"').is_some() {
            let opened = self.line;
            loop {
                match self.chars.next() {
                    None => return Err(Fatal::UnterminatedQuote { line: opened }),
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
                field: number,
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

/// RFC 4180 records, each with the physical line it begins on. A line with
/// no characters at all is skipped, though still counted.
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
        if s.line_break() {
            continue;
        }
        let line = s.line;
        let mut fields = Vec::new();
        loop {
            let (field, end) = s.field(fields.len() + 1)?;
            fields.push(field);
            if !matches!(end, End::Comma) {
                break;
            }
        }
        records.push(Record { line, fields });
    }
    Ok(records)
}
```

`scan` is private and unused outside the tests until Task 3. If `cargo clippy` reports it as dead code, that is expected for this commit only; do not add an `#[allow]` — Task 3 uses it.

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib csv::parse`
Expected: 9 passed. (`cargo clippy -- -D warnings` may flag `scan` and `Record` as unused until Task 3; the commit is a checkpoint, not a release.)

- [ ] **Step 5: Commit**

```bash
git add src/csv/mod.rs src/csv/parse.rs src/diagnostics.rs
git commit -m "feat(csv): an RFC 4180 scanner with physical line numbers"
```

---

### Task 3: `parse` — FR-90 removal, the header and field counts

**Files:**
- Modify: `src/csv/parse.rs` (append the public `parse` and `Table`, extend the tests)
- Modify: `src/diagnostics.rs` (add `NoHeader`, `DuplicateHeader` and `FieldCount` to `Fatal`)
- Modify: `docs/design/architecture.md` (§7: `build_catalogue(table: &Table, …)`)

**Interfaces:**
- Consumes: `scan` and `Record` (Task 2); `escape::displayable` (exists); `normalise` (exists); `diagnostics::{Warning, RowKind, Warnings}` (exist).
- Produces, in `radargen::csv::parse`:
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct Table { pub header: Vec<String>, pub header_line: usize, pub records: Vec<Record> }`
  - `pub fn parse(text: &str, w: &mut Warnings) -> Result<Table, Vec<Fatal>>`
- And in `Fatal`: `NoHeader`, `DuplicateHeader { line: usize, index: usize, name: String, first_index: usize }`, `FieldCount { line: usize, found: usize, expected: usize }`.

  PR 06's `build_catalogue(table: &Table, w: &mut Warnings) -> Result<Catalogue, Vec<Fatal>>` consumes `Table`: it maps header names to columns under FR-76 (FR-5) and keeps `header`'s order for the table view (FR-78). Every `Record` has exactly `header.len()` fields.

- [ ] **Step 1: Write the failing tests**

Append inside `mod tests`:

```rust
    use crate::diagnostics::Warnings;

    fn parsed(text: &str) -> (Result<Table, Vec<Fatal>>, Vec<String>) {
        let mut w = Warnings::default();
        let table = parse(text, &mut w);
        (table, w.render())
    }

    fn errors(text: &str) -> Vec<String> {
        parsed(text).0.expect_err("errors").iter().map(Fatal::render).collect()
    }

    #[test]
    fn a_table_keeps_the_header_and_every_record() {
        let (table, warnings) = parsed("name,ring\nKafka,Adopt\nRedis,Trial\n");
        let table = table.expect("parses");
        assert_eq!(table.header, ["name", "ring"]);
        assert_eq!(table.header_line, 1);
        assert_eq!(table.records.len(), 2);
        assert_eq!(table.records[1].line, 3);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn a_header_only_file_parses_to_no_records() {
        assert_eq!(parsed("name,ring\n").0.expect("parses").records.len(), 0);
    }

    #[test]
    fn an_empty_file_has_no_header() {
        for text in ["", "\n\n", "\r\n"] {
            assert_eq!(
                errors(text),
                ["error: the CSV has no header row; its first line must name the columns"],
                "{text:?}"
            );
        }
    }

    /// FR-4 under FR-76: `Name` and `name ` collide. Every one is reported.
    #[test]
    fn every_duplicate_header_is_reported() {
        assert_eq!(
            errors("name,ring,Name ,RING,x\n"),
            [
                "error: line 1: column 3 'Name' repeats column 1",
                "error: line 1: column 4 'RING' repeats column 2",
            ]
        );
    }

    /// FR-4, NFR-16 tier two: every bad row at once, in line order.
    #[test]
    fn every_field_count_mismatch_is_reported() {
        assert_eq!(
            errors("a,b\n1,2\n1\n1,2,3\n\"multi\nline\",2,3\n"),
            [
                "error: line 3: 1 fields, but the header has 2",
                "error: line 4: 3 fields, but the header has 2",
                "error: line 5: 3 fields, but the header has 2",
            ]
        );
    }

    #[test]
    fn duplicate_headers_and_bad_rows_are_reported_together() {
        assert_eq!(
            errors("a,A\n1\n").len(),
            2,
            "a duplicate header does not hide a bad row"
        );
    }

    /// Malformed quoting stops the run: nothing after it can be trusted.
    #[test]
    fn malformed_quoting_is_the_only_error_reported() {
        assert_eq!(
            errors("a,a\n1\n\"open\n"),
            ["error: line 3: a quoted field opens here and never closes"]
        );
    }

    /// FR-90, D-55: removed, with one warning per field naming line and column.
    #[test]
    fn undisplayable_characters_are_removed_and_warned() {
        let (table, warnings) = parsed("name,description\nKaf\u{1}ka,ok\n");
        assert_eq!(table.expect("parses").records[0].fields, ["Kafka", "ok"]);
        assert_eq!(
            warnings,
            ["warning: line 2, name: removed U+0001, which the output cannot display"]
        );
        // Several fields group into one line (NFR-20), each naming its column.
        let (table, warnings) = parsed("name,description\nKaf\u{1}ka,a\u{7f}b\u{85}c\n");
        assert_eq!(table.expect("parses").records[0].fields, ["Kafka", "abc"]);
        assert_eq!(
            warnings,
            [
                "warning: 2 fields had characters the output cannot display — name on line 2 (U+0001), description on line 2 (2, first U+007F); each was removed"
            ]
        );
    }


    /// SC-10: a cell of only such characters is empty, for FR-6 to reject.
    #[test]
    fn a_cell_of_only_undisplayable_characters_becomes_empty() {
        let (table, _) = parsed("name,ring\n\u{1}\u{2},Adopt\n");
        assert_eq!(table.expect("parses").records[0].fields, ["", "Adopt"]);
    }

    /// Review focus 5: removal comes first, so the duplicate is the one the
    /// author sees in any editor that hides the character.
    #[test]
    fn removal_runs_before_the_duplicate_header_check() {
        let (table, warnings) = parsed("name,name\u{1}\nx,y\n");
        let errors: Vec<String> = table.expect_err("duplicate").iter().map(Fatal::render).collect();
        assert_eq!(errors, ["error: line 1: column 2 'name' repeats column 1"]);
        assert_eq!(
            warnings,
            ["warning: line 1, name: removed U+0001, which the output cannot display"]
        );
    }

    /// Tab, LF and CR are displayable; they survive.
    #[test]
    fn tab_and_line_breaks_survive_removal() {
        let (table, warnings) = parsed("a\n\"x\ty\r\nz\"\n");
        assert_eq!(table.expect("parses").records[0].fields, ["x\ty\r\nz"]);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    /// A field past the header's end, or under an empty header cell, has no
    /// name to give; it gets its position instead.
    #[test]
    fn an_unnamed_column_is_named_by_position() {
        let (_, warnings) = parsed("name,\nx,\u{1}\n");
        assert_eq!(
            warnings,
            ["warning: line 2, column 2: removed U+0001, which the output cannot display"]
        );
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib csv::parse`
Expected: a compile error, `cannot find function parse`.

- [ ] **Step 3: Implement**

Add to `Fatal` in `src/diagnostics.rs`, after `TextAfterQuote`:

```rust
    /// FR-3: the file has no records at all, so no header names a column.
    #[error("the CSV has no header row; its first line must name the columns")]
    NoHeader,
    /// FR-4: a header name repeated under FR-76's normalisation, so
    /// `Name` and `name ` collide. Indices are 1-based.
    #[error("line {line}: column {index} '{name}' repeats column {first_index}")]
    DuplicateHeader {
        line: usize,
        index: usize,
        name: String,
        first_index: usize,
    },
    /// FR-4: a record whose field count is not the header's.
    #[error("line {line}: {found} fields, but the header has {expected}")]
    FieldCount {
        line: usize,
        found: usize,
        expected: usize,
    },
```

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
    /// Every data record; each has exactly `header.len()` fields.
    pub records: Vec<Record>,
}

/// FR-3, FR-4 and FR-90: the header and records, with undisplayable
/// characters removed — and warned about — before anything reads a field.
///
/// # Errors
///
/// Malformed quoting or an empty file: that one error, since nothing after
/// it parses (NFR-16 tier one). Otherwise every duplicate header and every
/// field-count mismatch, in line order (NFR-16 tier two).
pub fn parse(text: &str, w: &mut Warnings) -> Result<Table, Vec<Fatal>> {
    let mut rows = scan(text).map_err(|e| vec![e])?.into_iter();
    let Some(head) = rows.next() else {
        return Err(vec![Fatal::NoHeader]);
    };
    let (head, removed) = cleaned(head);
    let header = head.fields;
    warn(w, head.line, &removed, &header);

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
    if errors.is_empty() {
        Ok(Table {
            header,
            header_line: head.line,
            records,
        })
    } else {
        Err(errors)
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

/// One FR-90 warning per field that lost characters, naming its column by
/// header name, or by position when the header has no name for it.
fn warn(w: &mut Warnings, line: usize, removed: &[(usize, char, usize)], header: &[String]) {
    for &(column, first, count) in removed {
        let name = header
            .get(column)
            .map(|n| n.trim())
            .filter(|n| !n.is_empty())
            .map_or_else(|| format!("column {}", column + 1), str::to_string);
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

/// FR-4: every header name repeated under FR-76, naming the column it repeats.
fn duplicate_headers(header: &[String], line: usize) -> Vec<Fatal> {
    let mut seen: Vec<(String, usize)> = Vec::new();
    let mut errors = Vec::new();
    for (i, name) in header.iter().enumerate() {
        let key = normalise(name);
        if let Some((_, first)) = seen.iter().find(|(k, _)| *k == key) {
            errors.push(Fatal::DuplicateHeader {
                line,
                index: i + 1,
                name: name.trim().to_string(),
                first_index: *first,
            });
        } else {
            seen.push((key, i + 1));
        }
    }
    errors
}
```

In `docs/design/architecture.md` §7, change the first signature in the code block to `pub fn build_catalogue(table: &Table, w: &mut Warnings) -> Result<Catalogue, Vec<Fatal>>;`.

- [ ] **Step 4: Run everything**

Run: `cargo test && cargo clippy --all-targets -- -D warnings && just lint`
Expected: all pass; both lints clean (Task 2's dead-code warnings are gone).

- [ ] **Step 5: Commit**

```bash
git add src/csv/parse.rs src/diagnostics.rs docs/design/architecture.md
git commit -m "feat(csv): parse — FR-90 removal, duplicate headers, field counts"
```

---

## Finishing the PR

Follow the house workflow in `~/.claude/CLAUDE.md`. For this PR:

1. Run `crumbs start rg-aqb` before Task 1. Work on branch `feat/pr-05-csv`, created from `main`.
2. After Task 3: push, then open the PR with `gh pr create` and **no reviewer flag**.
3. Run the code-reviewer skill until clean, then the Copilot step (this repo has no `copilot_code_review` ruleset, so a local multi-agent review substitutes, and the PR body says so). Then run simplify, then code-reviewer once more.
4. Close `rg-aqb`, changing the status field only. **No version bump:** nothing a user runs changes.
5. Move this file to `docs/plans/done/` in the same PR. Merge; there's no tag.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-10 | Initial version. The `csv` crate was measured and rejected (CRLF line numbers, silent unterminated quote). All three tasks were applied to `main` in a scratch worktree before review: 142 tests pass, and `cargo clippy --all-targets -- -D warnings` and `just lint` are clean |
