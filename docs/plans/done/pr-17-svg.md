---
tags:
  - plan
  - svg
  - output
aliases: []
doc_id: RG-017A
doc_name: pr-17-svg
crumb_id: rg-m2l
document_title: "PR 17 — SVG export with outlined, deduplicated text: Implementation Plan"
created_date: 2026-10-16
synopsis: "-o radar.svg writes the wide arrangement (FR-71) as a standalone SVG: literal colours for --theme (FR-56) over D-87's ground, every text drawn as glyph outlines defined once per face and size in <defs> and placed with <use xlink:href> (architecture §11), so no font is referenced (SC-15) and other applications draw it as resvg does. --width sets the intrinsic size, the height following (FR-57). The file carries <!-- radargen 1.2 --> (SC-25), a <title> and <desc>, and names every outlined text for a screen reader opening it directly. Ships as 1.2.0."
status: complete
type: Plan
revision: "1.2"
review_date: 2026-10-16
reviewed_by:
  - "Rust (subagent; compiled the plan's code in a scratch copy)"
  - "QA (subagent; measured the pixel test's sensitivity)"
  - "Architecture (subagent)"
  - "Security (subagent)"
  - "Accessibility and visual design (subagent)"
completed_date: 2026-10-16
comments: "1.1 folds in the five plan reviews; every finding and its ruling is in the Review rulings table. The maintainer confirmed D-88 and keeping the PR whole on 2026-10-16. 1.2 records completion; MAX_DELTA was set to 64 rather than the measured 56, for another platform's antialiasing."
---
# PR 17 — SVG export with outlined, deduplicated text: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-m2l` · **Release:** 1.2.0 · **After:** 1.1.0 (#90) · **Then:** PR 18, `--palette`, 1.3.0 (`rg-cdo`)

**Goal:** `radargen radar.csv -o radar.svg --theme dark --width 1920` writes a standalone SVG of the wide arrangement. Rasterised with no font at all, it matches our PNG to within antialiasing (SC-7). It references no font or other resource (SC-15), and it is byte-identical run to run and across platforms (NFR-4, SC-8).

**Architecture:**

- **Text is outlined, as architecture §11 and `docs/spikes/svg-font.md` decided.** An embedded `@font-face` draws nothing in resvg and the default face in PowerPoint. So each glyph a text uses is shaped by the same `harfrust` pass that measures it and drawn by `skrifa` into an SVG path. It is defined **once per face and size** in `<defs>`, and placed with `<use xlink:href="#gN" x= y=>`.
- **The converter is a text mode of the one serialiser.** Two alternatives were rejected:
  - **A second serialiser** would walk `Document` again and copy its colour binding, group semantics, ground and dropping of interactive-only nodes, and the copies would drift.
  - **String-splicing** the HTML serialiser's output was already rejected by the crumb's PR 13 note.

  So `serialise`'s private `Context` gains a `TextMode`: `Text` (HTML and PNG, `<text>` as today) or `Outlined` (SVG). Only the text arm of `node()` reads it. `serialise::standalone` writes the root's size, `<title>`, `<desc>` and `<defs>`, then the same ground and body `serialise` writes. It hard-codes `Target::Static`, so an outlined *interactive* document cannot be built.
- **`Document` is unchanged.** The HTML and PNG bytes are unchanged too, except the HTML's generator line, which says 1.2. PNG carries no version.
- **`src/text.rs` is the only reader of the fonts.** It gains:
  - `FontMetrics::shape`: glyph ids, clusters and font-unit advances;
  - `place`: each glyph's position;
  - `outline`: a glyph's contours as a crate-local `Segment` list, so skrifa stays private to `text.rs`.

  `measure` is rebuilt on `shape`, and a test pins its bits.
- **`src/emit/svg.rs`** (architecture §3's planned file) is thin:
  - FR-57's intrinsic size;
  - the `<title>`, from the title or D-75's *Technology Radar*, which moves to `emit::UNTITLED`;
  - the `<desc>`;
  - the call to `standalone`;
  - the `<!-- radargen X.Y -->` line. It is an emitter's line (§6), as the HTML writes its own `<meta>`.
- **The pipeline:** `.svg` is a format. `render` arranges the wide arrangement only (FR-71) and returns the bytes for `write_atomic` (NFR-14). `Fatal::FormatNotInThisVersion` loses its last trigger and is removed.

**Tech stack:**

- Rust 2024.
- `skrifa` 0.44 moves from dev-dependency to dependency, with `default-features = false, features = ["std"]`. usvg already builds it, and it shares `read-fonts` 0.41 with `harfrust`, so `harfrust::FontRef` is the type skrifa reads (Rust review: verified in `Cargo.lock` and by compiling). No new crate enters the tree.

**Spec:**

- `docs/product/radargen-prd.md` v0.72, amended to v0.73 by Task 0. The plan rests on: FR-45, FR-52, FR-54, FR-56, FR-57, FR-64, FR-71, NFR-4, NFR-5, NFR-6, NFR-7, NFR-12, NFR-14, NFR-17, SC-7, SC-8, SC-9, SC-15, SC-18, SC-25, D-75, D-78, D-87, §14.1, and the new D-88.
- Architecture §3, §5, §6, §9, §10, §11 and §12. Task 0 amends §5 and §12's "exactly two" rule (Ruling 8).
- `docs/spikes/svg-font.md` and `tests/outlines_match_text.rs`, which measured the outlined form.

## Global Constraints

- SVG output is the **wide arrangement** only, drawn as one document (FR-71), not D-85's split parts.
- Colours are **literal**, resolved for `--theme` (FR-56), through `Target::Static`, which also drops every interactive-only node (architecture §5).
- **No text element and no font reference** in the exported file: no `<text>`, no `font-family`, no `@font-face`, no `<style>` (SC-15, D-88). Every `xlink:href` is a fragment `#gN` naming a `<path>` in the file's own `<defs>` (NFR-5).
- **Byte-identical** run to run and across platforms (NFR-4, SC-8). Every number is written by `num::fmt_coord`, and every attribute by `attr` or `coord`; nothing is quoted by hand. All arithmetic is plain `*` and `+`, never `mul_add` (`suboptimal_flops` is expected, with the project's standard reason).
- `--width` sets `width` and `height` on the root, the height following the document's aspect (FR-57), and is held to FR-64's range by clap. The SVG is **not clamped, capped or warned about** (D-88): no pixmap is allocated, and a vector scales without loss.
- The file opens with `<!-- radargen X.Y -->` from `emit::OUTPUT_VERSION` (SC-25), and ends with one LF (NFR-17).
- Every string the CSV or a flag supplies reaches the file through `escape::xml_text` or `escape::xml_attr` (NFR-6, SC-9). Nothing user-supplied reaches the comment.
- No silent failure: a glyph the writer meets that the table never collected is a loud invariant failure, not an omitted glyph.
- A failure writes **nothing** and leaves an existing `-o` file byte-unchanged (NFR-8, NFR-14, SC-18).
- Strict clippy: pedantic, nursery and cargo, and `clippy.toml`'s disallowed methods. Use `radargen::trim`, never `str::trim`. `as_conversions` and `string_slice` are denied, so every cast needs an `#[expect]` with a reason. No `unwrap`; `missing_docs`.
- A test that writes a golden uses `UPDATE_GOLDEN=1`, which is refused under `CI`.

## Rulings

1. **A glyph is defined once per face and size, already scaled.**
   - **The alternative:** one definition per face at a reference size, say 100 units per em, placed under `scale(size / 100)`. At wide scale 1 every size is a multiple of 0.1, so that scale is exact at three decimals, and the file is smaller.
   - **Why the scaled form wins:**
     - its exactness depends on no size rule, since the narrow arrangement's sizes are not round and nothing stops a future wide size being so;
     - a halo's `stroke-width` would have to be rescaled under the transform;
     - only transform-free paths at user-unit precision have been measured against `<text>`;
     - transform-free `<use>`s are plainer to edit in PowerPoint or Illustrator.
   - **The cost, measured by the reviews:** the PRD example draws 395 glyphs over 134 (face, size, glyph) keys, against 65 (face, glyph). That is roughly 30% more file, about 20 kB, which stays roughly fixed as items are added, since legend names share one size. `grown.csv` exports at 84 kB: 118 paths and 443 `<use>`s. Task 5 records the goldens' sizes.
   - **Also rejected:** converting through usvg, which flattens text to paths, and writing with usvg's own SVG writer. It would delete the shaping code, but it has no `<use>` deduplication (the spike's 9× file), it loses every `role` and `aria-label`, and it formats floats its own way (NFR-17). (D-88)
2. **`xlink:href`, not SVG 2's bare `href`.** SVG 1.1 consumers read only `xlink:href`, and SVG 2 still requires it to be honoured. resvg reads both. (D-88)
3. **Every filled text outside a control is `role="img"`, named by its words.**
   - **Why:** outlined text has no text content. Without this, a heading group would be nameless, and the ring, quadrant and key labels would be silent.
   - **How headings stay named:** a heading's name is computed from its content, including its images' names (accname 1.2, step 2F; Accessibility review: valid, the familiar `<h1><img alt>` pattern). So the title is a level-1 heading named by the title.
   - **Not done to:**
     - a halo, already `aria-hidden`;
     - text inside a marker or legend row, whose control carries the name and has presentational children (architecture §5).
   - **The root is not `role="img"`.** That would flatten headings and markers into one image. SVG-AAM names the root from its `<title>` and describes it from its `<desc>`.
   - **Costs accepted:**
     - browse mode says "graphic" before each label;
     - text cannot be selected or searched (the spike);
     - author data in another language is read as English under `xml:lang="en"`, as in the HTML;
     - with no `--title`, there is no level-1 heading, and the root `<title>` names the document instead.
   - **Scope:** these names reach a reader only when the file is opened directly. Through `<img>`, or in a slide, the placement's alt text is what is read, which the README says. NFR-1 stays scoped to the HTML. (D-88)
4. **The intrinsic size is unrounded and unclamped.** `--width 1000` on a 1025.4 × 824 document writes `width="1000" height="803.588"`. An SVG's size is a length, not a pixel count. D-87's clamp and cap bound a pixmap, and an SVG allocates none. The file does grow with what is drawn, about 50 bytes per glyph, so a huge CSV makes a large file. That is the author's own disk, as for the HTML. The small-text warning is the PNG's alone. (D-88)
5. **`Fatal::FormatNotInThisVersion` is removed, not left idle.** With `.svg` written, no FR-52 extension is deferred. A variant with no trigger is dead code, and PR 18 adds no format. PRD §14.1 gains a sentence saying the message had its last trigger in 1.1 and is gone (D-88). `UnrecognisedExtension` now reads "must end in .html, .png or .svg".
6. **An outline failure is `FontError`, so it reads as NFR-15's font failure.** `FontMetrics::outline` fails only if the face lacks a glyph id its own shaper produced, or skrifa cannot draw it. Either means the vendored face is broken. Like `Fatal::Rasterise`, it has no end-to-end trigger.
7. **The release is 1.2.0** (maintainer's rule, 2026-10-16). Every PR is a release, and a feature earns a minor bump. The HTML golden is re-blessed for `radargen 1.2`, and so are the SVG goldens, whose comment names the version. The PNG goldens are untouched.
8. **§5 and §12's rule is about `Document`'s shape, not the serialiser's branches.**
   - **The test:** a per-format difference is legitimate when it needs no new `Document` field or node, and follows from what the reader can do:
     - choose a theme (colour binding);
     - point and focus (interactive-only nodes);
     - have the font (text mode, with the names outlining forces).
   - **Why amend it:** the architecture says "exactly two". Leaving that would make this PR fail its own rule, or teach that the rule can be ignored. Task 0 amends §5 and the §12 row to the test above, and lists the three cases.
9. **The PR stays whole** (architecture review's split suggestion, declined pending the maintainer). Task 1 alone would be `measure` rebuilt on `shape`, a refactor with no consumer: `place` and `outline` would be dead code until the export, which the lints refuse. A no-bump PR holding only the rebuild would carry the risk without the feature that justifies it. The bit-pinning test (Task 1, Step 1) carries that risk inside this PR instead.
10. **The pixel test has two criteria, set from the real residue, plus a negative control.**
    - **What the reviews found:** the spike's 82-subpixel figure is stale (38 ppm today, 43 for the `<use>` form) and from an easier corpus. The real export measured 48–95 ppm. A whole-image ppm budget alone misses a dropped glyph (5–16 ppm).
    - **Every defect the QA review planted** made subpixels differ by more than 64 levels. The antialiasing residue never exceeded 64.
    - **So:** Task 4 measures the residue first, then sets:
      - a ppm budget at 1.5× the worst fixture;
      - `MAX_DELTA`, the worst residue channel;
      - a negative control, which drops one glyph's `<use>` and requires the comparison to fail.
11. **Glyph ids are file-local.** Two exports pasted *inline* into one page would share `g0…` and draw each other's glyphs. Through `<img>`, an editor, or the file itself, each file is its own document. A hash prefix would cost a dependency and long ids for a case the README can name instead. (D-88)
12. **A colour inherited through `<use>` is the SVG-correct form, and a manual check verifies it.** An importer that copies `<use>`d paths without their context paint would draw text black, which is invisible in the dark theme. Manual check 16 tests exactly that in PowerPoint and an editor, in both themes. The recorded fallback, should it fail, is paint on each `<use>`: about 15 bytes a glyph, more for halos.

## Review Focus

1. **`--theme dark`, rasterised by someone else.** Any text left as `<text>`, any `var()`, or any unpainted glyph would draw wrong, or not at all, outside a browser. Expected: rasterised by resvg with an **empty font database**, the SVG matches our PNG under Ruling 10's two criteria, in both themes, on five documents. A dropped glyph fails the comparison. Tests: `the_svg_draws_what_the_png_draws` and `the_comparison_catches_a_dropped_glyph` (Task 4).
2. **Hostile names, titles and quadrants:** `</title>`, `]]>`, `<script>`, `&`, `"` and U+2028 (SC-9). Their routes into the file are `<title>`, `<desc>`, every `aria-label`, and the static tooltips' `<title>`s. Expected:
   - the file parses as XML and holds only the export's elements and attributes;
   - each filled text outside a control has exactly one name equal to its words;
   - each control's label and tooltip read back as written.

   Tests: `the_export_names_every_text_exactly` and `the_export_uses_only_its_vocabulary` (Task 4).
3. **A name in a script Inter lacks** (東京, 😀). Expected: the shaper's `.notdef` glyph is outlined like any other, so the SVG draws the boxes the PNG draws, and the run succeeds. Tests: `a_name_inter_lacks_draws_as_the_png_does` (under the magnitude criterion) and `an_svg_of_hostile_text_renders` (Task 4).
4. **`--width` on SVG:** 320, 10000, and a default past 10,000 units. Expected: `width` and `height` on the root, the height in proportion and unrounded, no clamp, and no warning. Tests:
   - `the_intrinsic_size_follows_the_width` and `a_wide_default_is_not_clamped` (Task 4);
   - `renders_an_svg_at_the_width_asked` and `an_svg_gives_no_png_warnings`, which parses the root's width (Task 4).
5. **Every text shape in the wide arrangement:** middle-anchored numbers, a title wrapped onto two lines, letter-spaced headings, halos, and legend tails placed at `tail_x` (D-78). Expected: each glyph sits where resvg draws it. Tests:
   - `place_ends_where_measure_does` (Task 1);
   - the writer unit tests (Task 2);
   - the pixel comparison (Task 4), whose tails come from `three-quadrant.csv` and `text-spacing.csv`, and whose wrapped title is a fifth case.

---

## Review rulings

All five reviews, 2026-10-16. **Fixed** means the plan below now does it.

| # | Seat | Finding | Ruling |
| --- | --- | --- | --- |
| 1 | Rust (Critical), Architecture | `Context` derives `Debug`; `TextMode` holds `&Fonts`, which has none | Fixed: `Context` drops `Debug`; nothing formats one. Task 2 Step 3 |
| 2 | Rust (Critical), QA (Critical), Architecture | `place`'s exact-equality test fails: per-glyph spacing reassociates | Fixed: `place_ends_where_measure_does` compares within 1e-9, from the pen's end, offsets excluded. Only `measure` claims bit-identity |
| 3 | Rust (Critical), QA, Security, Accessibility | The hostile test calls a missing `fonts()` and expects names on control text, and its `starts_with` match passes when a name is truncated or doubled | Fixed: `the_export_names_every_text_exactly` walks with control context, asserts exact names and counts, checks control labels, tooltips, `<desc>` and the one comment |
| 4 | Rust, QA | `Neutral::Ink` and `Muted` do not exist | Fixed: `TextPrimary`, `TextMuted` |
| 5 | Rust | `Key::new` must be `const` (`missing_const_for_fn`) | Fixed |
| 6 | Rust | `Standalone`'s fields lack docs | Fixed: each documented |
| 7 | Rust | `str::trim` is disallowed | Fixed: `chars().all(char::is_whitespace)` |
| 8 | Rust | `a_blank_glyph_has_no_definition` expects the wrong ids for `"a b"` | Fixed: `"a a"` |
| 9 | Rust, QA, Architecture | The glyph table alone is dead code, so its commit fails clippy | Fixed: the table and the writer are one task (Task 2) |
| 10 | Rust, QA | The oracles allow 0.01, so they do not guard `measure`'s bits | Fixed: `measure_keeps_its_bits` pins `to_bits()` over a corpus before the rebuild |
| 11 | Rust | `as f32` casts need one remedy | Fixed: `#[expect]` on `compare`, and the scale computed as `rasterise` computes it (QA) |
| 12 | Rust | Wrong slice offset in the tail test | Fixed |
| 13 | Rust | `Pen::push` allocates per command | Fixed: `write!` into the string |
| 14 | QA, Architecture, Security | A glyph missing from the table is dropped silently | Fixed: the table records blank glyphs; collecting and writing share one `runs` function; a missing key is a loud `expect`; `every_drawn_glyph_is_in_the_table` covers the fixtures |
| 15 | QA | A 100 ppm budget misses a dropped glyph; the figure is stale | Fixed: Ruling 10 |
| 16 | QA | Two SVG failure tests pass before the change | Fixed: exact messages and exit code, as the PNG twins assert |
| 17 | QA | The CLI wiring (wide, palette, title) is untested end to end | Fixed: `the_cli_writes_the_golden_exports` byte-compares the binary's output with both goldens |
| 18 | QA, Security | SC-18 is not exercised: a missing directory fails before rendering | Fixed: `a_failed_svg_run_leaves_the_old_file` hashes an existing `radar.svg` across a row-error run |
| 19 | QA | No header-only CSV, subtitle-only, `--width 10000` or wrapped-title case | Fixed: Task 4 adds each |
| 20 | QA, Accessibility | A token broken mid-word gets a space in its name ("Kuber netes") | Ruling: accepted, and crumbed. `Document` holds lines, not the source, and carrying it would widen the waist for a title longer than its column. The HTML's tspans may have the mirror-image fault (no space), which is crumbed with it |
| 21 | QA | usvg skips letter-spacing for some scripts that `measure` spaces | Ruling: known, and pre-existing: the PNG and the layout already differ there. Inter draws none of those scripts, so their glyphs are `.notdef` either way. Crumbed |
| 22 | QA | Fixture descriptions are wrong: `long-legend` and `grown` have no tails or wraps | Fixed in Review Focus 5 |
| 23 | QA | The diagnostics rows are `radar.pdf` and `radar`; the HTML golden is `page::prd_page_matches_its_golden` | Fixed |
| 24 | QA | Run the failing tests per target, so a compile error does not hide runtime failures | Fixed: Task 4 Step 3 |
| 25 | QA | The `-0` check cannot fail; the spacing test misses "none after the last" and clusters | Fixed |
| 26 | QA | Cross-platform byte identity | Ruling: low risk, as the Rust review confirmed: integer outlines, plain arithmetic. No change |
| 27 | Architecture | §5 and §12 say "exactly two" | Fixed: Ruling 8, Task 0 |
| 28 | Architecture | Architecture text left contradictory: §3 rows, §5's no-id rule, §10, §11's figures, §12's row, the spike | Fixed: Task 0 Step 3 lists each |
| 29 | Architecture | Ruling 1's reasoning was a strawman and its cost unquantified | Fixed: Ruling 1 |
| 30 | Architecture | skrifa leaked into the serialiser and the public API | Fixed: `outline` returns `Vec<Segment>` |
| 31 | Architecture | `emit::svg` importing `emit::html::UNTITLED` | Fixed: `UNTITLED` moves to `emit` |
| 32 | Architecture, Security | The serialiser writing the version comment from a free field | Fixed: `emit::svg::export` writes it; `Standalone` has no `generator` |
| 33 | Architecture | Split Task 1 into its own PR | Ruling 9: kept whole, pending the maintainer |
| 34 | Architecture | Record the usvg-flatten alternative as rejected | Fixed: Ruling 1 |
| 35 | Architecture, Accessibility | `<desc>` repeats the radar group's name | Ruling: kept. It is the one summary a reader gets before entering the drawing, and where the root alone is exposed. The repetition on entering the group is one sentence. `radar_name`'s doc names its one-group assumption |
| 36 | Architecture | PRD §14.1 and FR-54's "written directly" | Fixed: Task 0 |
| 37 | Security | The export bypasses the attribute allow-list | Fixed: `the_export_uses_only_its_vocabulary`, with checks on ids, hrefs and `d` |
| 38 | Security | "Allocates nothing" is false for the file | Fixed: Ruling 4 |
| 39 | Security | `defs` quoted attributes by hand | Fixed: through `attr` |
| 40 | Security | Ids collide when two exports are inlined | Ruling 11: documented, not prefixed |
| 41 | Accessibility | README overstates what a screen reader gets | Fixed: Task 6 |
| 42 | Accessibility | Colour inherited through `<use>` may be lost in importers | Ruling 12: manual check, fallback recorded |
| 43 | Accessibility | Manual check 16 cannot be passed or failed | Fixed: rewritten with a title, levels, negative expectations and split result rows |
| 44 | Accessibility | "Graphic" verbosity; root role; `xml:lang` | Ruling 3: accepted and recorded in D-88 |
| 45 | Accessibility | Ring labels read outermost first, as in the HTML | Ruling: unchanged, the same as the HTML, and named in check 16 |
| 46 | Accessibility | Deleting the ground in a slide breaks dark-theme contrast | Fixed: a README line |
| 47 | Accessibility | No test that text under a decorative group is unnamed | Ruling: not adopted. A name under `aria-hidden` is never exposed, so the test would pin nothing a reader meets |

---

## Tasks

### Task 0: PRD v0.73 (D-88) and architecture

**Files:**
- Modify: `docs/product/radargen-prd.md` (frontmatter; FR-54; §14.1; D-88 after D-87; revision 0.73)
- Modify: `docs/design/architecture.md` (§3, §5, §10, §11, §12)
- Modify: `docs/spikes/svg-font.md` (a superseded note)

- [x] **Step 1: Add D-88 after D-87**

```markdown
| **D-88** | **The exported SVG outlines its text, names it, and takes its size from `--width` unclamped** (FR-54, FR-57, NFR-1, SC-15, SC-25, §14.1). | *Added by the PR 17 plan, crumb `rg-m2l`.* **Outlines:** every text is drawn as glyph outlines, defined once per face and size in `<defs>` and placed by `<use xlink:href x y>` (architecture §11, `docs/spikes/svg-font.md`): no `<text>`, no `font-family`, no `@font-face`, so other applications draw it as resvg does. `xlink:href` rather than SVG 2's bare `href`, which SVG 1.1 consumers do not read. Glyph ids are local to the file: two exports pasted inline into one page would share them, which the README says. **Size:** `width` and `height` on the root are `--width` and the height in proportion, unrounded, or the document's own size by default; neither is clamped, capped or warned about, because no pixmap is allocated and a vector scales without loss (D-87's limits are the PNG's). The file grows by about 50 bytes per drawn glyph. **Names:** the root carries a `<title>` (the title, or D-75's *Technology Radar*) and a `<desc>` (the radar's group name), and is not `role="img"`, which would flatten its headings and markers. Every filled text outside a control is `role="img"` named by its words, so headings keep their names and labels stay readable; halos stay hidden, and a control's own name covers its text. With no title there is no level-1 heading; the root's `<title>` names the document. Author data is read under `xml:lang="en"`, as in the HTML. These names reach a reader who opens the file directly; placed in a page or a slide, the placement's alt text is what is read. NFR-1 stays scoped to the HTML, and text cannot be selected or searched, as the spike decided. **Errors:** with `.svg` written, no extension FR-52 names is deferred, so §14.1's "not supported in this version" message has no trigger and is removed. Ships as 1.2.0. |
```

- [x] **Step 2: FR-54, §14.1, revision row and frontmatter**

- **FR-54:** "the shared document (FR-44) written directly, no rasterisation" becomes "the shared document (FR-44) converted directly, its text outlined (D-88), no rasterisation".
- **§14.1:** after the paragraph requiring the "not supported in this version" message, add: "From 1.2 every format FR-52 names ships, so the message has no trigger and was removed (D-88)."
- **Frontmatter:** set `Version: 0.73` and `date modified: 2026-10-16`.
- **Revision row:** append:

```markdown
| 0.73 | 2026-10-16 | **The exported SVG** (D-88; FR-54, §14.1 amended; rg-m2l): outlined text deduplicated through `<use xlink:href>`, an unclamped intrinsic size from `--width`, a `<title>` and `<desc>` on a root that is not an image, every outlined text named for a reader opening the file. SVG ships as 1.2.0. |
```

- [x] **Step 3: Architecture and the spike**

- **§3, the `text.rs` row:** add "shaping, placing and outlining glyphs (D-88); the only module that reads `skrifa`."
- **§3, the `svg/serialise/` row:** add "its `TextMode` (`<text>` or outlines) and `standalone`, the exported file's root."
- **§3, the `emit/svg.rs` row:** "The SVG export (FR-54, D-88): FR-57's intrinsic size, the `<title>`, `<desc>` and version comment, around `serialise::standalone`."
- **§5, "one of two places a single `Document` legitimately serialises differently":** replace with Ruling 8's test, and list its three cases:
  - colour binding;
  - interactive-only nodes;
  - text mode, and the names outlining forces.
- **§5, "Identity in the output":** after "No node emits an `id`", add: "The exported SVG's glyph definitions are the one exception (§11): one file, one arrangement, ids only on `<path>`s in `<defs>`, referenced only by `<use>`."
- **§10's oracle row:** add `tests/svg_matches_png.rs`.
- **§11:**
  - "The SVG emitter's internals. v1.1 — and *not* thin" becomes "The SVG export. 1.2 (D-88): a text mode of the serialiser, so `emit/svg.rs` is thin and the conversion is `svg/serialise/outline.rs`."
  - The first consequence reads: "**The SVG export is a converter, not a serialiser**, built as a text mode of the one serialiser rather than as a second one, so colour binding, semantics and the dropping of interactive-only nodes are shared. It reads glyph outlines from `skrifa` through `text.rs`, and shapes with the pass `text.rs` measures with."
  - The dedupe bullet's "one `<path>` per distinct glyph" becomes "per distinct glyph in each face and size". Replace its figures with Ruling 1's measured ones, the PRD example's goldens to be filled in by Task 5.
- **§12, the `emit/svg.rs` row:** "Bounded to one file in v1.1" becomes "Bounded to `text.rs`'s glyph reading, `svg/serialise/outline.rs` and a thin `emit/svg.rs` (D-88)". Rewrite the "exactly two" row to Ruling 8's test.
- **The spike, under "Consequences":** add "*Superseded, 2026-10-16: the converter is a text mode of the serialiser, and `emit/svg.rs` is thin (D-88).*"

- [x] **Step 4: Commit**

```bash
git add docs/product/radargen-prd.md docs/design/architecture.md docs/spikes/svg-font.md
git commit -m "docs(prd): D-88, the exported SVG (v0.73)"
```

---

### Task 1: `text.rs` shapes, places and outlines

**Files:**
- Modify: `Cargo.toml` (`skrifa` from dev-dependencies to dependencies)
- Modify: `src/text.rs` (and its tests module)

**Interfaces:**
- Produces:
  - `pub struct ShapedGlyph { pub id: u32, pub cluster: u32, pub x_advance: i32, pub x_offset: i32, pub y_offset: i32 }`, in font units;
  - `pub struct Placed { pub glyph: u32, pub x: f64, pub y: f64 }`, in user units from the line's start, y down;
  - `pub enum Segment { MoveTo(f32, f32), LineTo(f32, f32), QuadTo(f32, f32, f32, f32), CurveTo(f32, f32, f32, f32, f32, f32), Close }`, in font units, y up;
  - `FontMetrics::shape(&self, text: &str) -> Vec<ShapedGlyph>`;
  - `FontMetrics::place(&self, text: &str, size_px: f64, letter_spacing_px: f64) -> Vec<Placed>`;
  - `FontMetrics::outline(&self, glyph: u32) -> Result<Vec<Segment>, FontError>`;
  - `FontMetrics::units_per_em(&self) -> f64`;
  - `Weight` derives `PartialOrd, Ord`.
- `measure` keeps its signature and its bits.

- [x] **Step 1: Pin `measure`'s bits first, on the current code**

Add to `src/text.rs`'s tests module:

```rust
/// The corpus `measure_keeps_its_bits` pins: both weights, the sizes the
/// layout uses, spacings binary cannot represent exactly, a no-break
/// space, a combining mark, and kerned pairs.
const PINNED: [(Weight, &str, f64, f64); 8] = [
    (Weight::Regular, "Kubernetes", 12.5, 0.0),
    (Weight::SemiBold, "PLATFORMS", 11.0, 0.8),
    (Weight::Regular, "AV Wa. To", 17.0, 0.0),
    (Weight::Regular, "\u{a0}· New", 10.5, 0.2),
    (Weight::SemiBold, "Technology Radar", 17.0, 0.0),
    (Weight::Regular, "Cafe\u{301} Zu\u{308}rich", 12.0, 0.3),
    (Weight::SemiBold, "12", 9.0, 0.0),
    (Weight::Regular, "Infrastructure as Code", 12.5, 0.1),
];

/// D-88: rebuilding `measure` on `shape` moves no bit of any width, so
/// no wrap point and no golden moves. The bits were recorded from the
/// pre-rebuild `measure`.
#[test]
fn measure_keeps_its_bits() {
    let fonts = Fonts::embedded().expect("loads");
    let got: Vec<u64> = PINNED
        .iter()
        .map(|&(w, text, size, spacing)| fonts.face(w).measure(text, size, spacing).to_bits())
        .collect();
    assert_eq!(got, RECORDED, "{got:?}");
}
```

Run it once with `const RECORDED: [u64; 8] = [0; 8];` and read the `got` values from the failure. Paste them in as `RECORDED`, with a comment naming the commit they were taken at, and run again.
Expected: PASS, on the unchanged `measure`.

- [x] **Step 2: Move skrifa**

In `Cargo.toml`, delete the `skrifa` entry and its comment from `[dev-dependencies]`, and add to `[dependencies]`:

```toml
# SVG export (D-88): glyph outlines, read only by text.rs. The version usvg
# reads fonts with, sharing read-fonts with harfrust, so no second copy
# builds (rg-v3a).
skrifa = { version = "0.44", default-features = false, features = ["std"] }
```

Run: `cargo tree -d -e normal | rg -v miniz_oxide`
Expected: no duplicate crates beyond `miniz_oxide`, which `clippy.toml` allows.

- [x] **Step 3: Write the failing tests**

```rust
/// Placing and measuring share one shaping pass: the pen ends where
/// `measure` says the line does, to well under NFR-17's precision. Not
/// bit-equal: `place` adds spacing gap by gap, `measure` all at once.
#[test]
fn place_ends_where_measure_does() {
    let fonts = Fonts::embedded().expect("loads");
    for &(weight, text, size, spacing) in &PINNED {
        let face = fonts.face(weight);
        let shaped = face.shape(text);
        let placed = face.place(text, size, spacing);
        let (last, glyph) = (placed.last().expect("glyphs"), shaped.last().expect("glyphs"));
        let scale = size / face.units_per_em();
        let end = last.x - f64::from(glyph.x_offset) * scale + f64::from(glyph.x_advance) * scale;
        let measured = face.measure(text, size, spacing);
        assert!((end - measured).abs() < 1e-9, "{text:?} at {size}: {end} vs {measured}");
    }
}

/// Spacing falls between clusters, not after the last, and never inside
/// one (FR-84).
#[test]
fn spacing_falls_between_clusters() {
    let fonts = Fonts::embedded().expect("loads");
    let face = fonts.face(Weight::Regular);
    let tight = face.place("AB", 12.0, 0.0);
    let loose = face.place("AB", 12.0, 2.0);
    assert_eq!(tight.first().map(|g| g.x), Some(0.0), "the first glyph starts the line");
    let second = |p: &[Placed]| p.get(1).expect("two glyphs").x;
    assert_eq!(second(&loose) - second(&tight), 2.0, "one gap before B");
    assert_eq!(face.measure("AB", 12.0, 2.0) - face.measure("AB", 12.0, 0.0), 2.0, "none after B");
    // e + U+0301 is one cluster: its mark gains no gap of its own.
    let shaped = face.shape("e\u{301}x");
    if shaped.len() == 3 {
        let marked = face.place("e\u{301}x", 12.0, 2.0);
        let plain = face.place("e\u{301}x", 12.0, 0.0);
        let mark = |p: &[Placed]| p.get(1).expect("the mark").x;
        assert_eq!(mark(&marked), mark(&plain), "no gap inside a cluster");
    }
}

/// A shaped glyph has contours; a space has none; an id the face lacks
/// is an error.
#[test]
fn shaped_glyphs_outline() {
    let fonts = Fonts::embedded().expect("loads");
    let face = fonts.face(Weight::SemiBold);
    let [k, space] = face.shape("K ").try_into().expect("two glyphs");
    let ink = face.outline(k.id).expect("draws");
    assert!(matches!(ink.first(), Some(Segment::MoveTo(..))), "{ink:?}");
    assert!(matches!(ink.last(), Some(Segment::Close)), "{ink:?}");
    assert_eq!(face.outline(space.id).expect("draws"), []);
    assert!(face.outline(u32::MAX).is_err(), "no such glyph");
}
```

- [x] **Step 4: Run them to verify they fail**

Run: `cargo test --lib text::tests 2>&1 | tail -5`
Expected: compile errors: `shape`, `place`, `outline`, `units_per_em` and `Segment` are not found.

- [x] **Step 5: Implement**

In `FontMetrics`, add `face: &'static str` and `upem: f64`, set in `load`:

```rust
let data = ShaperData::new(&font);
let upem = f64::from(data.shaper(&font).build().units_per_em());
Ok(Self { font, data, face, upem })
```

Derive `PartialOrd, Ord` on `Weight`. Then add:

```rust
/// One glyph as the shaper lays it out, in font units (FR-84).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapedGlyph {
    /// The glyph's id in its face.
    pub id: u32,
    /// The character cluster it belongs to; a letter-spacing gap falls
    /// between clusters, never inside one.
    pub cluster: u32,
    /// How far the pen moves past it.
    pub x_advance: i32,
    /// How far it sits right of the pen.
    pub x_offset: i32,
    /// How far it sits above the baseline.
    pub y_offset: i32,
}

/// Where one glyph is drawn: user units from its line's start, y down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// The glyph's id in its face.
    pub glyph: u32,
    /// From the line's start.
    pub x: f64,
    /// From the baseline, down.
    pub y: f64,
}

/// One step of a glyph's contours, in font units, y up (D-88).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    /// Start a contour.
    MoveTo(f32, f32),
    /// A straight edge.
    LineTo(f32, f32),
    /// A quadratic curve: control point, end.
    QuadTo(f32, f32, f32, f32),
    /// A cubic curve: two control points, end.
    CurveTo(f32, f32, f32, f32, f32, f32),
    /// Close the contour.
    Close,
}

/// Collects skrifa's pen calls, so skrifa stays private to this module.
struct Segments(Vec<Segment>);

impl skrifa::outline::OutlinePen for Segments {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(Segment::MoveTo(x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(Segment::LineTo(x, y));
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.push(Segment::QuadTo(cx, cy, x, y));
    }
    fn curve_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        self.0.push(Segment::CurveTo(c1x, c1y, c2x, c2y, x, y));
    }
    fn close(&mut self) {
        self.0.push(Segment::Close);
    }
}
```

In `impl FontMetrics`:

```rust
/// `text` shaped as `usvg` shapes it: left to right (A-4), segment
/// properties guessed, default options.
#[must_use]
pub fn shape(&self, text: &str) -> Vec<ShapedGlyph> {
    if text.is_empty() {
        return Vec::new();
    }
    // A built shaper borrows the font and its data, so it is built per
    // call rather than stored; `usvg` does the same.
    let shaper = self.data.shaper(&self.font).build();
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(Direction::LeftToRight);
    buffer.guess_segment_properties();
    let output = shaper.shape(buffer, ShapeOptions::new());
    output
        .glyph_infos()
        .iter()
        .zip(output.glyph_positions())
        .map(|(info, position)| ShapedGlyph {
            id: info.glyph_id,
            cluster: info.cluster,
            x_advance: position.x_advance,
            x_offset: position.x_offset,
            y_offset: position.y_offset,
        })
        .collect()
}

/// The face's units per em.
#[must_use]
pub const fn units_per_em(&self) -> f64 {
    self.upem
}

/// Where each glyph of `text` is drawn at `size_px`, with
/// `letter_spacing_px` between clusters as [`Self::measure`] counts it
/// (D-88).
#[must_use]
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub fn place(&self, text: &str, size_px: f64, letter_spacing_px: f64) -> Vec<Placed> {
    let scale = size_px / self.upem;
    let mut pen = 0.0;
    let mut previous = None;
    self.shape(text)
        .into_iter()
        .map(|glyph| {
            if previous.is_some_and(|cluster| cluster != glyph.cluster) {
                pen += letter_spacing_px;
            }
            previous = Some(glyph.cluster);
            let placed = Placed {
                glyph: glyph.id,
                x: pen + f64::from(glyph.x_offset) * scale,
                y: -f64::from(glyph.y_offset) * scale,
            };
            pen += f64::from(glyph.x_advance) * scale;
            placed
        })
        .collect()
}

/// Glyph `glyph`'s contours in font units, y up, unhinted.
///
/// # Errors
///
/// [`FontError`] when the face has no such glyph or cannot draw it:
/// the shaper produced an id its own face does not hold (NFR-15).
pub fn outline(&self, glyph: u32) -> Result<Vec<Segment>, FontError> {
    use skrifa::MetadataProvider as _;
    use skrifa::instance::{LocationRef, Size};
    use skrifa::outline::DrawSettings;
    let fail = |reason: String| FontError { face: self.face, reason };
    let outlines = self.font.outline_glyphs();
    let found = outlines
        .get(skrifa::GlyphId::new(glyph))
        .ok_or_else(|| fail(format!("glyph {glyph} is not in the face")))?;
    let mut segments = Segments(Vec::new());
    found
        .draw(DrawSettings::unhinted(Size::unscaled(), LocationRef::default()), &mut segments)
        .map_err(|e| fail(format!("glyph {glyph} cannot be outlined: {e}")))?;
    Ok(segments.0)
}
```

`measure` keeps its doc comment and its `expect`s. Its body becomes:

```rust
let shaped = self.shape(text);
let units: i32 = shaped.iter().map(|g| g.x_advance).sum();
let boundaries = shaped
    .windows(2)
    .filter(|pair| matches!(pair, [a, b] if a.cluster != b.cluster))
    .count();
// …the existing `gaps` conversion and `width` expression, with
// `self.upem` in place of `f64::from(shaper.units_per_em())`.
```

`measure` now allocates the shaped `Vec`. Layout calls it once per wrap prefix, and an export of `grown.csv` still takes about 10 ms (Rust review), so the cost is accepted.

- [x] **Step 6: Run the tests to verify they pass, and that no bit moved**

Run: `cargo test --lib text::tests && cargo test --test serialise && cargo test --test measure --test wrap --test agrees_with_resvg --test tail_oracle`
Expected: all pass. `measure_keeps_its_bits` and the byte-exact HTML goldens are the guard: the oracles allow 0.01.

- [x] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src/text.rs
git commit -m "feat(text): shape, place and outline glyphs for the SVG export"
```

---

### Task 2: Outlined text, and the standalone document

**Files:**
- Create: `src/svg/serialise/outline.rs`
- Modify: `src/svg/serialise/mod.rs` (`mod outline;`, `TextMode` in `Context`, `node()`'s text arm, `serialise` split into opening and `body`, `Standalone`, `standalone`)
- Modify: `src/svg/serialise/text.rs` (`ink` becomes `pub(super)`)
- Modify: `src/svg/mod.rs` (re-export `Standalone`, `standalone`; drop `tail_x`'s `qual:api` note, since it now has a production caller)
- Create: `src/svg/serialise/tests/outline.rs`, registered in `src/svg/serialise/tests/mod.rs`

**Interfaces:**
- Consumes (Task 1): `shape`, `place`, `outline`, `units_per_em`, `Segment`, `Weight: Ord`; `TextStyle::advance`; `Node::tail_x`.
- Produces:

```rust
/// The exported SVG's root around the shared document (D-88).
pub struct Standalone<'a> {
    /// The theme's palette (FR-56).
    pub palette: &'a Palette,
    /// The faces text is shaped and outlined from.
    pub fonts: &'a Fonts,
    /// The root's `width` (FR-57).
    pub width: f64,
    /// The root's `height` (FR-57).
    pub height: f64,
    /// The root's `<title>`.
    pub title: &'a str,
    /// The root's `<desc>`, when there is one.
    pub desc: Option<&'a str>,
}

/// `document` as the exported file's root element, its text outlined
/// (D-88), ending in one LF.
/// # Errors
/// [`FontError`] when a glyph cannot be outlined (Ruling 6).
pub fn standalone(document: &Document, file: &Standalone<'_>) -> Result<String, FontError>
```

- `serialise(document, target)` keeps its signature and its bytes.

- [x] **Step 1: Write the failing tests**

`src/svg/serialise/tests/outline.rs`:

```rust
//! Outlined text and the exported root (D-88).

use crate::layout::block::fonts;
use crate::num::fmt_coord;
use crate::palette::{Neutral, Palette, Role, Theme};
use crate::style::TextStyle;
use crate::svg::{
    Anchor, Control, Document, Group, ItemKey, Node, Paint, Point, Semantics, Standalone, Tail,
    TextPaint, standalone,
};
use crate::text::Weight;

const NAME: TextStyle = TextStyle { weight: Weight::Regular, size: 12.5, letter_spacing: 0.0 };
const HEAD: TextStyle = TextStyle { weight: Weight::SemiBold, size: 12.5, letter_spacing: 0.0 };
const INK: Paint = Paint::Role(Role::Neutral(Neutral::TextPrimary));

fn text_at(style: TextStyle, anchor: Anchor, ink: TextPaint, lines: &[&str], tail: Option<Tail>) -> Node {
    Node::Text {
        origin: Point { x: 10.0, y: 20.0 },
        style,
        ink,
        anchor,
        lines: lines.iter().map(|&l| l.to_owned()).collect(),
        leading: 15.0,
        tail,
    }
}

fn text(style: TextStyle, lines: &[&str]) -> Node {
    text_at(style, Anchor::Start, TextPaint::Fill(INK), lines, None)
}

fn doc(children: Vec<Node>) -> Document {
    Document { width: 200.0, height: 100.0, root: Node::Group(Group::new(Point::ORIGIN, children)) }
}

fn file(document: &Document) -> String {
    let palette = Palette::built_in(Theme::Light);
    standalone(
        document,
        &Standalone {
            palette: &palette,
            fonts: fonts(),
            width: 400.0,
            height: 200.0,
            title: "Radar <&>",
            desc: Some("Technology radar: 1 quadrant"),
        },
    )
    .expect("outlines")
}

/// Every `<path>` in `<defs>`.
fn paths(svg: &str) -> usize {
    svg.matches("<path id=").count()
}

// ---- the glyph table

#[test]
fn a_glyph_is_defined_once() {
    let svg = file(&doc(vec![text(NAME, &["aaa", "a"])]));
    assert_eq!(paths(&svg), 1, "{svg}");
    assert_eq!(svg.matches("xlink:href=\"#g0\"").count(), 4);
}

#[test]
fn a_face_or_size_is_a_glyph_of_its_own() {
    let small = TextStyle { size: 9.0, ..NAME };
    let svg = file(&doc(vec![text(NAME, &["a"]), text(HEAD, &["a"]), text(small, &["a"])]));
    assert_eq!(paths(&svg), 3, "{svg}");
}

#[test]
fn a_blank_glyph_has_no_definition_and_no_use() {
    let svg = file(&doc(vec![text(NAME, &["a a"])]));
    assert_eq!(paths(&svg), 1, "{svg}");
    assert_eq!(svg.matches("<use ").count(), 2, "the space draws nothing");
}

/// Ids follow (face, size, glyph), never the order text is met (NFR-4).
#[test]
fn ids_follow_the_glyphs_not_the_document() {
    let defs = |svg: &str| -> String {
        svg.lines().filter(|l| l.starts_with("<path id=")).collect::<Vec<_>>().join("\n")
    };
    let ab = file(&doc(vec![text(NAME, &["b"]), text(NAME, &["a"])]));
    let ba = file(&doc(vec![text(NAME, &["a"]), text(NAME, &["b"])]));
    assert_eq!(defs(&ab), defs(&ba));
}

/// What a static target drops is neither outlined nor placed (§5).
#[test]
fn interactive_only_text_is_not_outlined() {
    let indicator = Node::Group(
        Group::new(Point::ORIGIN, vec![text(NAME, &["x"])]).with(Semantics::Indicator(ItemKey(2))),
    );
    let svg = file(&doc(vec![indicator]));
    assert!(!svg.contains("<defs>") && !svg.contains("<use"), "{svg}");
}

/// A definition is the glyph at its size, y down, through `fmt_coord`,
/// with no paint of its own, so each `<use>` paints it.
#[test]
fn a_definition_is_a_bare_path() {
    let svg = file(&doc(vec![text(NAME, &["l"])]));
    let def = svg.lines().find(|l| l.starts_with("<path id=\"g0\" d=\"M")).expect("a definition");
    assert!(def.ends_with("Z\"/>"), "{def}");
    assert!(!def.contains("fill") && !def.contains("stroke"), "{def}");
    assert!(def.contains(" -"), "an l rises above its baseline: {def}");
    let negative_zero = def.match_indices("-0").any(|(at, _)| {
        !def.get(at + 2..).and_then(|s| s.chars().next()).is_some_and(|c| c == '.' || c.is_ascii_digit())
    });
    assert!(!negative_zero, "no -0: {def}");
}

// ---- the root

/// SC-15: no text, and nothing that names a font.
#[test]
fn outlined_text_writes_no_text() {
    let svg = file(&doc(vec![text(NAME, &["Kafka", "Streams"])]));
    for banned in ["<text", "<tspan", "font-", "@font-face", "<style"] {
        assert!(!svg.contains(banned), "{banned} in {svg}");
    }
}

/// D-88: the root and its names, then the glyphs, then the ground, then
/// the drawing; one LF at the end.
#[test]
fn the_root_opens_with_its_names() {
    let svg = file(&doc(vec![text(NAME, &["a"])]));
    assert!(svg.starts_with(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" \
         xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"400\" height=\"200\" \
         viewBox=\"0 0 200 100\" xml:lang=\"en\">\n\
         <title>Radar &lt;&amp;&gt;</title>\n\
         <desc>Technology radar: 1 quadrant</desc>\n<defs>\n"
    ), "{svg}");
    let ground = svg.find("<rect").expect("a ground");
    assert!(svg.find("</defs>").expect("defs") < ground, "{svg}");
    assert!(svg.ends_with("</svg>\n") && !svg.ends_with("\n\n"), "one LF");
}

// ---- the writer

/// D-88: a text is an image named by its words; one `<use>` per glyph.
#[test]
fn a_text_is_an_image_named_by_its_words() {
    let svg = file(&doc(vec![text(NAME, &["Kafka", "Streams"])]));
    assert!(svg.contains("<g fill=\"#") && svg.contains(" role=\"img\" aria-label=\"Kafka Streams\">"), "{svg}");
    assert_eq!(svg.matches("<use xlink:href=\"#g").count(), "KafkaStreams".len());
}

/// Each line at its own baseline (FR-46); a start-anchored line at the
/// origin.
#[test]
fn lines_sit_at_their_baselines() {
    let svg = file(&doc(vec![text(NAME, &["l", "l"])]));
    assert!(svg.contains("<use xlink:href=\"#g0\" x=\"10\" y=\"20\"/>"), "{svg}");
    assert!(svg.contains("<use xlink:href=\"#g0\" x=\"10\" y=\"35\"/>"), "{svg}");
}

/// A middle-anchored line is centred on its origin by its measured advance.
#[test]
fn a_middle_anchored_line_is_centred() {
    let svg = file(&doc(vec![text_at(NAME, Anchor::Middle, TextPaint::Fill(INK), &["ll"], None)]));
    let x = fmt_coord(10.0 - NAME.advance("ll", fonts()) / 2.0);
    assert!(svg.contains(&format!("<use xlink:href=\"#g0\" x=\"{x}\" y=\"20\"/>")), "{x} in {svg}");
}

/// A halo is an outline only, hidden, and not named again (§5).
#[test]
fn a_halo_is_hidden_and_unnamed() {
    let halo = TextPaint::Halo { stroke: Paint::Role(Role::Neutral(Neutral::Surface1)), width: 3.0 };
    let svg = file(&doc(vec![text_at(NAME, Anchor::Start, halo, &["a"], None)]));
    assert!(
        svg.contains("<g fill=\"none\" stroke=\"#")
            && svg.contains("stroke-width=\"3\" stroke-linejoin=\"round\" aria-hidden=\"true\">"),
        "{svg}"
    );
    assert!(!svg.contains("role=\"img\""), "{svg}");
}

/// D-78: the tail is placed at `tail_x`, in its own paint and size, on the
/// last line, and read as part of the text's name.
#[test]
fn a_tail_sits_at_tail_x() {
    let tail = Tail {
        gap: 4.0,
        style: TextStyle { size: 10.5, ..NAME },
        ink: Paint::Role(Role::Neutral(Neutral::TextMuted)),
        text: "\u{a0}New".to_owned(),
    };
    let node = text_at(NAME, Anchor::Start, TextPaint::Fill(INK), &["a", "Kafka"], Some(tail));
    let x = node.tail_x(fonts()).expect("a tail");
    let svg = file(&doc(vec![node]));
    assert!(svg.contains("aria-label=\"a Kafka\u{a0}New\""), "{svg}");
    let first = svg.find("<g fill=\"#").expect("the text's group");
    let at = svg
        .get(first + 1..)
        .and_then(|rest| rest.find("<g fill=\"#"))
        .map(|rel| first + 1 + rel)
        .expect("the tail's group");
    let tail_group = svg.get(at..).expect("in range");
    let placed = fonts().face(NAME.weight).place("\u{a0}N", 10.5, 0.0);
    let n_x = fmt_coord(x + placed.get(1).expect("N").x);
    assert!(tail_group.contains(&format!("x=\"{n_x}\" y=\"35\"/>")), "{n_x} in {tail_group}");
}

/// A control's own name covers its text, whose image would read it twice.
#[test]
fn text_in_a_control_is_not_named_again() {
    let control = Control { key: ItemKey(2), label: "1. Kafka".into(), tooltip: None };
    let marker = Node::Group(
        Group::new(Point::ORIGIN, vec![text(NAME, &["1"])]).with(Semantics::Marker(control)),
    );
    let svg = file(&doc(vec![marker]));
    assert_eq!(svg.matches("role=\"img\"").count(), 1, "the control's alone: {svg}");
    assert!(svg.contains("aria-label=\"1. Kafka\""), "{svg}");
}

/// A heading's name is computed from its content: its image's name.
#[test]
fn a_heading_is_named_by_its_text() {
    let heading = Node::Group(
        Group::new(Point::ORIGIN, vec![text(HEAD, &["Platforms"])])
            .with(Semantics::Heading(std::num::NonZeroU8::MIN)),
    );
    let svg = file(&doc(vec![heading]));
    assert!(svg.contains("<g role=\"heading\" aria-level=\"1\">\n<g fill="), "{svg}");
    assert!(svg.contains("aria-label=\"Platforms\""), "{svg}");
}

/// The page and the PNG keep `<text>`: the mode is the export's alone.
#[test]
fn serialise_still_writes_text() {
    let palette = Palette::built_in(Theme::Light);
    let svg = crate::svg::serialise(&doc(vec![text(NAME, &["a"])]), crate::svg::Target::Static(&palette));
    assert!(svg.contains("<text") && !svg.contains("<use"), "{svg}");
}
```

Register it with `mod outline;` in `src/svg/serialise/tests/mod.rs`.

- [x] **Step 2: Run them to verify they fail**

Run: `cargo test --lib serialise::tests::outline 2>&1 | tail -5`
Expected: compile error: `Standalone` and `standalone` are not found.

- [x] **Step 3: The text mode in `Context`**

In `src/svg/serialise/mod.rs`:

```rust
/// How text is written: as `<text>` for a reader with the font (the page,
/// and resvg through our own font database), or as outlines for one
/// without it (the exported SVG, D-88).
#[derive(Clone, Copy)]
enum TextMode<'a> {
    Text,
    Outlined(outline::Outlined<'a>),
}
```

`Context` gains `text: TextMode<'a>` and drops `Debug` from its derive; nothing formats one. `Context::top(target)` sets `TextMode::Text`. `node()`'s `Node::Text` arm builds the same `text::Run` as today, then:

```rust
match context.text {
    TextMode::Text => text::text(&run, context),
    TextMode::Outlined(outlined) => {
        outline::text(&run, node.tail_x(outlined.fonts), outlined, context)
    }
}
```

`node` is the function's `&Node` parameter. The function does not recurse by name, so it does not shadow anything that matters. Make `text::ink` `pub(super)`.

- [x] **Step 4: `src/svg/serialise/outline.rs`**

```rust
//! Text as glyph outlines, for the exported SVG (architecture §11, D-88).
//!
//! Each glyph a static document draws is defined once per face and size
//! in `<defs>`, already at its size, and placed by `<use>`. Ids are safe
//! here, though §5 bans them in the page: this file holds one
//! arrangement, not two. Collecting and writing read text through one
//! function, [`runs`], so the table holds every glyph the writer places.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::text::{Run, ink};
use super::{Context, attr, coord, paint};
use crate::num::fmt_coord;
use crate::style::TextStyle;
use crate::svg::{Anchor, Document, Node, Point, Tail, TextPaint};
use crate::text::{FontError, Fonts, Segment, Weight};

/// A glyph in one face at one size: one definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    weight: Weight,
    /// The size's bits: sizes are positive and finite, so their bits
    /// order as they do.
    size: u64,
    glyph: u32,
}

impl Key {
    const fn new(style: TextStyle, glyph: u32) -> Self {
        Self { weight: style.weight, size: style.size.to_bits(), glyph }
    }
}

/// What a collected glyph draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    /// The definition `gN`.
    Ink(usize),
    /// Nothing: a space.
    Blank,
}

/// Every glyph a document draws, the inked ones numbered in key order.
pub(super) struct Glyphs {
    entries: BTreeMap<Key, Entry>,
    paths: Vec<String>,
}

/// What outlined text reads: the faces to place with, and the table.
#[derive(Clone, Copy)]
pub(super) struct Outlined<'a> {
    pub(super) fonts: &'a Fonts,
    pub(super) glyphs: &'a Glyphs,
}

/// A text's runs, in the order they are drawn: each line in the text's
/// style, then the tail in its own. Collecting and writing both read text
/// through this, so they cannot disagree about what is drawn.
fn runs<'a>(
    style: TextStyle,
    lines: &'a [String],
    tail: Option<&'a Tail>,
) -> impl Iterator<Item = (TextStyle, &'a str)> {
    lines
        .iter()
        .map(move |line| (style, line.as_str()))
        .chain(tail.map(|t| (t.style, t.text.as_str())))
}

impl Glyphs {
    /// The glyphs `document`'s static rendering draws: every run of every
    /// text outside an interactive-only node.
    ///
    /// # Errors
    ///
    /// [`FontError`] when a shaped glyph cannot be outlined (Ruling 6).
    pub(super) fn of(document: &Document, fonts: &Fonts) -> Result<Self, FontError> {
        let mut sizes = BTreeMap::new();
        collect(&document.root, fonts, &mut sizes);
        let mut entries = BTreeMap::new();
        let mut paths = Vec::new();
        for (key, size) in sizes {
            let face = fonts.face(key.weight);
            let d = path(&face.outline(key.glyph)?, size / face.units_per_em());
            let entry = if d.is_empty() {
                Entry::Blank
            } else {
                paths.push(d);
                Entry::Ink(paths.len() - 1)
            };
            entries.insert(key, entry);
        }
        Ok(Self { entries, paths })
    }

    /// `<defs>` with one path per inked glyph, or nothing when none inks.
    fn defs(&self) -> String {
        if self.paths.is_empty() {
            return String::new();
        }
        let mut out = String::from("<defs>\n");
        for (id, d) in self.paths.iter().enumerate() {
            out.push_str("<path");
            attr(&mut out, "id", &format!("g{id}"));
            attr(&mut out, "d", d);
            out.push_str("/>\n");
        }
        out.push_str("</defs>\n");
        out
    }

    /// The entry for a glyph the writer places. Every placed glyph was
    /// collected through [`runs`]; a miss is a broken invariant, and
    /// silently dropping the glyph would hide it.
    fn entry(&self, style: TextStyle, glyph: u32) -> Entry {
        *self
            .entries
            .get(&Key::new(style, glyph))
            .expect("every placed glyph was collected through `runs`")
    }
}

/// Each glyph `node` draws, keyed, with the size it is drawn at.
fn collect(node: &Node, fonts: &Fonts, sizes: &mut BTreeMap<Key, f64>) {
    if node.is_interactive_only() {
        return;
    }
    match node {
        Node::Group(group) => {
            for child in &group.children {
                collect(child, fonts, sizes);
            }
        }
        Node::Text { style, lines, tail, .. } => {
            for (style, text) in runs(*style, lines, tail.as_ref()) {
                for glyph in fonts.face(style.weight).shape(text) {
                    sizes.insert(Key::new(style, glyph.id), style.size);
                }
            }
        }
        _ => {}
    }
}

/// A glyph's contours as a path's `d`: font units scaled to the size and
/// flipped into SVG's y-down space, every number through `fmt_coord`.
fn path(segments: &[Segment], scale: f64) -> String {
    let mut d = String::new();
    let mut push = |command: char, points: &[(f32, f32)]| {
        d.push(command);
        for (i, &(x, y)) in points.iter().enumerate() {
            let separator = if i == 0 { "" } else { " " };
            write!(
                d,
                "{separator}{} {}",
                fmt_coord(f64::from(x) * scale),
                fmt_coord(-f64::from(y) * scale)
            )
            .expect("a String accepts writes");
        }
    };
    for &segment in segments {
        match segment {
            Segment::MoveTo(x, y) => push('M', &[(x, y)]),
            Segment::LineTo(x, y) => push('L', &[(x, y)]),
            Segment::QuadTo(cx, cy, x, y) => push('Q', &[(cx, cy), (x, y)]),
            Segment::CurveTo(c1x, c1y, c2x, c2y, x, y) => {
                push('C', &[(c1x, c1y), (c2x, c2y), (x, y)]);
            }
            Segment::Close => push('Z', &[]),
        }
    }
    d
}

/// A text as a group of placed glyphs: its paint on the group, each line
/// at its baseline, its tail at `tail_x` (D-78). Filled and outside a
/// control, an image named by its words (D-88).
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub(super) fn text(
    run: &Run<'_>,
    tail_x: Option<f64>,
    outlined: Outlined<'_>,
    context: Context<'_>,
) -> String {
    if run.lines.is_empty() {
        return String::new();
    }
    let mut out = String::from("<g");
    ink(&mut out, run.ink, context);
    if matches!(run.ink, TextPaint::Fill(_)) && context.control.is_none() {
        attr(&mut out, "role", "img");
        attr(&mut out, "aria-label", &words(run));
    }
    out.push_str(">\n");
    let mut baseline = run.origin.y;
    for (i, line) in (0_u32..).zip(run.lines) {
        baseline = run.origin.y + f64::from(i) * run.leading;
        let x = match run.anchor {
            Anchor::Start => run.origin.x,
            Anchor::Middle => run.origin.x - run.style.advance(line, outlined.fonts) / 2.0,
        };
        out.push_str(&uses(line, run.style, Point { x, y: baseline }, outlined));
    }
    if let (Some(tail), Some(x)) = (run.tail, tail_x) {
        out.push_str("<g");
        attr(&mut out, "fill", &paint(tail.ink, context.target));
        out.push_str(">\n");
        out.push_str(&uses(&tail.text, tail.style, Point { x, y: baseline }, outlined));
        out.push_str("</g>\n");
    }
    out.push_str("</g>\n");
    out
}

/// The text's name: its lines as one, then its tail. A line broken
/// mid-token reads with a space there (crumbed; Review rulings 20).
fn words(run: &Run<'_>) -> String {
    let mut name = run.lines.join(" ");
    if let Some(tail) = run.tail {
        name.push_str(&tail.text);
    }
    name
}

/// One `<use>` per inked glyph of `text`, from `at`.
fn uses(text: &str, style: TextStyle, at: Point, outlined: Outlined<'_>) -> String {
    let mut out = String::new();
    for placed in outlined.fonts.face(style.weight).place(text, style.size, style.letter_spacing) {
        if let Entry::Ink(id) = outlined.glyphs.entry(style, placed.glyph) {
            out.push_str("<use");
            attr(&mut out, "xlink:href", &format!("#g{id}"));
            coord(&mut out, "x", at.x + placed.x);
            coord(&mut out, "y", at.y + placed.y);
            out.push_str("/>\n");
        }
    }
    out
}
```

The writer walks only what `node()` reaches, which is what `collect` walks: both skip `is_interactive_only` nodes, and both read text through `runs`. The `expect` is therefore an invariant, and Task 4's `every_drawn_glyph_is_in_the_table` exercises it over every fixture. If clippy's `expect_used` is enabled, it is not today, so do not add an allow.

- [x] **Step 5: The standalone root**

In `src/svg/serialise/mod.rs`, split `serialise` so its ground and body are shared:

```rust
const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

pub fn serialise(document: &Document, target: Target<'_>) -> String {
    let mut out = String::from("<svg");
    attr(&mut out, "xmlns", SVG_NS);
    attr(&mut out, "viewBox", &view_box(document));
    out.push_str(">\n");
    out.push_str(&body(document, Context::top(target)));
    out.push_str("</svg>");
    out
}

/// The ground, for a static target (D-87), then the drawing.
fn body(document: &Document, context: Context<'_>) -> String {
    // …the existing ground rect, when `!context.target.interactive()`,
    // then `node(&document.root, context)`.
}

pub fn standalone(document: &Document, file: &Standalone<'_>) -> Result<String, FontError> {
    let glyphs = outline::Glyphs::of(document, file.fonts)?;
    let context = Context {
        target: Target::Static(file.palette),
        control: None,
        text: TextMode::Outlined(outline::Outlined { fonts: file.fonts, glyphs: &glyphs }),
    };
    let mut out = String::from("<svg");
    attr(&mut out, "xmlns", SVG_NS);
    attr(&mut out, "xmlns:xlink", XLINK_NS);
    coord(&mut out, "width", file.width);
    coord(&mut out, "height", file.height);
    attr(&mut out, "viewBox", &view_box(document));
    attr(&mut out, "xml:lang", "en");
    out.push_str(">\n<title>");
    out.push_str(&xml_text(file.title));
    out.push_str("</title>\n");
    if let Some(desc) = file.desc {
        out.push_str("<desc>");
        out.push_str(&xml_text(desc));
        out.push_str("</desc>\n");
    }
    out.push_str(&glyphs.defs());
    out.push_str(&body(document, context));
    out.push_str("</svg>\n");
    Ok(out)
}
```

`defs` is private to `outline`. Expose it as `pub(super) fn defs`, or have `outline` provide a `pub(super) fn defs(glyphs: &Glyphs) -> String`; whichever reads better. `xml:lang="en"` matches the page's `lang="en"`: the file's own words are English. Re-export `Standalone` and `standalone` from `src/svg/mod.rs` beside `Target` and `serialise`.

- [x] **Step 6: Run the tests to verify they pass, and that the old bytes did not move**

Run: `cargo test --lib serialise && cargo test --test serialise && cargo clippy --all-targets -- -D warnings`
Expected: all pass, the serialiser and HTML goldens included, so `serialise` writes what it wrote before. Clippy is clean, since the table now has its consumer.

- [x] **Step 7: Commit**

```bash
git add src/svg/
git commit -m "feat(svg): outlined text and the standalone root (D-88)"
```

---

### Task 3: `UNTITLED` moves to `emit`

**Files:**
- Modify: `src/emit/mod.rs`, `src/emit/html/mod.rs`, and every importer of `emit::html::UNTITLED` (`rg -n UNTITLED src tests`)

- [x] **Step 1: Move it**

Move `pub const UNTITLED: &str = "Technology Radar";` and its doc to `src/emit/mod.rs`, beside `OUTPUT_VERSION`, with the doc "D-75: what names an untitled output: the page's tab and hidden heading, the exported SVG's `<title>`." Point every importer at `crate::emit::UNTITLED`. This is a move only.

Run: `cargo test --test serialise && cargo clippy --all-targets -- -D warnings`
Expected: all pass, and the bytes are unchanged.

- [x] **Step 2: Commit**

```bash
git add src/ tests/
git commit -m "refactor(emit): UNTITLED is every output's, not the page's"
```

---

### Task 4: `-o radar.svg`

**Files:**
- Create: `src/emit/svg.rs`
- Modify: `src/emit/mod.rs` (`pub mod svg;`)
- Modify: `src/cli.rs`, `src/diagnostics.rs`, `src/pipeline.rs`
- Create: `tests/svg_matches_png.rs`
- Create: `tests/serialise/export.rs` (registered in `tests/serialise/main.rs`)
- Modify: `tests/run.rs`, `tests/cli.rs`, `tests/diagnostics.rs`

**Interfaces:**
- Consumes (Task 2): `svg::{Standalone, standalone}`. (Task 3): `emit::UNTITLED`. (PR 16): `emit::png::{pixel_size, rasterise, usvg_options}`.
- Produces:
  - `emit::svg::intrinsic_size(document: &Document, requested: Option<u32>) -> (f64, f64)`;
  - `emit::svg::export(document: &Document, export: &Export<'_>) -> Result<String, Fatal>`, with `pub struct Export<'a> { pub palette: &'a Palette, pub fonts: &'a Fonts, pub width: Option<u32>, pub title: Option<&'a str> }`.

- [x] **Step 1: Write the failing unit tests in `src/emit/svg.rs`**

Create the file with only its tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::block::fonts;
    use crate::layout::test_support::{PRD_EXAMPLE, plotted};
    use crate::palette::Theme;

    fn prd(title: Option<&str>) -> Document {
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        let blocks = Blocks::new(&catalogue, &order, title, None, fonts());
        arrange(Arrangement::Wide, &blocks)
    }

    /// FR-57, D-88: the height follows the width, unrounded.
    #[test]
    fn the_intrinsic_size_follows_the_width() {
        let doc = prd(None);
        assert_eq!(intrinsic_size(&doc, None), (doc.width, doc.height));
        let (w, h) = intrinsic_size(&doc, Some(320));
        assert_eq!(w, 320.0);
        assert_eq!(h, 320.0 * doc.height / doc.width);
    }

    /// D-88: no clamp; no pixmap is allocated.
    #[test]
    fn a_wide_default_is_not_clamped() {
        let doc = Document { width: 12_500.0, ..prd(None) };
        assert_eq!(intrinsic_size(&doc, None).0, 12_500.0);
    }

    /// SC-25, D-75, D-88: the comment, then the title or Technology Radar,
    /// then the radar's name.
    #[test]
    fn the_file_is_named() {
        let palette = Palette::built_in(Theme::Light);
        let export = |title| Export { palette: &palette, fonts: fonts(), width: None, title };
        let titled = super::export(&prd(Some("Q3 & co")), &export(Some("Q3 & co"))).expect("exports");
        assert!(titled.starts_with(&format!("<!-- {} -->\n<svg ", crate::emit::OUTPUT_VERSION)));
        assert!(titled.contains("<title>Q3 &amp; co</title>"), "{titled}");
        let untitled = super::export(&prd(None), &export(None)).expect("exports");
        assert!(untitled.contains("<title>Technology Radar</title>"));
        assert!(untitled.contains("<desc>Technology radar: 3 quadrants, "), "{untitled}");
    }
}
```

- [x] **Step 2: Write the failing integration tests**

**`tests/svg_matches_png.rs`:**

```rust
//! SC-7 and SC-15 for the exported SVG (D-88): drawn by resvg with no
//! font at all, it draws what our PNG draws, to within antialiasing.
//!
//! Two criteria (Ruling 10): few subpixels differ, and none by more than
//! antialiasing does. A planted defect (a dropped glyph) differs by far
//! more than 64 levels; the antialiasing residue never does.

use std::sync::Arc;

use radargen::emit::png::{pixel_size, rasterise, usvg_options};
use radargen::emit::svg::{Export, export};
use radargen::layout::arrange::{Arrangement, Blocks, arrange};
use radargen::order::resolve;
use radargen::palette::{Palette, Theme};
use radargen::svg::Document;
use radargen::text::Fonts;
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

#[path = "common/catalogue.rs"]
mod catalogue;

/// Set in Step 6 from the measured residue: 1.5× the worst fixture's
/// ppm, and the worst residue channel.
const BUDGET_PPM: usize = 0;
const MAX_DELTA: u8 = 0;

const LONG_TITLE: &str = "Engineering Technology Radar for the Platform and Delivery Groups";

/// The documents compared, with the title each is drawn with.
const CASES: [(&str, &str, Option<&str>); 5] = [
    ("three-quadrant", include_str!("fixtures/three-quadrant.csv"), Some("Technology Radar")),
    ("long-legend", include_str!("fixtures/long-legend.csv"), None),
    ("text-spacing", include_str!("fixtures/text-spacing.csv"), Some("Text spacing")),
    ("grown", include_str!("fixtures/grown.csv"), None),
    // A title long enough to wrap onto two lines.
    ("wrapped title", include_str!("fixtures/three-quadrant.csv"), Some(LONG_TITLE)),
];

/// Our options with an empty font database: any text the SVG still
/// carried would draw as nothing.
fn fontless() -> usvg::Options<'static> {
    usvg::Options { fontdb: Arc::new(usvg::fontdb::Database::new()), ..usvg_options() }
}

fn wide(csv: &str, title: Option<&str>, fonts: &Fonts) -> Document {
    let catalogue = catalogue::catalogue(csv, None);
    let plotted = resolve(&catalogue);
    arrange(Arrangement::Wide, &Blocks::new(&catalogue, &plotted, title, None, fonts))
}

/// Differing subpixels, the worst channel delta, and the total.
struct Difference {
    differ: usize,
    worst: u8,
    total: usize,
}

impl Difference {
    fn within(&self) -> bool {
        self.differ * 1_000_000 <= BUDGET_PPM * self.total && self.worst <= MAX_DELTA
    }
}

/// `svg` drawn fontless at the PNG's size, against the PNG.
#[expect(
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    reason = "tiny-skia transforms are f32, as `rasterise` narrows them"
)]
fn against_png(doc: &Document, svg: &str, palette: &Palette) -> Difference {
    let size = pixel_size(doc, None).expect("fits");
    let png = Pixmap::decode_png(&rasterise(doc, palette, size).expect("renders")).expect("a PNG");
    let tree = usvg::Tree::from_str(svg, &fontless()).expect("usvg parses the export");
    let mut drawn = Pixmap::new(size.width(), size.height()).expect("a pixmap");
    // The scale `rasterise` uses, so the two differ only in the text.
    resvg::render(
        &tree,
        Transform::from_scale(
            (f64::from(size.width()) / doc.width) as f32,
            (f64::from(size.height()) / doc.height) as f32,
        ),
        &mut drawn.as_mut(),
    );
    let pairs = || png.data().iter().zip(drawn.data());
    Difference {
        differ: pairs().filter(|(a, b)| a != b).count(),
        worst: pairs().map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0),
        total: png.data().len(),
    }
}

fn svg_of(doc: &Document, palette: &Palette, fonts: &Fonts, title: Option<&str>) -> String {
    export(doc, &Export { palette, fonts, width: None, title }).expect("exports")
}

/// SC-7 for SVG: every case, both themes.
#[test]
fn the_svg_draws_what_the_png_draws() {
    let fonts = Fonts::embedded().expect("loads");
    for (name, csv, title) in CASES {
        for theme in [Theme::Light, Theme::Dark] {
            let doc = wide(csv, title, &fonts);
            let palette = Palette::built_in(theme);
            let d = against_png(&doc, &svg_of(&doc, &palette, &fonts, title), &palette);
            eprintln!(
                "{name} {theme:?}: {} of {} subpixels ({} ppm), worst {}",
                d.differ, d.total, d.differ * 1_000_000 / d.total, d.worst
            );
            assert!(d.within(), "{name} {theme:?}");
        }
    }
}

/// The comparison is not vacuous: one glyph dropped fails it.
#[test]
fn the_comparison_catches_a_dropped_glyph() {
    let fonts = Fonts::embedded().expect("loads");
    let (_, csv, title) = CASES[0];
    let doc = wide(csv, title, &fonts);
    let palette = Palette::built_in(Theme::Light);
    let svg = svg_of(&doc, &palette, &fonts, title);
    let at = svg.find("<use ").expect("a glyph");
    let end = at + svg.get(at..).and_then(|s| s.find('\n')).expect("its line") + 1;
    let damaged = [svg.get(..at).expect("before"), svg.get(end..).expect("after")].concat();
    assert!(!against_png(&doc, &damaged, &palette).within(), "a dropped glyph must fail");
}

/// NFR-7: a name Inter lacks draws the shaper's `.notdef`, as resvg does.
#[test]
fn a_name_inter_lacks_draws_as_the_png_does() {
    let fonts = Fonts::embedded().expect("loads");
    let csv = "name,quadrant,ring,description\n東京,Tools,Adopt,d\n😀,Tools,Trial,d\nKafka,Tools,Assess,d\n";
    let doc = wide(csv, None, &fonts);
    let palette = Palette::built_in(Theme::Light);
    assert!(against_png(&doc, &svg_of(&doc, &palette, &fonts, None), &palette).within());
}

/// The table's invariant (Task 2): every glyph the writer places was
/// collected, over every case; an `expect` in the writer would fire.
#[test]
fn every_drawn_glyph_is_in_the_table() {
    let fonts = Fonts::embedded().expect("loads");
    let palette = Palette::built_in(Theme::Dark);
    for (_, csv, title) in CASES {
        let doc = wide(csv, title, &fonts);
        let svg = svg_of(&doc, &palette, &fonts, title);
        assert!(svg.contains("<use "), "drew glyphs");
    }
}
```

**`tests/serialise/export.rs`,** with `mod export;` in `tests/serialise/main.rs`:

```rust
//! The exported SVG's names, vocabulary and escaping (SC-9, SC-15, NFR-6,
//! D-88).

use radargen::emit::OUTPUT_VERSION;
use radargen::emit::svg::{Export, export};
use radargen::layout::arrange::{Arrangement, arrange};
use radargen::palette::{Palette, Theme};
use radargen::svg::{Document, Node, Semantics, TextPaint};
use radargen::text::Fonts;

use super::build;
use super::hostile::{QUADRANT, RING, TITLE, csv};

/// The elements the export writes.
const ELEMENTS: [&str; 10] =
    ["svg", "title", "desc", "defs", "path", "use", "g", "rect", "circle", "line"];

/// The attributes it writes, by local name: the static serialiser's
/// shapes, groups and controls, and the export's own. No `font-*`, no
/// `text-anchor`, `letter-spacing` or `style`.
const ATTRIBUTES: [&str; 28] = [
    "viewBox", "width", "height", "lang", "id", "href", "d", "x", "y", "x1", "y1", "x2", "y2",
    "cx", "cy", "r", "rx", "fill", "stroke", "stroke-width", "stroke-linejoin", "transform",
    "role", "aria-label", "aria-level", "aria-hidden", "visibility", "pointer-events",
];

/// The hostile CSV, wide, exported dark at 800.
fn hostile() -> (Document, String) {
    let built = build(&csv(), Some(&format!("{RING},Adopt")), Some(TITLE));
    let doc = arrange(Arrangement::Wide, &built.blocks);
    let palette = Palette::built_in(Theme::Dark);
    let fonts = Fonts::embedded().expect("loads");
    let svg = export(&doc, &Export { palette: &palette, fonts: &fonts, width: Some(800), title: Some(TITLE) })
        .expect("exports");
    (doc, svg)
}

/// What the export must name, in document order: each filled text
/// outside a control by its words (and tail), each control by its
/// label, and the radar's group by its name.
fn names(node: &Node, in_control: bool, out: &mut Vec<String>) {
    match node {
        _ if node.is_interactive_only() => {}
        Node::Group(group) => {
            let inner = match &group.semantics {
                Semantics::Marker(c) | Semantics::LegendRow(c) => {
                    out.push(c.label.clone());
                    true
                }
                Semantics::Group(name) => {
                    out.push(name.clone());
                    in_control
                }
                _ => in_control,
            };
            for child in &group.children {
                names(child, inner, out);
            }
        }
        Node::Text { ink: TextPaint::Fill(_), lines, tail, .. } if !in_control && !lines.is_empty() => {
            let mut name = lines.join(" ");
            if let Some(tail) = tail {
                name.push_str(&tail.text);
            }
            out.push(name);
        }
        _ => {}
    }
}

/// SC-9 through an XML parser: every name reads back exactly, once, in
/// order; the root's title and description read back; the only comment
/// is the version.
#[test]
fn the_export_names_every_text_exactly() {
    let (doc, svg) = hostile();
    let parsed = roxmltree::Document::parse(&svg).expect("well-formed XML");
    let mut want = Vec::new();
    names(&doc.root, false, &mut want);
    let got: Vec<&str> = parsed.descendants().filter_map(|n| n.attribute("aria-label")).collect();
    assert_eq!(got, want);
    // The hostile strings reached the names at all.
    for needle in [QUADRANT, RING] {
        assert!(got.iter().any(|l| l.contains(needle)), "{needle} in {got:?}");
    }
    let root = parsed.root_element();
    let child = |tag: &str| root.children().find(|n| n.has_tag_name(tag)).and_then(|n| n.text());
    assert_eq!(child("title"), Some(TITLE));
    assert_eq!(child("desc"), want.iter().find(|n| n.starts_with("Technology radar: ")).map(String::as_str));
    // The static tooltips read back as the document holds them.
    let tooltips: Vec<&str> = parsed
        .descendants()
        .filter(|n| n.has_tag_name("title") && n.parent() != Some(root))
        .filter_map(|n| n.text())
        .collect();
    assert_eq!(tooltips, super::hostile::Strings::of(&doc.root, false).tooltips);
    let comments: Vec<&str> = parsed.root().children().filter(|n| n.is_comment()).filter_map(|n| n.text()).collect();
    assert_eq!(comments, [format!(" {OUTPUT_VERSION} ")]);
}

/// SC-15, NFR-5: only the export's vocabulary; every id a glyph in
/// `<defs>`; every reference an XLink fragment naming one; every number
/// NFR-17's shape.
#[test]
fn the_export_uses_only_its_vocabulary() {
    const XLINK: &str = "http://www.w3.org/1999/xlink";
    for (_, svg) in [hostile(), prd_export()] {
        let parsed = roxmltree::Document::parse(&svg).expect("well-formed XML");
        let ids: Vec<&str> = parsed.descendants().filter_map(|n| n.attribute("id")).collect();
        for node in parsed.descendants().filter(roxmltree::Node::is_element) {
            let name = node.tag_name().name();
            assert!(ELEMENTS.contains(&name), "<{name}>");
            for a in node.attributes() {
                assert!(ATTRIBUTES.contains(&a.name()), "<{name} {}>", a.name());
                assert!(!a.value().contains("url("), "<{name} {}={}>", a.name(), a.value());
            }
            if let Some(id) = node.attribute("id") {
                assert!(name == "path" && node.parent().is_some_and(|p| p.has_tag_name("defs")), "{id}");
                assert!(id.strip_prefix('g').is_some_and(|n| n.chars().all(|c| c.is_ascii_digit())), "{id}");
            }
            if name == "use" {
                let href = node.attribute((XLINK, "href")).expect("an XLink href");
                assert!(ids.contains(&href.strip_prefix('#').expect("a fragment")), "{href}");
            }
            if !["title", "desc"].contains(&name) {
                assert!(
                    node.children().filter(roxmltree::Node::is_text).filter_map(|t| t.text())
                        .all(|t| t.chars().all(char::is_whitespace)),
                    "text in <{name}>"
                );
            }
        }
        super::vocabulary::assert_numbers_are_fmt_coord(&svg);
    }
}

fn prd_export() -> (Document, String) {
    let doc = arrange(Arrangement::Wide, &super::prd().blocks);
    let palette = Palette::built_in(Theme::Light);
    let fonts = Fonts::embedded().expect("loads");
    let svg = export(&doc, &Export { palette: &palette, fonts: &fonts, width: None, title: Some("Technology Radar") })
        .expect("exports");
    (doc, svg)
}
```

If `vocabulary.rs`'s number-shape check is not already a reusable function, extract it as `pub fn assert_numbers_are_fmt_coord(markup: &str)`, covering `d` and every coordinate attribute. Its existing caller keeps its behaviour, so it is a move, not a change. Match `QUADRANT`, `RING`, `TITLE` and `csv` to `hostile.rs`'s real exports, and `Strings::of`'s tooltip order to its real walk. If a name in `ATTRIBUTES` turns out never to be written, remove it: the list is what the export writes, not what it might write.

**`tests/run.rs`:**
- Rename `bad_extensions_and_svg` to `bad_extensions`, expecting `["error: -o radar.txt: the output file name must end in .html, .png or .svg"]`, with no `.svg` case.
- After the PNG section, add:

```rust
// ---- SVG (FR-54, FR-57, FR-71, D-88)

/// The root's `width`, parsed.
fn root_width(svg: &str) -> f64 {
    let parsed = roxmltree::Document::parse(svg).expect("well-formed");
    parsed.root_element().attribute("width").expect("a width").parse().expect("a number")
}

/// FR-54, FR-57, D-88: a standalone SVG at --width, its height in proportion.
#[test]
fn renders_an_svg_at_the_width_asked() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.SVG", "--width", "1920", "--theme", "dark"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let svg = read(dir.path(), "radar.SVG");
    assert!(svg.starts_with("<!-- radargen 1."), "{}", svg.lines().next().unwrap_or_default());
    assert_eq!(root_width(&svg), 1920.0);
    assert!(!svg.contains("<text") && svg.ends_with("</svg>\n"));
    let ran = run_in(dir.path(), &["radar.csv", "-o", "max.svg", "--width", "10000"]);
    assert!(ran.ok() && ran.lines().is_empty(), "{}", ran.stderr);
    assert_eq!(root_width(&read(dir.path(), "max.svg")), 10_000.0);
}

/// D-88: the PNG's clamp and small-text warnings are not the SVG's.
#[test]
fn an_svg_gives_no_png_warnings() {
    let dir = seeded(&[("radar.csv", &too_wide())]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "wide.svg"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert!(ran.lines().is_empty(), "no warning: {}", ran.stderr);
    assert!(root_width(&read(dir.path(), "wide.svg")) > 12_500.0, "not clamped");
    let ran = run_in(dir.path(), &["radar.csv", "-o", "narrow.svg", "--width", "320"]);
    assert!(ran.ok() && ran.lines().is_empty(), "{}", ran.stderr);
}

/// FR-62 for SVG, its exact line.
#[test]
fn an_svg_into_a_missing_directory_fails() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["radar.csv", "-o", "nowhere/radar.svg"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(ran.lines(), ["error: -o nowhere/radar.svg: the directory nowhere does not exist"]);
    assert_eq!(snapshot(dir.path()), before);
}

/// D-74 for SVG: the input under a .svg name is refused, untouched.
#[test]
fn an_svg_output_that_is_the_input_is_refused() {
    let dir = seeded(&[("data.svg", PRD)]);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["data.svg", "-o", "data.svg"]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(ran.lines(), ["error: -o data.svg: is the input CSV; choose another output path"]);
    assert_eq!(snapshot(dir.path()), before);
}

/// SC-18: a run that fails after `-o` exists leaves it byte-identical.
#[test]
fn a_failed_svg_run_leaves_the_old_file() {
    let dir = seeded(&[("radar.csv", PRD), ("bad.csv", "name,quadrant,ring,description\n,Tools,Adopt,d\n")]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.svg"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["bad.csv", "-o", "radar.svg"]);
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(snapshot(dir.path()), before);
}

/// The edges: a header-only CSV, and a subtitle with no title.
#[test]
fn an_empty_or_subtitled_svg_renders() {
    let dir = seeded(&[("empty.csv", HEAD), ("radar.csv", PRD)]);
    let ran = run_in(dir.path(), &["empty.csv", "-o", "empty.svg"]);
    assert!(ran.ok(), "{}", ran.stderr);
    roxmltree::Document::parse(&read(dir.path(), "empty.svg")).expect("well-formed");
    let ran = run_in(dir.path(), &["radar.csv", "-o", "sub.svg", "--subtitle", "Edition 2026.2"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let svg = read(dir.path(), "sub.svg");
    assert!(svg.contains("<title>Technology Radar</title>"), "D-75's fallback");
    assert!(svg.contains("aria-label=\"Edition 2026.2\""), "the subtitle is named");
}

/// NFR-7: names in scripts Inter lacks still export, escaped.
#[test]
fn an_svg_of_hostile_text_renders() {
    let csv = format!("{HEAD}Кафка,Tools,Adopt,d\n東京,Tools,Trial,d\n\"😀 <b>&amp;\",Tools,Assess,d\n");
    let dir = seeded(&[("radar.csv", &csv)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.svg", "--title", "</title>]]>"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let svg = read(dir.path(), "radar.svg");
    assert!(svg.contains("<title>&lt;/title&gt;]]&gt;</title>"), "escaped");
    roxmltree::Document::parse(&svg).expect("well-formed");
}
```

Check the row-error CSV and the header-only `HEAD` against what `tests/run.rs` already uses. If an empty-`name` row is not a row error, use whichever row error the existing `a_failed_run_leaves_the_old_file` uses. If a header-only CSV is an error by FR rule rather than an empty radar, assert that error's exact line instead, and keep the subtitle case.

**`tests/cli.rs`:**
- Rename `png_is_a_format_and_svg_arrives_in_1_2` to `png_and_svg_are_formats`, asserting `OutputFormat::Svg` for `e.svg`, `E.SVG` and `e.Svg`.
- The about-line assertion becomes `about.contains("PNG") && about.contains("SVG")`.
- `an_unrecognised_or_absent_extension_is_a_different_error`'s panic text drops "not deferred".

**`tests/diagnostics.rs`:**
- Delete the `FormatNotInThisVersion` row.
- The two `UnrecognisedExtension` rows (`radar.pdf`, `radar`) expect `…must end in .html, .png or .svg`.

- [x] **Step 3: Run each target to verify it fails for the right reason**

Run each separately, since one target's compile error hides another's runtime failures:
- `cargo test --lib emit::svg 2>&1 | tail -3`
- `cargo test --test svg_matches_png 2>&1 | tail -3`
- `cargo test --test serialise export 2>&1 | tail -3`
- `cargo test --test run svg 2>&1 | tail -8`
- `cargo test --test cli 2>&1 | tail -3`
- `cargo test --test diagnostics 2>&1 | tail -3`

Expected:
- the library, `svg_matches_png`, `serialise` and `cli` fail to compile (`emit::svg` and `OutputFormat::Svg` don't exist);
- `run` compiles, and each new SVG test fails on its exact message or on the missing file. None passes;
- `diagnostics` fails on the changed message.

- [x] **Step 4: Implement `src/emit/svg.rs`**

Above the tests module:

```rust
//! The SVG export (FR-54, FR-57, FR-71, D-88): the wide document with
//! literal colours for one theme over its ground, its text as outlines,
//! sized by `--width`, under the version comment (SC-25).

use crate::diagnostics::Fatal;
use crate::emit::{OUTPUT_VERSION, UNTITLED};
use crate::palette::Palette;
use crate::svg::{Document, Node, Semantics, Standalone, standalone};
use crate::text::Fonts;

/// What one export needs besides the document.
pub struct Export<'a> {
    /// The theme's palette (FR-56).
    pub palette: &'a Palette,
    /// The faces the text is shaped and outlined from.
    pub fonts: &'a Fonts,
    /// `--width` (FR-57).
    pub width: Option<u32>,
    /// FR-65's title, resolved.
    pub title: Option<&'a str>,
}

/// FR-57: `requested` wide and the height in proportion, or the
/// document's own size. Unrounded and unclamped (D-88).
#[must_use]
pub fn intrinsic_size(document: &Document, requested: Option<u32>) -> (f64, f64) {
    requested.map_or((document.width, document.height), |width| {
        let width = f64::from(width);
        (width, width * document.height / document.width)
    })
}

/// FR-54: `document` as a standalone SVG file (D-88).
///
/// # Errors
///
/// [`Fatal::Font`] when a glyph cannot be outlined (Ruling 6).
pub fn export(document: &Document, export: &Export<'_>) -> Result<String, Fatal> {
    let (width, height) = intrinsic_size(document, export.width);
    let root = standalone(
        document,
        &Standalone {
            palette: export.palette,
            fonts: export.fonts,
            width,
            height,
            title: export.title.unwrap_or(UNTITLED),
            desc: radar_name(&document.root),
        },
    )?;
    // SC-25: the emitter's line, before the root (§6).
    Ok(["<!-- ", OUTPUT_VERSION, " -->\n", &root].concat())
}

/// The radar's group name (D-75): the first named group, depth first.
/// The wide arrangement has exactly one, the radar's; an arrangement
/// with a second would need this to say which.
fn radar_name(node: &Node) -> Option<&str> {
    match node {
        Node::Group(group) => match &group.semantics {
            Semantics::Group(name) => Some(name),
            _ => group.children.iter().find_map(radar_name),
        },
        _ => None,
    }
}
```

Add `pub mod svg;` to `src/emit/mod.rs`.

- [x] **Step 5: CLI, diagnostics and pipeline**

- **`src/cli.rs`:**
  - `OutputFormat` gains `/// FR-54: the standalone SVG.` `Svg,`.
  - `format()` maps `"svg" => Ok(OutputFormat::Svg)`. Its `# Errors` names only `UnrecognisedExtension`, and the `§14.1` comment goes.
  - The about line: `"Render a technology radar from a CSV file to a self-contained HTML page, a PNG image or an SVG file."`
  - `-o`'s help: `The extension picks the format: .html, .png or .svg, in any case.`
  - `--theme`'s first line: `The theme for PNG and SVG.`
  - `--width`'s first line: `The width in pixels for PNG and SVG.`
  - Every `-h` line stays within 120 columns; `tests/cli.rs` checks it.
- **`src/diagnostics.rs`:** delete `FormatNotInThisVersion` and its doc. `UnrecognisedExtension`'s message becomes `"-o {path}: the output file name must end in .html, .png or .svg"`.
- **`src/pipeline.rs`:** import `crate::emit::svg`. In `render`, add the arm below. The doc's last sentence becomes "…or the wide arrangement as a PNG or an SVG (FR-71)."

```rust
OutputFormat::Svg => Ok(svg::export(
    &arrange(Arrangement::Wide, &blocks),
    &svg::Export {
        palette: &config.palette,
        fonts,
        width: config.width,
        title: config.title.as_deref(),
    },
)?
.into_bytes()),
```

- [x] **Step 6: Measure the residue, set the criteria, and verify**

1. Run `cargo test --test svg_matches_png the_svg_draws_what_the_png_draws -- --nocapture 2>&1 | rg ppm`, with `BUDGET_PPM` and `MAX_DELTA` still 0, so every case prints.
2. **Ledger** each case's ppm and worst channel.
3. **Set the criteria:**
   - `BUDGET_PPM` is the worst case's ppm × 1.5, rounded up.
   - `MAX_DELTA` is the worst case's worst channel. The Rust review saw at most 95 ppm, and the spike a worst of 64.
4. **Write both values into the constants' doc,** with the date, so the next change to them is a visible decision.
5. **If any case's worst channel is far above 64, stop.** A glyph has moved: use superpowers:systematic-debugging. Render both, write the difference image, and find it.

Then run:
- `cargo test --lib emit::svg`
- `cargo test --test svg_matches_png`
- `cargo test --test serialise`
- `cargo test --test run`
- `cargo test --test cli`
- `cargo test --test diagnostics`
- `cargo clippy --all-targets -- -D warnings`

Expected: all pass, including `the_comparison_catches_a_dropped_glyph`.

- [x] **Step 7: Commit**

```bash
git add src/ tests/
git commit -m "feat(svg): -o radar.svg writes the standalone export (FR-54)"
```

---

### Task 5: Goldens, the binary against them, determinism and size

**Files:**
- Modify: `tests/serialise/golden.rs`
- Create: `tests/golden/svg/prd-light.svg`, `tests/golden/svg/prd-dark.svg` (blessed)
- Modify: `tests/run.rs` (the binary against the goldens)
- Modify: `tests/determinism.rs`

- [x] **Step 1: Write the golden tests, and watch them fail**

In `tests/serialise/golden.rs`:

```rust
/// The PRD example, wide, exported in `theme` at its own width, titled
/// as `prd()` titles it.
fn prd_svg(theme: Theme) -> String {
    let doc = arrange(Arrangement::Wide, &prd().blocks);
    let palette = Palette::built_in(theme);
    let fonts = radargen::text::Fonts::embedded().expect("loads");
    radargen::emit::svg::export(
        &doc,
        &radargen::emit::svg::Export {
            palette: &palette,
            fonts: &fonts,
            width: None,
            title: Some("Technology Radar"),
        },
    )
    .expect("exports")
}

/// FR-54, NFR-4: the PRD example as a light SVG, byte for byte, on every
/// platform. One test per golden, since a blessing run stops at the first.
#[test]
fn prd_example_matches_its_light_svg_golden() {
    golden("svg", "prd-light.svg", &prd_svg(Theme::Light));
}

/// FR-54, FR-56: the same, dark.
#[test]
fn prd_example_matches_its_dark_svg_golden() {
    golden("svg", "prd-dark.svg", &prd_svg(Theme::Dark));
}
```

Run: `cargo test --test serialise golden::prd_example_matches_its_light_svg_golden`
Expected: FAIL, because the golden is missing.

- [x] **Step 2: Bless, and look**

Run each, one per command:
- `UPDATE_GOLDEN=1 cargo test --test serialise golden::prd_example_matches_its_light_svg_golden`
- `UPDATE_GOLDEN=1 cargo test --test serialise golden::prd_example_matches_its_dark_svg_golden`

Expected: each fails with "blessed …". Then `cargo test --test serialise golden` passes.

Open both goldens in a browser and compare them with `tests/golden/png/prd-{light,dark}.png`. **Ledger:**
- each golden's size;
- its `<path>` count;
- its `<use>` count.

Task 0's §11 figures are filled in from these numbers, in the same commit.

- [x] **Step 3: The binary writes the goldens; two processes write the same bytes**

In `tests/run.rs`:

```rust
/// FR-71, FR-56 end to end: the binary's export is the library's golden,
/// so the CLI arranges wide, binds the theme and passes the title.
#[test]
fn the_cli_writes_the_golden_exports() {
    let dir = seeded(&[("radar.csv", PRD)]);
    for (theme, golden) in [("light", "prd-light.svg"), ("dark", "prd-dark.svg")] {
        let ran = run_in(dir.path(), &["radar.csv", "-o", golden, "--theme", theme, "--title", "Technology Radar"]);
        assert!(ran.ok(), "{}", ran.stderr);
        let want = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/svg").join(golden))
            .expect("the golden");
        assert!(read(dir.path(), golden) == want, "{golden}: the binary's export is the golden");
    }
}
```

`PRD` in `tests/run.rs` must be `three-quadrant.csv`, the fixture `prd()` reads. If it is not, read that fixture here instead.

In `tests/determinism.rs`:

```rust
/// NFR-4, SC-8 for SVG: two processes write the same bytes, in both
/// themes, at a width, and over the hostile case. Written after the
/// export, so a regression lock rather than a first failing test.
#[test]
fn two_processes_write_the_same_svg() {
    let dir = tempfile::tempdir().expect("tempdir");
    for (csv, args) in [
        (PRD, &["--theme", "dark", "--width", "1200"][..]),
        (HOSTILE, &["--title", "</title><svg onload=x>"][..]),
    ] {
        let first = render(dir.path(), csv, "first.svg", args);
        let second = render(dir.path(), csv, "second.svg", args);
        assert!(first == second, "NFR-4: two processes, one SVG");
    }
}
```

Run: `cargo test --test run the_cli_writes_the_golden_exports && cargo test --test determinism`
Expected: PASS.

- [x] **Step 4: Commit**

```bash
git add tests/ docs/design/architecture.md
git commit -m "test(svg): goldens, the binary against them, two-process determinism"
```

---

### Task 6: Docs, version, release

**Files:**
- Modify: `README.md` (lines 4 and 66 say "SVG arrives in 1.2"; add an SVG section after PNG's)
- Modify: `docs/a11y/manual-checks.md` (step 16, result rows, revision 1.4)
- Modify: `docs/design/architecture.md` (§9 step 7: SVG done)
- Modify: `Cargo.toml` (`version = "1.2.0"`, the description)
- Modify: `tests/golden/html/prd.html` and `tests/golden/svg/*` (re-blessed for `radargen 1.2`)
- Modify: `CHANGELOG.md` (regenerated; it carries the 1.1.0 reconciliation left uncommitted on `main`)
- Move: `docs/plans/pr-17-svg.md` to `docs/plans/done/`
- Crumbs: create two (Step 5)

- [x] **Step 1: README**

- **Line 4:** "…to a self-contained HTML page, a PNG image or an SVG file. One binary, no runtime, no network."
- **Status (line 66):** "1.2 writes HTML, PNG and SVG."
- **After the PNG section:**

```markdown
A `.svg` output draws the same wide arrangement as the PNG, as vectors: sharp at any size, and editable in PowerPoint, Figma or Illustrator. `--theme` picks its colours, and `--width` sets its size, with the height following.

- **Text as shapes.** The text is drawn as outlines, so the file needs no font and looks the same everywhere; its words cannot be selected or searched.
- **Accessibility.** Opened directly in a browser, a screen reader reads its title, its headings and each label. Placed in a page or a slide, it is an image: give it alt text there, and link the HTML page or the CSV as the full description.
- **The background.** The theme's background is part of the drawing. Deleting it in an editor puts dark-theme text on whatever lies behind.
- **Pasting inline.** Two exports pasted inline into one web page share their glyph names and draw each other's text. Place each as an image (`<img src>`) instead.
```

- [x] **Step 2: Manual check 16**

```markdown
16. **The SVG** (D-88, from 1.2.0). Render `three-quadrant.csv` with `--title "Technology Radar"` to `light.svg`, and again with `--theme dark` to `dark.svg`.
    - *Expect, opened in a browser:* each looks like its PNG (step 15), in Inter, with every legend name readable at 100% zoom. Text that cannot be selected is expected, not a fail.
    - *Expect, with NVDA + Firefox, NVDA + Chrome and VoiceOver + Safari, opening the file directly:*
      - the document is named "Technology Radar", and its description reads "Technology radar: 3 quadrants, …";
      - browse mode moves into the drawing rather than stopping at one graphic;
      - the headings list holds: level 1 "Technology Radar"; level 2 "Key" and "Legend"; level 3 each quadrant;
      - ring labels read outermost first, as in the page;
      - each marker reads once, by its full name ("1. …; … quadrant, … ring"), never its number alone;
      - each legend row reads once;
      - no label reads twice, no halo is read, and nothing reads as an unnamed "graphic" or "group".
      - Record whether a marker's tooltip is read again as a description.
    - *Expect, in PowerPoint (Insert ▸ Pictures), and in Figma or Illustrator if available, in both themes:*
      - the text is the theme's colour, not black;
      - ring labels show no dark smudge behind them, which would be a halo drawn filled;
      - the drawing can be ungrouped, and each glyph is a shape.
      - Record the alt text PowerPoint assigns.
      - A black or smudged result is a fail. The fallback is paint on each `<use>` (plan Ruling 12).
    - *Expect, without `--title`:* the document is named "Technology Radar", and no level-1 heading exists.
```

Add three results rows: "The SVG: NVDA + Firefox, NVDA + Chrome, VoiceOver + Safari", "The SVG: PowerPoint, light and dark", and "The SVG: Figma or Illustrator, light and dark". Add the revision row `| 1.4 | 2026-10-16 | Step 16: the exported SVG (D-88, rg-m2l). |`, and bump the frontmatter revision to match.

- [x] **Step 3: Version, description, goldens**

`Cargo.toml`: `version = "1.2.0"`. The description reads "…to a self-contained HTML page, a PNG image or an SVG file."

Run each:
- `cargo build`
- `UPDATE_GOLDEN=1 cargo test --test serialise page::prd_page_matches_its_golden`
- `UPDATE_GOLDEN=1 cargo test --test serialise golden::prd_example_matches_its_light_svg_golden`
- `UPDATE_GOLDEN=1 cargo test --test serialise golden::prd_example_matches_its_dark_svg_golden`

Then `git diff tests/golden`. Expected: each file's diff is the version line alone (`radargen 1.1` → `1.2`). Run `cargo test` afterwards; `the_cli_writes_the_golden_exports` passes against the re-blessed files.

- [x] **Step 4: Full check**

Run: `just ci 2>&1 | tail -30`
Expected: green, except the known browser check "a long legend scrolls beside the radar". It fails identically on `main` on this machine, from macOS overlay scrollbars, and has its own crumb. Name it in the PR body.

- [x] **Step 5: Crumbs, changelog, plan**

```bash
CRUMBS=/Users/evensolberg/.cargo/bin/crumbs
$CRUMBS create 'A line broken mid-token reads with a space in its SVG name' --type bug --priority 3 --tags svg,a11y --message 'D-88 names an outlined text by its lines joined with spaces; a token wrap_by broke mid-token reads "Kuber netes". Document holds lines, not the source. Check too whether the HTML multi-line <text> (tspans with no space between) is named without one, the mirror fault. From the PR 17 plan reviews.'
$CRUMBS create 'Letter-spacing on scripts usvg does not space' --type bug --priority 4 --tags png,svg,layout --message 'usvg skips letter-spacing for some scripts (layout.rs, Devanagari and others); measure and place add it, so layout and the SVG differ from the PNG there. Inter draws none of those scripts (.notdef), so the effect is box spacing. From the PR 17 plan QA review.'
$CRUMBS close rg-m2l --reason "PR 17: SVG export (1.2.0)"
git mv docs/plans/pr-17-svg.md docs/plans/done/pr-17-svg.md
```

In the moved plan, set `status: complete` and `completed_date`, and tick every task. Regenerate `CHANGELOG.md` with git-cliff as the project's release recipe does. Then:

```bash
git add -A
git commit -m "chore(release): 1.2.0; close rg-m2l"
```

---

## Self-review

- **Spec coverage:**
  - FR-54: Tasks 2 and 4.
  - FR-56: Task 4, and Task 5's dark golden and CLI test.
  - FR-57: Task 4.
  - FR-71: Task 4, and Task 5's CLI test.
  - NFR-4 and SC-8: Task 5.
  - SC-7: Task 4's two-criteria comparison and its negative control.
  - SC-9: Task 4's exact-name test, and Task 5's hostile determinism case.
  - SC-15: Tasks 2 and 4, the comparison drawn with no font.
  - SC-18: Task 4.
  - SC-25: Task 4, and Task 5's goldens.
  - §14.1: Task 0.
  - D-75's fallback: Task 4.
  - D-78's tail: Task 2.
- **Not covered by automation:** PowerPoint, Figma and Illustrator, which manual check 16 covers (Ruling 12).
- **Placeholders:**
  - `BUDGET_PPM` and `MAX_DELTA` are set by Task 4 Step 6 from a measurement the step prescribes.
  - `RECORDED` is set by Task 1 Step 1 the same way.
  - Where a name is copied from existing tests (`HEAD`, the row-error CSV, `hostile.rs`'s exports, `vocabulary.rs`'s number check), the step names the file holding the real one.
- **Types:**
  - `Standalone`'s fields (no `generator`) match between Tasks 2 and 4.
  - `Export`'s fields match between Tasks 4 and 5.
  - `Glyphs::entry(style, glyph) -> Entry` is used only in Task 2.
  - `Segment` is the same in Tasks 1 and 2.
