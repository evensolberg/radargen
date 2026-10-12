---
tags:
  - plan
  - pipeline
  - cli
aliases: []
doc_id: RG-014A
doc_name: pr-14a-pipeline
crumb_id: rg-f27
document_title: "PR 14a — the pipeline, end to end: Implementation Plan"
created_date: 2026-10-12
synopsis: "main.rs runs the whole pipeline: flags, CSV, catalogue, order, layout, arrange, serialise, a minimal HTML page, and the atomic write. Diagnostics print in NFR-16's three tiers and NFR-11's order, with warnings before errors and nothing on stdout. A failure leaves -o untouched. Adds FR-62's missing-directory check, a font-load error, FR-95's narrow-width warning (rg-pti) and the radar group's accessible name (rg-eae). The full page (both arrangements, dual palette, inlined font) is PR 14b."
status: draft
type: Plan
revision: "1.0"
review_date:
reviewed_by: []
completed_date:
comments: "For the plan review. Splits PR 14 (rg-f27) per the maintainer's ruling of 2026-10-12."
---
# PR 14a — the pipeline, end to end: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumbs:** `rg-f27` (part one), `rg-pti`, `rg-eae` · **Phase:** v1.0 · **After:** PR 13, PR 58 (D-72) · **Then:** PR 14b (the full page), PR 15 (interactivity, first release)

**Goal:** `radargen radar.csv -o radar.html` works end to end on real data:

- it writes a minimal, valid HTML page holding the wide radar;
- it reports problems the way NFR-8, NFR-11 and NFR-16 say;
- it leaves `-o` untouched when anything fails.

**Architecture:**

- **`src/pipeline.rs`** holds the run, as a library function so tests can drive it: `run(&Cli) -> Report`.
  - `Report` carries the rendered warning lines, then either the rendered error lines or nothing.
  - **`main.rs`** is a thin shell: parse the arguments (clap handles usage errors itself, FR-60/FR-64), call `run`, print the warnings and then the errors to stderr, and exit 0 or 1.
- **The stages** are the existing ones in architecture §2's order:
  - `decode::read`, then `parse`, `build_catalogue`, `resolve` and `warn_crowding`;
  - the four blocks, then `arrange` (Wide and Narrow, the narrow one only for FR-95);
  - then `serialise`, `emit::html::page`, and `write_atomic`.
- **`src/emit/html/mod.rs`** writes the minimal page. PR 14b replaces its body with both arrangements, the palette stylesheet and the font.

**Tech Stack:** Rust 2024, `std` only beyond what is already present. Integration tests drive the built binary through `std::process::Command` with `env!("CARGO_BIN_EXE_radargen")`; no new dev-dependency.

**Spec:** `docs/product/radargen-prd.md` v0.55:

- FR-3, FR-52, FR-56, FR-57, FR-60, FR-62, FR-64, FR-65, FR-88, FR-89, FR-90, FR-95
- NFR-5, NFR-8, NFR-10, NFR-11, NFR-12, NFR-14, NFR-15, NFR-16, NFR-20
- SC-10, SC-18, SC-25
- D-24, D-52, D-64, D-71

Design: `docs/design/architecture.md` §2 (the pipeline), §7 (diagnostics threading, the three failure points).

## Global Constraints

- **NFR-8:** exit 0 when the render succeeds, warnings or not; non-zero, and **nothing written**, otherwise. Warnings and errors go to stderr; **stdout stays empty**; the file goes only to `-o`.
- **NFR-16's tiers:**
  1. File and flag errors abort at once: a bad `-o` (FR-52, FR-62), a bad `--rings` (FR-63, every violation together), an unreadable or non-UTF-8 CSV (FR-61), malformed quoting, no header, a non-comma separator, and the font failing to load (NFR-15).
  2. Row-level errors from `parse` and `build_catalogue` are collected and printed together through `errors::render_errors`, never `Fatal::render`.
  3. Warnings print and the render proceeds.
- **NFR-11:** warnings print first, in the order `Warnings::render` gives; errors follow as one block. A tier-one error still prints any warnings already collected (FR-90 on a flag value) before it.
- **FR-90 on flag values:** `cli::clean_flag` cleans `--title`, `--subtitle` and `--rings` before anything reads them; `--rings` is cleaned before `parse_rings` (FR-63 sees the cleaned list).
- **Ignored on HTML** (FR-56, FR-57): `--theme` and `--width`, without a warning.
- **NFR-12:** the page carries `<meta name="generator" content="radargen X.Y">`, `X.Y` from `CARGO_PKG_VERSION_MAJOR`/`_MINOR`, never the patch.
- **NFR-5:** the page makes no external request. There is no `<link>`, no `src=` and no `href=` in it.
- **One `Blocks`**, built once, arranged twice: Wide is written, and Narrow is used for FR-95 now and written in 14b.
- **No version bump** (maintainer, 2026-10-12): the first release is cut with PR 15.
- Lints, rustqual, commit format, the `Co-Authored-By` trailer and `git-mit es` as in every plan. `pub` entry points that only `main.rs` calls need no `qual:api`, since `main.rs` is the production caller.

## Review Focus

1. **A failure at any tier leaves `-o` byte-unchanged and writes nothing else** (SC-18, NFR-8): a bad `--rings`, an unreadable CSV, row errors, and an unwritable `-o`, each run against an existing file whose SHA-256 is compared before and after, with the directory listing unchanged. *Test: Task 5, `a_failed_run_leaves_the_old_file`.*
2. **Diagnostics order and channel.** Warnings print before errors, render-level warnings before row ones (NFR-11), row errors grouped (NFR-20), and nothing reaches stdout. *Test: Task 5, `warnings_then_errors_on_stderr_only`.*
3. **The title edge cases.** An absent, empty or whitespace-only `--title` gives the tab title and a visually hidden `<h1>` "Technology Radar". A hostile title is escaped in both the `<title>` and the SVG. *Tests: Task 4, `the_title_falls_back` and `a_hostile_title_is_escaped`.*
4. **A missing output directory** fails as FR-62 says, naming the path, before the CSV is read: a missing CSV *and* a missing directory report the directory. *Test: Task 5, `a_missing_directory_is_reported_first`.*
5. **FR-95 fires only past 480.**
   - A set widened past 480 warns once, naming the quadrant whose label sets the width.
   - A set widened within the headroom (Infrastructure Platforms, Tools, Practice) does not warn.
   - Ordinary radars do not warn.
   - *Tests: Task 3, `fr95_names_the_binding_quadrant` and `fr95_is_quiet_within_the_headroom`.*

## Decisions taken in this plan (flag on review)

- **The minimal page (14a only):**
  - `<!DOCTYPE html>`, then `<html lang="en">` (FR-80).
  - `<head>`: `<meta charset="utf-8">`, the viewport meta `width=device-width, initial-scale=1` (zoom not disabled), the generator meta, `<title>`, and one `<style>` holding only the visually-hidden class.
  - `<body>`: the fallback `<h1>` when there is no title, then the wide SVG serialised `Target::Static(&Palette::built_in(Theme::Light))`. Static, so colours resolve with no stylesheet; 14b switches to `Interactive` with both palettes.
  - Text renders in a fallback font until 14b inlines Inter. Measurements are Inter's, so wrapping is approximate in this PR's output only.
- **The title fallback** (maintainer): when `--title` is absent or `trim`s to empty (FR-65), the `<title>` and a visually hidden `<h1 class="visually-hidden">` read **"Technology Radar"**. The drawn title block stays empty (FR-65). With a title, `<title>` is that title and no extra `<h1>` is added, since the SVG's title group is `Heading(1)`. A subtitle never enters `<title>`.
- **The radar group's accessible name** (maintainer: with detail, in PR 14). `RadarBlock` labels its centring group:
  - **"Technology radar: 3 quadrants, 4 rings, 6 items"**, singular for 1 (`1 quadrant`), counting declared rings and plotted items.
  - This is a new `Semantics::Group(String)`, serialised as `role="group" aria-label="…"` in both targets. The static vocabulary allows `group`.
  - The serialiser goldens change by that one line and are re-blessed.
  - **The wording is flagged for review.**
- **FR-95's message** (D-64 style, flagged for review): `warning: the narrow arrangement is {W} units wide, past the 480 its rows and marker targets are sized for, so on a 320-pixel screen they fall below 24 pixels; a shorter name for quadrant "{Q}" would fix it`. `W` is rounded up to a whole unit. It is a `RenderKind::NarrowWidened { quadrant: String, width: u32 }`, render-level, so it sorts first (NFR-11).
- **The binding quadrant** is `RadarBlock::binding_quadrant(scale) -> Option<usize>`: the FR-10 position of the label with the largest horizontal reach at `scale` (the term `half_extent` maximises). `None` for zero quadrants. The pipeline maps it to its label.
- **FR-62** gets a new `Fatal::MissingDirectory { path, dir }`: `-o {path}: the directory {dir} does not exist`. It is checked in tier one, right after FR-52's extension and before any flag value or the CSV is read. `write_atomic` stays the authority for every other write failure.
- **NFR-15's font failure:** `Fatal::Font(#[from] FontError)`, and `FontError` names the face: `the embedded Inter SemiBold font cannot be read: …` (PR 08a review note). `Fonts::embedded` loads both faces, so `FontMetrics::load` takes the face name and `FontError` gains a face field.
- **`Report`:** `{ warnings: Vec<String>, errors: Vec<String> }`, both already rendered. `main` prints `warnings` then `errors` and exits 1 when `errors` is non-empty. A `Fatal` from the write (tier three, architecture §7) becomes one error line after the warnings.
- **Exit code:** 1 for every failure `run` reports. clap's usage errors keep clap's own code (2), which NFR-8's "non-zero" allows.
- **`--theme`, `--width`** are read and ignored on HTML (FR-56, FR-57).

---

### Task 1: the new diagnostics and the output version

**Files:**

- `src/diagnostics.rs`: `Fatal::MissingDirectory`, `Fatal::Font`, `RenderKind::NarrowWidened` and its message.
- `src/errors.rs`: the exhaustive `Fatal` matches (`Alone`, line 0, rank 0).
- `src/text.rs`: `FontError` names its face.
- `src/emit/mod.rs`: `pub const OUTPUT_VERSION: &str`.
- `tests/diagnostics.rs`: the messages.

**Interfaces:** produces the variants above, and `OUTPUT_VERSION` = `concat!("radargen ", env!("CARGO_PKG_VERSION_MAJOR"), ".", env!("CARGO_PKG_VERSION_MINOR"))`.

- [ ] **Step 1: Failing tests:**
  - **Exact messages:**
    - `MissingDirectory`: `error: -o out/radar.html: the directory out does not exist`.
    - `Font`: `error: the embedded Inter SemiBold font cannot be read: bad table`.
    - `NarrowWidened { quadrant: "Infrastructure Platforms and Tooling", width: 503 }`: the exact line from Decisions.
  - `output_version_is_major_minor`: `OUTPUT_VERSION == format!("radargen {}.{}", MAJOR, MINOR)` and contains no second `.` (SC-25).
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(diagnostics): FR-62's missing directory, the font failure, FR-95's warning`.

---

### Task 2: the radar group's accessible name (rg-eae)

**Files:**

- `src/svg/mod.rs`: `Semantics::Group(String)`, and `is_interactive_only` → `false`.
- `src/svg/serialise/group.rs`: `role="group" aria-label`.
- `src/layout/radar/mod.rs`: the label, and the render wraps its centring group.
- Tests in each, plus `tests/serialise/vocabulary.rs` (static roles: `heading`, `img`, `group`).
- Re-bless `tests/golden/serialiser/*.svg` with `UPDATE_GOLDEN=1`, then read the one-line diff.

**Interfaces:** produces `Semantics::Group(String)` and `fn label(quadrants: usize, rings: usize, items: usize) -> String` in `radar/mod.rs`.

- [ ] **Step 1: Failing tests:**
  - `the_radar_is_a_named_group`: the PRD example's radar render has a `Semantics::Group("Technology radar: 3 quadrants, 4 rings, 6 items")`.
  - `label_counts_singular_and_plural`: (1, 1, 1) gives `… 1 quadrant, 1 ring, 1 item`; (0, 4, 0) gives `… 0 quadrants, 4 rings, 0 items`.
  - `a_named_group_in_both_targets`: `<g role="group" aria-label="…">` exactly, in both targets.
- [ ] **Step 2–4:** red, implement, green. Re-bless the goldens and check the diff is the one group line. Lints; rustqual.
- [ ] **Step 5: Commit** `feat(svg): the radar is a named group for assistive technology (rg-eae)`.

---

### Task 3: FR-95's narrow-width warning (rg-pti)

**Files:**

- `src/layout/radar/mod.rs`: `RadarBlock::binding_quadrant(scale) -> Option<usize>`.
- `src/layout/arrange.rs`: `pub fn narrow_warning(blocks: &Blocks, labels: &[&str]) -> Option<Warning>`, which builds `RenderKind::NarrowWidened` when `placements(Narrow, blocks).width > NARROW_MAX_WIDTH`. `labels` are the quadrant labels in FR-10 order.

**Interfaces:**

- Consumes: `placements`, `NARROW_MAX_WIDTH`.
- Produces: `narrow_warning`, called by the pipeline (Task 5).

- [ ] **Step 1: Failing tests** (fixtures reuse `arrange.rs`'s test functions `beyond()`, `widened()`, `thoughtworks()`, and the PRD example):
  - `fr95_names_the_binding_quadrant`:
    - `beyond()` (one 70-character name) warns, naming it, with `width` the narrow width rounded up.
    - A four-quadrant set whose third name is the long one, on a diagonal, names that one, not a longer 6 o'clock name.
  - `fr95_is_quiet_within_the_headroom`: `widened()`, `thoughtworks()` and the PRD example give `None`.
  - `binding_quadrant_is_none_without_quadrants`.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(layout): FR-95, the narrow arrangement past 480 (rg-pti)`.

---

### Task 4: the minimal HTML page

**Files:** Create `src/emit/html/mod.rs` (`pub mod html;` in `src/emit/mod.rs`).

**Interfaces:** produces `pub fn page(wide: &str, title: Option<&str>) -> String`, where `wide` is the serialised wide SVG. The title is the cleaned `--title`; the page applies FR-65's trim rule itself (`crate::trim`).

- [ ] **Step 1: Failing tests:**
  - `the_page_is_minimal_and_valid`:
    - it starts `<!DOCTYPE html>\n<html lang="en">`;
    - it has `<meta charset="utf-8">`, the viewport meta (no `user-scalable`, no `maximum-scale`) and `<meta name="generator" content="{OUTPUT_VERSION}">`;
    - the SVG appears once inside `<body>`;
    - LF only, no BOM, ending `</html>\n`;
    - no `<link`, `src=` or `href=` (NFR-5).
  - `the_title_falls_back` (Review Focus 3): for `None`, `Some("")` and `Some("  ")`, `<title>Technology Radar</title>` and `<h1 class="visually-hidden">Technology Radar</h1>`. With `Some("Q3 Radar")`, `<title>Q3 Radar</title>` and no `<h1>`.
  - `a_hostile_title_is_escaped`: `</title><script>` arrives as `&lt;/title&gt;&lt;script&gt;` in `<title>`, and no `<script` element appears.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(emit): a minimal HTML page around the wide radar`.

---

### Task 5: `pipeline::run` and `main.rs`

**Files:**

- Create `src/pipeline.rs` (`pub mod pipeline;`).
- Modify `src/main.rs`.
- Create `tests/run.rs`, which drives the binary.

**Interfaces:**

```rust
/// What a run printed: warnings, then errors (NFR-11). Empty `errors`
/// means the file was written.
pub struct Report {
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// The whole pipeline for `cli` (architecture §2).
#[must_use]
pub fn run(cli: &Cli) -> Report;
```

`main`: `let report = run(&Cli::parse());`, then `eprintln!` each line, then `ExitCode::from(u8::from(!report.errors.is_empty()))`. The run is split into small operations, each returning `Result<_, Vec<String>>` once rendered:

- `output(&Cli)`: FR-52, then FR-62's directory;
- `flags(&Cli, &mut Warnings)`: the cleaned title, subtitle and rings;
- `catalogue(path, &RingSet, &mut Warnings)`: read, parse and build; tier-two errors merged and rendered;
- `document(&Catalogue, &Plotted, titles, &Fonts, &mut Warnings) -> (String, Option<Warning>)`;
- `write`.

Warnings are rendered once at the end, so tier one prints the warnings gathered so far.

- [ ] **Step 1: Failing tests** in `tests/run.rs`. Each runs the binary in a `tempfile::tempdir()` and reads `status`, `stdout` and `stderr`.
  - `renders_the_prd_example`:
    - exit 0, stdout empty, the file exists;
    - it parses as HTML-ish: it contains the SVG with `role="group" aria-label="Technology radar: 3 quadrants, 4 rings, 6 items"`;
    - stderr names the unplotted `Rejected` row's FR-17 warning.
  - `a_failed_run_leaves_the_old_file` (Review Focus 1). For each of:
    - `--rings ","`;
    - a missing CSV;
    - `tests/fixtures/row-errors.csv`;
    - `-o` pointing at a read-only file;

    seed `-o`, then check exit ≠ 0, the SHA-256 unchanged, the listing unchanged, and stdout empty.
  - `warnings_then_errors_on_stderr_only` (Review Focus 2): a CSV with an FR-90 character (a warning) and row errors. stderr is every `warning:` line, then every `error:` line; stdout is empty.
  - `a_missing_directory_is_reported_first` (Review Focus 4): `nope/radar.html` with a missing CSV gives exactly the FR-62 error.
  - `bad_extensions_and_v1_1_formats`: `.txt` gives FR-52's message; `.png` gives `arrives in radargen 1.1`; no file is written.
  - `theme_and_width_are_ignored_on_html`: `--theme dark --width 800` gives exit 0 and the same bytes as without them.
  - `fr95_reaches_stderr`: a CSV with one 70-character quadrant name warns, naming it, and still writes the file (exit 0).
  - `no_arguments_is_a_usage_error`: exit ≠ 0 and `Usage:` on stderr (FR-60).
- [ ] **Step 2–4:** red, implement, green; lints; rustqual. Run the binary on `tests/fixtures/three-quadrant.csv` and open the HTML in a browser by eye (a screenshot via the render skill if available).
- [ ] **Step 5: Gate** `just ci`.
- [ ] **Step 6: Commit** `feat: the pipeline, end to end (PR 14a)`.

---

## Finishing

1. **Reviews:** a final whole-branch review on the most capable model in Copilot's seat (Copilot is out of credits until November 2026; say so), then simplify, then a final check.
2. **Crumbs:**
   - `crumbs close rg-pti` and `crumbs close rg-eae`.
   - `rg-f27` stays open for 14b, with a note that 14a is done and 14b replaces `emit::html::page`'s body:
     - both arrangements, `Interactive`;
     - the palette stylesheet with `prefers-color-scheme` and `[data-theme]`;
     - the inlined WOFF2 (NFR-5);
     - the indicator CSS;
     - the media query from `breakpoint_px`;
     - the cross-process determinism test;
     - the HTML5-parser round trip.
3. Move this plan to `docs/plans/done/`, tick the steps, and set `status: complete` and `completed_date`.
4. **No version bump.** Commit, push, open the PR, check CI. Merge when clean if the maintainer authorises it.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-12 | Initial version, for the plan review. |
