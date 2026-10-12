---
tags:
  - plan
  - svg
  - output
aliases: []
doc_id: RG-013
doc_name: pr-13-serialiser
crumb_id: rg-6ku
document_title: "PR 13 — the SVG serialiser and the atomic write: Implementation Plan"
created_date: 2026-10-12
synopsis: "svg::serialise turns a Document into SVG markup for two targets: interactive (HTML: var() colours, controls with role, tabindex and data-row, hit areas and indicators kept) and static (SVG and PNG: literal colours, interactive-only nodes dropped, controls as role='img' with aria-label and title). emit::write_atomic writes bytes through a same-directory temp file and a rename, with fs::write's permissions, leaving any existing file byte-unchanged on failure (architecture §5, §6; NFR-4, NFR-6, NFR-14, NFR-17, SC-18)."
status: complete
type: Plan
revision: "1.2"
review_date: 2026-10-12
reviewed_by: [rust-engineer, architect-reviewer, qa-expert, security-auditor, accessibility-tester]
completed_date: 2026-10-12
comments: "1.1 folds in the five-seat plan review. See Review rulings."
---
# PR 13 — the SVG serialiser and the atomic write: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-6ku` · **Phase:** v1.0 · **After:** PR 12 (`arrange`, `svg::Document`), PR 03 (`escape`, `num`) · **Then:** PR 14 (HTML output, `rg-f27`)

**Goal:** Two pieces, each tested on its own and together:

- **The serialiser.** `svg::serialise(&Document, Target)` writes a `Document` as one `<svg>` element, one element per line.
- **The atomic write.** `emit::write_atomic(path, bytes)` puts bytes at `-o` atomically, with the permissions `fs::write` would give.

PR 14 inlines the interactive output into HTML. PR 16 rasterises the static output. PR 17 extends the serialiser (outlined text, a root `<title>`/`<desc>`) rather than wrapping its string.

**Architecture:**

- **`src/svg/serialise/`**, re-exported as `svg::serialise`. A recursive writer over `Node` into one `String`.
  - `mod.rs` holds the entry point, `Target`, `Context`, the attribute-name constants, and the `attr`/`coord` push helpers.
  - `shape.rs` holds circle, rect, line, path and hit area.
  - `text.rs` holds text, tspans and halos.
  - `group.rs` holds groups and semantics.
- **Every attribute goes through one helper.**
  - `attr(out, name, value)` always uses `escape::xml_attr`.
  - `coord(out, name, v)` always uses `num::fmt_coord`.
  - Text content goes through `escape::xml_text`.
- **Colours are bound late.** `Target::Interactive` writes `Palette::var`; `Target::Static` writes `Palette::value`. `Paint::NumberOn` is a literal in both.
- **Interactive-only nodes** are dropped at any depth in `Static`.
- **A group writes its children into a buffer first.** An empty buffer writes nothing: one rule, no separate predicate to drift from the writer.
- **`src/emit/write.rs`** uses `tempfile::Builder` with mode 0o666 on unix, so the umask applies as with `fs::write`, then `tempfile_in(dir)`, `write_all`, `sync_all` and `persist`. tempfile's drop guard removes the temp file on every error return.

**Tech Stack:** Rust 2024. One new dependency, `tempfile` 3 (MIT OR Apache-2.0). Verified in review:

- `cargo deny check licenses` passes;
- `multiple_crate_versions` stays clean;
- it adds errno, fastrand, getrandom, rustix, linux-raw-sys and r-efi.

Library-first: it provides the random `O_EXCL` name, the rename and the drop guard. The dev-dependencies `roxmltree`, `resvg`, `usvg`, `tiny-skia` and `fontdb` are present, along with `tests/common/fonts.rs::usvg_options()`.

**Spec:** `docs/product/radargen-prd.md` v0.54:

- FR-36, FR-47, FR-48, FR-50 (role and name; state in PR 15), FR-54, FR-56, FR-62, FR-82, FR-94
- NFR-1, NFR-4, NFR-6, NFR-8, NFR-10, NFR-12, NFR-14, NFR-15, NFR-17
- SC-17, SC-18, SC-25
- D-15, D-40, D-52, D-54, D-66

Design: `docs/design/architecture.md` §3, §5, §6, §10. The crumb `rg-6ku` carries the per-node rules from PRs 09–12; all are carried below.

## Global Constraints

- **No hex in a `Node`; binding is the emitter's.**
  - Interactive writes `palette.var(role)`.
  - Static writes `palette.value(role)`. resvg renders an unresolved `var()` black; review verified this.
  - `NumberOn(i)` is `palette.number_ink(i)` in both. `Paint::None` is `none`.
- **One quoting path.** Every attribute is written by `attr`/`coord`. No hand-written `="…"` and no `format!` of a float. `clippy::format_push_string` is denied: push only, never `push_str(&format!(..))`.
- **Escapers, by field.**
  - `Control.label` → `aria-label` via `xml_attr`.
  - `Control.tooltip` → `<title>` content via `xml_text`.
  - `Text.lines` → text and `<tspan>` content via `xml_text`.
  - No other `Node` field is a string. Attribute values the program builds (numbers, keys, levels, `var()` names, constants) pass through `attr` all the same.
- **No `id` and no `url(#…)` inside the SVG.** No `<foreignObject>` (FR-47). An HTML page may give its one panel an `id` outside both arrangements (PR 15, `aria-controls`).
- **Identity is the CSV line:** `data-row="{key.0}"`, never FR-34's number.
- **"Omit when zero" is decided on the formatted value** (`fmt_coord(v) == "0"`), never the raw float. That covers `transform`, `rx` and `letter-spacing`. Otherwise a 1e-13 trig residue writes `translate(0 0)` on one platform and nothing on another (NFR-4).
- **Byte-identical output.**
  - Attribute order is fixed by the code, and nothing iterates a hash map.
  - Output is one element per line (`\n` after each tag), with no indentation and no trailing newline after `</svg>`.
  - LF only, no BOM (NFR-17).
- **Lints:**
  - every `pub` item has docs;
  - `#[must_use]` on `pub fn`s, **except** those returning `Result` (`clippy::double_must_use`);
  - `# Errors` docs on fallible `pub fn`s;
  - descriptive bindings;
  - `indexing_slicing` is denied;
  - ≤ 4 parameters: bundle `fill`, `stroke` and `stroke_width` into a `Paints` struct.
- **rustqual IOSP shape**, as verified in review:
  - dispatch `match`es whose arms are each one call;
  - early-return guards;
  - children built with `.map(..).collect::<String>()`;
  - small operations for the rest (`enclose(open, children)`).
  - Not `for` loops mixing logic and calls, and not multi-statement match arms.
  - `pub` entry points carry `// qual:api — PR 14's main.rs is its first production caller`.
  - If the score rises, run `just quality-baseline`.
- Commits: `type(scope): description`, ≤ 72 characters per line, trailer `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`; run `git-mit es` first.

## Review Focus

1. **Colours really bind.**
   - Interactive and static output of the same document differ only in `fill`/`stroke` values and in interactive-only nodes. Every interactive `var(--x)` matches the static literal `palette.value(x)`, in both themes.
   - A Series(0) rect rendered by resvg from static output is #0072B2 at its centre; from interactive output it is black.
   - *Tests: Task 4, `colours_bind_late_across_the_document` and `static_colours_are_what_resvg_paints`.*
2. **Hostile CSV text** (NFR-6) in every column that reaches a node (name, quadrant, ring) and in the title: parsed back, each text equals the `Document`'s string, nothing outside the element allow-list. *Test: Task 4, `hostile_text_round_trips`.*
3. **Interactive-only nodes nested in a control** (a legend row's indicator) are dropped in static output. In interactive output they are written as `data-row-indicator`, as a direct child of the `data-legend-row` group, since PR 14's `>` selector needs that. *Test: Task 3, `indicators_follow_their_enclosing_control`.*
4. **A write that fails or would surprise:**
   - `-o` is a directory, a read-only file, or in a read-only directory: the existing file is byte-unchanged and the directory listing is unchanged.
   - The output has `fs::write`'s mode.
   - A symlink at `-o` is replaced, not followed.
   - *Tests: Task 1.*
5. **Determinism and vocabulary.** The PRD example's static wide document matches its golden byte for byte on every CI OS. Every element and attribute is from the allow-list, and every number has at most 3 decimals and no exponent. *Tests: Task 4, `prd_example_matches_its_golden` and `output_uses_only_its_vocabulary`.*

## Review rulings (plan review, 2026-10-12)

Five seats reviewed revision 1.0. The security seat replaced the UI seat, since this PR has an injection surface and no visual design. The Rust and QA seats ran probes in scratch copies. Rulings:

- **Headings stay in static output** (blocker raised by all four other seats). Static writes `role="heading" aria-level`: a standalone SVG opened in a browser or by a screen reader keeps its outline. The static check forbids `role="button"`, `tabindex`, `data-*`, `pointer-events`, `visibility`, `var(`, `id` and `url(`, on parsed attributes rather than by grepping text. "data-mesh" is a plausible item name.
- **Static controls are `role="img" aria-label`** (a11y). A `<g>` with no role is "generic", where `aria-label` is prohibited, and engines vary. `img` makes the children presentational, so the label, which already holds the visible text (FR-50, 2.5.3), is read once and the number and name are not repeated. Architecture §5's "write a Control as `<g aria-label><title>`" is updated to match. Interactive stays `role="button" tabindex="0"`.
- **`<title>` beside `aria-label` on markers** (a11y). The title becomes the accessible description, so a marker reads "1. Kafka, Default; Platform quadrant, Adopt ring, button, 1. Kafka". Accepted for now. The tooltip is FR-36's, and the alternative (the name in `<title>` alone) changes FR-36's wording. Crumb: PR 15's headless-browser test asserts the computed name and description from the accessibility tree, and the maintainer decides there.
- **`aria-expanded`/`aria-controls` → PR 15** (a11y, architecture). Every interactive control attribute is written by one operation, `control_attrs`, in a fixed order, so PR 15 adds them in one place. FR-50 is cited as "role and name; state in PR 15".
- **Output permissions** (blocker raised by four seats; verified 0600). `tempfile::Builder::new()` with `.permissions(Permissions::from_mode(0o666))` under `#[cfg(unix)]` gives 0666 less the umask, as `fs::write` does. The rename replaces the inode, so a replaced file takes the new mode, owner and no extended attributes. This is documented, not preserved.
- **Symlinks are replaced, not followed** (security, QA): rename semantics. A hostile or stale link cannot redirect the write. A `latest.html → editions/…` workflow sees its link replaced by a file. Documented and pinned by a unix test.
- **A read-only existing `-o` file is refused on every OS** (QA). Rename over it succeeds on unix (the directory's permissions decide) and fails on Windows. Refusing everywhere matches `fs::write` and Windows. `write_atomic` checks `metadata(path)?.permissions().readonly()` first and returns `Unwritable`. The check-then-rename race is benign: it can only allow or refuse, never misdirect.
- **Clean error text** (Rust; verified). tempfile's own errors append its random temp path (`at path ".../.tmpXXXX"`). That is non-deterministic, names a file the user never gave, and breaks SC-10's exact messages. So:
  - write through `file.as_file_mut().write_all` (raw OS errors);
  - map a creation error to `io::Error::from(e.kind())`;
  - the message is `"-o {path}: cannot be written: {source}"`, matching the other `-o` errors (NFR-10).
- **Missing directory** (architecture). `write_atomic` is the authority and reports it as `Unwritable`. 1.0's "can't tell apart without racing" was wrong: `NotFound` says so after the fact. PR 14 may pre-check FR-62 as a fail-fast courtesy, but `write_atomic`'s result decides.
- **Palette literals are validated by construction** (security). Today `Palette::built_in` is the only constructor, and its values are constants. A rendered `fill` must match `^#[0-9A-F]{6}$|^none$`, and a unit test pins it. Crumb `rg-cdo` (PR 18): every new `Palette` constructor validates FR-86's `#RRGGBB` before storing a value. A `HexColour` newtype is the likely shape, so a `--palette` value can never reach an attribute unchecked. A `url(…)` in `fill` would be an external paint server.
- **Module and names** (architecture, Rust):
  - `svg::serialise`, from `src/svg/serialise/`, not `svg::write::svg` (stutter), and distinct from `emit::write`.
  - `pub const` attribute names (`DATA_ROW`, `DATA_MARKER`, `DATA_LEGEND_ROW`, `DATA_HIT`, `DATA_MARKER_INDICATOR`, `DATA_ROW_INDICATOR`) for PR 14's CSS generator.
  - `font-family` from `text::FAMILY`, the name PR 14's `@font-face` uses.
  - `Target::palette()`.
- **Empty and zero-line text**, as written in 1.0: `Text { lines: [] }` writes nothing, and a `""` line writes an empty `<tspan>`. A line is never a run of whitespace (wrap normalises it); `debug_assert!`. An empty tooltip (`Some("")`) writes no `<title>`.
- **Golden files** (architecture, QA):
  - The switch is architecture §10's `UPDATE_GOLDEN=1`, not a new name.
  - Blessing writes the file and then **panics** ("blessed …; unset UPDATE_GOLDEN and re-run"), so a blessing run is never green.
  - It refuses when `CI` is set.
  - A missing golden fails and is never auto-created.
  - A mismatch reports the first differing byte with context.
  - The bytes are checked for no `\r` and no BOM before the compare.
  - Serialiser goldens live in `tests/golden/serialiser/`, and `.gitattributes` gains `tests/golden/** -text`. Architecture §6 already says so.
- **Determinism**: the in-process twice-over test is serialiser-level only. The cross-process `tests/determinism.rs` goes to PR 14, since it needs the binary. A golden failing only on Windows is a real NFR-4 bug: `sin_cos`/`atan2` precision is platform-defined.
- **PR 17 extends; it does not wrap** (architecture). The rg-m2l note says PR 17 adds a text-emission seam (outlined vs `<text>`) and a root-prelude hook (`<title>`, `<desc>`, `<defs>`) to `svg::serialise`, plus the version comment before the root. It is not built now (YAGNI).
- **The radar group's accessible name** gets its own crumb (layout + serialiser + copy, before or with PR 15). Its name must not duplicate PR 17's root `<title>`.
- **Not adopted:**
  - `aria-hidden` on hit areas: an unnamed `fill="none"` circle is exposed by no engine.
  - An HTML-parser round trip here: that goes to PR 14, which inlines into HTML.
  - Preserving a replaced file's mode: rename cannot without extra syscalls, and a fresh create is the rule `fs::write` users expect.

## Decisions taken in this plan

- **Target, not format.** `Target::{Interactive(&Palette), Static(&Palette)}`, because the binding and interactive-only omission are both decided by whether the consumer chooses (architecture §5).
- **Root:** `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}">`, with no `role`. The radar's controls are its content, so the root must not be `role="img"`.
- **Groups:**
  - `<g>`, with `transform="translate({x} {y})"` unless both coordinates format as `0`.
  - Children are written to a buffer first, and an empty buffer writes nothing: the empty title block, and the static hit and indicator layers.
  - A control group whose children all drop would vanish with its label. No block produces one, and a `debug_assert!` pins that.
- **Semantics:**
  - `Decorative` → `aria-hidden="true"`; the key's rule is such a group.
  - `Heading(n)` → `role="heading" aria-level="{n}"`, in both targets.
  - `Marker(c)`:
    - interactive: `role="button" tabindex="0" aria-label data-row="{line}" data-marker="{line}"`;
    - static: `role="img" aria-label`;
    - both: a first-child `<title>` when the tooltip is non-empty.
  - `LegendRow(c)`: the same, with `data-legend-row` in place of `data-marker`; legend rows have no tooltip.
  - `Indicator(key)`: interactive only.
    - Inside a `LegendRow`: `data-row-indicator="{line}"`.
    - Elsewhere: `data-marker-indicator="{line}"`.
    - Both: `visibility="hidden" aria-hidden="true" pointer-events="none"`.
  - `None` → a plain `<g>`.
  - The enclosing control is carried down the whole subtree in `Context` and reset at the control's end.
- **Shapes:**
  - `Circle` → `<circle cx cy r fill stroke [stroke-width]/>`.
  - `Rect` → `<rect x y width height [rx] fill stroke [stroke-width]/>`.
  - `Line` → `<line x1 y1 x2 y2 stroke [stroke-width]/>`.
  - `Path` → `<path d fill stroke [stroke-width]/>`, with `d` = `M x y`, `A r r 0 {large} {sweep} x y`, as 0/1 flags.
  - `stroke-width` is written only when the stroke is not `none`.
  - `HitArea` → `<circle cx cy r fill="none" pointer-events="all" data-hit="{line}"/>`, interactive only.
- **Text:**
  - `<text x y font-family font-size font-weight [letter-spacing] [text-anchor="middle"] fill>`.
  - One line is the content. More are `<tspan x y>` at absolute `y` = baseline + i·leading.
  - A halo is `fill="none" stroke stroke-width stroke-linejoin="round" aria-hidden="true"`.
  - In interactive output, text anywhere inside a control gets `pointer-events="none"`.
- **`write_atomic`:**
  - `dir` = the path's parent, or `.` when it is empty (`temp_dir_for`, unit-tested).
  - Refuse a read-only existing target.
  - Then `Builder` (0o666 on unix), `tempfile_in(dir)`, `write_all`, `sync_all` and `persist`.
  - Errors map to `Fatal::Unwritable { path, #[source] source }`, written `"-o {path}: cannot be written: {source}"`.
  - No directory fsync: atomicity holds without it (you get the old file or the new one), and durability across power loss is out of scope; documented.
  - The rustdoc records the reasoning: random `O_EXCL` names, a sticky directory fails closed, the TOCTOU is benign. A SIGKILL can leave a `.radargen-*.tmp` (`Builder::prefix`), since only error returns run the drop guard.
- **No version bump:** `main.rs` is still a stub.

---

### Task 1: `emit::write_atomic` (NFR-14, NFR-15, SC-18)

**Files:**

- Create `src/emit/mod.rs` and `src/emit/write.rs`.
- Modify:
  - `src/lib.rs` (`pub mod emit;`);
  - `Cargo.toml` (`tempfile = "3"`);
  - `.gitattributes` (`tests/golden/** -text`);
  - `src/diagnostics.rs` (`Fatal::Unwritable`);
  - `src/errors.rs` (its exhaustive `classify`, `line_of` and `rank_of` matches: `Alone`, line 0, rank 0);
  - `tests/diagnostics.rs` (the message).

**Interfaces:**

- Produces:
  - `pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Fatal>` (`// qual:api`, `# Errors`, no `#[must_use]`);
  - `fn temp_dir_for(path: &Path) -> &Path`;
  - `Fatal::Unwritable { path: String, source: std::io::Error }`.

```rust
/// `bytes` at `path`, atomically (NFR-14): written to a temporary file
/// beside it with the permissions `fs::write` would give, flushed to disk
/// and renamed into place. On an error return any file already at `path`
/// is byte-unchanged and the temporary file is gone. A symlink at `path` is
/// replaced, not followed, and a replaced file takes the new file's mode
/// and owner (rename semantics).
///
/// # Errors
///
/// [`Fatal::Unwritable`] when the directory is missing or unwritable, the
/// path names a directory or a read-only file, or the disk is full.
// qual:api — PR 14's main.rs is its first production caller
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Fatal> {
    let unwritable = |source| Fatal::Unwritable { path: path.display().to_string(), source };
    refuse_read_only(path).map_err(unwritable)?;
    let mut builder = tempfile::Builder::new();
    builder.prefix(".radargen-").suffix(".tmp");
    // tempfile creates 0600; give the output what fs::write would.
    #[cfg(unix)]
    builder.permissions(std::os::unix::fs::PermissionsExt::from_mode(0o666));
    let mut file = builder
        .tempfile_in(temp_dir_for(path))
        .map_err(|e| unwritable(std::io::Error::from(e.kind())))?;
    file.as_file_mut().write_all(bytes).map_err(unwritable)?;
    file.as_file().sync_all().map_err(unwritable)?;
    file.persist(path).map_err(|e| unwritable(e.error))?;
    Ok(())
}
```

The shape follows the IOSP rule: if rustqual flags the guard plus calls, split the read-only check into its own operation.

- [x] **Step 1: Failing tests.** Use `tempfile::tempdir()` (removed on drop, even after a panic), compare bytes with `fs::read`, and use no mocks.
  - **The plain cases:**
    - `writes_a_new_file`: exact bytes back (LF text, no BOM).
    - `replaces_an_existing_file`.
    - `temp_dir_for_names_the_parent`: `out.html` → `.`, `a/out.html` → `a`, `/out.html` → `/`.
  - **Failures leave everything as it was:**
    - `a_directory_at_the_path_is_unwritable`. This case creates a temp file and then fails `persist`, so it exercises the drop guard. Check:
      - the parent's listing is identical before and after;
      - `Error::source` is `Some`;
      - the message starts `"-o {path}: cannot be written: "` and contains no `.radargen-`/`.tmp`;
      - no `io::ErrorKind` is asserted, since it varies by OS.
    - `a_read_only_file_is_refused`: `set_readonly(true)` and restored by a guard. `Unwritable` with the old bytes unchanged, on every OS.
    - `#[cfg(unix)] a_read_only_directory_is_unwritable`:
      - mode 0o555, restored by a guard's `Drop`;
      - first probe that creating a file there fails, and fail with "running as root?" otherwise;
      - the old bytes are unchanged and the listing holds only that file.
    - `a_missing_directory_is_unwritable`: no directory is created.
  - **Files come out as users expect:**
    - `#[cfg(unix)] the_output_gets_the_mode_fs_write_gives`: compared with a sibling written by `fs::write`, so it is independent of the umask.
    - `#[cfg(unix)] a_symlink_at_the_path_is_replaced_not_followed`: the link's target is byte-unchanged, and `-o` is a regular file.
    - `concurrent_writes_leave_one_whole_file`: two threads, distinct 1 MiB payloads, 20 rounds each. The file equals one payload and the directory holds only it.
- [x] **Step 2–4:** red, implement, green. Run clippy, `cargo deny check licenses` and rustqual.
- [x] **Step 5: Commit** `feat(emit): write output atomically (NFR-14, SC-18)`.

---

### Task 2: `svg::serialise`, shapes, text, paint and groups

**Files:** Create `src/svg/serialise/{mod,shape,text,group}.rs`; modify `src/svg/mod.rs` (`pub mod serialise; pub use serialise::{serialise, Target};`).

**Interfaces:**

- Consumes: `svg::{Document, Node, Group, Paint, TextPaint, Anchor, PathCommand, Point}`, `palette::Palette`, `num::fmt_coord`, `escape::{xml_text, xml_attr}`, `text::{Weight::css, FAMILY}`.
- Produces:

```rust
/// Who reads the markup, which decides how colours bind and whether
/// interactive-only nodes are kept (architecture §5).
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    /// HTML: the viewer chooses the theme, so colours are `var()`s; hit
    /// areas, indicators and control roles are kept (FR-56, FR-50). The
    /// palette serves series cycling and number inks only.
    Interactive(&'a Palette),
    /// SVG and PNG: colours are this palette's literals, and nodes that need
    /// a pointer or focus are dropped (FR-56, FR-54).
    Static(&'a Palette),
}

impl Target<'_> {
    /// The palette either way.
    #[must_use]
    pub const fn palette(&self) -> &Palette;
}

/// `document` as one `<svg>` element, one element per line.
#[must_use]
// qual:api — PR 14's main.rs is its first production caller
pub fn serialise(document: &Document, target: Target<'_>) -> String;

/// The data attributes PR 14's stylesheet and script select on.
pub const DATA_ROW: &str = "data-row";
// … DATA_MARKER, DATA_LEGEND_ROW, DATA_HIT, DATA_MARKER_INDICATOR, DATA_ROW_INDICATOR
```

The internal helpers are `attr(out: &mut String, name: &str, value: &str)`, which always calls `xml_attr`, and `coord(out: &mut String, name: &str, v: f64)`, which always calls `fmt_coord`.

- [x] **Step 1: Failing tests.** Hand-built nodes, with exact expected strings, since attribute order is part of NFR-4.
  - `the_root_carries_the_view_box`: an empty 420.5 × 100 document gives exactly `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 420.5 100">\n</svg>`.
  - `shapes_write_their_attributes`:
    - each shape, with `stroke-width` absent for `Paint::None`;
    - `Rect` with radius 0, and with radius 0.0004, gives no `rx`;
    - arcs exactly: `(large=true, sweep=false)` → `A r r 0 1 0 x y` and `(false, true)` → `A r r 0 0 1 x y`.
  - `paint_binds_late`:
    - `Role(Series(9))` gives `var(--series-2)` interactive and the hard-coded light literal for index 1 static;
    - `NumberOn(9)` == `NumberOn(1)` and `NumberOn(8)` == `NumberOn(0)` in both targets;
    - `None` gives `none`.
  - `static_paint_is_a_hex_literal_or_none`: every `Role` the palette declares, plus `NumberOn` 0..16, bind to `^#[0-9A-F]{6}$` (or `none`), in both themes.
  - `text_writes_lines_and_style`:
    - one line;
    - two `Middle` lines (tspans at `y` and `y + leading`, both through `fmt_coord`);
    - `letter-spacing` written only when its formatted value is non-zero (1e-5 → absent);
    - `font-weight` 600;
    - `TextPaint::Fill(Paint::None)` → `fill="none"`;
    - `lines: []` writes nothing;
    - `["", "a"]` writes an empty tspan.
  - `a_halo_is_an_outline_hidden_from_assistive_technology`.
  - `groups_translate_and_empty_ones_vanish`:
    - at the origin and at (1e-7, 0): no `transform`;
    - at (5, 7.25): `translate(5 7.25)`;
    - `Group[]`, `Group[Group[]]` and `Group[Group[Group[]]]` write nothing.
  - `every_float_goes_through_fmt_coord`:
    - one node of every kind, with every float field 1.234_567: every number in every attribute matches `^-?\d+(\.\d{1,3})?$`;
    - an origin of (-0.0, -3.5) writes `0` and `-3.5`;
    - no `-0` anywhere.
- [x] **Step 2–4:** red, implement in the IOSP shape the Global Constraints give, green; lints; rustqual.
- [x] **Step 5: Commit** `feat(svg): serialise shapes, text and groups, colours bound late`.

---

### Task 3: semantics, interactive vs static

**Files:** `src/svg/serialise/group.rs`, with `control_attrs` as the one operation that writes a control's attributes.

**Interfaces:**

- Consumes: Task 2.
- `Context { target: Target, control: Option<ControlKind> }` (`Copy`), with `enum ControlKind { Marker, LegendRow }`.

- [x] **Step 1: Failing tests:**
  - `headings_and_decoration`: `Heading(2)` gives `role="heading" aria-level="2"` and `Decorative` gives `aria-hidden="true"`, exactly, in **both** targets.
  - `controls_differ_by_target`:
    - a tooltipped `Marker`, interactive: `<g role="button" tabindex="0" aria-label="…" data-row="12" data-marker="12">\n<title>…</title>`;
    - the same, static: `<g role="img" aria-label="…">\n<title>…</title>`;
    - a `LegendRow`: `data-legend-row`, no `<title>`;
    - an empty tooltip: no `<title>`.
  - `the_pointer_reaches_the_control`:
    - interactive text two groups deep inside a `LegendRow` gets `pointer-events="none"`;
    - a text after that `LegendRow`, as its sibling, does not;
    - a halo inside a control gets both `aria-hidden` and `pointer-events` (`roxmltree` would reject a duplicate attribute);
    - static text has no `pointer-events`.
  - `hit_areas_are_interactive_only`: interactive gives the exact `<circle … data-hit="12"/>`; static gives nothing.
  - `indicators_follow_their_enclosing_control` (Review Focus 3):
    - in a `LegendRow`: `data-row-indicator="12"`, as a direct child of the `data-legend-row` `<g>`;
    - an indicator layer after the `LegendRow`, as a sibling: `data-marker-indicator`;
    - in static, both vanish, including the nested one, and so does a `Group[Group(Indicator), Group[HitArea]]` wrapper.
- [x] **Step 2–4:** red, implement, green; lints; rustqual.
- [x] **Step 5: Commit** `feat(svg): controls, indicators and hit areas per target (FR-50 role and name, FR-94)`.

---

### Task 4: whole documents (NFR-4, NFR-6, FR-54)

**Files:**

- Create `tests/serialise.rs`, through the public API.
- Create `tests/golden/serialiser/prd-wide-light.svg` and `prd-wide-interactive.svg`.
- Extend `tests/common` with a catalogue-plus-order builder if `catalogue.rs` lacks the order.
- Modify `docs/design/architecture.md`:
  - §3: a `src/svg/serialise/` row, and the `svg/mod.rs` row reworded;
  - §5: static controls are `role="img"`;
  - §10: `UPDATE_GOLDEN` as used.

**Interfaces:** consumes `layout::arrange::{Blocks, Arrangement, arrange}`, the block constructors, `svg::{serialise, Target, DATA_*}`, `palette::{Palette, Theme}`, and `tests/common/fonts.rs::usvg_options()`.

A shared helper, `vocabulary(doc: &roxmltree::Document, target)`, asserts:

- elements are in {svg, g, circle, rect, line, path, text, tspan, title};
- attribute names come from a per-element allow-list. Static excludes `tabindex`, `data-*`, `pointer-events` and `visibility`, and allows `role` only as `heading` or `img`;
- no `id`, no `on*` and no `href`;
- no value contains `url(`;
- every numeric attribute token matches `^-?\d+(\.\d{1,3})?$` and none is `-0`.

- [x] **Step 1: Failing tests:**
  - `output_uses_only_its_vocabulary` (Review Focus 5): the PRD example, both arrangements, both targets, both themes, plus the hostile document.
  - `colours_bind_late_across_the_document` (Review Focus 1):
    - walk the interactive and static trees in parallel, skipping interactive-only elements;
    - every `fill`/`stroke` either matches exactly or maps `var(--x)` → `palette.value(x)`, looked up through `palette.declarations()` by display name;
    - for both themes; and light-static vs dark-static differ only in `fill`/`stroke` values.
  - `static_colours_are_what_resvg_paints`: a hand-built `Rect` filled `Role(Series(0))`, rendered by resvg.
    - From static output, the centre pixel is (0x00, 0x72, 0xB2).
    - From interactive output, it is black. This documents why binding is late.
    - The usvg tree's text-node count equals the `<text>` count, since usvg silently drops text whose font does not resolve.
  - `interactive_output_is_keyed_by_csv_line`, with expectations from the catalogue, not the `Document`:
    - the `data-row` values are exactly the plotted lines, and the unplotted line (8) appears in no `data-*`;
    - each plotted line appears once each as `data-marker`, `data-legend-row`, `data-hit`, `data-marker-indicator` and `data-row-indicator`;
    - `data-row` == `data-marker` on every marker;
    - the sequence of `(kind, line)` in tree order is every marker in FR-34 order, then every legend row in the same order;
    - the sequence is identical for wide and narrow;
    - a `data-marker-indicator` and its `data-marker` are in the same `<svg>`.
  - `crumb_rules_hold`: the root `<svg>` has no `role`, and the key's rule `<line>` has an `aria-hidden` ancestor.
  - `hostile_text_round_trips` (Review Focus 2):
    - The inputs are hostile name, quadrant and ring values, plus the title: `</text><script>alert(1)</script>`, `A & B "q" 'a'`, `&amp;`, `]]>`, `<!-- x -->`, a lone `<`, `</script>`, U+2028, and a tab inside a quoted value. One hostile name is long enough to wrap.
    - Each `aria-label` == its `Control.label` and each `<title>` == `Control.tooltip`.
    - Each `<text>` with its tspans joined by `\n` == `lines.join("\n")`, compared with the `Document` the serialiser was given.
    - Separately, the hostile name is a substring of its label and tooltip.
    - `vocabulary` holds.
  - `output_is_deterministic`: twice in one process gives equal bytes. Serialiser-level only; the cross-process test is PR 14's.
  - `prd_example_matches_its_golden`:
    - static/light/wide, and interactive/wide, both with the title "Technology Radar";
    - no `\r` or BOM in the bytes, then a byte compare;
    - `UPDATE_GOLDEN=1` writes and then panics, refused under `CI`;
    - missing goldens fail;
    - a mismatch shows the first differing byte with context.
- [x] **Step 2–4:** red, implement. Bless once and read the golden by eye: the colours resolve, the text reads right, and the headings and roles are as decided. Then green; lints; rustqual.
- [x] **Step 5: Gate** `just ci`.
- [x] **Step 6: Commit** `test(svg): whole documents, golden and resvg round trip (NFR-4, NFR-6)` and `docs(architecture): the serialiser as built`.

---

## Finishing

1. **Reviews:** a final whole-branch review on the most capable model in Copilot's seat (Copilot is out of credits until November 2026; say so), then simplify, then a final check.
2. **Crumbs:** `crumbs close rg-6ku`. Notes:
   - **`rg-f27` (PR 14):**
     - **Rendering:**
       - inline `serialise(&arrange(Wide, ..), Target::Interactive(&palette))` and the narrow one;
       - the generator `<meta>` and the output-version constant from `CARGO_PKG_VERSION_MAJOR`/`_MINOR`;
       - `write_atomic` for `-o`, with an optional FR-62 pre-check as a courtesy only.
     - **Indicator rules:**
       - generate them from `svg::DATA_*`, walking the `Document`, not `Plotted`;
       - `svg:has([data-marker="L"]:focus-visible) [data-marker-indicator="L"]` per key, inside `@supports selector(:has(*))`;
       - plus `[data-legend-row]:focus-visible > [data-row-indicator]`;
       - this supersedes PR 10b's `[data-indicator]` note.
     - **Display:**
       - `@font-face` family = `text::FAMILY`;
       - check the indicator in `forced-colors`;
       - add no `<h1>` of its own, since the SVG carries `Heading(1)`;
       - hide the other arrangement with `display: none` on its container.
     - **Tests:**
       - an HTML5-parser round trip of the hostile document, inlined;
       - the cross-process `tests/determinism.rs`.
   - **`rg-6zu` (PR 15):**
     - `aria-expanded`/`aria-controls`, added in `control_attrs`; the panel's one `id` sits outside both arrangements;
     - the headless test asserts each marker's computed name and description (the `<title>` duplication; maintainer decision);
     - FR-34 tab order through the markers.
   - **`rg-42m` (PR 16):** `Target::Static(&palette)`, where `palette` is the run's resolved palette for `--theme` (and `--palette` in v1.1), into resvg with the vendored fonts.
   - **`rg-m2l` (PR 17):**
     - extend `svg::serialise` with a text-emission seam and a root-prelude hook, not string splicing;
     - `<!-- radargen X.Y -->` before the root;
     - the root `<title>`/`<desc>` via `xml_text`;
     - no `role="img"` on the root; consider `xml:lang`;
     - outlined text leaves headings nameless, so give them an `aria-label`.
   - **`rg-cdo` (PR 18):** every `Palette` constructor validates `#RRGGBB` (FR-86) before storing; consider a `HexColour` newtype.
   - **Create:** "Name the radar group for assistive technology" (layout + serialiser + copy; before or with PR 15), and "Bidi overrides in names reach aria-label and title" (an idea; FR-76/FR-90 do not remove Cf controls such as U+202E).
3. Move this plan to `docs/plans/done/`, tick the steps, and set `status: complete` and `completed_date`.
4. **No version bump.** Commit, push, open the PR, check CI. Merge when clean if the maintainer authorises it.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-12 | Initial version, for the plan review. |
| 1.1 | 2026-10-12 | The five-seat review (security in place of UI): headings kept in static; static controls `role="img"`; output permissions as `fs::write`; symlinks replaced; read-only targets refused; clean error text; palette literals validated by construction; `svg::serialise` module, shared attribute constants; omit-when-zero on formatted values; one element per line; the golden workflow per architecture §10; the resvg and late-binding checks that actually test colour; catalogue-derived identity tests; hostile text across columns. |
| 1.2 | 2026-10-12 | Shipped as built. The whole-branch review found Windows CI red (a rename race in the concurrency test, which under fail-fast hid every later test binary; the test now tolerates the refusal on Windows only and CI runs `--no-fail-fast`) and whitespace between tspans that a browser would render; both fixed. The interactive/static comparison covers every attribute bar paint, the interactive-only ones and a control's role. Task 3's semantics were written with Task 2 and their tests shown to fail under mutation. The goldens match byte for byte on all three CI OSes. |
