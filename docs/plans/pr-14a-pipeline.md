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
synopsis: "main.rs runs the whole pipeline: flags, CSV, catalogue, order, layout, arrange, serialise, a minimal HTML page, and the atomic write. Diagnostics print in NFR-16's three tiers and NFR-11's order, with warnings before errors and nothing on stdout. A failure leaves -o untouched. Adds FR-62's missing-directory check, the refusal to overwrite the input (D-74), a font-load error, FR-95's narrow-width warning naming the quadrant or ring that sets the width (rg-pti, D-73) and the radar group's accessible name (rg-eae). The full page (both arrangements, dual palette, inlined font) is PR 14b."
status: draft
type: Plan
revision: "1.1"
review_date: 2026-10-12
reviewed_by:
  - Rust seat
  - QA seat
  - architecture seat
  - accessibility seat
  - security seat
completed_date:
comments: "1.1 folds in the five-seat plan review and the maintainer's rulings D-73 and D-74. See Review rulings."
---
# PR 14a — the pipeline, end to end: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumbs:** `rg-f27` (part one), `rg-pti`, `rg-eae` · **Phase:** v1.0 · **After:** PR 13, PR 58 (D-72) · **Then:** PR 14b (the full page), PR 15 (interactivity, first release)

**Goal:** `radargen radar.csv -o radar.html` works end to end on real data:

- it writes a minimal, valid HTML page holding the wide radar;
- it reports problems the way NFR-8, NFR-11 and NFR-16 say;
- it leaves `-o` untouched when anything fails.

**This page is a pipeline check, not the accessible deliverable.** It does not meet FR-48–FR-50, FR-94 or NFR-1, and it ships only the wide arrangement, static. Nothing is released (no bump); 14b and 15 bring the rest. Nothing in this PR claims SC-17.

**Architecture:**

- **`src/pipeline.rs`** holds the run, as a library function so tests can drive it: `run(&Cli) -> Report`. `main.rs` is a thin shell: parse the arguments (clap handles usage errors itself, FR-60/FR-64), call `run`, print, exit.
- **`Config`** (architecture §7) is the resolved run, built once from `Cli` and read-only after: the output path, the cleaned and resolved title and subtitle, and the `RingSet`. 14b adds the palettes, PR 19 the `--config` merge.
- **The stages** are the existing ones in architecture §2's order:
  - `decode::read`, then `parse`, `build_catalogue`, `resolve` and `warn_crowding`;
  - the four blocks, built once; `arrange` Wide for the page; `warn_narrow` (FR-95) reads the Narrow placement;
  - `emit::html::page`, which serialises, then `write_atomic`.
- **`src/emit/html/mod.rs`** writes the minimal page from the wide `Document`. 14b widens its parameters (narrow `Document`, `Catalogue`, palettes) and its body.

**Tech Stack:** Rust 2024, `std` only beyond what is already present. Integration tests drive the built binary through `std::process::Command` with `env!("CARGO_BIN_EXE_radargen")`; `tempfile` is already a dependency. Bytes are compared directly, not hashed.

**Spec:** `docs/product/radargen-prd.md`, v0.56 after Task 0:

- FR-3, FR-17, FR-42, FR-52, FR-56, FR-57, FR-60, FR-62, FR-64, FR-65, FR-88, FR-89, FR-90, FR-95
- NFR-5, NFR-8, NFR-10, NFR-11, NFR-12, NFR-14, NFR-15, NFR-16, NFR-20
- SC-10, SC-18, SC-25
- D-24, D-52, D-64, D-71, D-73, D-74, D-75

Design: `docs/design/architecture.md` §2 (the pipeline), §3 (the module table), §7 (diagnostics threading, `Config`, the three failure points).

## Global Constraints

- **NFR-8:** exit 0 when the render succeeds, warnings or not; non-zero, and **nothing written**, otherwise. Warnings and errors go to stderr; **stdout stays empty** (`#![deny(clippy::print_stdout)]` in `main.rs` and `pipeline.rs`; clap's `--help` and `--version` are exempt); the file goes only to `-o`.
- **NFR-16's tiers:**
  1. File and flag errors abort at once, in this order: FR-52's extension; FR-62's directory; D-74's same-file check; the cleaned flags and `--rings` (FR-63, every violation together); the font (NFR-15); then the CSV's own file-level failures (FR-61, malformed quoting, no header, a non-comma separator). Each is rendered with `Fatal::render`.
  2. Row-level errors from `parse` and `build_catalogue` are merged and printed together through `errors::render_errors(&all, table.records.len())`, never `Fatal::render`. **`parse` errors are a failure even when `build_catalogue` returns `Ok`**, since `parse` drops the rows it rejects.
  3. Warnings print and the render proceeds. A `Fatal` from the write (architecture §7's third failure point) prints with `Fatal::render` after the warnings.
- **NFR-11:** warnings print first, in `Warnings::render`'s order (render-level before row-level); errors follow as one block. A tier-one error still prints the warnings already collected (FR-90 on a flag value) before it.
- **Every stderr line passes through the existing escapers** (`Fatal::render`, `render_errors`, `Warnings::render`, all of which apply `escape_controls`). No `to_string()` or `{}` of a `Fatal` in `pipeline.rs`, and no `expect` on a value carrying user data, since a panic message skips the escaper. clap's own usage errors print argv as clap prints it; that is out of scope.
- **FR-90 on flag values:** `cli::clean_flag` cleans `--title`, `--subtitle` and `--rings` before anything reads them; `--rings` is cleaned before `parse_rings`.
- **FR-65 is resolved once**, in `Config`: a title or subtitle that is absent or empty after `crate::trim` is `None`. `TitleBlock::new` and `page` both receive the resolved value.
- **Ignored on HTML** (FR-56, FR-57): `--theme` and `--width`, without a warning.
- **NFR-12:** the page carries `<meta name="generator" content="radargen X.Y">`, never the patch.
- **NFR-5, by allowlist:** every element and attribute in the page is one the tests name (Task 4); `<style>` holds no `url(`, `@import` or `src:`. 14b's one permitted `url(` is `url(data:font/woff2;base64,…)` inside `@font-face`, and it widens the allowlist for exactly that.
- **One `Blocks`**, built once: Wide is arranged and written; Narrow is placed for FR-95 now and arranged and written in 14b.
- **No version bump** (maintainer, 2026-10-12): the first release is cut with PR 15.
- Lints, rustqual, commit format, the `Co-Authored-By` trailer and `git-mit es` as in every plan. `missing_docs` is denied, so every `pub` field has a doc comment.

## Review Focus

1. **A failure at any tier leaves `-o` byte-unchanged and writes nothing else** (SC-18, NFR-8), and the same command with the fault removed does write. *Test: Task 5, `a_failed_run_leaves_the_old_file`.*
2. **Diagnostics order and channel**: render warnings, then row warnings, then errors, all on stderr; stdout empty. *Tests: Task 5, `warnings_then_errors_on_stderr_only`, `tier_one_still_prints_flag_warnings`.*
3. **Hostile text** in `--title`, `--subtitle` and CSV fields reaches no element, attribute or entity outside the allowlist, in the `<title>` or the SVG. *Test: Task 5, `hostile_text_stays_text`.*
4. **`-o` edge cases**: a bare file name, `./`, an absolute missing directory, a parent that is a file, `-o` naming a directory, and `-o` naming the input CSV. *Tests: Task 5, `output_paths`.*
5. **FR-95 names what sets the width**: a quadrant on a diagonal over a longer 6 o'clock one, a ring with short quadrants, a ring with no quadrants, and silence within the headroom. *Tests: Task 3.*

## Decisions taken in this plan

- **The minimal page (14a only):**
  - `<!DOCTYPE html>`, then `<html lang="en">` (FR-80).
  - `<head>`: `<meta charset="utf-8">`, `<meta name="viewport" content="width=device-width, initial-scale=1">` (no scale limits), the generator meta, `<title>`, and one `<style>` holding only the visually-hidden rule:
    `.visually-hidden{position:absolute;width:1px;height:1px;margin:-1px;padding:0;border:0;overflow:hidden;clip:rect(0 0 0 0);clip-path:inset(50%);white-space:nowrap}`
  - `<body><main>`: the fallback `<h1>` when there is no title, then the wide SVG serialised `Target::Static(&Palette::built_in(Theme::Light))`. `</main></body></html>`.
  - Text renders in a fallback font until 14b inlines Inter, so wrapping is approximate in this PR's output only.
- **The title fallback** (maintainer, recorded as D-75): with no resolved title, the `<title>` and a visually hidden `<h1 class="visually-hidden">` read **"Technology Radar"**, and the drawn title block stays empty (FR-65). With a title, `<title>` is that title (escaped with `escape::xml_text`) and no extra `<h1>` is added, since the SVG's title group is `Heading(1)`. A subtitle never enters `<title>`.
- **The radar group's accessible name** (maintainer, recorded in D-75): **"Technology radar: 3 quadrants, 4 rings, 6 items"**, counting quadrants, declared rings and plotted items, singular for 1. `Semantics::Group(String)` serialises as `role="group" aria-label="…"` in both targets. `role="img"` would hide the controls; `graphics-document` is poorly supported.
- **FR-95** (maintainer, D-73): the warning names the label that sets the radar's width, quadrant or ring. Messages (D-64 style; user values single-quoted and trimmed, as FR-89's are):
  - `warning: the narrow arrangement is {W} units wide, past the 480 its legend rows and marker targets are sized for, so on a 320-pixel screen they fall below 24 pixels; a shorter name for quadrant '{Q}' would fix it`
  - `… ; a shorter name for ring '{R}' in --rings would fix it`
  - `W` is the narrow width rounded up to a whole unit, a `u32`, since `RenderKind` derives `Eq`.
- **The binding label:** `RadarBlock::binding_label(scale: RadarScale) -> Option<Binding>`, where `pub enum Binding { Quadrant(String), Ring(String) }`. It is the label with the largest `quadrant_reach_x` or `ring_reach_x` at the narrow placement's scale, the first such in FR-10 order with quadrants before rings (a strict `>`). `None` only with no labels at all, which `rings.rs` rules out by rejecting an empty ring list.
- **`warn_narrow(blocks: &Blocks, warnings: &mut Warnings)`** in `layout/arrange.rs`, as `warn_crowding` takes `&mut Warnings` (§7). It pushes `RenderKind::NarrowWidened { binding, width }` when `placements(Narrow, blocks).width > NARROW_MAX_WIDTH`.
- **FR-62:** `Fatal::MissingDirectory { path, dir }`, `-o {path}: the directory {dir} does not exist`. The directory is `write.rs`'s `temp_dir_for` (made `pub(crate)` and renamed `parent_dir`), so an empty parent is `.`. Only `fs::metadata(dir)` failing with `ErrorKind::NotFound` is this error; a parent that is a file, or one that cannot be searched, is left to `write_atomic`'s `Unwritable`, which stays the final authority. A directory removed after the check still fails cleanly there.
- **D-74, `-o` naming the input:** `Fatal::OutputIsInput { path }`, `-o {path}: is the input CSV; choose another output path`. Checked when `-o` exists, by comparing `fs::canonicalize` of both paths, so `./`, `..` and symlinks do not slip through. A hard link is not caught; recorded in D-74.
- **NFR-15's font failure:** `Fatal::Font(#[from] FontError)`. `FontMetrics::load(bytes, face: &'static str)` names the face, and `FontError`'s message becomes `the embedded {face} font cannot be read: {reason}`. Loaded in tier one after the flags and before the CSV.
- **`Report`:** `{ warnings: Vec<String>, outcome: Result<(), Vec<String>> }`, all rendered. `main` prints `warnings`, then the `Err` lines, and exits 1 on `Err`. clap's usage errors keep clap's own code (2).
- **The `qual:api` markers** on `cli.rs:103`, `errors.rs:16`, `emit/write.rs:38`, `layout/arrange.rs:77` and `svg/serialise/mod.rs:101` become orphaned once `main.rs` calls those functions; Task 5 deletes them.

## Review rulings

Five seats reviewed revision 1.0: Rust, QA, architecture, accessibility, and security in place of UI. The Rust and QA seats ran probes in scratch copies. Rulings:

- **FR-95 can be driven by a ring** (blocker from QA; Important from Rust and architecture). `label_half_width` takes rings as well as quadrants. A 70-character `--rings` entry with one quadrant measured 749.6 units, and so did a header-only CSV with zero quadrants. The maintainer ruled the warning names whichever label binds: D-73, FR-95 amended.
- **`-o` naming the input CSV** (security) silently replaced the CSV. The maintainer ruled it a tier-one error: D-74, FR-62 amended.
- **The four-quadrant diagonal fixture cannot exist** (QA, Rust): `FIRST_DIVIDER = -90` puts all four middles on diagonals. Task 3 uses three quadrants, whose FR-10 position 0 sits at 6 o'clock.
- **The failure matrix would pass against today's stub** (QA). Each case now has a control run with the fault removed that must exit 0 and change the file.
- **Review Focus 2 had no render-level warning** (QA). `--title "T\u{1}"` supplies one.
- **NFR-5 by allowlist** (security), and **hostile text end to end** (security, QA): through `--title`, `--subtitle` and the CSV, `&` included.
- **FR-62's check** (all seats): NotFound only, empty parent as `.`, via the writer's own `temp_dir_for`.
- **`page` takes a `Document`** (architecture) so 14b adds parameters instead of rewriting; target choice stays in the emitter.
- **`Config`** (architecture) as §7 describes, so FR-65 is resolved once (architecture) and 14b and PR 19 widen one struct.
- **`warn_narrow` takes `&mut Warnings`** (architecture, Rust), and `RadarBlock` returns the label's text, so no parallel `labels` slice can drift from FR-10's order.
- **`<main>` landmark** (accessibility), and the visually-hidden rule pinned.
- **Orphaned `qual:api` markers** (Rust), the font test built from a real `FontError` (Rust), `tests/measure.rs` and `tests/wrap.rs` in Task 1's files (Rust), and `Report::outcome` explicit (architecture).
- **Added to the 14b handoff** (accessibility): the inactive arrangement is `display:none`, so one radar group and one heading outline reach assistive technology.
- **Declined:** a 10k-row smoke test (QA, optional), left to the performance work. `-o` with a trailing slash is covered by `-o` naming a directory.

---

### Task 0: the PRD and the architecture

**Files:** `docs/product/radargen-prd.md` (v0.56), `docs/design/architecture.md`.

- [ ] **Step 1:** In the PRD:
  - **FR-95** names "the label, a quadrant's or a ring's, whose width sets the radar's width"; a ring is named with its `--rings` source.
  - **FR-62** adds: "`-o` naming the input CSV is a hard error (D-74)." NFR-9's list and NFR-16's tier one gain it.
  - **FR-65** adds: "With no title, the HTML page's tab title and a visually hidden level-1 heading read *Technology Radar* (D-75)."
  - **D-73** (ring-driven width), **D-74** (same file, canonicalised; hard links not caught), **D-75** (the title fallback and the radar group's name, with the reasons in this plan's Decisions).
  - Version 0.56, `date modified`, and the revision-history row.
- [ ] **Step 2:** In architecture §3, add `src/pipeline.rs` ("the run: `Config`, the stages, `Report`") and narrow `main.rs` to "parse args, run, print, exit code". In §7, `Config` is built in `pipeline.rs`.
- [ ] **Step 3: Commit** `docs(prd): D-73 to D-75, FR-95's ring, -o as the input, the title fallback`.

---

### Task 1: the new diagnostics

**Files:**

- `src/diagnostics.rs`: `Fatal::MissingDirectory`, `Fatal::OutputIsInput`, `Fatal::Font`; `RenderKind::NarrowWidened { binding: Binding, width: u32 }` and its message. `Binding` lives in `diagnostics.rs` beside `RenderKind`.
- `src/errors.rs`: the exhaustive `Fatal` matches (`Alone`, line 0, rank 0).
- `src/text.rs`: `FontMetrics::load(bytes, face)`, `FontError { face, reason }`; `tests/measure.rs` and `tests/wrap.rs` follow the signature.
- `src/emit/mod.rs`: `pub const OUTPUT_VERSION: &str = concat!("radargen ", env!("CARGO_PKG_VERSION_MAJOR"), ".", env!("CARGO_PKG_VERSION_MINOR"));`.
- `tests/diagnostics.rs`.

- [ ] **Step 1: Failing tests, exact messages:**
  - `MissingDirectory`: `error: -o out/radar.html: the directory out does not exist`.
  - `OutputIsInput`: `error: -o radar.csv: is the input CSV; choose another output path`.
  - `Font`: `Fatal::Font(FontMetrics::load(b"x", "Inter SemiBold").unwrap_err()).render()` starts `error: the embedded Inter SemiBold font cannot be read: `.
  - `NarrowWidened`, both bindings, the exact lines from Decisions, a padded label trimmed.
  - `output_version_has_no_patch`: `OUTPUT_VERSION` is `radargen ` followed by `CARGO_PKG_VERSION`'s first two dot-separated fields, and has exactly one `.`.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(diagnostics): FR-62's directory, D-74, the font failure, FR-95`.

---

### Task 2: the radar group's accessible name (rg-eae)

**Files:**

- `src/svg/mod.rs`: `Semantics::Group(String)`; `is_interactive_only` is `false` for it.
- `src/svg/serialise/group.rs`: `role="group"` then `aria-label`.
- `src/layout/radar/mod.rs`: `fn label(quadrants, rings, items) -> String`; the render's centring group carries it.
- `tests/serialise/vocabulary.rs`: `group` joins the static roles.
- Re-bless `tests/golden/serialiser/*.svg` (`UPDATE_GOLDEN=1`), then read the diff: one changed line each.

- [ ] **Step 1: Failing tests:**
  - `the_radar_is_a_named_group`: the PRD example's radar render has `Semantics::Group("Technology radar: 3 quadrants, 4 rings, 6 items")`. The example has a seventh, unplotted row, so this also proves plotted items are counted.
  - `label_counts_singular_and_plural`: (1, 1, 1) gives `Technology radar: 1 quadrant, 1 ring, 1 item`; (0, 4, 0) gives `Technology radar: 0 quadrants, 4 rings, 0 items`.
  - `a_named_group_in_both_targets`: a hand-built group at `Point::ORIGIN` serialises `<g role="group" aria-label="…">` exactly, in both targets. For the radar, assert the substring ` role="group" aria-label="Technology radar: 3 quadrants, 4 rings, 6 items"`, after its `transform`.
- [ ] **Step 2–4:** red, implement, green; re-bless and read the diff; lints; rustqual.
- [ ] **Step 5: Commit** `feat(svg): the radar is a named group for assistive technology (rg-eae)`.

---

### Task 3: FR-95's narrow-width warning (rg-pti)

**Files:** `src/layout/radar/mod.rs` (`binding_label`), `src/layout/arrange.rs` (`warn_narrow`), tests in each.

- [ ] **Step 1: Failing tests** (fixtures: `arrange.rs`'s `beyond()`, `widened()`, `thoughtworks()`, and the PRD example):
  - `fr95_names_the_binding_quadrant`: `beyond()` warns once, naming its long quadrant, with `width` the narrow width rounded up.
  - `fr95_names_a_diagonal_over_a_longer_six_oclock`: three quadrants, the one at FR-10 position 0 (6 o'clock) the longest name, a diagonal one shorter but reaching further; the diagonal one is named. Assert both names' lengths in the test, so the fixture cannot silently stop meaning this.
  - `fr95_names_a_binding_ring`: `--rings` with a 70-character entry and quadrant `Tools` names the ring.
  - `fr95_with_no_quadrants`: the same rings and a header-only CSV names the ring.
  - `fr95_is_quiet_within_the_headroom`: `widened()`, `thoughtworks()` and the PRD example push nothing.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(layout): FR-95 names the label past 480 (rg-pti, D-73)`.

---

### Task 4: the minimal HTML page

**Files:** Create `src/emit/html/mod.rs` (`pub mod html;` in `src/emit/mod.rs`).

**Interfaces:** `pub fn page(wide: &Document, title: Option<&str>) -> String`. `title` is `Config`'s resolved title. `page` serialises `wide` with `Target::Static(&Palette::built_in(Theme::Light))`.

- [ ] **Step 1: Failing tests** (the tests build `wide` with `arrange(Wide, &prd().blocks)`):
  - `the_page_is_minimal_and_valid`:
    - it starts `<!DOCTYPE html>\n<html lang="en">`, and ends `</html>\n`; LF only, no BOM;
    - it has the charset meta, the viewport meta exactly (no `maximum-scale`, `minimum-scale` or `user-scalable`), and `<meta name="generator" content="{OUTPUT_VERSION}">`;
    - it has the visually-hidden rule exactly;
    - the SVG appears once, inside `<main>`, inside `<body>`.
  - `the_page_requests_nothing` (NFR-5): a tag scan finds only `html head meta title style body main h1` outside the SVG, and the SVG's elements and attributes are `tests/serialise/vocabulary.rs`'s; no attribute is `src`, `srcset`, `href`, `xlink:href`, `action`, `data`, `poster` or `http-equiv`; `<style>` has no `url(`, `@import` or `src:`.
  - `the_title_falls_back`: for `None`, `<title>Technology Radar</title>` and `<h1 class="visually-hidden">Technology Radar</h1>` before the SVG; with `Some("Q3 Radar")`, `<title>Q3 Radar</title>` and no `<h1`.
  - `a_hostile_title_is_escaped`: `</title><script>&amp;"` arrives as `&lt;/title&gt;&lt;script&gt;&amp;amp;"` in `<title>`, and the tag scan finds no `script` element.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(emit): a minimal HTML page around the wide radar`.

---

### Task 5: `pipeline::run` and `main.rs`

**Files:** Create `src/pipeline.rs` (`pub mod pipeline;`); modify `src/main.rs` and `src/emit/write.rs` (`parent_dir`); delete the five orphaned `qual:api` markers; create `tests/run.rs`.

**Interfaces:**

```rust
/// The resolved run (architecture §7), read-only once built.
pub struct Config {
    /// `-o`, as given.
    pub output: PathBuf,
    /// FR-65, resolved: cleaned (FR-90) and `None` when empty after trim.
    pub title: Option<String>,
    /// As `title`.
    pub subtitle: Option<String>,
    /// The declared rings and their source (FR-12, FR-17).
    pub rings: RingSet,
}

/// What a run printed (NFR-11).
pub struct Report {
    /// Rendered warning lines, in print order.
    pub warnings: Vec<String>,
    /// `Ok` when `-o` was written; otherwise the rendered error lines.
    pub outcome: Result<(), Vec<String>>,
}

/// The whole pipeline for `cli` (architecture §2).
#[must_use]
pub fn run(cli: &Cli) -> Report;
```

The run is split into integration and operation functions under IOSP:

- `config(&Cli, &mut Warnings) -> Result<Config, Vec<Fatal>>`: FR-52, FR-62, D-74, the cleaned flags, `parse_rings`.
- `catalogue(&Path, &RingSet, &mut Warnings) -> Result<Catalogue, Vec<String>>`: read, parse and build; tier one rendered alone, tier two merged and rendered.
- `render(&Catalogue, &Config, &Fonts, &mut Warnings) -> String`: `resolve`, `warn_crowding`, the blocks, `warn_narrow`, `arrange(Wide)`, `page`.
- `write_atomic`.

Warnings are rendered once at the end, so every exit prints those gathered so far. `main`: `let report = run(&Cli::parse());`, `eprintln!` each warning then each error, `ExitCode::from(u8::from(report.outcome.is_err()))`.

- [ ] **Step 1: Failing tests** in `tests/run.rs`. Each runs the binary with `current_dir` a `tempfile::tempdir()` and relative paths, so messages carry no platform-specific prefix, and reads `status`, `stdout` and `stderr`. A `Writable` guard as in `src/emit/write.rs`'s tests restores permissions on Windows.
  - `renders_the_prd_example`: `tests/fixtures/three-quadrant.csv` gives exit 0, empty stdout, stderr exactly the FR-17 line for `Rejected` (as `tests/model.rs:65`), and a file holding the generator meta and ` role="group" aria-label="Technology radar: 3 quadrants, 4 rings, 6 items"`.
  - `a_failed_run_leaves_the_old_file` (Review Focus 1). For each case, seed `-o`, run, and check exit ≠ 0, the bytes unchanged, the directory listing unchanged and stdout empty; then run the **control** with the fault removed and check exit 0 and changed bytes. Cases: `--rings ","`; a missing CSV; `tests/fixtures/row-errors.csv`; `-o` a read-only file; `-o` naming an existing directory (`out.html/`).
  - `warnings_then_errors_on_stderr_only` (Review Focus 2): `--title "T\u{1}"` and a CSV with an FR-90 character and row errors. stderr is the `FlagUndisplayable` line, then the row warning, then every `error:` line; stdout is empty.
  - `tier_one_still_prints_flag_warnings`: `--title "T\u{1}" --rings ","` gives the warning line, then the `--rings` error.
  - `hostile_text_stays_text` (Review Focus 3): `tests/serialise/hostile.rs`'s `HOSTILE` plus `</title>`, `</svg>`, `</style>`, `<![CDATA[`, `]]>` and `&amp;`, through `--title`, `--subtitle` and a CSV's `name` and `quadrant`. Task 4's tag scan passes; `&amp;` arrives as `&amp;amp;`.
  - `output_paths` (Review Focus 4):
    - `-o radar.html` and `-o ./radar.html` succeed;
    - `nope/radar.html` with a missing CSV gives exactly `error: -o nope/radar.html: the directory nope does not exist`;
    - an absolute missing directory gives the same message with its path;
    - `in.csv/radar.html`, where `in.csv` is a file, gives `Unwritable`, not "does not exist";
    - `-o in.csv` where `in.csv` is the input, and `-o ./in.csv`, give `OutputIsInput` and leave the CSV unchanged. (`in.csv` fails FR-52 first, so these use an input named `in.html`.)
  - `escaped_paths`: an `-o` inside a missing directory whose name holds ESC and U+202E: stderr has no raw 0x1B or U+202E.
  - `bad_extensions_and_v1_1_formats`: `.txt` gives FR-52's message; `.png` gives `arrives in radargen 1.1`; no file is written.
  - `theme_and_width_are_ignored_on_html`: `--theme dark --width 800` gives exit 0, empty stderr, and the same bytes as without them.
  - `fr95_reaches_stderr`: a CSV with one 70-character quadrant name and an FR-17 row: FR-95's line first, then the row warning; exit 0.
  - `realistic_inputs`:
    - a header-only CSV (FR-42) gives exit 0, empty stderr, `0 quadrants, 4 rings, 0 items`;
    - an empty file gives exit 1 and the no-header message exactly;
    - a BOM and CRLF export of the PRD example gives exit 0 and the same bytes as the LF one.
  - `the_binary_prints_usage`: no arguments gives exit ≠ 0 and `Usage:` on stderr (FR-60).
- [ ] **Step 2–4:** red, implement, green; delete the five `qual:api` markers; lints; rustqual. Run the binary on `tests/fixtures/three-quadrant.csv` and look at the page in a browser.
- [ ] **Step 5: Gate** `just ci`.
- [ ] **Step 6: Commit** `feat: the pipeline, end to end (PR 14a)`.

---

## Finishing

1. **Reviews:** a final whole-branch review on the most capable model in Copilot's seat (Copilot is out of credits until November 2026; say so), then simplify, then a final check.
2. **Crumbs:**
   - `crumbs close rg-pti` and `crumbs close rg-eae`.
   - `rg-f27` stays open for 14b, with a note that 14a is done and 14b:
     - widens `emit::html::page` to both arrangements, `Interactive`, the inactive one `display:none` so one radar group and one heading outline reach assistive technology;
     - adds the palette stylesheet with `prefers-color-scheme` and `[data-theme]`, the inlined WOFF2 (the one `url(data:…)` NFR-5 allows), the indicator CSS and the media query from `breakpoint_px`;
     - keeps the fallback `<h1>`, `<main>` and `lang`;
     - adds the cross-process determinism test and the HTML5-parser round trip.
3. Move this plan to `docs/plans/done/`, tick the steps, and set `status: complete` and `completed_date`.
4. **No version bump.** Commit, push, open the PR, check CI. Merge when clean if the maintainer authorises it.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-12 | Initial version, for the plan review. |
| 1.1 | 2026-10-12 | The five-seat review and the maintainer's rulings: FR-95 names a quadrant or ring (D-73); `-o` naming the input refused (D-74); the title fallback and group name recorded (D-75); `Config`; `page` takes a `Document`; `warn_narrow` pushes; FR-62 on NotFound only; NFR-5 by allowlist; hostile text end to end; control runs in the failure matrix; `<main>`; the 14b `display:none` handoff. |
