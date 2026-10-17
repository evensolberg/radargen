---
tags:
  - plan
  - palette
  - cli
aliases: []
doc_id: RG-018A
doc_name: pr-18-palette
crumb_id: rg-cdo
document_title: "PR 18 — --palette: the file, its grammar and validation: Implementation Plan"
created_date: 2026-10-17
synopsis: "--palette <file.css> colours every format from one set of colours. The file is read under FR-102's strict grammar into a Palette whose values are a #RRGGBB newtype, so luminance is total and nothing unvalidated reaches an SVG attribute or the <style> block. Every problem in the file is reported in one run. --palette and --theme conflict in clap (FR-59). An HTML page drawn with a palette carries that one set and no theme toggle; the script no longer assumes the toggle exists. palette.sample.css takes FR-101's single-:root form, and docs/palette.md explains each role, with an annotated illustration. Ships as 1.3.0."
status: draft
type: Plan
revision: "1.1"
review_date: 2026-10-17
reviewed_by:
  - "Rust (subagent; static only: its worktree lost its shell, so nothing compiled; implementation is the compile check)"
  - "QA (subagent; built Task 2's parser in a scratch crate and ran its tests and mutations)"
  - "Architecture (subagent)"
  - "Security (subagent)"
  - "Accessibility and visual design (subagent; rendered the illustration)"
completed_date:
comments: "Scoped by PRD v0.78 (FR-101, FR-102, D-91, D-92). --create-palette is PR 18b (rg-0fm, 1.4.0). 1.1 folds in the five plan reviews: every finding and its ruling is in the Review rulings table, which overrides any task text it contradicts."
---
# PR 18 — `--palette`: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-cdo` · **Release:** 1.3.0 · **After:** 1.2.6 (#102), PRD v0.78 (#103) · **Then:** PR 18b, `--create-palette`, 1.4.0 (`rg-0fm`)

**Goal:** `radargen radar.csv -o radar.png --palette corporate.css` draws the radar in the file's colours, in every format. A file that is not exactly one `:root` rule of the sixteen-plus known properties, each `#RRGGBB`, is refused with every problem named and located, and nothing is written.

**Architecture:**

- **`Hex`, a validated colour** (`src/palette/hex.rs`). `Hex::parse` accepts exactly `#` and six ASCII hex digits, in any case, and keeps the value uppercased with its three channels. `Palette` holds `[Hex; 8]` and `Vec<Hex>`, so every value a `Palette` can hand an emitter has passed the check. That retires three crumb notes at once:
  - `luminance` reads the channels and can no longer panic (PR 6, PR 13 reviews);
  - no `--palette` value reaches an SVG `fill` unchecked, so no `url(...)` paint server (PR 13 security);
  - no value reaches the HTML `<style>` block unchecked, so no `;`, `}` or `<` (PR 14b).

  `escape::css_value` was written for PR 18 to call (PR 03). `Hex` is a strictly narrower allow-list enforced by type, so `css_value` and `CssValueError` are removed rather than left uncalled (Ruling 2).
- **The parser** (`src/palette/file.rs`) is not a CSS parser (D-92). It blanks comments, keeping line breaks (FR-102), scans for rules, accepts the first `:root` rule, and checks its declarations. Every problem is collected as a `PaletteError` carrying its line. `read` wraps each in a `Fatal::Palette`, so the pipeline's existing tier-one path prints one line per problem, sorted by line.
- **`Colours`** (`src/palette.rs`) replaces `Config::palette`: `BuiltIn(Theme)` or `File(Palette)`. `snapshot()` is the palette a PNG or SVG binds to. HTML reads the enum: with `File`, `css::palettes` writes one `:root` block and the forced-colours block, and no `prefers-color-scheme` or `[data-theme]` blocks. `chrome::controls` omits the toggle, and the script skips its toggle block when there is none.
- **The CLI:** `-p, --palette <FILE>`, `conflicts_with = "theme"`. `--create-template`'s `exclusive` already covers it.

**Tech stack:** Rust 2024; no new crates. `thiserror` for `PaletteError`, as `RingsError` does.

**Spec:**

- `docs/product/radargen-prd.md` v0.78. The plan rests on: FR-56, FR-59, FR-61, FR-67, FR-81, FR-82, FR-86, FR-87, FR-91 (its palette clause applies when PR 19 adds `--config`), FR-101 (the sample's form only; the flag is PR 18b), FR-102, NFR-1, NFR-4, NFR-6, NFR-8, NFR-9, NFR-10, NFR-14, NFR-15, NFR-16, NFR-17, SC-17, SC-18, SC-20, SC-21, D-44, D-54, D-91, D-92, and §6.6's `--palette` row.
- Architecture §3 (`src/palette.rs` owns parsing), §5 (colour bound late; HTML's binding), §7 (palette read while `Config` is built).
- Crumb `rg-cdo`'s notes, each answered under Rulings.

## Global Constraints

- **Every value is `#RRGGBB`** (FR-86), stored uppercased (FR-102). No other colour syntax is accepted anywhere, including in built-ins.
- **A `--palette` file is one `:root` rule of known properties and nothing else** (FR-102). The selector is matched ASCII case-insensitively; property names match exactly.
- **Every problem in the file is reported in one run**, sorted by line, the missing-role and missing-series lines last (NFR-16 tier one). FR-86 and FR-87 read the accepted rule only.
- **Line numbers are those of the file as written**: a comment counts as whitespace and keeps its line breaks. CR, LF and CRLF each end one line, counted as `csv::decode::line_at` counts them (NFR-10).
- **A BOM at the start is stripped and a NUL byte refused**, through `csv::decode::decode`, which the CSV already uses (FR-2, FR-61).
- **An unreadable or non-UTF-8 palette file is a runtime failure** under NFR-15 (`Fatal::PaletteUnreadable`, `Fatal::PaletteNotUtf8`). Grammar and value errors are `Fatal::Palette`, one per problem.
- **`--palette` with `--theme` is a clap usage error** (FR-59), including an explicit `--theme light`.
- **A failure writes nothing** and leaves an existing `-o` byte-unchanged (NFR-8, NFR-14, SC-18).
- **Determinism:** a palette file's values are bound exactly as `Hex` holds them, so a lowercase copy of a file renders the same bytes (NFR-4).
- **Diagnostics name the flag and the file:** `error: --palette <path>, line <n>: …` or `error: --palette <path>: …` for a problem with no line (NFR-10). Every message goes through `Fatal::render`, which escapes control characters (NFR-10's terminal rule).
- **`--help`** carries no requirement numbers (`help_speaks_to_users`), and `-h` stays one line per flag, at most 120 columns.
- **Strict clippy:** pedantic, nursery and cargo; `unwrap_used`, `indexing_slicing`, `string_slice` and `clippy.toml`'s disallowed `str::trim*` are errors. CSS whitespace is trimmed with `trim_matches(css_space)`, not `radargen::trim`, since FR-76's trim also strips characters CSS does not treat as whitespace. Every `#[expect]` carries a reason; `missing_docs` is denied.
- A test that writes a golden uses `UPDATE_GOLDEN=1`, which is refused under `CI`.

## Rulings

These are this plan's decisions where the PRD is silent. Reviewers: challenge any of them.

1. **`Hex` holds both its channels and its text.** `value()` must keep returning `&str` to the serialiser, so `Hex` stores the uppercased text beside `[u8; 3]`. A `Hex` exists only through `Hex::parse`; the built-ins parse their constants once, behind an `expect` that a unit test proves cannot fire.
2. **`escape::css_value` and `CssValueError` are removed.** They were the planned guard for NFR-6 context 5. `Hex` enforces a strictly smaller set (`#` and six hex digits) by type, at construction, which is where PR 13 and PR 14b asked for it. Their hostile-input table moves to `Hex`'s tests, so the same breakouts stay pinned.
3. **A gap in the series is one error per run of missing indices**, `--series-2 to --series-999999 are missing`. FR-87 says a gap names "the missing index". A file declaring `--series-1` and `--series-1000000` would otherwise print 999,998 lines.
4. **An HTML page with a palette declares `color-scheme`** from the palette: `dark` when `--surface-1` is darker than `--text-primary`, otherwise `light`. Without it, a dark palette gets light scrollbars and form controls. The PRD does not name this; it follows from §6.6's "exactly what the file contains".
5. **The forced-colours block stays on a palette page.** Windows high contrast must still remap the neutrals (rg-min). FR-39 already keeps the series out of it.
6. **`-p` is `--palette`'s short form.** It is a render flag, and every render flag has one (OQ-2, settled at implementation planning per §6.6). `-p` is free.
7. **Palette errors come after `--rings` in tier one**, as the next flag checked. NFR-16 tier one stops at the first failing flag, and within that flag reports every violation, as FR-63 does for `--rings`.
8. **`docs/palette.md`'s illustration is a hand-drawn SVG**, `docs/palette-roles.svg`, in the built-in light colours, with each role named by a leader line. A render by radargen could not carry callouts. The guide's tables are normative for values; the drawing is explanatory only.
9. **The sample's header names no requirement number and no flag a release lacks** (not `--create-palette`), so it is true in 1.3.0 and 1.4.0 alike (PRD §13) and reads as a user's file.

## Review Focus

The five inputs the spec implies but no task's tests would otherwise exercise, most likely first. Each has its test in the owning task.

1. **A file saved by Windows Notepad**: CRLF line ends and a BOM. Expected: it parses, and an error's line number matches the editor's. Test in Task 2 (`crlf_and_a_bom_parse_and_count_lines`).
2. **A huge series index** (`--series-1000000`). Expected: one ranged error, not a million lines. Test in Task 2 (`a_gap_is_one_error_per_run`).
3. **A palette path that is a directory, or does not exist.** Expected: `--palette <path>: cannot be read: …`, exit 1, nothing written. Test in Task 4 (`an_unreadable_palette_names_its_flag`).
4. **The same palette written in lowercase.** Expected: byte-identical PNG and SVG. Test in Task 4 (`palette_case_does_not_change_the_bytes`).
5. **Non-ASCII or very long text where a selector or property is expected** (`--sürface-1`, a pasted paragraph). Expected: an error quoting at most 40 characters of it, cut on a character boundary, never a panic. Test in Task 2 (`long_and_non_ascii_text_is_quoted_safely`).

## Review rulings

Five seats reviewed revision 1.0. **Where a ruling below and a task's code disagree, the ruling wins**; the task code is brought into line during implementation, which is also where the Rust seat's compile and lint findings are confirmed (its worktree could not run a shell).

| # | Seat(s) | Finding | Ruling |
| --- | --- | --- | --- |
| R1 | Arch H1, Rust 9 | `file::read` returning `Vec<Fatal>` makes a `diagnostics` ↔ `palette::file` cycle (rustqual counts cycles) and fixes the `--palette` prefix in three variants, which PR 19's `--config` would rework. | **Adopted.** `read` returns `ReadError { Unreadable(io::Error), NotUtf8 { line }, TooLarge, Invalid(Vec<PaletteError>) }` and imports nothing from `diagnostics`. `pipeline::config` maps it to `Fatal`, as it maps `RingsError`. The `Fatal` variants move to the CLI task; their `path` is the string shown, kept apart from the path opened, so PR 19 adds a variant and changes nothing here. |
| R2 | Arch M2 | `palette::file` importing `csv::decode` points the colour layer at the input stage. | **Adopted.** `decode`, `NotUtf8` and `line_at` move to a crate-root `src/decode.rs`, a pure refactor commit at the start of Task 2; `csv::decode::read` stays as the CSV's file read. Architecture §3 is updated. |
| R3 | Security 1, Arch M6, QA 15, Rust 14 | `line_at` rescans from byte 0 per rule and declaration, and `seen` is a linear scan: quadratic on a large file. | **Adopted.** A `Lines` index of line starts is built once per parse and looked up with `partition_point`; `line_at` stays the reference its tests check against. `seen` is a `BTreeMap`. A test parses a 1 MiB file of `;` in well under a second. |
| R4 | Security 2 | `fs::read` is unbounded: `--palette /dev/zero` exhausts memory. | **Adopted.** Read through `take(MAX_PALETTE_BYTES + 1)`, 1 MiB; more is `ReadError::TooLarge`, a runtime failure (`Fatal::PaletteTooLarge`). Not `is_file()`, so `-p <(…)` still works. The CSV's own unbounded read is pre-existing: a crumb. |
| R5 | Security 3, QA 9, Rust 13 | `Unknown.name`, `Duplicate.name` and `NotHex.value` are echoed whole. | **Adopted.** All three go through `excerpt`, which now trims the first line before cutting it, so a forgotten `;` quotes `#000001…` rather than the blanked comment's spaces. |
| R6 | Security 4, QA 3, Rust 12, Arch L2 | `next = index + 1` overflows at `--series-18446744073709551615`. | **Adopted.** `saturating_add`; a test expects one `MissingSeries { from: 2, to: usize::MAX - 1 }`. An index past `usize` stays `Unknown`. |
| R7 | QA 4, Arch M5 | Anything before `:root` (`@charset "UTF-8";`, `@import`, a stray word) becomes its selector, so the rule is lost and every role reported missing. | **Adopted.** A top-level `;` before `{` ends a statement: `OtherRule` when it starts with `@`, otherwise `TextOutside`, and scanning goes on. When a prelude's last line is `:root` and text precedes it, that text is `TextOutside` and the rule is still accepted. Tests assert the full error list for each. |
| R8 | Arch M1 | The flag landing before HTML honours it would draw built-in colours on `.html` silently. | **Adopted.** Task 4 plumbs `Colours` through `Config`, PNG, SVG and HTML with library-level tests and no flag; Task 5 adds the clap flag and the end-to-end tests. |
| R9 | Arch H2, QA 2, Rust 6 | The page golden inlines `script.js`, so it changes. | **Adopted.** The HTML task re-blesses `tests/golden/html/prd.html`; the expected diff is the `<script>` block alone. |
| R10 | QA 1, Rust 5 | `!html.contains("prefers-color-scheme")` fails against a correct page: the inlined script still names it. | **Adopted.** Absence checks run on the `<style>` text (`tests/common/html.rs`'s `style`), and assert `@media (prefers-color-scheme`, `:root:not([data-theme` and `:root[data-theme="dark"]` absent there; `class="theme-toggle"` absent from the page. |
| R11 | A11y 6 | Without the toggle, focusing the skip link pushes the page down by about 50 px. | **Adopted.** Palette pages only (the built-in bytes stay): `.page-controls{position:relative}.skip:focus{position:absolute;z-index:1}`. The browser scenario checks the radar's top does not move on the first Tab, and that Enter lands on a legend row. |
| R12 | QA (browser) | The scenario never checks that a palette page ignores the system preference. | **Adopted.** Loaded with `scheme: 'dark'`, the body stays `rgb(255, 255, 255)` and `color-scheme` is `light`. |
| R13 | Rust 3, 15, Arch L3 | `Colours::File(Palette)` trips `large_enum_variant`; the HTML binding matches `Colours` in three places and clones. | **Adopted.** `File(Box<Palette>)`. `Colours` gains `snapshot() -> Cow<'_, Palette>`, `interactive() -> Cow<'_, Palette>` and `has_toggle()`, so the per-format binding lives in `palette`. `Hex` keeps its `String`: making it `Copy` needs an unchecked `str` view. |
| R14 | Rust 1, 2, 4, 7, 8, 10, 11, 17 | Duplicate `BUTTON_OPEN`; `ink_is_whichever_contrasts_more_not_a_threshold` passes `&str` to `ink_for`; `built_in` needs `# Panics`; `format_collect` in four test helpers; `ratio` can be `const fn`; `qual:allow` belongs above `fn`; no fmt or rustqual step. | **Adopted, in implementation:** the new tag constant is `BUTTON_TAG`, replacing lines 45–49; the ink test wraps its values in `Hex::parse`; each task runs `cargo fmt` and clippy, and Task 7 runs rustqual against the baseline. |
| R15 | Rust 16 | `src/palette.rs` beside `src/palette/` mixes module styles. | **Adopted.** `git mv src/palette.rs src/palette/mod.rs`. `file` keeps its name: its call site reads `palette::file::read`. |
| R16 | Arch M3, Rust 17 | Task 7 leaves architecture and `escape.rs` text false. | **Adopted.** Also updated: §2's one-file-read exception, §3's `escape.rs` row and bullet (four functions; context 5 is `palette::Hex`, by intra-doc link), §3 rows for `palette/hex.rs`, `palette/file.rs` and `decode.rs`, §7's tier-one `Vec<Fatal>`, §8's OQ-6 home, and `escape.rs`'s module doc. |
| R17 | Arch L1, L2 | Ruling 4 (`color-scheme`) and Ruling 3 (ranged gaps) are observable behaviour the PRD does not state. | **Adopted.** PRD v0.79 in Task 7: §6.6's HTML clause names `color-scheme` from the palette; FR-87 says a gap names "the missing index, or the first and last of a run". |
| R18 | Arch L4 | The forced-colours selector still names `[data-theme]` on a palette page. | **Kept**, harmless; the plan's "no `[data-theme]` blocks" means no theme blocks. |
| R19 | Arch L5 | `Cli::theme`'s default hides whether it was given, which PR 19's FR-59-across-sources check needs. | **A crumb for PR 19** (`value_source` or `Option<Theme>`). |
| R20 | Security 5 | No cap on the number of problems printed. | **Not adopted.** R4's 1 MiB bound limits it; FR-102 promises every problem. |
| R21 | Security 6 | Defence in depth at the `<style>` emitter. | **Adopted as a test**, not a runtime check: `css/tests.rs` asserts a palette page's colour block holds only `[A-Za-z0-9#:;{}(),@ -]`. A `Hex` doc note says any future serde support must go through `parse`. |
| R22 | QA 5–8, 10–13, 16 | Weak or missing tests: the sort untested; coincidental line 1s; NUL; CRLF+BOM through `read`; five-entry cycling cannot fail; `#FFF !important` fails for the wrong reason; untested grammar cases; `contains` hiding extra errors; SC-18's fault matrix. | **Adopted**, with QA's fixtures: the six-line broken sample (duplicate, bad value, unknown, second block, two missing); `UnclosedComment` at 12 and `UnclosedRule` at 3; a NUL before a grammar error; CRLF+BOM end to end at line 40; no built-in series value in the five-entry SVG and the 2:1 count; `#0072B2 !important`; `:root, html`, `@media`, an empty name, `--series-0`, an empty file, a comment holding `; } {`, a trailing `}`; `assert_eq!` on full lists; a `Fault` in `FAULTS`. |
| R23 | A11y 1, 4, 5, 11 | The illustration is not valid XML (`--` in comments); leaders cross and miss; markers in two quadrants share a colour; the label sits where radargen never draws one. | **Adopted.** The seat's corrected drawing replaces Task 6's, checked with `xmllint --noout`, which Task 6 runs. Alt text says the table lists the same. |
| R24 | A11y 2, 3, 7, 9, 10 | The guide and sample omit contrast minimums FR-82 and SC-20 require (`--focus` and both strokes on both surfaces; marker numbers 4.5:1 against white or `#16191F`), and places each role is drawn; the AA sentence overstates. | **Adopted** in the guide's table and prose and the sample's header, with the seat's wording. The column is "Minimum contrast against". |
| R25 | A11y 8 | The manual check renders a palette that looks like the built-in theme. | **Adopted.** Steps for a dark palette against a light system (and the reverse, scrollbars), NVDA and VoiceOver on the controls, the skip link, high contrast with a dark palette, and a 6-quadrant, 5-entry palette. |

## Tasks

### Task 1: `Hex`, and a `Palette` built only from it

**Files:**
- Create: `src/palette/hex.rs`
- Modify: `src/palette.rs` (fields, `built_in`, `luminance`, `ink_for`; add `new`, `is_dark`; `mod hex;`)
- Modify: `src/escape.rs` (remove `css_value`, `CssValueError` and their two tests)

**Interfaces:**
- Produces: `palette::Hex` with `Hex::parse(&str) -> Option<Hex>`, `Hex::as_str(&self) -> &str`, `Hex::rgb(&self) -> [u8; 3]`. `Palette::new(neutrals: [Hex; 8], series: Vec<Hex>) -> Option<Palette>` (`None` for an empty series). `Palette::is_dark(&self) -> bool`. `Palette` keeps `built_in`, `series_len`, `number_ink`, `value`, `var` and `declarations` with their signatures unchanged.

- [ ] **Step 1: Write the failing tests** in `src/palette/hex.rs`:

```rust
//! FR-86: a `#RRGGBB` colour, the one form a palette value takes.
//!
//! A [`Palette`](super::Palette) holds nothing else, so every value an
//! emitter writes, into an SVG attribute or the page's `<style>` block, has
//! passed [`Hex::parse`] (NFR-6 context 5). It is written back uppercased
//! (FR-102), so one colour is always the same bytes (NFR-4).

/// A colour given as `#RRGGBB`, in any case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hex {
    rgb: [u8; 3],
    /// `#` and six uppercase hex digits.
    text: String,
}

impl Hex {
    /// `value` when it is exactly `#` and six ASCII hex digits.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let digits = value.strip_prefix('#')?;
        // Checked byte by byte, not by whether `from_str_radix` succeeds: it
        // accepts a leading `+` (`#+F+F+F`), and slicing a non-ASCII string
        // by byte would panic (rg-cdo, PR 13 review).
        if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let channel = |at: usize| {
            digits
                .get(at..at + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
        };
        Some(Self {
            rgb: [channel(0)?, channel(2)?, channel(4)?],
            text: ["#", &digits.to_ascii_uppercase()].concat(),
        })
    }

    /// `#RRGGBB`, uppercase.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The red, green and blue channels.
    #[must_use]
    pub const fn rgb(&self) -> [u8; 3] {
        self.rgb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_hex_digits_parse_and_read_back_uppercase() {
        let hex = Hex::parse("#0072b2").expect("a colour");
        assert_eq!(hex.as_str(), "#0072B2");
        assert_eq!(hex.rgb(), [0x00, 0x72, 0xB2]);
        assert_eq!(Hex::parse("#0072B2"), Some(hex), "case never matters");
    }

    #[test]
    fn anything_else_is_not_a_colour() {
        for value in [
            "", "#", "#FFF", "#FFFFFFF", "FFFFFF", "FFFFFF#", "#GGGGGG", "#+F+F+F",
            " #FFFFFF", "#FFFFFF ", "#ÿÿÿ", "#ＦＦＦＦＦＦ", "rgb(0 114 178)", "red",
        ] {
            assert_eq!(Hex::parse(value), None, "{value:?}");
        }
    }

    /// NFR-6 (5): nothing that could end a declaration, open a comment or
    /// fetch a resource survives. The table `escape::css_value` held, kept
    /// here now that `Hex` is the guard (plan RG-018A, Ruling 2).
    #[test]
    fn css_breakouts_are_not_colours() {
        for value in [
            "red; color: x", "red}", "red</style>", "red/*", "red\\", "url(https://x)",
            "x{", "a&b", "\"red", "red\n", "#FFFFFF;", "#FFF}FF", "#FFFFFF!important",
        ] {
            assert_eq!(Hex::parse(value), None, "{value:?}");
        }
    }
}
```

In `src/palette.rs`, add `mod hex;` and `pub use hex::Hex;` after the `use std::fmt;` line, and add to its `tests` module:

```rust
    fn six(values: [&str; 6]) -> Vec<Hex> {
        values.map(|v| Hex::parse(v).expect("a colour")).to_vec()
    }

    fn light_neutrals() -> [Hex; 8] {
        LIGHT.map(|v| Hex::parse(v).expect("a colour"))
    }

    #[test]
    fn a_palette_needs_a_series() {
        assert!(Palette::new(light_neutrals(), Vec::new()).is_none());
    }

    /// FR-87, SC-21: the count is the file's. Five entries cycle at the sixth.
    #[test]
    fn a_built_palette_cycles_over_its_own_length() {
        let mut series = six(["#000001", "#000002", "#000003", "#000004", "#000005", "#000006"]);
        series.truncate(5);
        let p = Palette::new(light_neutrals(), series).expect("a palette");
        assert_eq!(p.series_len(), 5);
        assert_eq!(p.value(Role::Series(5)), "#000001");
        assert_eq!(p.var(Role::Series(5)), "var(--series-1)");
    }

    #[test]
    fn the_built_ins_parse() {
        // `built_in`'s `expect` cannot fire: every constant is #RRGGBB.
        for value in BUILT_IN_SERIES.iter().chain(&LIGHT).chain(&DARK) {
            assert!(Hex::parse(value).is_some(), "{value}");
        }
        assert_eq!(Hex::parse(INK_LIGHT).map(|h| h.rgb()), Some(INK_LIGHT_RGB));
        assert_eq!(Hex::parse(INK_DARK).map(|h| h.rgb()), Some(INK_DARK_RGB));
    }

    /// Plan Ruling 4: a palette is dark when its ground is darker than its text.
    #[test]
    fn darkness_compares_the_ground_with_the_text() {
        assert!(!Palette::built_in(Theme::Light).is_dark());
        assert!(Palette::built_in(Theme::Dark).is_dark());
    }
```

The existing tests call `contrast(a, b)` with `&str` values. Move `contrast` into the `tests` module, rewritten over `Hex`:

```rust
    /// WCAG contrast ratio of two `#RRGGBB` strings, from 1 to 21.
    fn contrast(a: &str, b: &str) -> f64 {
        let y = |v: &str| luminance(Hex::parse(v).expect("a colour").rgb());
        ratio(y(a), y(b))
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib palette 2>&1 | tail -20`
Expected: compile errors, because `Palette::new`, `is_dark`, `INK_LIGHT_RGB`, `ratio` and `luminance([u8; 3])` do not exist yet.

- [ ] **Step 3: Implement.** In `src/palette.rs`:

Replace the ink constants, `luminance`, `ink_for` and `contrast` with:

```rust
/// FR-82's two marker-number inks (D-54), as text and as channels.
const INK_LIGHT: &str = "#FFFFFF";
const INK_DARK: &str = "#16191F";
const INK_LIGHT_RGB: [u8; 3] = [0xFF, 0xFF, 0xFF];
const INK_DARK_RGB: [u8; 3] = [0x16, 0x19, 0x1F];

/// WCAG 2.x relative luminance of an sRGB colour. Total: every `[u8; 3]`
/// is a colour, so nothing here can fail.
fn luminance(rgb: [u8; 3]) -> f64 {
    let channel = |v: u8| {
        let c = f64::from(v) / 255.0;
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let [r, g, b] = rgb;
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    let y = 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
    y
}

/// WCAG contrast ratio of two luminances, from 1 to 21.
fn ratio(a: f64, b: f64) -> f64 {
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Whichever ink contrasts more with `fill`; white on an exact tie (D-54). The
/// rule is "more contrast", not "white unless it fails 4.5:1", which differ on
/// a mid grey where both inks fail.
fn ink_for(fill: &Hex) -> &'static str {
    let y = luminance(fill.rgb());
    if ratio(y, luminance(INK_LIGHT_RGB)) >= ratio(y, luminance(INK_DARK_RGB)) {
        INK_LIGHT
    } else {
        INK_DARK
    }
}
```

The struct and constructors become:

```rust
/// One complete set of colours: eight neutrals and a non-empty series, each
/// a [`Hex`], so every value has passed FR-86's check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    neutrals: [Hex; 8],
    /// Never empty — FR-87 requires `--series-1` — so the modulo below is safe.
    series: Vec<Hex>,
}

impl Palette {
    /// A palette of `neutrals`, in [`Neutral::ALL`]'s order, and `series`;
    /// `None` when `series` is empty, which FR-87 rules out.
    #[must_use]
    pub fn new(neutrals: [Hex; 8], series: Vec<Hex>) -> Option<Self> {
        (!series.is_empty()).then_some(Self { neutrals, series })
    }

    /// The built-in palette for `theme` (FR-81, FR-82).
    #[must_use]
    pub fn built_in(theme: Theme) -> Self {
        let neutrals = match theme {
            Theme::Light => LIGHT,
            Theme::Dark => DARK,
        };
        // qual:allow(error_handling) reason: "constants; the_built_ins_parse proves each is #RRGGBB"
        let hex = |v: &str| Hex::parse(v).expect("FR-81 and FR-82's values are #RRGGBB");
        Self {
            neutrals: neutrals.map(hex),
            series: BUILT_IN_SERIES.map(hex).to_vec(),
        }
    }

    /// Whether the ground is darker than the text (plan Ruling 4): an HTML
    /// page drawn with this palette alone declares `color-scheme: dark`.
    #[must_use]
    pub fn is_dark(&self) -> bool {
        let [surface, _, text, ..] = &self.neutrals;
        luminance(surface.rgb()) < luminance(text.rgb())
    }
```

In `number_ink`, the body stays `ink_for(...)` but must pass the `Hex`. Change `value` to bind through a private `hex(&self, role) -> &Hex` and return `self.hex(role).as_str()`:

```rust
    #[must_use]
    pub fn number_ink(&self, index: usize) -> &'static str {
        ink_for(self.hex(Role::Series(index)))
    }

    /// The `Hex` `role` binds to (FR-67's cycling applied).
    // qual:allow(complexity, error_handling) reason: "`series` is never empty (FR-87) and `bind` reduces modulo its length"
    fn hex(&self, role: Role) -> &Hex {
        match self.bind(role) {
            Role::Neutral(n) => {
                let [s1, s2, tp, tm, sr, sd, mh, focus] = &self.neutrals;
                match n {
                    Neutral::Surface1 => s1,
                    Neutral::Surface2 => s2,
                    Neutral::TextPrimary => tp,
                    Neutral::TextMuted => tm,
                    Neutral::StrokeRing => sr,
                    Neutral::StrokeDivider => sd,
                    Neutral::MarkerHollow => mh,
                    Neutral::Focus => focus,
                }
            }
            Role::Series(i) => self
                .series
                .get(i)
                .expect("`bind` reduced the index modulo the series length"),
        }
    }

    /// The literal value — SVG and PNG, whose consumers do not resolve custom
    /// properties (`resvg` renders an unresolved `var()` black).
    ///
    /// # Panics
    ///
    /// If the palette has no series entries, which FR-87 rules out and no
    /// constructor produces: `bind` then panics on `% 0`, before the lookup.
    #[must_use]
    pub fn value(&self, role: Role) -> &str {
        self.hex(role).as_str()
    }
```

In `src/escape.rs`, delete `CssValueError`, `css_value`, and the tests `css_values_pass_through_when_safe` and `css_values_outside_the_allow_list_are_rejected`. Update the module's doc comment wherever it lists context 5's function to say: "Context 5, a CSS custom-property value, has no function here: `palette::Hex` admits only `#RRGGBB`, checked when a palette is built."

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib 2>&1 | tail -5 && cargo clippy --all-targets -- -D warnings 2>&1 | tail -5`
Expected: all pass, no warnings. `tests/serialise` goldens are unchanged: the values written are identical.

- [ ] **Step 5: Commit**

```bash
git add src/palette.rs src/palette/hex.rs src/escape.rs
git commit -m "refactor(palette): a palette holds Hex values, so luminance is total"
```

---

### Task 2: The `--palette` file's grammar (FR-102)

**Files:**
- Create: `src/palette/file.rs`
- Modify: `src/palette.rs` (`pub mod file;`)
- Modify: `src/csv/decode.rs` (`line_at` becomes `pub(crate)`)
- Modify: `src/diagnostics.rs` (three `Fatal` variants and a helper)

**Interfaces:**
- Consumes: `Hex::parse`, `Palette::new`, `Neutral::ALL` and `Neutral::as_str` (Task 1); `csv::decode::{decode, NotUtf8, line_at}`.
- Produces: `palette::file::parse(text: &str) -> Result<Palette, Vec<PaletteError>>`; `palette::file::read(path: &Path) -> Result<Palette, Vec<Fatal>>`; `PaletteError` with `line(&self) -> Option<usize>`; `Fatal::Palette { path: String, error: PaletteError }`, `Fatal::PaletteUnreadable { path: String, source: io::Error }`, `Fatal::PaletteNotUtf8 { path: String, line: usize }`.

- [ ] **Step 1: Write the failing tests** at the end of `src/palette/file.rs` (the module's code follows in Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::{Role, Theme};

    /// Every role at its built-in light value, then `series`, inside one rule.
    fn file(series: &str) -> String {
        let roles: String = Neutral::ALL
            .iter()
            .map(|n| {
                let value = Palette::built_in(Theme::Light).value(Role::Neutral(*n)).to_string();
                format!("  {n}: {value};\n")
            })
            .collect();
        format!(":root {{\n{roles}{series}}}\n")
    }

    fn errors(text: &str) -> Vec<PaletteError> {
        parse(text).expect_err("refused")
    }

    #[test]
    fn a_complete_rule_parses() {
        let p = parse(&file("  --series-1: #0072B2;\n")).expect("parses");
        assert_eq!(p.series_len(), 1);
        assert_eq!(p.value(Role::Series(0)), "#0072B2");
    }

    /// FR-87, SC-21: the count is read from the file, five or twelve.
    #[test]
    fn the_series_is_as_long_as_the_file_says() {
        for count in [5, 12] {
            let series: String = (1..=count)
                .map(|i| format!("  --series-{i}: #0000{i:02X};\n"))
                .collect();
            let p = parse(&file(&series)).expect("parses");
            assert_eq!(p.series_len(), count);
        }
    }

    #[test]
    fn values_are_stored_uppercased() {
        let p = parse(&file("  --series-1: #00a1b2;\n")).expect("parses");
        assert_eq!(p.value(Role::Series(0)), "#00A1B2");
    }

    /// FR-102: comments anywhere, including between a value and its `;`.
    #[test]
    fn comments_count_as_whitespace() {
        let text = file("  --series-1: /* blue */ #0072B2 /* dark: same */;\n/* trailing */");
        assert!(parse(&format!("/* head\n */{text}")).is_ok());
        // A comment separates tokens, as CSS's does, so it cannot join them.
        let joined = file("  --ser/**/ies-1: #0072B2;\n");
        assert!(matches!(errors(&joined).first(), Some(PaletteError::Unknown { .. })));
    }

    #[test]
    fn root_matches_in_any_ascii_case_and_the_last_semicolon_is_optional() {
        let text = file("  --series-1: #0072B2\n").replacen(":root", ":ROOT", 1);
        assert!(parse(&text).is_ok());
        assert!(parse(&file("  --series-1: #0072B2;;\n")).is_ok(), "an empty declaration is ignored");
    }

    /// Review Focus 1: Notepad's CRLF and BOM. `read` strips the BOM;
    /// lines count CRLF once.
    #[test]
    fn crlf_and_a_bom_parse_and_count_lines() {
        let text = file("  --series-1: #0072B2;\n  --nope: #000000;\n").replace('\n', "\r\n");
        let bytes = [b"\xEF\xBB\xBF".as_slice(), text.as_bytes()].concat();
        let decoded = decode(&bytes).expect("UTF-8");
        assert_eq!(
            errors(&decoded),
            [PaletteError::Unknown { line: 11, name: "--nope".into() }]
        );
    }

    /// FR-102: each grammar error names what it found and its line.
    #[test]
    fn each_grammar_error_is_named_and_located() {
        let cases: [(&str, PaletteError); 7] = [
            ("/* open\n", PaletteError::UnclosedComment { line: 1 }),
            (":root {\n  --focus: #000000;\n", PaletteError::UnclosedRule { line: 1 }),
            ("html { }\n", PaletteError::OtherRule { line: 1, selector: "html".into() }),
            ("\n\n:root{}\n:root{}\n", PaletteError::SecondRoot { line: 4 }),
            ("hello\n", PaletteError::TextOutside { line: 1, text: "hello".into() }),
            ("}\n", PaletteError::TextOutside { line: 1, text: "}".into() }),
            (":root {\n  oops\n}\n", PaletteError::Malformed { line: 2, text: "oops".into() }),
        ];
        for (text, want) in cases {
            assert!(errors(text).contains(&want), "{text:?}: {:?}", errors(text));
        }
    }

    #[test]
    fn unknown_duplicate_and_malformed_values_are_named() {
        let all = errors(&file(
            "  --series-1: #0072B2;\n  --Surface-1: #FFFFFF;\n  --series-01: #000000;\n  --series-1: #111111;\n  --series-2: rgb(0 0 0);\n  --series-3: #FFF !important;\n",
        ));
        let first = 10; // the line of `--series-1`: roles fill lines 2 to 9
        assert_eq!(
            all,
            [
                PaletteError::Unknown { line: first + 1, name: "--Surface-1".into() },
                PaletteError::Unknown { line: first + 2, name: "--series-01".into() },
                PaletteError::Duplicate { line: first + 3, name: "--series-1".into(), first },
                PaletteError::NotHex { line: first + 4, name: "--series-2".into(), value: "rgb(0 0 0)".into() },
                PaletteError::NotHex { line: first + 5, name: "--series-3".into(), value: "#FFF !important".into() },
            ]
        );
    }

    /// SC-21: the old two-block sample is refused at its dark rule.
    #[test]
    fn a_second_block_is_refused_at_its_line() {
        let text = include_str!("../../tests/fixtures/palette-two-block.css");
        assert_eq!(
            errors(text),
            [PaletteError::OtherRule { line: 59, selector: ":root[data-theme=\"dark\"]".into() }]
        );
    }

    #[test]
    fn missing_roles_and_series_come_last_in_order() {
        let all = errors(":root {\n  --focus: nope;\n}\n");
        let roles: Vec<PaletteError> = Neutral::ALL
            .iter()
            .filter(|n| **n != Neutral::Focus)
            .map(|role| PaletteError::MissingRole { role: *role })
            .collect();
        let mut want = vec![PaletteError::NotHex { line: 2, name: "--focus".into(), value: "nope".into() }];
        want.extend(roles);
        want.push(PaletteError::MissingSeries { from: 1, to: 1 });
        assert_eq!(all, want);
    }

    /// FR-102: only the accepted rule counts, and a swallowed rule leaves every role missing.
    #[test]
    fn a_rejected_rule_declares_nothing() {
        let all = errors(&format!("html {{ --series-1: #000000; }}\n{}", file("")));
        assert!(all.contains(&PaletteError::MissingSeries { from: 1, to: 1 }), "{all:?}");
        let swallowed = errors(&format!("/*{}", file("  --series-1: #000000;\n")));
        assert_eq!(swallowed.len(), 1 + 8 + 1, "{swallowed:?}");
    }

    /// Review Focus 2, plan Ruling 3.
    #[test]
    fn a_gap_is_one_error_per_run() {
        let all = errors(&file(
            "  --series-1: #000001;\n  --series-3: #000003;\n  --series-1000000: #000004;\n",
        ));
        assert_eq!(
            all,
            [
                PaletteError::MissingSeries { from: 2, to: 2 },
                PaletteError::MissingSeries { from: 4, to: 999_999 },
            ]
        );
        assert_eq!(all.last().map(ToString::to_string).as_deref(), Some("--series-4 to --series-999999 are missing; number series entries from --series-1 with no gaps"));
    }

    /// Review Focus 5.
    #[test]
    fn long_and_non_ascii_text_is_quoted_safely() {
        let long = "ä".repeat(100);
        let all = errors(&format!("{long} {{ }}\n"));
        let Some(PaletteError::OtherRule { selector, .. }) = all.first() else {
            panic!("{all:?}");
        };
        assert_eq!(selector.chars().count(), EXCERPT_CHARS + 1, "40 characters and an ellipsis");
        assert!(selector.ends_with('…'));
        assert!(errors(&file("  --sürface-1: #000000;\n")).contains(&PaletteError::Unknown {
            line: 10,
            name: "--sürface-1".into(),
        }));
    }

    #[test]
    fn the_property_list_names_every_role() {
        for role in Neutral::ALL {
            assert!(PROPERTIES.contains(role.as_str()), "{role}");
        }
    }
}
```

Add to `tests/fixtures/palette-two-block.css` a byte-for-byte copy of today's `docs/product/palette.sample.css`, **before** Task 3 rewrites it:

```bash
cp docs/product/palette.sample.css tests/fixtures/palette-two-block.css
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib palette::file 2>&1 | tail -5`
Expected: compile errors (`parse`, `PaletteError` not defined).

- [ ] **Step 3: Implement** `src/palette/file.rs` above the tests:

```rust
//! FR-102: a `--palette` file, read into a [`Palette`] (D-92).
//!
//! Not a CSS parser. It accepts the one shape the sample file has and an
//! author edits — comments, and one `:root` rule of custom-property
//! declarations — and names everything else with its line, so a file is
//! fixed in one pass (NFR-16 tier one).

use std::collections::BTreeMap;
use std::path::Path;

use super::{Hex, Neutral, Palette};
use crate::csv::decode::{NotUtf8, decode, line_at};
use crate::diagnostics::Fatal;

/// How much of an unexpected selector or text a message quotes.
const EXCERPT_CHARS: usize = 40;

/// The properties a palette may declare, as an unknown-property message
/// lists them.
const PROPERTIES: &str = "--series-1, --series-2 and on, --surface-1, --surface-2, --text-primary, --text-muted, --stroke-ring, --stroke-divider, --marker-hollow and --focus";

/// One problem with a `--palette` file. Messages leave out the line, which
/// [`Fatal::Palette`] prefixes from [`PaletteError::line`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PaletteError {
    /// A `/*` with no `*/` after it.
    #[error("a comment opens here and never closes; add */")]
    UnclosedComment {
        /// Where it opens.
        line: usize,
    },
    /// A rule with no closing `}`.
    #[error("this rule never closes; add }}")]
    UnclosedRule {
        /// Where its selector starts.
        line: usize,
    },
    /// Any rule but the first `:root`: another selector, or an at-rule.
    #[error("'{selector}' is not allowed; a palette is one :root rule and nothing else")]
    OtherRule {
        /// Where its selector starts.
        line: usize,
        /// The selector, quoted by [`excerpt`].
        selector: String,
    },
    /// A second `:root` rule.
    #[error("a second :root rule; a palette is one :root rule, so merge them")]
    SecondRoot {
        /// Where its selector starts.
        line: usize,
    },
    /// Text that is neither a comment nor part of a rule.
    #[error("'{text}' is outside the :root rule; delete it")]
    TextOutside {
        /// Where it starts.
        line: usize,
        /// The text, quoted by [`excerpt`].
        text: String,
    },
    /// A declaration with no `:`, or an empty name.
    #[error("'{text}' is not a declaration; write --name: #RRGGBB;")]
    Malformed {
        /// Where it starts.
        line: usize,
        /// The declaration, quoted by [`excerpt`].
        text: String,
    },
    /// A property that is not one of FR-82's roles or `--series-N`.
    #[error("{name} is not a palette property; the properties are {}", PROPERTIES)]
    Unknown {
        /// Where it is declared.
        line: usize,
        /// The name as written.
        name: String,
    },
    /// A property declared again (FR-102: both lines are named).
    #[error("{name} repeats line {first}; keep one")]
    Duplicate {
        /// The repeat's line.
        line: usize,
        /// The name.
        name: String,
        /// The first declaration's line.
        first: usize,
    },
    /// FR-86: a value that is not `#RRGGBB`.
    #[error("{name}: '{value}' is not a #RRGGBB colour")]
    NotHex {
        /// Where it is declared.
        line: usize,
        /// The property.
        name: String,
        /// The value, trimmed.
        value: String,
    },
    /// FR-86: a role the accepted rule does not declare.
    #[error("{role} is missing; a palette defines all eight roles")]
    MissingRole {
        /// The role.
        role: Neutral,
    },
    /// FR-87: series indices `from..=to` are not declared (plan Ruling 3).
    #[error("{} missing; number series entries from --series-1 with no gaps", missing_series(*from, *to))]
    MissingSeries {
        /// The first missing 1-based index.
        from: usize,
        /// The last missing 1-based index.
        to: usize,
    },
}

impl PaletteError {
    /// The line the problem is on, as the file is written; `None` for a
    /// missing role or series, which has no line.
    #[must_use]
    pub const fn line(&self) -> Option<usize> {
        match self {
            Self::UnclosedComment { line }
            | Self::UnclosedRule { line }
            | Self::OtherRule { line, .. }
            | Self::SecondRoot { line }
            | Self::TextOutside { line, .. }
            | Self::Malformed { line, .. }
            | Self::Unknown { line, .. }
            | Self::Duplicate { line, .. }
            | Self::NotHex { line, .. } => Some(*line),
            Self::MissingRole { .. } | Self::MissingSeries { .. } => None,
        }
    }
}

fn missing_series(from: usize, to: usize) -> String {
    if from == to {
        format!("--series-{from} is")
    } else {
        format!("--series-{from} to --series-{to} are")
    }
}

/// CSS whitespace (CSS Syntax §4.2).
const fn css_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0C')
}

fn css_trim(s: &str) -> &str {
    s.trim_matches(css_space)
}

/// `text`'s first line, at most [`EXCERPT_CHARS`] characters, with `…` when
/// cut (Review Focus 5).
fn excerpt(text: &str) -> String {
    let first = text.lines().next().unwrap_or_default();
    let mut chars = first.chars();
    let head: String = chars.by_ref().take(EXCERPT_CHARS).collect();
    if chars.next().is_some() || text.lines().nth(1).is_some() {
        head + "…"
    } else {
        head
    }
}

/// `text` with each comment's characters blanked and its line breaks kept,
/// so lines stay the file's (FR-102), and the offset of a comment that
/// never closes. Past that offset nothing is read.
fn uncommented(text: &str) -> (String, Option<usize>) {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find("/*") {
        let (before, from) = rest.split_at(open);
        out.push_str(before);
        let Some(close) = from.get(2..).and_then(|after| after.find("*/")) else {
            return (out, Some(text.len() - from.len()));
        };
        let (comment, after) = from.split_at(close + 4);
        out.extend(comment.chars().map(|c| if c == '\n' || c == '\r' { c } else { ' ' }));
        rest = after;
    }
    out.push_str(rest);
    (out, None)
}

/// One declaration in the accepted rule, trimmed.
struct Declaration<'a> {
    line: usize,
    name: &'a str,
    value: &'a str,
}

/// The length of the block whose body starts `body` (after its `{`), up to
/// its matching `}`; `None` when it never closes.
fn block_len(body: &str) -> Option<usize> {
    let mut depth = 1_usize;
    for (at, c) in body.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

/// The accepted `:root` rule's declarations, every grammar error pushed to
/// `errors`; `None` when the file has no `:root` rule.
fn rules<'a>(src: &'a str, errors: &mut Vec<PaletteError>) -> Option<Vec<Declaration<'a>>> {
    let mut root = None;
    let mut pos = 0;
    loop {
        let rest = src.get(pos..).unwrap_or_default();
        let lead = rest.len() - rest.trim_start_matches(css_space).len();
        pos += lead;
        let rest = src.get(pos..).unwrap_or_default();
        if rest.is_empty() {
            return root;
        }
        let line = line_at(src.as_bytes(), pos);
        let Some(brace) = rest.find(['{', '}']) else {
            errors.push(PaletteError::TextOutside { line, text: excerpt(rest) });
            return root;
        };
        let (prelude, from_brace) = rest.split_at(brace);
        if from_brace.starts_with('}') {
            let text = css_trim(rest.get(..=brace).unwrap_or_default());
            errors.push(PaletteError::TextOutside { line, text: excerpt(text) });
            pos += brace + 1;
            continue;
        }
        let body_start = pos + brace + 1;
        let Some(len) = block_len(src.get(body_start..).unwrap_or_default()) else {
            errors.push(PaletteError::UnclosedRule { line });
            return root;
        };
        let selector = css_trim(prelude);
        if !selector.eq_ignore_ascii_case(":root") {
            let shown = if selector.is_empty() { "{" } else { selector };
            errors.push(PaletteError::OtherRule { line, selector: excerpt(shown) });
        } else if root.is_some() {
            errors.push(PaletteError::SecondRoot { line });
        } else {
            let body = src.get(body_start..body_start + len).unwrap_or_default();
            root = Some(declarations(src, body_start, body, errors));
        }
        pos = body_start + len + 1;
    }
}

/// `body`'s declarations, which starts at byte `start` of `src`; a malformed
/// one is pushed to `errors` and left out.
fn declarations<'a>(
    src: &'a str,
    start: usize,
    body: &'a str,
    errors: &mut Vec<PaletteError>,
) -> Vec<Declaration<'a>> {
    let mut out = Vec::new();
    let mut offset = start;
    for part in body.split(';') {
        let lead = part.len() - part.trim_start_matches(css_space).len();
        let line = line_at(src.as_bytes(), offset + lead);
        offset += part.len() + 1;
        let text = css_trim(part);
        if text.is_empty() {
            continue;
        }
        match text.split_once(':') {
            Some((name, value)) if !css_trim(name).is_empty() => out.push(Declaration {
                line,
                name: css_trim(name),
                value: css_trim(value),
            }),
            _ => errors.push(PaletteError::Malformed { line, text: excerpt(text) }),
        }
    }
    out
}

/// What a property names.
enum Property {
    /// [`Neutral::ALL`]'s index.
    Neutral(usize),
    /// The 1-based series index.
    Series(usize),
}

/// `name` as FR-102 reads it: exact, and `--series-N` with no leading zero.
fn property(name: &str) -> Option<Property> {
    if let Some(at) = Neutral::ALL.iter().position(|n| n.as_str() == name) {
        return Some(Property::Neutral(at));
    }
    let digits = name.strip_prefix("--series-")?;
    let plain = !digits.starts_with('0') && digits.bytes().all(|b| b.is_ascii_digit());
    plain.then(|| digits.parse().ok()).flatten().map(Property::Series)
}

/// FR-86 and FR-87 over `declared`; the palette when they hold.
fn resolve(declared: &[Declaration<'_>], errors: &mut Vec<PaletteError>) -> Option<Palette> {
    let mut neutrals: [Option<Hex>; 8] = Default::default();
    let mut series: BTreeMap<usize, Hex> = BTreeMap::new();
    // Every property declared, valid or not, so one declared with a bad
    // value is reported once, as `NotHex`, and not again as missing.
    let mut seen: Vec<(&str, usize)> = Vec::new();
    let mut declared_series: Vec<usize> = Vec::new();
    for d in declared {
        if let Some(&(_, first)) = seen.iter().find(|(name, _)| *name == d.name) {
            errors.push(PaletteError::Duplicate { line: d.line, name: d.name.into(), first });
            continue;
        }
        seen.push((d.name, d.line));
        let Some(slot) = property(d.name) else {
            errors.push(PaletteError::Unknown { line: d.line, name: d.name.into() });
            continue;
        };
        if let Property::Series(index) = slot {
            declared_series.push(index);
        }
        let Some(hex) = Hex::parse(d.value) else {
            errors.push(PaletteError::NotHex { line: d.line, name: d.name.into(), value: d.value.into() });
            continue;
        };
        match slot {
            Property::Neutral(at) => {
                if let Some(cell) = neutrals.get_mut(at) {
                    *cell = Some(hex);
                }
            }
            Property::Series(index) => {
                series.insert(index, hex);
            }
        }
    }
    for (cell, role) in neutrals.iter().zip(Neutral::ALL) {
        if cell.is_none() && !seen.iter().any(|(name, _)| *name == role.as_str()) {
            errors.push(PaletteError::MissingRole { role });
        }
    }
    declared_series.sort_unstable();
    let mut next = 1;
    for &index in &declared_series {
        if index > next {
            errors.push(PaletteError::MissingSeries { from: next, to: index - 1 });
        }
        next = index + 1;
    }
    if declared_series.is_empty() {
        errors.push(PaletteError::MissingSeries { from: 1, to: 1 });
    }
    let neutrals: [Hex; 8] = neutrals.into_iter().collect::<Option<Vec<_>>>()?.try_into().ok()?;
    Palette::new(neutrals, series.into_values().collect())
}

/// FR-102: `text`, decoded, as a palette; or every problem in it, sorted by
/// line, the missing roles and series last.
///
/// # Errors
///
/// Every [`PaletteError`] in the file.
pub fn parse(text: &str) -> Result<Palette, Vec<PaletteError>> {
    let mut errors = Vec::new();
    let (src, unclosed) = uncommented(text);
    if let Some(open) = unclosed {
        errors.push(PaletteError::UnclosedComment { line: line_at(text.as_bytes(), open) });
    }
    let declared = rules(&src, &mut errors).unwrap_or_default();
    let palette = resolve(&declared, &mut errors);
    errors.sort_by_key(|e| e.line().unwrap_or(usize::MAX));
    match palette {
        Some(palette) if errors.is_empty() => Ok(palette),
        _ => Err(errors),
    }
}

/// FR-102: the palette in the file at `path`.
///
/// # Errors
///
/// [`Fatal::PaletteUnreadable`] or [`Fatal::PaletteNotUtf8`] alone (NFR-15);
/// otherwise one [`Fatal::Palette`] per problem, in [`parse`]'s order.
pub fn read(path: &Path) -> Result<Palette, Vec<Fatal>> {
    let shown = path.display().to_string();
    let bytes = std::fs::read(path).map_err(|source| {
        vec![Fatal::PaletteUnreadable { path: shown.clone(), source }]
    })?;
    let text = decode(&bytes)
        .map_err(|NotUtf8 { line }| vec![Fatal::PaletteNotUtf8 { path: shown.clone(), line }])?;
    parse(&text).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| Fatal::Palette { path: shown.clone(), error })
            .collect()
    })
}
```

In `src/csv/decode.rs`, make `line_at` `pub(crate)`.

In `src/diagnostics.rs`, import `crate::palette::file::PaletteError` and add to `Fatal`, after `Rings`:

```rust
    /// FR-102, FR-86, FR-87: one problem in the `--palette` file; a file
    /// reports every one (NFR-16 tier one).
    #[error("{}", palette_problem(path, error))]
    Palette {
        /// The `--palette` argument as given.
        path: String,
        /// What is wrong, and where.
        error: PaletteError,
    },
    /// NFR-15: the `--palette` file does not exist or cannot be read.
    #[error("--palette {path}: cannot be read: {}", io_reason(source))]
    PaletteUnreadable {
        /// The `--palette` argument as given.
        path: String,
        /// The underlying I/O failure.
        #[source]
        source: std::io::Error,
    },
    /// NFR-15, FR-102: the `--palette` file is not UTF-8, or holds a NUL.
    #[error("--palette {path}, line {line}: not UTF-8 text; re-save the file as UTF-8")]
    PaletteNotUtf8 {
        /// The `--palette` argument as given.
        path: String,
        /// The physical line holding the first bad byte.
        line: usize,
    },
```

and beside `io_reason`:

```rust
/// A palette problem as stderr shows it: located when it has a line.
fn palette_problem(path: &str, error: &PaletteError) -> String {
    match error.line() {
        Some(line) => format!("--palette {path}, line {line}: {error}"),
        None => format!("--palette {path}: {error}"),
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib palette 2>&1 | tail -5 && cargo clippy --all-targets -- -D warnings 2>&1 | tail -5`
Expected: all pass, no warnings. If `rules` or `resolve` trips `too_many_lines` or `cognitive_complexity`, split out the brace handling into a helper. Do not add an `#[expect]`.

- [ ] **Step 5: Commit**

```bash
git add src/palette.rs src/palette/file.rs src/csv/decode.rs src/diagnostics.rs tests/fixtures/palette-two-block.css
git commit -m "feat(palette): read a --palette file under FR-102's grammar"
```

---

### Task 3: The sample in FR-101's form

**Files:**
- Modify: `docs/product/palette.sample.css` (rewritten)
- Modify: `tests/palette.rs` (the drift check, rewritten)

**Interfaces:**
- Consumes: `palette::file::parse` (Task 2), `Palette::built_in`, `Palette::value`, `Neutral::ALL`.

- [ ] **Step 1: Write the failing test.** Replace `tests/palette.rs` with:

```rust
//! FR-81, FR-101: `docs/product/palette.sample.css` is a valid `--palette`
//! file holding the built-in light palette, and each neutral's line names
//! its dark value, so an author copying it gets exactly what the tool draws.

use radargen::palette::file::parse;
use radargen::palette::{Neutral, Palette, Role, Theme};

const SAMPLE: &str = include_str!("../docs/product/palette.sample.css");

#[test]
fn the_sample_is_the_built_in_light_palette() {
    assert_eq!(parse(SAMPLE), Ok(Palette::built_in(Theme::Light)));
}

#[test]
fn each_neutral_line_names_its_dark_value() {
    let dark = Palette::built_in(Theme::Dark);
    for role in Neutral::ALL {
        let opener = format!("{role}:");
        let line = SAMPLE
            .lines()
            .find(|l| l.trim_start_matches(' ').starts_with(&opener))
            .unwrap_or_else(|| panic!("no line declares {role}"));
        let want = format!("dark: {}", dark.value(Role::Neutral(role)));
        assert!(line.contains(&want), "{role}: {line}");
    }
}

/// FR-101's order: series first, then FR-82's roles in order.
#[test]
fn declarations_are_in_fr_101_order() {
    let names: Vec<&str> = SAMPLE
        .lines()
        .map(|l| l.trim_start_matches(' '))
        .filter(|l| l.starts_with("--"))
        .filter_map(|l| l.split_once(':').map(|(n, _)| n))
        .collect();
    let mut want: Vec<String> = (1..=8).map(|i| format!("--series-{i}")).collect();
    want.extend(Neutral::ALL.iter().map(ToString::to_string));
    assert_eq!(names, want);
}

/// The header names no flag or requirement (plan Ruling 9), and LF only (NFR-17).
#[test]
fn the_sample_reads_as_a_users_file() {
    assert!(!SAMPLE.contains("FR-") && !SAMPLE.contains("--create-palette"));
    assert!(!SAMPLE.contains('\r') && SAMPLE.ends_with("}\n"));
    assert!(SAMPLE.contains("https://github.com/evensolberg/radargen/blob/main/docs/palette.md"));
}
```

`Result<Palette, Vec<PaletteError>>` needs `PartialEq`; both derive it.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test palette 2>&1 | tail -8`
Expected: FAIL. `parse` reports `OtherRule` at line 59, and the header test fails.

- [ ] **Step 3: Rewrite `docs/product/palette.sample.css`** to exactly:

```css
/* radargen palette: the built-in light colours, as one set.
 *
 * Give this file to --palette to draw a radar in these colours, in every
 * format. It is exactly one set of colours, which is why --palette cannot
 * be combined with --theme, and why an HTML page drawn with it has no
 * theme button.
 *
 * Every value is a #RRGGBB colour; anything else is an error. Each
 * neutral's comment gives the built-in dark theme's value: swap them all
 * in for a dark palette.
 *
 * Once you change a colour, its contrast is yours to check: quadrant
 * colours and strokes need 3:1 against both surfaces, and text needs 4.5:1.
 * The palette guide shows where each colour is drawn:
 * https://github.com/evensolberg/radargen/blob/main/docs/palette.md
 */

:root {
  /* Quadrant colours, in the quadrants' sorted order, starting again from
   * --series-1 when there are more quadrants than entries. Add or remove
   * entries freely, numbered from --series-1 with no gaps. The first
   * entries are the most distinct, so a short list stays readable. */
  --series-1: #0072B2; /* blue */
  --series-2: #C25400; /* vermillion */
  --series-3: #00857A; /* teal */
  --series-4: #787832; /* olive */
  --series-5: #B5476B; /* rose */
  --series-6: #2F9626; /* green */
  --series-7: #C431B8; /* magenta */
  --series-8: #9D5A43; /* terracotta */

  --surface-1: #FFFFFF;      /* dark: #15181D, the radar's ground */
  --surface-2: #F4F5F7;      /* dark: #1E222A, legend, detail panel and table */
  --text-primary: #16191F;   /* dark: #E8EAEE, names, headings and numbers */
  --text-muted: #585F6B;     /* dark: #A6AEBB, ring labels, key and details */
  --stroke-ring: #7E858F;    /* dark: #6C7480, ring circles */
  --stroke-divider: #848B96; /* dark: #666E7A, quadrant dividers */
  --marker-hollow: #FFFFFF;  /* dark: #15181D, inside a hollow marker */
  --focus: #0B5FA5;          /* dark: #7FB8E6, the keyboard focus ring */
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --test palette 2>&1 | tail -5`
Expected: 4 passed.

Check that nothing else reads the old two-block shape: `rg -n "palette.sample" src tests docs/product/mockup.html scripts`. `mockup.html`'s comment only names the file as a source of values, which still holds.

- [ ] **Step 5: Commit**

```bash
git add docs/product/palette.sample.css tests/palette.rs
git commit -m "docs(palette): the sample is one :root set, valid as --palette input"
```

---

### Task 4: `--palette` on the command line, for PNG and SVG

**Files:**
- Modify: `src/cli.rs` (the flag)
- Modify: `src/palette.rs` (`Colours`)
- Modify: `src/pipeline.rs` (`Config::colours`, reading the file, `snapshot()` for PNG and SVG)
- Test: `tests/cli.rs`, `tests/run.rs`

**Interfaces:**
- Consumes: `palette::file::read` (Task 2).
- Produces: `Cli::palette: Option<PathBuf>`; `palette::Colours { BuiltIn(Theme), File(Palette) }` with `Colours::snapshot(&self) -> Palette`; `pipeline::Config::colours: Colours`, replacing `Config::palette`.

- [ ] **Step 1: Write the failing tests.**

In `tests/cli.rs`:

```rust
// ---- FR-59: --palette, and its conflict with --theme

#[test]
fn palette_takes_a_path_and_a_short_form() {
    for flag in ["--palette", "-p"] {
        let cli = parse(&["in.csv", "-o", "e.png", flag, "brand.css"]).expect("parses");
        assert_eq!(cli.palette.as_deref().and_then(|p| p.to_str()), Some("brand.css"));
    }
}

#[test]
fn palette_with_theme_is_a_usage_error() {
    // Explicit `light` is still given, though it is the default (FR-59).
    for theme in ["light", "dark"] {
        for args in [
            ["in.csv", "-o", "e.png", "--palette", "b.css", "--theme", theme],
            ["in.csv", "-o", "e.png", "--theme", theme, "--palette", "b.css"],
        ] {
            let kind = parse(&args).map(|_| ()).map_err(|e| e.kind());
            assert_eq!(kind, Err(clap::error::ErrorKind::ArgumentConflict), "{args:?}");
        }
    }
}
```

Add `&["-p", "b.css"]` to the list in `create_template_with_anything_else_is_a_usage_error`.

In `tests/run.rs`, add after the PNG section's helpers:

```rust
// ---- --palette (FR-59, FR-86, FR-87, FR-102, SC-21)

const SAMPLE: &str = include_str!("../docs/product/palette.sample.css");
const TWO_BLOCK: &str = include_str!("fixtures/palette-two-block.css");

/// The sample with its series replaced by `series`, one `#RRGGBB` each.
fn with_series(series: &[&str]) -> String {
    let neutrals: String = SAMPLE
        .lines()
        .filter(|l| l.trim_start_matches(' ').starts_with("--") && !l.contains("--series-"))
        .map(|l| format!("{l}\n"))
        .collect();
    let entries: String = series
        .iter()
        .enumerate()
        .map(|(i, v)| format!("  --series-{}: {v};\n", i + 1))
        .collect();
    format!(":root {{\n{entries}{neutrals}}}\n")
}

/// A CSV with `count` quadrants, Q01 to Qnn, one item each.
fn quadrants(count: usize) -> String {
    let rows: String = (1..=count).map(|q| format!("Item {q},Q{q:02},Adopt,d\n")).collect();
    format!("{HEAD}{rows}")
}

/// The sample draws exactly what the built-in light theme draws (FR-81).
#[test]
fn the_sample_palette_draws_the_light_theme() {
    let dir = seeded(&[("radar.csv", PRD), ("brand.css", SAMPLE)]);
    for ext in ["png", "svg"] {
        let built = format!("built.{ext}");
        let file = format!("file.{ext}");
        let plain = run_in(dir.path(), &["radar.csv", "-o", &built]);
        assert!(plain.ok(), "{}", plain.stderr);
        let ran = run_in(dir.path(), &["radar.csv", "-o", &file, "--palette", "brand.css"]);
        assert!(ran.ok(), "{}", ran.stderr);
        // The PRD example's one FR-17 warning, and nothing from the palette.
        assert_eq!(ran.stderr, plain.stderr, "no warning of its own");
        assert!(
            fs::read(dir.path().join(&built)).expect("built") == fs::read(dir.path().join(&file)).expect("file"),
            "{ext}: byte-identical"
        );
    }
}

/// Review Focus 4: case is not part of a value (FR-102).
#[test]
fn palette_case_does_not_change_the_bytes() {
    let lower = SAMPLE.to_lowercase();
    let dir = seeded(&[("radar.csv", PRD), ("upper.css", SAMPLE), ("lower.css", &lower)]);
    for css in ["upper.css", "lower.css"] {
        let out = format!("{css}.svg");
        assert!(run_in(dir.path(), &["radar.csv", "-o", &out, "-p", css]).ok());
    }
    assert_eq!(read(dir.path(), "upper.css.svg"), read(dir.path(), "lower.css.svg"));
}

/// SC-21: five entries cycle at the sixth quadrant; twelve are honoured,
/// where the built-in eight would repeat four.
#[test]
fn the_series_count_is_the_files() {
    let twelve: Vec<String> = (1..=12).map(|i| format!("#1{i:02X}0A0")).collect();
    let twelve: Vec<&str> = twelve.iter().map(String::as_str).collect();
    for (count, quads) in [(5, 6), (12, 12)] {
        let series = twelve.get(..count).expect("enough");
        let dir = seeded(&[("radar.csv", &quadrants(quads)), ("brand.css", &with_series(series))]);
        let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.svg", "-p", "brand.css"]);
        assert!(ran.ok(), "{}", ran.stderr);
        let svg = read(dir.path(), "radar.svg");
        for value in series {
            assert!(svg.contains(value), "{count}: {value} is drawn");
        }
        for unused in twelve.get(count..).unwrap_or_default() {
            assert!(!svg.contains(unused), "{count}: {unused} is not in the file");
        }
    }
}

/// SC-21, FR-102: every problem in one run, each located; nothing written.
#[test]
fn a_bad_palette_reports_everything_and_writes_nothing() {
    let broken = SAMPLE
        .replace("--surface-2: #F4F5F7;", "")
        .replace("--series-3: #00857A;", "")
        .replace("--focus: #0B5FA5;", "--focus: rgb(0 114 178);\n  --surface1: #FFFFFF;");
    let dir = seeded(&[("radar.csv", PRD), ("brand.css", &broken), ("radar.html", "old")]);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.html", "-p", "brand.css"]);
    assert_eq!(ran.code, Some(1));
    assert_starts(
        &ran,
        &[
            "error: --palette brand.css, line 39: --focus: 'rgb(0 114 178)' is not a #RRGGBB colour",
            "error: --palette brand.css, line 40: --surface1 is not a palette property",
            "error: --palette brand.css: --surface-2 is missing",
            "error: --palette brand.css: --series-3 is missing",
        ],
    );
    assert_eq!(snapshot(dir.path()), before);
}

#[test]
fn the_old_two_block_sample_is_refused_at_its_dark_rule() {
    let dir = seeded(&[("radar.csv", PRD), ("old.css", TWO_BLOCK)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png", "-p", "old.css"]);
    assert_eq!(
        ran.lines(),
        ["error: --palette old.css, line 59: ':root[data-theme=\"dark\"]' is not allowed; a palette is one :root rule and nothing else"]
    );
}

/// Review Focus 3.
#[test]
fn an_unreadable_palette_names_its_flag() {
    let dir = seeded(&[("radar.csv", PRD)]);
    fs::create_dir(dir.path().join("folder.css")).expect("creates");
    for css in ["missing.css", "folder.css"] {
        let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png", "-p", css]);
        assert_eq!(ran.code, Some(1), "{css}");
        assert_starts(&ran, &[&format!("error: --palette {css}: cannot be read: ")]);
        assert!(!dir.path().join("radar.png").exists(), "{css}");
    }
}

#[test]
fn a_palette_that_is_not_utf8_says_how_to_fix_it() {
    let dir = seeded(&[("radar.csv", PRD)]);
    fs::write(dir.path().join("brand.css"), b":root {\n  --focus: #\xFF;\n}\n").expect("seeds");
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png", "-p", "brand.css"]);
    assert_eq!(
        ran.lines(),
        ["error: --palette brand.css, line 2: not UTF-8 text; re-save the file as UTF-8"]
    );
}

#[test]
fn palette_with_theme_is_refused_by_the_binary() {
    let dir = seeded(&[("radar.csv", PRD), ("brand.css", SAMPLE)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "r.png", "-p", "brand.css", "-T", "dark"]);
    assert_eq!(ran.code, Some(2), "clap's usage status");
    assert!(ran.stderr.contains("cannot be used with"), "{}", ran.stderr);
}
```

The line numbers 39 and 40 in `a_bad_palette_reports_everything_and_writes_nothing` hold for Task 3's sample, where `--focus` is on line 39 (the replacement adds `--surface1` on the line after it). Confirm with `grep -n -- '--focus' docs/product/palette.sample.css`; if Task 3's text changed, adjust both numbers.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test cli palette 2>&1 | tail -5; cargo test --test run palette 2>&1 | tail -5`
Expected: compile errors in `tests/cli.rs` (`cli.palette`); `tests/run.rs` fails with clap's "unexpected argument '--palette'".

- [ ] **Step 3: Implement.**

`src/palette.rs`, after `Theme`:

```rust
/// The colours a run draws with (FR-56, FR-59).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Colours {
    /// The built-in palettes. `--theme` picks the one a PNG or SVG is bound
    /// to; an HTML page carries both, and a toggle (FR-56).
    BuiltIn(Theme),
    /// A `--palette` file: one set for every format, and no toggle (§6.6).
    File(Palette),
}

impl Colours {
    /// The palette a PNG or SVG is bound to.
    #[must_use]
    pub fn snapshot(&self) -> Palette {
        match self {
            Self::BuiltIn(theme) => Palette::built_in(*theme),
            Self::File(palette) => palette.clone(),
        }
    }
}
```

`src/cli.rs`, after `width`:

```rust
    // FR-59, D-44: one set of colours for every format, so --theme has
    // nothing to choose between; clap refuses the pair, so the message is
    // standard usage output. `-p`: a render flag earns a short form (OQ-2,
    // plan RG-018A Ruling 6). FR-102's grammar is checked when the file is
    // read, in `pipeline::config`.
    /// Colours from a CSS file instead of the built-in themes.
    ///
    /// The file is one :root rule declaring --surface-1, --surface-2,
    /// --text-primary, --text-muted, --stroke-ring, --stroke-divider,
    /// --marker-hollow, --focus and --series-1 onward, each a #RRGGBB colour.
    /// The palette guide, docs/palette.md in the source repository, explains
    /// each one. Not with --theme; an HTML page drawn with it has no theme
    /// button.
    #[arg(short, long, value_name = "FILE", conflicts_with = "theme")]
    pub palette: Option<PathBuf>,
```

Update the `theme` field's `///` help: "The theme for PNG and SVG, unless --palette gives the colours."

`src/pipeline.rs`:

- Import `crate::palette::{Colours, file}` and drop `Palette` from the imports if it becomes unused.
- `Config`: replace the `palette` field with

```rust
    /// FR-56, FR-59: the built-in themes, or a `--palette` file's one set.
    pub colours: Colours,
```

  and fix the struct's doc comment to: "The palette is read here, so a bad `--palette` file is a tier-one error (FR-102)."
- In `config()`, after the rings:

```rust
    // Plan Ruling 7: the next flag after --rings; every problem in the file
    // at once (FR-102), each its own line.
    let colours = match &cli.palette {
        Some(path) => Colours::File(file::read(path)?),
        None => Colours::BuiltIn(cli.theme),
    };
```

  and `colours,` in the `Config` literal, in place of `palette: Palette::built_in(cli.theme),`.
- In `render`, PNG: `png::rasterise(&wide, &config.colours.snapshot(), size)`. SVG: bind `let palette = config.colours.snapshot();` before the `svg::export` call and pass `palette: &palette`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test 2>&1 | grep -E "^test result|FAILED|panicked" | head -20`
Expected: every suite passes. `the_cli_writes_the_golden_exports` is unchanged.

- [ ] **Step 5: Commit**

```bash
git add src/cli.rs src/palette.rs src/pipeline.rs tests/cli.rs tests/run.rs
git commit -m "feat(cli): --palette colours PNG and SVG, and conflicts with --theme"
```

---

### Task 5: An HTML page drawn with a palette

**Files:**
- Modify: `src/emit/html/mod.rs` (`Page::colours`, the bound palette, `controls(toggle)`)
- Modify: `src/emit/html/css.rs` (`palettes(&Colours)`)
- Modify: `src/emit/html/chrome.rs` (`controls(toggle: bool)`)
- Modify: `src/emit/html/script.js` (the toggle block guarded)
- Modify: `src/pipeline.rs` (pass `colours`)
- Modify: `scripts/interaction.mjs`, `justfile` (`interaction-only`)
- Modify: every test that builds a `Page` (find them with `rg -n "Page \{" tests src`)
- Test: `tests/run.rs`

**Interfaces:**
- Consumes: `Colours`, `Palette::is_dark` (Tasks 1, 4).
- Produces: `emit::html::Page::colours: &'a Colours`; `css::palettes(colours: &Colours) -> String`; `chrome::controls(toggle: bool) -> String`.

- [ ] **Step 1: Write the failing tests** in `tests/run.rs`:

```rust
/// §6.6: a palette page carries the file's one set, follows no system
/// preference, and has no toggle; forced colours still apply (Ruling 5).
#[test]
fn a_palette_page_has_one_set_and_no_toggle() {
    let dir = seeded(&[("radar.csv", PRD), ("brand.css", SAMPLE)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.html", "-p", "brand.css"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let html = read(dir.path(), "radar.html");
    assert!(html.contains(":root{color-scheme:light;--surface-1:#FFFFFF;"), "one light set");
    for absent in ["prefers-color-scheme", ":root[data-theme=\"dark\"]", "class=\"theme-toggle\""] {
        assert!(!html.contains(absent), "{absent}");
    }
    assert!(html.contains("@media (forced-colors: active)"));
    assert!(html.contains("class=\"skip\""), "the skip link stays");
}

/// Plan Ruling 4: a dark palette's page says so.
#[test]
fn a_dark_palette_page_declares_a_dark_scheme() {
    let dark = SAMPLE
        .lines()
        .map(|l| match l.split_once("/* dark: ") {
            Some((decl, rest)) => {
                let (name, _) = decl.split_once(':').expect("a declaration");
                let value = rest.get(..7).expect("a #RRGGBB");
                format!("{name}: {value};")
            }
            None => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let dir = seeded(&[("radar.csv", PRD), ("dark.css", &dark)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.html", "-p", "dark.css"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert!(read(dir.path(), "radar.html").contains(":root{color-scheme:dark;--surface-1:#15181D;"));
}

/// FR-67 in HTML: a five-entry palette's sixth quadrant names --series-1.
#[test]
fn a_palette_page_cycles_over_the_files_series() {
    let five = ["#100A0A", "#200A0A", "#300A0A", "#400A0A", "#500A0A"];
    let dir = seeded(&[("radar.csv", &quadrants(6)), ("brand.css", &with_series(&five))]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.html", "-p", "brand.css"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let html = read(dir.path(), "radar.html");
    assert!(!html.contains("--series-6"), "no sixth entry is named or declared");
    assert!(html.contains("--series-5:#500A0A;"));
}
```

Run the existing page golden too: `tests/serialise` has `prd_page_matches_its_golden`, which must stay byte-identical for built-in colours.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test run palette_page 2>&1 | tail -8`
Expected: FAIL. The page still carries both built-in sets and the toggle.

- [ ] **Step 3: Implement.**

`src/emit/html/chrome.rs`: split the controls so the toggle is optional, keeping the built-in page's bytes:

```rust
const CONTROLS_OPEN: &str = "<div class=\"page-controls\">";
const BUTTON_OPEN: &str = "<button type=\"button\" class=\"";
const SKIP_REST: &str = "\" hidden=\"\">Skip to the legend</button>";
const TOGGLE_REST: &str = "\" hidden=\"\">";
const TOGGLE_END: &str = "</button>";
const CONTROLS_CLOSE: &str = "</div>\n";

/// The skip link and, unless a `--palette` file gives the page one set of
/// colours (§6.6), the theme toggle: first in `<main>`, both `hidden` until
/// the script runs (FR-98's rule).
pub(super) fn controls(toggle: bool) -> String {
    let toggle = if toggle {
        [BUTTON_OPEN, TOGGLE_CLASS, TOGGLE_REST, TO_DARK, TOGGLE_END].concat()
    } else {
        String::new()
    };
    [CONTROLS_OPEN, BUTTON_OPEN, SKIP_CLASS, SKIP_REST, &toggle, CONTROLS_CLOSE].concat()
}
```

Update `chrome/tests.rs` callers to `controls(true)`, and add:

```rust
#[test]
fn a_palette_page_has_no_toggle() {
    assert!(!controls(false).contains(TOGGLE_CLASS));
    assert!(controls(false).contains(SKIP_CLASS));
    assert_eq!(
        controls(true),
        "<div class=\"page-controls\"><button type=\"button\" class=\"skip\" hidden=\"\">Skip to the legend</button><button type=\"button\" class=\"theme-toggle\" hidden=\"\">Switch to dark theme</button></div>\n",
        "the built-in page's bytes are unchanged"
    );
}
```

`src/emit/html/css.rs`: `palettes` takes the colours:

```rust
/// A `--palette` page's one set (§6.6): `color-scheme` from the palette
/// (plan Ruling 4), then its declarations.
const FILE_OPEN: &str = ":root{color-scheme:";

/// The page's colours (FR-56, architecture §5). Built in: light by default,
/// dark when the viewer prefers it unless the page is set light, and dark
/// when the page is set dark; the theme toggle sets `data-theme` on
/// `<html>`. From a `--palette` file: that one set, with no preference or
/// toggle (§6.6). Forced colours apply to both (rg-min, plan Ruling 5).
///
/// Values are written as `Palette` holds them, each a `Hex`, which admits
/// only `#RRGGBB` (NFR-6 context 5).
pub(super) fn palettes(colours: &Colours) -> String {
    let forced = [FORCED_OPEN, DATA_THEME, FORCED_REST, &forced(), CLOSE_TWICE].concat();
    match colours {
        Colours::BuiltIn(_) => {
            let light = declarations(&Palette::built_in(Theme::Light));
            let dark = declarations(&Palette::built_in(Theme::Dark));
            [
                LIGHT_OPEN, &light, CLOSE,
                DARK_PREFERRED_OPEN, DATA_THEME, NOT_LIGHT, &dark, CLOSE_TWICE,
                DARK_CHOSEN_OPEN, DATA_THEME, DARK_CHOSEN, &dark, CLOSE,
                &forced,
            ]
            .concat()
        }
        Colours::File(palette) => {
            let scheme = if palette.is_dark() { "dark;" } else { "light;" };
            [FILE_OPEN, scheme, &declarations(palette), CLOSE, &forced].concat()
        }
    }
}
```

Import `Colours`. Update `css/tests.rs` callers to `palettes(&Colours::BuiltIn(Theme::Light))`.

`src/emit/html/mod.rs`: add to `Page`:

```rust
    /// FR-56, §6.6: the built-in themes with a toggle, or a `--palette`
    /// file's one set without.
    pub colours: &'a Colours,
```

In `page`, destructure `colours`. Bind the target to the palette the page declares:

```rust
    // Built in, both themes share the series, so the light palette names
    // them; from a file, the file's series and length (FR-67, FR-87).
    let bound = match colours {
        Colours::BuiltIn(_) => Palette::built_in(Theme::Light),
        Colours::File(palette) => palette.clone(),
    };
    let target = Target::Interactive(&bound);
```

Replace `&css::palettes()` with `&css::palettes(colours)` and `&chrome::controls()` with `&chrome::controls(matches!(colours, Colours::BuiltIn(_)))`. Fix `page`'s doc comment: "The palette passed only names them…" becomes "The bound palette names them and cycles the series: the built-in light palette, whose series both themes share, or the `--palette` file's."

`src/pipeline.rs`: `colours: &config.colours,` in the `Page` literal. Every other `Page { … }` in tests gains `colours: &Colours::BuiltIn(Theme::Light),`.

`src/emit/html/script.js`: replace the toggle block and the two closing toggle lines. The FR-56 block becomes:

```js
  // FR-56: the viewer's preference until a press chooses; the label says
  // what the next press does. A page drawn with --palette has one set of
  // colours and no toggle, so none of this runs there (rg-cdo).
  const toggle = document.querySelector('.' + N.toggle);
  if (toggle) {
    const prefersDark = matchMedia('(prefers-color-scheme: dark)');
    const dark = () => {
      const chosen = root.getAttribute(N.theme);
      return chosen ? chosen === 'dark' : prefersDark.matches;
    };
    const label = () => {
      toggle.textContent = dark() ? N.toLight : N.toDark;
    };
    toggle.addEventListener('click', () => {
      root.setAttribute(N.theme, dark() ? 'light' : 'dark');
      label();
    });
    prefersDark.addEventListener('change', label);
    label();
    toggle.hidden = false;
  }
```

At the end, delete `toggle.hidden = false;` and the final `label();`. The end becomes:

```js
  root.classList.add(N.js);
  // With nothing plotted there is no legend to skip to.
  skip.hidden = !document.querySelector('[' + N.legend + ']');
})();
```

`scripts/interaction.mjs`: accept a third page, a `--palette` render:

```js
// Usage: node scripts/interaction.mjs <page.html> [<long-legend.html> [<palette.html>]]
const [file, longFile, paletteFile] = process.argv.slice(2);
const paletteUrl = paletteFile && pathToFileURL(resolve(paletteFile)).href;
```

(update the usage message to match), and before the `P15` scenario:

```js
  // rg-cdo: a --palette page has no toggle, and its script still runs.
  if (paletteUrl) {
    await scenario('a --palette page has no toggle', [WIDE], async (width) => {
      await load(width, { page: paletteUrl });
      check(await evaluate(`document.documentElement.classList.contains('js')`), 'the script ran to its end');
      check(!(await evaluate(`!!document.querySelector('.theme-toggle')`)), 'there is no toggle');
      await press('Tab');
      check((await active()).cls?.includes('skip'), 'the skip link is the first stop');
    });
  }
```

The scenario fails on any exception the page throws, which is the regression the crumb's rg-nuy note described.

`justfile`, `interaction-only`: after the long-legend render, add

```bash
    ./target/release/radargen tests/fixtures/interaction.csv -o "$out/palette.html" --palette docs/product/palette.sample.css 2>/dev/null
    node scripts/interaction.mjs "$out/interaction.html" "$out/long-legend.html" "$out/palette.html"
```

and delete the old `node` line.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test 2>&1 | grep -E "^test result|FAILED|panicked" | head -20 && just interaction 2>&1 | tail -8`
Expected: every suite passes, the page golden is unchanged, and the browser run prints `ok a --palette page has no toggle @1400px`. On this machine, "a long legend scrolls beside the radar" fails on `main` too (macOS overlay scrollbars, its own crumb). Name it in the PR body if it still does.

- [ ] **Step 5: Commit**

```bash
git add src/emit/html src/pipeline.rs scripts/interaction.mjs justfile tests
git commit -m "feat(html): a --palette page carries one set of colours and no toggle"
```

---

### Task 6: The palette guide

**Files:**
- Create: `docs/palette.md`
- Create: `docs/palette-roles.svg`
- Modify: `README.md` (a Palette section after SVG; the Documentation list)

**Interfaces:** none. Prose; a test in Step 3 keeps the guide's values equal to the code's.

- [ ] **Step 1: Write the illustration**, `docs/palette-roles.svg`, exactly:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 760 420" width="760" height="420" font-family="Inter, system-ui, sans-serif" role="img" aria-labelledby="t d">
  <title id="t">Where each palette colour is drawn</title>
  <desc id="d">A quarter of a radar on its ground and a legend panel beside it, each part labelled with the palette property that colours it.</desc>
  <rect width="760" height="420" fill="#FFFFFF"/>
  <!-- --surface-1: the radar's ground -->
  <rect x="20" y="20" width="360" height="360" fill="#FFFFFF" stroke="#D0D4DA"/>
  <!-- --stroke-ring: ring circles, centre at the bottom left -->
  <g fill="none" stroke="#7E858F" stroke-width="1.5">
    <path d="M 340 360 A 300 300 0 0 0 40 60"/>
    <path d="M 265 360 A 225 225 0 0 0 40 135"/>
    <path d="M 190 360 A 150 150 0 0 0 40 210"/>
    <path d="M 115 360 A 75 75 0 0 0 40 285"/>
  </g>
  <!-- --stroke-divider: a quadrant divider -->
  <line x1="40" y1="360" x2="252" y2="148" stroke="#848B96" stroke-width="1.5"/>
  <!-- --text-muted: a ring label -->
  <text x="290" y="352" font-size="13" fill="#585F6B">Assess</text>
  <!-- --series-1: a solid marker, its number in the automatic ink -->
  <circle cx="150" cy="190" r="13" fill="#0072B2"/>
  <text x="150" y="195" font-size="13" font-weight="600" text-anchor="middle" fill="#FFFFFF">1</text>
  <!-- --marker-hollow inside a hollow marker, --text-primary number -->
  <circle cx="230" cy="260" r="12" fill="#FFFFFF" stroke="#0072B2" stroke-width="2.5"/>
  <text x="230" y="265" font-size="13" font-weight="600" text-anchor="middle" fill="#16191F">2</text>
  <!-- --focus: the keyboard focus ring -->
  <circle cx="150" cy="190" r="19" fill="none" stroke="#0B5FA5" stroke-width="2.5"/>
  <!-- --surface-2: the legend panel -->
  <rect x="420" y="20" width="160" height="180" rx="6" fill="#F4F5F7"/>
  <text x="436" y="50" font-size="15" font-weight="600" fill="#16191F">Tools</text>
  <circle cx="446" cy="78" r="9" fill="#0072B2"/>
  <text x="446" y="82" font-size="11" font-weight="600" text-anchor="middle" fill="#FFFFFF">1</text>
  <text x="462" y="82" font-size="13" fill="#16191F">Kafka</text>
  <text x="436" y="112" font-size="12" fill="#585F6B">Trial · since 2026</text>
  <!-- labels and leader lines -->
  <g font-family="ui-monospace, Menlo, monospace" font-size="12" fill="#16191F" stroke-width="1">
    <text x="610" y="40">--surface-1</text>
    <line x1="606" y1="36" x2="372" y2="36" stroke="#16191F"/>
    <text x="610" y="70">--surface-2</text>
    <line x1="606" y1="66" x2="578" y2="66" stroke="#16191F"/>
    <text x="610" y="100">--text-primary</text>
    <line x1="606" y1="96" x2="500" y2="80" stroke="#16191F"/>
    <text x="610" y="130">--text-muted</text>
    <line x1="606" y1="126" x2="560" y2="110" stroke="#16191F"/>
    <text x="610" y="190">--series-1</text>
    <line x1="606" y1="186" x2="165" y2="186" stroke="#16191F"/>
    <text x="610" y="220">--focus</text>
    <line x1="606" y1="216" x2="167" y2="200" stroke="#16191F"/>
    <text x="610" y="270">--marker-hollow</text>
    <line x1="606" y1="266" x2="243" y2="262" stroke="#16191F"/>
    <text x="610" y="310">--stroke-ring</text>
    <line x1="606" y1="306" x2="246" y2="300" stroke="#16191F"/>
    <text x="610" y="340">--stroke-divider</text>
    <line x1="606" y1="336" x2="230" y2="170" stroke="#16191F"/>
  </g>
</svg>
```

Open it in a browser and check that each leader line ends on the part it names. Move an endpoint if the drawing reads otherwise; the coordinates above are a starting layout, not a contract.

- [ ] **Step 2: Write `docs/palette.md`**, with the repository's shared frontmatter (`docs/_Common/Frontmatter.md`; `doc_id: RG-PAL`; history in a `## Revisions` table):

```markdown
# Palettes

`--palette <file.css>` draws a radar in your colours instead of radargen's built-in themes, in every format. Start from [`docs/product/palette.sample.css`](product/palette.sample.css), which holds the built-in light colours.

![Where each palette colour is drawn](palette-roles.svg)

## The file

The file is exactly one `:root` rule of CSS custom properties, each a `#RRGGBB` colour, in any case. Comments are allowed anywhere. Anything else is an error that names the line: another rule (such as `[data-theme="dark"]` or `@media`), a property radargen does not know, a property declared twice, or a value such as `rgb(0 114 178)`, `#06C` or `red`. Every problem in the file is reported in one run, and nothing is drawn until the file is fixed.

A palette is one set of colours, so `--palette` cannot be combined with `--theme`. An HTML page drawn with a palette has no theme button and does not follow the reader's light or dark setting. Windows high-contrast mode still replaces its neutral colours.

## The colours

| Property | Where it is drawn | Light | Dark | Needs, against |
| --- | --- | --- | --- | --- |
| `--surface-1` | The radar's ground | `#FFFFFF` | `#15181D` | — |
| `--surface-2` | The legend, the detail panel and the table | `#F4F5F7` | `#1E222A` | — |
| `--text-primary` | Names, headings, and the number in a hollow marker | `#16191F` | `#E8EAEE` | 4.5:1, both surfaces and `--marker-hollow` |
| `--text-muted` | Ring labels, the key, and an item's details | `#585F6B` | `#A6AEBB` | 4.5:1, both surfaces |
| `--stroke-ring` | Ring circles | `#7E858F` | `#6C7480` | 3:1, `--surface-1` |
| `--stroke-divider` | Quadrant dividers | `#848B96` | `#666E7A` | 3:1, `--surface-1` |
| `--marker-hollow` | Inside a hollow marker; usually `--surface-1` | `#FFFFFF` | `#15181D` | — |
| `--focus` | The keyboard focus ring | `#0B5FA5` | `#7FB8E6` | 3:1, `--surface-1` |
| `--series-1` onward | Each quadrant's markers and legend badges | see below | same | 3:1, both surfaces |

All eight roles are required. The dark column is the built-in dark theme: the sample file lists each value in a comment beside its light one, so swapping them gives a dark palette.

## Quadrant colours

`--series-1`, `--series-2` and on colour the quadrants in their sorted order. With more quadrants than entries, the colours start again from `--series-1`. Number them from 1 with no gaps; there can be as few as one or as many as you like. The built-in eight are:

| Entry | Colour |
| --- | --- |
| `--series-1` | `#0072B2` blue |
| `--series-2` | `#C25400` vermillion |
| `--series-3` | `#00857A` teal |
| `--series-4` | `#787832` olive |
| `--series-5` | `#B5476B` rose |
| `--series-6` | `#2F9626` green |
| `--series-7` | `#C431B8` magenta |
| `--series-8` | `#9D5A43` terracotta |

They are ordered most distinct first, so a radar with three quadrants uses the three easiest to tell apart, for normal and colour-deficient vision alike. Keep your most distinct colours first too. Quadrants never depend on colour alone: position and label tell them apart.

The number on a solid marker or legend badge is not a property. radargen draws it in white or `#16191F`, whichever contrasts more with that entry.

## Checking contrast

radargen checks that the file is complete and well formed, never that its colours are legible. The built-in themes meet WCAG 2.1 AA; a custom palette meets it only if you check it. The table's last column gives each property's minimum ratio. Any contrast checker that takes two hex values will do.
```

- [ ] **Step 3: Pin the guide to the code.** Add to `tests/palette.rs`:

```rust
/// The guide's tables give each value the code draws.
#[test]
fn the_guide_lists_every_value() {
    let guide = include_str!("../docs/palette.md");
    for theme in [Theme::Light, Theme::Dark] {
        let p = Palette::built_in(theme);
        for role in Neutral::ALL {
            let value = p.value(Role::Neutral(role));
            let row = guide
                .lines()
                .find(|l| l.starts_with(&format!("| `{role}` |")))
                .unwrap_or_else(|| panic!("no row for {role}"));
            assert!(row.contains(&format!("`{value}`")), "{theme:?} {role}: {row}");
        }
        for i in 0..8 {
            let value = p.value(Role::Series(i));
            assert!(guide.contains(&format!("| `--series-{}` | `{value}`", i + 1)), "series {i}");
        }
    }
}
```

Run: `cargo test --test palette 2>&1 | tail -5`. Expected: 5 passed.

- [ ] **Step 4: README.** After the SVG section, add:

```markdown
## Your own colours

`--palette brand.css` draws the radar in your colours, in every format. Copy [`docs/product/palette.sample.css`](docs/product/palette.sample.css), change the colours, and pass it instead of `--theme`. The [palette guide](docs/palette.md) shows where each colour is drawn and the contrast each needs.
```

and add to the Documentation list: ``- [`docs/palette.md`](docs/palette.md) — the palette guide: each colour, where it is drawn, and its contrast``.

Run: `/Users/evensolberg/.cargo/bin/lychee --offline README.md docs/palette.md`. Expected: no broken links.

- [ ] **Step 5: Commit**

```bash
git add docs/palette.md docs/palette-roles.svg README.md tests/palette.rs
git commit -m "docs(palette): a guide to each colour, where it is drawn and its contrast"
```

---

### Task 7: Docs, version, release

**Files:**
- Modify: `docs/design/architecture.md` (§5's binding table, §7's `Config`, §9 step 7)
- Modify: `docs/a11y/manual-checks.md` (a palette page step; revision row)
- Modify: `Cargo.toml`, `Cargo.lock` (`1.3.0`)
- Modify: `tests/golden/html/prd.html`, `tests/golden/svg/*` (re-blessed for `radargen 1.3`)
- Modify: `CHANGELOG.md` (regenerated)
- Move: `docs/plans/pr-18-palette.md` to `docs/plans/done/`

- [ ] **Step 1: Architecture.**
  - In §5's binding table, the HTML row gains: "With `--palette`: the file's one set in a single `:root` block, `color-scheme` from the palette, the forced-colours block, and no toggle (§6.6, RG-018A)."
  - In §7, `Config`'s palette is now `colours: Colours`, read while `Config` is built (FR-102).
  - In §9 step 7, mark `--palette` (1.3.0) **done**.

- [ ] **Step 2: Manual check.** Add a step to `docs/a11y/manual-checks.md`, numbered after the last, with a results row and a revision row:

```markdown
N. **A `--palette` page** (rg-cdo, from 1.3.0). Render `three-quadrant.csv` with `--palette docs/product/palette.sample.css` to `palette.html`.
    - *Expect:* the page looks like the light theme, with no theme button. The skip link is still the first Tab stop.
    - *Expect, with the system set to dark:* the page stays light.
    - *Expect, in Windows high-contrast mode:* the rings, dividers, text and grounds take the system's colours, as on a built-in page.
```

- [ ] **Step 3: Version and goldens.** Run `mise run bump minor`, or set `version = "1.3.0"` in `Cargo.toml` and run `cargo build` to update `Cargo.lock`. Then run each:
  - `UPDATE_GOLDEN=1 cargo test --test serialise page::prd_page_matches_its_golden`
  - `UPDATE_GOLDEN=1 cargo test --test serialise golden::prd_example_matches_its_light_svg_golden`
  - `UPDATE_GOLDEN=1 cargo test --test serialise golden::prd_example_matches_its_dark_svg_golden`

  Then run `git diff tests/golden`. Expected: each file's diff is the version line alone (`radargen 1.2` → `1.3`).

- [ ] **Step 4: Full check.**

Run: `just ci 2>&1 | tail -30`
Expected: green, except the known browser check "a long legend scrolls beside the radar" if it still fails identically on `main`.

- [ ] **Step 5: Crumbs, changelog, plan.**

```bash
CRUMBS=/Users/evensolberg/.cargo/bin/crumbs
$CRUMBS close rg-cdo --reason "PR 18: --palette (1.3.0)"
git mv docs/plans/pr-18-palette.md docs/plans/done/pr-18-palette.md
```

In the moved plan, set `status: complete`, `completed_date` and the revision, and tick every task. Regenerate `CHANGELOG.md` with `git-cliff --tag v1.3.0 -o CHANGELOG.md`. Then:

```bash
git add -A
git commit -m "chore(release): 1.3.0; close rg-cdo"
```

---

## Self-review

- **Spec coverage:**
  - FR-59: Task 4 (clap conflict, both orders, explicit `light`; the binary's exit status). The `--config` half waits for PR 19, which has no `--config` to check.
  - FR-67 and FR-87: Tasks 1, 4 and 5 (cycling at the file's length in every format; five and twelve).
  - FR-81: Task 3. FR-82's contrast notes: Task 6.
  - FR-86: Tasks 1 and 2 (`Hex`; `NotHex`; `MissingRole`).
  - FR-101: Task 3 covers the sample's form only; the flag is PR 18b.
  - FR-102: Task 2, every listed error, with a test for each.
  - NFR-6 context 5: Task 1, by type.
  - NFR-8 and NFR-14: Task 4 (`snapshot` unchanged after a failed run).
  - NFR-15: Task 4 (unreadable, not UTF-8).
  - NFR-16 tier one: Task 2 (all problems, sorted).
  - SC-18: Task 4. SC-21: Tasks 2, 4 and 5.
  - §6.6's HTML clause: Task 5.
  - Crumb notes:
    - D-54 and the PR 6 newtype: Task 1.
    - PR 12: `css_value` accepting the empty string is moot; Task 1 removes it.
    - PR 13 (slicing, `+`): Task 1's tests.
    - PR 13 security and PR 14b: Task 1, by type.
    - rg-nuy's toggle: Task 5.
- **Placeholders:** none. The illustration's coordinates are explicitly a starting layout with a check step, and the manual check's number is "after the last", which the file determines.
- **Type consistency:**
  - `Colours` is in `palette`, and is used by `pipeline` and `emit::html`.
  - `PaletteError` is in `palette::file`; `Fatal::Palette` carries it.
  - `controls(bool)` and `palettes(&Colours)` match their callers.
  - `Palette::new` returns `Option`, and `resolve` returns it through.
- **Review Focus:** five lines, each with its test in Tasks 2 and 4.
