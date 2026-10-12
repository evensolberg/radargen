---
tags:
  - plan
  - layout
  - arrange
aliases: []
doc_id: RG-012
doc_name: pr-12-arrange
crumb_id: rg-ozb
document_title: "PR 12 — the wide and narrow arrangements: Implementation Plan"
created_date: 2026-10-11
synopsis: "arrange.rs places the four blocks two ways from one set: wide, with the legend beside the radar at scale 1, and narrow, with the legend below and the radar scaled to fit a 400-unit column (never below 0.3; the document widens when even that does not fit, with FR-94 headroom to 480 and a warning beyond). Ships as PR 12a (FR-94 headroom, PRD v0.54) and PR 12b (arrange) (FR-68, FR-70, FR-71, FR-94, D-31, D-56, D-70, D-71)."
status: complete
type: Plan
revision: "1.4"
review_date: 2026-10-11
reviewed_by: [rust-engineer, architect-reviewer, qa-expert, ui-designer, accessibility-tester]
completed_date: 2026-10-11
comments: "1.1 folds in the five-seat plan review and the maintainer's re-ruling on the narrow fit (headroom to 480, warn beyond). See Review rulings."
---
# PR 12 — the wide and narrow arrangements: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-ozb` · **Phase:** v1.0 · **After:** PR 09, PR 10a/10b, PR 11, rg-rvh · **Then:** PR 13 (`svg` serialiser, `rg-6ku`)

**Two PRs.** **PR 12a** is Task 1: the narrow fit's PRD contract (v0.54) and FR-94's headroom, meaning taller legend rows and larger hit areas, with the ripple through the legend and radar tests. **PR 12b** is Tasks 2–4: `arrange.rs`. They split by concern: 12a changes what existing blocks draw, and 12b only places them.

**Goal:** `layout::arrange` turns one `Blocks` (title, radar, key, legend) into a `Document` in either arrangement.

- **Wide** has the legend beside the radar, every block at scale 1.
- **Narrow** has the legend below the key, with the radar scaled to the largest `s` in [0.3, 1] that fits the 400-unit radar column. When none fits, `s` stays at 0.3 and the document widens to hold the radar.
  - Up to `NARROW_MAX_WIDTH` (480) the widening costs nothing: FR-94's targets are sized for it.
  - Beyond that, FR-95 warns.
- The title, key and legend are drawn at scale 1 in both. Only the translates and the radar's scale differ between the arrangements (FR-68).

**Architecture:** `src/layout/arrange.rs`.

- `placements(Arrangement, &Blocks) -> Placements` is an integration. It computes the narrow scale with `narrow_scale` and hands it to the operations `wide(&Blocks)` / `narrow(&Blocks, scale)`. These compute each block's `Placement`, the key's rule and the document size from the blocks' `size`s alone.
- `arrange(Arrangement, &Blocks) -> Document` renders each block at its placement, in document order: title, radar, key rule, key, legend.
- `narrow_scale(&impl Block, column) -> f64` is a fixed-iteration bisection. It returns only values that pass the fit test, so its result always fits. It is the largest such value because the radar's size is non-decreasing in `s` (PR 10a's proof, for `s ≥ 0.0034`).
- `svg` gains `Document { width, height, root }`, with no dependency on `layout`.

**Tech Stack:** Rust 2024, `std` only. No new dependency.

**Spec:** `docs/product/radargen-prd.md` v0.53 → v0.54:

- FR-42, FR-65, FR-66, FR-68, FR-69, FR-70, FR-71, FR-79, FR-88, FR-89, FR-94, new FR-95
- NFR-1, SC-6, SC-17
- D-16, D-31, D-56, D-66, D-69, D-70, new D-71

Design: `docs/design/architecture.md` §3 (module table), §4 (`Block`, `Arrangement`, FR-69's computed breakpoint), §5 (`Document`) and §9 (build order). Reference: `docs/product/mockup.html` `buildDocument` (l.764–795) and `buildKey` (l.675–682). Do not read layout values off the mockup (PRD §10); every value here is derived below.

## Global Constraints

- Lints are as strict as `Cargo.toml` says. Suppress only with `#[expect(lint, reason)]` whose reason is true; rustqual markers likewise. Never reshape code to hide from a lint or rustqual. No silent failures.
  - **Bindings are descriptive** (`top`, `radar`, `key`, `legend`, `scale`). The letters in this plan's formulas are notation only: `clippy::many_single_char_names` is denied.
  - Every `pub` item and field has `///` docs (`missing_docs` is denied). `pub fn` carries `#[must_use]`.
- **Scale is applied to coordinates, never as an SVG transform.** `Group` only translates. Each block wraps itself in a translating `Group` at `Placement.origin` (`block::assert_translates` is the shared test). `arrange` adds no transform. `Group::new` takes a `Vec<Node>`.
- **One set of blocks.** `arrange` never re-lays-out, re-measures or re-wraps a block. It reads `size(scale)` and calls `render(placement)`, nothing else (FR-68, architecture §4).
- **Title, key and legend at scale 1** in both arrangements: FR-94's row height and SC-6 assume it (D-31, D-56).
- **No hex in a `Node`.** The key's rule is `Paint::Role(Role::Neutral(Neutral::StrokeDivider))`.
- **NFR-4:**
  - Use plain `*`/`+`, with the per-site `suboptimal_flops` expect where it fires, never `mul_add`.
  - Take halves with `f64::midpoint`.
  - The bisection runs a **fixed** number of iterations.
  - Tests compare floats with `close`/`near`, never `==` (`clippy::float_cmp` is denied), except a deliberate `to_bits()` pin.
- `indexing_slicing` is denied, so destructure with slice patterns. Helpers take ≤ 4 parameters (bundle into a struct).
- rustqual:
  - `pub fn arrange` carries `// qual:api — PR 14's main.rs is its first production caller` (the precedent is `src/cli.rs:103`).
  - A `2.0 * …` "one inset each side" term carries `// qual:allow(complexity, magic_numbers) reason: "an inset on each side"` (the precedent is `legend/mod.rs:86`), or uses a named constant.
  - If the score rises, run `just quality-baseline`; if it falls, find the cause.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `just lint` and `just ci` are clean at every commit.
- Commits: `type(scope): description`, ≤ 72 characters per line, trailer `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`; run `git-mit es` first.

## Review Focus

1. **A quadrant name too long for the narrow column.**
   - Up to 480 units the document widens with FR-94 intact.
   - Beyond 480 it widens further, and the caller can see `width > NARROW_MAX_WIDTH` for FR-95.
   - Nothing is clipped or overlaps.
   - Ordinary radars, including Thoughtworks' four quadrant names, keep 420.
   - *Tests: Task 3, `ordinary_radars_keep_the_narrow_width` and `narrow_widens_when_the_radar_cannot_fit`; Task 4, `blocks_do_not_overlap`.*
2. **A legend taller than the radar in wide.** The key stays under the radar column; neither the key nor its rule crosses the legend panel; the height comes from the legend's bottom. *Test: Task 3, `a_tall_legend_sets_the_wide_height`.*
3. **No title and no subtitle.** No space is reserved, and every block keeps `DOC_TOP` from the top edge (FR-65). *Test: Task 3, `no_title_reserves_nothing`.*
4. **Zero quadrants.** The legend is omitted with no column gap in wide and no key-to-legend gap in narrow (FR-42). *Test: Task 3, `an_empty_legend_takes_no_space`.*
5. **Nothing outside the document, nothing overlapping.**
   - Every node lies inside `width × height`.
   - The title, radar, key and legend panel are pairwise disjoint.
   - Fixtures:
     - `PRD_EXAMPLE`
     - a 200-character title
     - no title
     - a one-row legend
     - long names (`NAME_THREE_LINES`)
     - nine quadrants
     - the Thoughtworks quadrants
     - a widened-within-headroom set
     - a set widened beyond 480
     - an N = 4 radar wider than 700
   - *Tests: Task 4, `every_node_is_inside_the_document` and `blocks_do_not_overlap`.*

## Review rulings (plan review, 2026-10-11)

Five seats reviewed revision 1.0. The architecture, QA and Rust seats each measured the blocks with the real fonts in scratch copies and got the same numbers. Rulings:

- **The narrow fit (architecture, QA, Rust, a11y blockers).**
  - Revision 1.0 told the maintainer that widening needed "a quadrant name over roughly 360 units". That holds only for N = 1, where the name hangs at 6 o'clock. On the diagonals and at 3 and 9 o'clock a name pushes outward:
    - In a 380 column, widening started near 90 / 116 / 155 units for N = 2 / 3 / 4.
    - Thoughtworks' four names (Techniques, Tools, Platforms, Languages & Frameworks) measure 386.75 at s = 0.3 and widened the document to ≈ 427, where FR-94's 32-unit targets render at 23.995 px.
  - **The maintainer re-ruled with the measurements in hand: headroom, and a warning.** FR-94's targets are sized for a narrow document up to `NARROW_MAX_WIDTH` = 480:
    - `MIN_PX_PER_UNIT` = 320/480;
    - `LEGEND_ROW_MIN_HEIGHT` 32 → 36;
    - `HIT_RADIUS` 16 → 18.
  - So widening up to 480 costs nothing. Beyond 480, FR-94 and SC-6 fall short for that dataset, and **FR-95** warns once per render, naming the quadrant whose label is widest. The pipeline PR emits the warning, using `width > NARROW_MAX_WIDTH`; PR 12 adds no API for it.
  - The rejected options were: no headroom (D-71 as first ruled); shrinking quadrant labels in narrow; wrapping them; overflow; and a fatal error.
- **The radar's column (UI, my extension).**
  - The radar block sits at `RADAR_INSET` = `MARGIN − RADAR_LABEL_PAD` (10) in both arrangements, so its widest label's ink lands on the 20-unit margin. That is the rule the legend panel already follows: content at the margin, padding in it.
  - The narrow radar column is therefore `NARROW_RADAR_COLUMN` = `NARROW_WIDTH − 2·RADAR_INSET` = 400, not 380. Thoughtworks' four names (386.75 at 0.3) then fit at 420, and widening starts near 100 / 130 / 170 units for N = 2 / 3 / 4.
  - Revision 1.0's radar at `MARGIN` put label ink 30 from the edge, and the mockup's x = 0 put it 10 from the edge.
- **Wrong pins (architecture, QA, Rust blockers).**
  - `PRD_EXAMPLE`'s narrow width is set by its quadrant labels, not the 660·s floor. In the 380 column it was s ≈ 0.5164, not 380/660.
  - Pin the floor case with `plotted(HEAD, None)` (zero quadrants: the floor sets the width, so s = 400/660 exactly).
  - Pin `PRD_EXAMPLE` by property: it fits the column, and `s + 1e-6` does not.
- **The PRD carries D-71 everywhere (architecture, a11y).**
  - FR-70's lead sentence ("rather than the document being widened") is reworded.
  - D-31 gets "*Amended by D-71*".
  - FR-94, NFR-1's 2.5.8 claim, SC-6 and SC-17 are scoped to narrow documents up to 480 wide.
  - D-66's "24 → 32, about a third" becomes 24 → 36, half again.
  - D-69's "about 0.58" becomes "typically 0.3–0.6". At 0.3, markers grow up to 2.6× against the geometry.
  - Architecture:
    - §3 l.149 and §9 l.633 no longer say `Document` arrives in PR 13;
    - §4's "the narrow document always has room" applies to the legend only;
    - "Units become pixels" names the widened case.
  - `MIN_PX_PER_UNIT`'s doc is replaced, not appended to.
- **The breakpoint survives (architecture, QA).** The wide width is `r(1)` plus two insets plus the legend column. That is ≥ `r(0.3)` + 2·`RADAR_INSET`, the widest narrow document, because size is non-decreasing in `s`. So below the breakpoint the narrow document is never less legible than the wide one. Pinned by `the_wide_document_is_never_narrower`.
- **A top margin (UI).**
  - The PNG and SVG exports have no HTML frame. Without one the title's capitals sat about 10 from the top against 20 at the left and bottom, and with no title the wide legend panel sat 6 from the top.
  - `DOC_TOP` = 10 = `MARGIN − TITLE_TOP − (TITLE_FIRST_BASELINE − cap height)` rounded, with the cap height being Inter's ≈ 0.727 em × 17 ≈ 12.4.
  - Every block starts at `DOC_TOP`. It costs 10 units of height in both arrangements.
- **Derived spacings (UI).**
  - `KEY_TO_LEGEND` = `COLUMN_GAP − LEGEND_PANEL_INSET_X` (16): from a block's ink to the legend panel, the same distance as from the radar's ink to the panel in wide.
  - `LEGEND_WIDE_DROP` (6) clears the subtitle's descenders by about 20 below its baseline. It is deliberately not aligned to the radar's ring, which would add about 56 units to every tall-legend document.
  - The bottom margin is measured from the panel (20), the sides from content. One rule, written down.
- **Narrow centring dropped (architecture, QA, UI).**
  - `size` is continuous and non-decreasing, and every `RadarBlock` is ≥ 660 wide at s = 1. So the bisection always lands with the radar filling the column, or the document widens to it.
  - The offset was identically zero. The radar sits at `RADAR_INSET`.
- **IOSP (architecture).** `narrow` takes the scale as a parameter, and `placements` (the integration) computes it.
- **Tests (QA, a11y, Rust):**
  - literal pins replace formulas the implementation would share;
  - a radar wider than 700 in wide;
  - `blocks_do_not_overlap`;
  - `Linear` boundary tests for the bisection, including the one-ulp miss and a `to_bits` golden for NFR-4;
  - the narrow-width invariant;
  - `plotted(HEAD, None)` replaces `empty_catalogue()`, which has zero rings and is not a catalogue the pipeline produces;
  - a `blocks_of(&Catalogue, &Plotted, Titles)` helper.
- **Key at 320 px (a11y; corrects 1.0's reasoning).**
  - 1.0 said "SC-17 is about contrast and targets". It is wrong: SC-17 covers reflow (1.4.10) and resize (1.4.4) too.
  - The conclusion stands: WCAG AA has no minimum text size; 1.4.4 is met by browser zoom (desktop zoom switches to narrow; pinch zoom on a phone); and 1.4.10 is met because the SVG fits the viewport.
  - `KEY_LABEL` 11 renders at 11 × 320/420 ≈ 8.4 px, the same factor as the legend: `LEGEND_META` 10.5 renders 8.0 px.
  - PR 14 must not disable pinch zoom (see Finishing).
- **Document order (a11y endorsed):** title, radar, rule, key, legend. Each marker's accessible name carries its qualifier and status (FR-50), so a screen reader needs neither the key nor the legend first. In wide, the tab jump from the last marker to the first legend row crosses columns, which is reasonable.
- **Accepted as-is:**
  - the wide key rule spanning the radar column only;
  - `Blocks` as four `pub` fields without a constructor;
  - `Document { width, height, root }`;
  - no version bump.
- **Not adopted:**
  - a narrow-specific quadrant label scale, and wrapping quadrant labels (maintainer);
  - renaming `Placements` (Rust nit; it reads clearly where used).

## Decisions taken in this plan

All of these are named constants in `style.rs`, at scale 1. With T = the title block's height, `r` = the radar's size at its scale, `k` = `key.size(1)` and `l` = `legend.size(1)`:

- **Wide:**
  - The title is at (`MARGIN`, `DOC_TOP`).
  - The radar is at (`RADAR_INSET`, `DOC_TOP + T`), scale 1. Call its right edge `R` = `RADAR_INSET + r.width`; its widest label ink ends at `R − RADAR_LABEL_PAD`.
  - The key is at (`MARGIN`, `DOC_TOP + T + r.height`). The rule runs from (`MARGIN`, key y) to (`R − RADAR_LABEL_PAD`, key y).
  - When `l` is non-empty, the legend's content x is `R − RADAR_LABEL_PAD + COLUMN_GAP` and its panel is at (content x − `LEGEND_PANEL_INSET_X`, `DOC_TOP + T + LEGEND_WIDE_DROP`). Width = content x + (`l.width − 2·LEGEND_PANEL_INSET_X`) + `MARGIN`.
  - When `l` is empty, the legend is at (`R`, `DOC_TOP + T`) and draws nothing. Width = `R + RADAR_INSET`.
  - Height = max(key bottom, legend bottom) + `MARGIN`.
- **Narrow:**
  - s = `narrow_scale(&radar, NARROW_RADAR_COLUMN)`; width = max(`NARROW_WIDTH`, `r.width + 2·RADAR_INSET`).
  - The title is at (`MARGIN`, `DOC_TOP`).
  - The radar is at (`RADAR_INSET`, `DOC_TOP + T`), scale s.
  - The key is at (`MARGIN`, radar bottom). The rule runs from (`MARGIN`, key y) to (width − `MARGIN`, key y).
  - The legend is at (`MARGIN − LEGEND_PANEL_INSET_X`, key bottom + `KEY_TO_LEGEND`), or at the key's bottom when `l` is empty.
  - Height = (legend bottom, or key bottom when `l` is empty) + `MARGIN`.
- **The key's rule** is a `Line`, `KEY_RULE_STROKE` (1) wide, `StrokeDivider`, in a `Decorative` group drawn just before the key. The "Key" heading, not the rule, marks the boundary.
- **`svg::Document { width, height, root }`.** The viewBox origin is always (0, 0). Its docs state the invariant that `width` and `height` are finite and positive.
- **`Blocks`** has four `pub` fields, derives `Debug, Clone, PartialEq`, and has no constructor. The pipeline builds it once and passes it to both `arrange` calls (Finishing).
- **No version bump** in either PR: nothing a user sees until PR 13 and PR 14.

---

### Task 1 (PR 12a): PRD v0.54, FR-94 headroom, the arrangement constants, `svg::Document`

**Files:**

- `docs/product/radargen-prd.md`, `docs/design/architecture.md`
- `src/style.rs`, `src/svg/mod.rs`
- Tests whose literals move with the row height and hit radius: `src/layout/legend/{layout,row,mod}.rs` and `src/layout/radar/{mod,markers}.rs`.

**Interfaces:**

- Produces in `style`, all `f64`:
  - `NARROW_MAX_WIDTH = 480.0`
  - `DOC_TOP = 10.0`
  - `RADAR_INSET = MARGIN - RADAR_LABEL_PAD`
  - `NARROW_RADAR_COLUMN = NARROW_WIDTH - 2.0 * RADAR_INSET`
  - `LEGEND_WIDE_DROP = 6.0`
  - `KEY_TO_LEGEND = COLUMN_GAP - LEGEND_PANEL_INSET_X`
  - `KEY_RULE_STROKE = 1.0`
- Changes in `style`:
  - `MIN_PX_PER_UNIT = ((NARROW_MIN_VIEWPORT_PX - NARROW_HORIZONTAL_INSET_PX) / NARROW_MAX_WIDTH).min(MIN_LEGIBLE_PX / LEGEND_NAME.size)`, which is 0.6667;
  - `LEGEND_ROW_MIN_HEIGHT = 36.0`;
  - `HIT_RADIUS = 18.0`.
- Produces in `svg`: `Document { pub width: f64, pub height: f64, pub root: Node }` (`Debug, Clone, PartialEq`).

- [x] **Step 1: PRD v0.54.**
  - **D-71: the narrow fit.**
    - The rule: the largest `s` in [0.3, 1] fitting the 400-unit radar column. The radar sits at `MARGIN − RADAR_LABEL_PAD`, so its ink lands on the margin.
    - Widening: the document grows to the radar plus both insets.
    - FR-94's headroom to 480, and why 480: it covers the realistic sets measured in review, with the Infrastructure Platforms set at ≈ 458.
    - The measured thresholds by N, with Thoughtworks' four names fitting at 420. Verify the figures with a probe before writing them.
    - The cost beyond 480: px/unit at 320 px is 320/W; legend name text is 12.5 × 320/W px.
    - The rejected options and the reason for each:
      - no headroom: common datasets fail 2.5.8;
      - narrow label shrink and label wrapping: radar legibility and scope;
      - overflow: clips content;
      - a fatal error: refuses a valid CSV.
  - **FR-95**: modelled on FR-88. A narrow arrangement wider than 480 units emits one render-level warning naming the quadrant with the widest label, and says that on a 320 px screen legend rows, marker targets and text fall below FR-94's and SC-6's sizes. It renders normally (exit 0) and is emitted before the row warnings (NFR-11).
  - **FR-70:** reword the lead sentence to "scaled down within the document, which widens only when the radar cannot fit at 0.3 (D-71)".
  - **FR-94, NFR-1 (2.5.8), SC-17:** scope each to narrow documents up to `NARROW_MAX_WIDTH`. SC-17's audit includes one FR-95 fixture, recorded as the known exception. **SC-6** is not scoped (12a review): its claim is that text is not scaled with the radar, which holds at any width; it states the px instead (12.5 × 320/W).
  - **D-31:** add "*Amended by D-71*".
  - **D-66:** 24 → 36, half again.
  - **D-69:** the narrow scale is typically 0.3–0.6.
  - Add a revision row and bump `Version:`.
- [x] **Step 2: Architecture.**
  - `Implements:` v0.54.
  - §3 l.149 and §9 l.633: `Document` arrives in PR 12; PR 13 is the serialiser.
  - §4: the narrow scale rule; that the title, key and legend stay at scale 1; "always has room" means the legend; "Units become pixels" names D-71; and the breakpoint-survives argument.
  - §5: `Document { width, height, root }`.
  - Retitle crumb `rg-6ku` to "PR 13: svg serialiser, emit/write.rs" with `crumbs update rg-6ku --title`.
- [x] **Step 3: Failing tests and asserts.**
  - Update `style.rs`'s `MIN_PX_PER_UNIT` test from `320/420` to `320/480`. It fails, and so do the FR-94 const asserts once `MIN_PX_PER_UNIT` changes, until the row height and hit radius rise.
  - The FR-94 const asserts also check that the row height and hit radius are the least whole values that work, so oversizing fails the build as well (12a simplify).
- [x] **Step 4: Implement.**
  - Write the constants, each with a doc comment giving its derivation (for `DOC_TOP`, the cap-height arithmetic above).
  - Replace `MIN_PX_PER_UNIT`'s doc: it is the fewest px per unit at any narrow width up to `NARROW_MAX_WIDTH` or at the wide breakpoint. A D-71 document wider than that renders smaller, and FR-95 warns.
  - Name D-71 once, on `MIN_PX_PER_UNIT` (12a simplify).
  - Add `svg::Document`.
- [x] **Step 5: The ripple.** Rerun the legend and radar tests and update every literal that moves:
  - **One-line legend rows** are 36 tall: badge at the row's top + 18, first baseline at top + 18.5. Two-line rows stay 37.
  - **Pins in `legend/layout.rs`:**
    - `pins_for_the_prd_example`: row 58 / 36, badge y 76, name y 76.5.
    - `rows_of_one_two_and_three_lines`: tops 58 / 94 / 131, heights 36 / 37 / 52, baselines 76.5 / 106.5 / 143.5.
    - `groups_follow_fr10_…`: Adopt's baseline is 94 + 12 = 106.
    - `empty_rings_and_unplotted_rows`: content height 58 + 36 + 36 = 130.
  - **Pins in `legend/row.rs`:** (36, 18.5), and the inline meta at y 18.5.
  - **Radar:** pins that mention 16 or a hit-limited size. `size_is_non_decreasing`'s comment ("near 0.135") moves.
  - Derive each new literal by hand in the test's comment, as before. Do not copy what the run prints.
- [x] **Step 6:** `cargo fmt`, clippy, `just lint`, `just ci`; `yq --front-matter=extract '.' docs/plans/pr-12-arrange.md`.
- [x] **Step 7: Commits:**
  - `docs(prd): v0.54 — the narrow fit (D-71, FR-95)`
  - `feat(layout): FR-94 headroom for a narrow document up to 480 (D-71)`
  - `feat(svg): Document; the arrangement constants`
  - Then open PR 12a.

---

### Task 2 (PR 12b): the types and `narrow_scale`

**Files:** Create `src/layout/arrange.rs`. Modify `src/layout/mod.rs` (`pub mod arrange;`).

**Interfaces:**

- Consumes: `layout::block::{Block, Placement, Size}`; the four block types; Task 1's `NARROW_MAX_WIDTH`.
- Adds to `style` (deferred from Task 1 by ledger ruling: unused until here): `NARROW_MIN_SCALE = 0.3`, with a named constant for the radar's monotone bound (0.0034, from `half_extent`'s doc) and a const assert that `NARROW_MIN_SCALE` lies above it and at most 1, linked from `QuadrantLabel::push`'s doc; `RADAR_INSET`, `NARROW_RADAR_COLUMN`, `DOC_TOP`, `LEGEND_WIDE_DROP`, `KEY_TO_LEGEND`, `KEY_RULE_STROKE` as Task 1's Interfaces describe them. `svg::Document` comes with Task 4.
- Produces (docs abbreviated here; every item and field gets one in the code):

```rust
/// The four blocks, laid out once (FR-68).
#[derive(Debug, Clone, PartialEq)]
pub struct Blocks {
    /// The title block (FR-65).
    pub title: TitleBlock,
    /// The radar (FR-30–FR-36).
    pub radar: RadarBlock,
    /// The key (FR-66).
    pub key: KeyBlock,
    /// The legend (FR-35).
    pub legend: LegendBlock,
}

/// Which way the blocks are placed (FR-68).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrangement {
    /// The legend beside the radar, everything at scale 1.
    Wide,
    /// The legend below the key, the radar scaled to fit (FR-70, D-71).
    Narrow,
}

/// The key's top rule, the arrangement's (PR 09).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rule { pub from: Point, pub to: Point }

/// Where everything goes, before anything is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placements {
    pub width: f64, pub height: f64,
    pub title: Placement, pub radar: Placement,
    pub rule: Rule, pub key: Placement, pub legend: Placement,
}

/// Halvings of [NARROW_MIN_SCALE, 1]: past f64 resolution after ~53; the
/// count is fixed so every machine gets the same bits (NFR-4).
const BISECTIONS: u32 = 60;

/// The largest `s` in [`NARROW_MIN_SCALE`, 1] at which `radar` is at most
/// `column` wide, or `NARROW_MIN_SCALE` when none is (D-71). Only values
/// that passed the test are returned, so the result always fits when
/// anything does; it is the largest because the radar's size is
/// non-decreasing in `s`. A `RadarBlock` is at least `MIN_RADAR_WIDTH` wide
/// at 1, so the first return serves blocks that fit at natural size.
fn narrow_scale(radar: &impl Block, column: f64) -> f64 {
    let fits = |scale: f64| radar.size(scale).width <= column;
    if fits(1.0) {
        return 1.0;
    }
    if !fits(NARROW_MIN_SCALE) {
        return NARROW_MIN_SCALE;
    }
    let (mut lo, mut hi) = (NARROW_MIN_SCALE, 1.0);
    for _ in 0..BISECTIONS {
        let mid = f64::midpoint(lo, hi);
        if fits(mid) { lo = mid } else { hi = mid }
    }
    lo
}
```

- [x] **Step 1: Failing tests.** `Linear(f64)` is a test-only `Block` whose size is `w · s` square and which renders an empty group. Use `close` throughout.
  - `narrow_scale_finds_the_largest_fit`: `close(narrow_scale(&Linear(1000.0), 400.0), 0.4)`. For each column in [300, 333.3, 399.999], `Linear(1000).size(s).width <= column` by the same comparison.
  - `narrow_scale_boundaries`:
    - `Linear(400)` against 400 gives 1, inclusively.
    - `Linear(300)` gives 1 (never enlarges).
    - With `at_floor = Linear(2000).size(NARROW_MIN_SCALE).width`, column `at_floor` gives `NARROW_MIN_SCALE`, and column `f64::from_bits(at_floor.to_bits() - 1)` gives `NARROW_MIN_SCALE` too (misses by one ulp).
  - `narrow_scale_is_bit_stable` (NFR-4): `narrow_scale(&Linear(1000.0), 400.0).to_bits()` equals a golden value recorded once. Its comment says it guards the fixed count and the plain arithmetic.
  - `narrow_scale_of_the_radar`:
    - `plotted(HEAD, None)` has zero quadrants, so the floor sets the width: `close(s, 400.0 / 660.0)`.
    - `PRD_EXAMPLE` is set by its labels: `close(radar.size(s).width, 400.0)` and `radar.size(s + 1e-6).width > 400.0`. The second assertion is valid only off a plateau; say so.
- [x] **Step 2–4:** red, implement, green; lints; rustqual.
- [x] **Step 5: Commit** `feat(layout): the arrangement types and the narrow scale (D-71)`.

---

### Task 3 (PR 12b): `placements`

**Files:** `src/layout/arrange.rs`.

**Interfaces:**

- Consumes: Task 2.
- Produces:
  - `#[must_use] pub fn placements(arrangement: Arrangement, blocks: &Blocks) -> Placements`, an integration;
  - `fn wide(blocks: &Blocks) -> Placements` and `fn narrow(blocks: &Blocks, scale: f64) -> Placements`, operations implementing the Decisions' formulas.

**Test fixtures** (local). `Titles { title: Option<&str>, subtitle: Option<&str> }`; `blocks_of(&Catalogue, &Plotted, Titles) -> Blocks`; `blocks(csv, Titles)` wraps `plotted(csv, None)`. Named CSVs:

| Name | What it holds |
|---|---|
| `THOUGHTWORKS` | Techniques, Tools, Platforms, Languages & Frameworks: one item each |
| `WIDENED` | Infrastructure Platforms, Tools, Practice |
| `BEYOND` | one quadrant with a 70-character name |
| `WIDE_RADAR` | N = 4, one name of 40+ characters (radar > 700 at 1) |
| `TALL` | 30 items in one quadrant |
| `ONE_ROW` | one item |
| `NINE` | quadrants Q0–Q8 |

`TITLED` is "Technology Radar" + "Managed estates, 2026", T = 53 (`TITLE_TOP` 4 + baseline 18 + `SUBTITLE_AFTER_TITLE` 17 + `TITLE_BOTTOM` 14).

- [x] **Step 1: Failing tests.** Pin literals where the arithmetic is independent of the code; read block sizes only where a block's own tests pin them.
  - `wide_places_the_legend_beside_the_radar` (`PRD_EXAMPLE`, `TITLED`):
    - title (20, 10); radar (10, 63), scale 1; key (20, 723), because the radar is 660 at 1 (radar's `size_pins`);
    - rule (20, 723) → (660, 723);
    - legend panel (676, 69), because content x = 670 − 10 + 28 = 688;
    - width = 688 + (`l.width` − 24) + 20 (`l` from the block; message prints the literal);
    - height = 723 + 146 + 20 = 889, because the key is 146 tall (QA's measurement; pin it from `KeyBlock`'s own test). Assert `legend bottom + 20 < 889` (the shorter-legend branch).
  - `narrow_stacks_and_scales_the_radar` (`PRD_EXAMPLE`, `TITLED`):
    - width 420; radar at (10, 63) with `close(r.width, 400.0)`;
    - key (20, 63 + r.height); legend (8, key bottom + 16); title, key and legend at scale 1;
    - height = legend bottom + 20; rule (20 → 400).
  - `ordinary_radars_keep_the_narrow_width`: `PRD_EXAMPLE`, `THOUGHTWORKS`, `NINE`, `TALL`, a 200-character title and `NAME_THREE_LINES` all keep width 420. This guards FR-94's premise against a regression in the radar's size.
  - `narrow_widens_when_the_radar_cannot_fit`:
    - `WIDENED`: scale `close(NARROW_MIN_SCALE)`; width `close(r.width + 20)`, > 420 and ≤ `NARROW_MAX_WIDTH`; radar at x 10.
    - `BEYOND`: width > `NARROW_MAX_WIDTH`.
    - Both: title (20, 10), key x 20, legend x 8, and the rule to `close(width − 20)`.
  - `narrow_width_invariant`: over every fixture, width is 420 when `r.width <= 400` (the same comparison) and `close(r.width + 20)` otherwise.
  - `wide_follows_a_wide_radar` (`WIDE_RADAR`): `r.width > 700`; the rule ends at `r.width`; the legend panel x is `r.width + 28 − 12`.
  - `a_tall_legend_sets_the_wide_height` (`TALL`, `TITLED`): legend bottom > key bottom; height = 10 + 53 + 6 + `l.height` + 20; the key's right edge (20 + `k.width`) and the rule's end + half its stroke are < the legend panel's x.
  - `no_title_reserves_nothing`:
    - With no titles, T = 0: title (20, 10), radar y 10 and the wide legend panel y 16, in both arrangements.
    - `title.render(p.title)` has no bounds.
    - A subtitle-only case gives T > 0.
  - `an_empty_legend_takes_no_space` (`plotted(HEAD, None)`, no titles):
    - wide width 680 (10 + 660 + 10);
    - height = key bottom + 20 in both arrangements, with no `KEY_TO_LEGEND` term;
    - narrow scale `close(400/660)`.
  - `the_wide_document_is_never_narrower`: over every fixture, wide width ≥ narrow width and > `NARROW_WIDTH`.
  - `columns_hold_their_blocks`: over every fixture:
    - the title and key widths are ≤ 380 and ≤ the radar's ink width (`r.width − 2·RADAR_LABEL_PAD`);
    - the legend content (`l.width − 24`) is ≤ 380, so the narrow panel's right edge is ≤ 412 < 420.
- [x] **Step 2–4:** red, implement, green; lints; rustqual.
- [x] **Step 5: Commit** `feat(layout): place the blocks wide and narrow (FR-68, FR-70, D-71)`.

---

### Task 4 (PR 12b): `arrange`, rendering a `Document`

**Files:** `src/layout/arrange.rs`.

**Interfaces:**

- Consumes: Task 3; `svg::{Document, Group, Node, Paint, Point, Semantics}`; `palette::{Neutral, Role}`.
- Produces: `#[must_use] pub fn arrange(arrangement: Arrangement, blocks: &Blocks) -> Document` (with `// qual:api`).
  - `root` = `Node::Group(Group::new(Point::ORIGIN, vec![title, radar, rule, key, legend]))`.
  - The rule = `Node::Group(Group::new(Point::ORIGIN, vec![Node::Line { from, to, stroke: Paint::Role(Role::Neutral(Neutral::StrokeDivider)), stroke_width: KEY_RULE_STROKE }]).with(Semantics::Decorative))`.

- [x] **Step 1: Failing tests:**
  - `blocks_render_at_their_placements` (both arrangements): destructure `root`'s children as `[title, radar, rule, key, legend]`. Each block child equals `block.render(p.<block>)` exactly. The rule is a `Decorative` group with one `Line` from `p.rule.from` to `p.rule.to`.
  - `both_arrangements_hold_the_same_controls` (FR-68): collect the `Semantics::Marker` and `Semantics::LegendRow` controls in tree order from each document. The key sequences are equal, and every marker precedes the first legend row. PR 14's focus hand-over relies on this mapping.
  - `the_document_is_the_placements_size`.
  - `every_node_is_inside_the_document` (Review Focus 5's fixtures, both arrangements): `assert_inside(&doc.root, Size { width, height }, fonts())` and `assert_finite`.
  - `blocks_do_not_overlap` (the same fixtures, both arrangements), using `test_support::bounds` on each child:
    - the title is above the radar, and the radar above the key (edges may touch);
    - wide: the radar's and key's right edges, and the rule's end + half its stroke, are < the legend panel's left;
    - narrow: the key's bottom is < the legend panel's top.
- [x] **Step 2–4:** red, implement, green; lints; rustqual.
- [x] **Step 5: Gate** `just ci`.
- [x] **Step 6: Commit** `feat(layout): arrange the blocks into a document (FR-68, FR-71)`.

---

## Finishing

1. **Reviews**, for each PR: a final whole-branch review on the most capable model in Copilot's seat (Copilot is out of credits until November 2026; say so), then simplify, then a final check.
2. **Crumbs:** `crumbs close rg-ozb` with PR 12b. Create:
   - "Emit FR-95 when the narrow document passes 480" (pipeline PR): name the quadrant with the widest measured label.
   - "The narrow legend panel spans the column" (UI idea): a short-named legend's panel is narrower than the rule above it. Arrange cannot stretch it, because it must not re-lay-out a block.
3. **Notes:**
   - `rg-6ku` (PR 13):
     - serialise `Document` as `viewBox="0 0 {width} {height}"`;
     - the key's rule is `<line>` with `aria-hidden`;
     - an empty title block's group may be dropped;
     - give the radar group an accessible name (`role="group"` plus `aria-label`), so heading navigation can reach the markers.
   - `rg-f27` (PR 14):
     - `main.rs` builds **one** `Blocks` (title and subtitle after FR-92's expansion; radar, key and legend from the same `Catalogue`/`Plotted`/`Fonts`) and calls `arrange` twice on it. If a constructor is wanted, bundle the titles into one parameter.
     - The media query is `breakpoint_px(wide.width)`. Both documents are emitted and `display: none` hides one (FR-69).
     - Keep the narrow SVG spanning the viewport (`NARROW_HORIZONTAL_INSET_PX`), sized relative to it, never in fixed px.
     - The viewport meta must not disable zoom (no `user-scalable=no` or `maximum-scale=1`): pinch zoom is how 1.4.4 is met on a phone.
     - On a media-query change, move focus to the control with the same `ItemKey` in the newly visible document and carry `aria-expanded` over (2.4.3, 2.4.7).
     - Consider a "skip to legend" link.
     - Supply an H1 when the title is omitted.
     - Check the 1-unit key rule at 320 px (≈ 0.67 px).
   - `rg-42m` and `rg-m2l` (PR 16/17): export `arrange(Wide, ..)` only (FR-71).
4. Move this plan to `docs/plans/done/`, tick the steps, and set `status: complete` and `completed_date`.
5. **No version bump** in either PR. For each: commit, push, open the PR, check CI. Merge when clean if the maintainer authorises it.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-11 | Initial version, for the plan review. D-71 ruled by the maintainer on a wrong premise (widening only past ~360 units). |
| 1.1 | 2026-10-11 | The five-seat review: measured widening thresholds; the maintainer re-ruled (FR-94 headroom to 480, FR-95 warns beyond); radar at `RADAR_INSET` with a 400 column; `DOC_TOP`; derived spacings; centring dropped; IOSP split; corrected pins and fuller tests; the PRD carries D-71 through FR-94, NFR-1, SC-6, SC-17, D-31, D-66, D-69; split into PR 12a and PR 12b. |
| 1.2 | 2026-10-11 | PR 12a review: SC-6 is not scoped to 480 (its claim holds at any width; it states the px); FR-95 names the quadrant whose label sets the radar's width; `svg::Document` and the arrangement-only constants move to PR 12b. |
| 1.3 | 2026-10-11 | PR 12a shipped: Task 1 ticked; `NARROW_MIN_SCALE` and the arrangement constants listed under Task 2, where they first have a user; Task 1's asserts and D-71 doc placement as shipped. |
| 1.4 | 2026-10-11 | PR 12b shipped (Tasks 2–4). As built: Task 2 committed with Task 3 (`narrow_scale` has no caller until `placements`); `LEGEND_PANEL_INSETS_X` and `RADAR_INSETS` name the two-inset terms (a `qual:allow` marker did not suppress); the rustqual baseline's stored score is one digit short as a float read-back workaround (crumb `rg-ksg`); the simplify pass kept local test helpers because sharing them pushed `test_support` past rustqual's module limit (crumb `rg-aea`). Follow-ups: `rg-pti` (emit FR-95), `rg-7gi` (narrow legend panel width). |
