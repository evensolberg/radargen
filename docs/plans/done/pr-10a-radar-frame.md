---
tags:
  - plan
  - layout
  - radar
aliases: []
doc_id: RG-010A
doc_name: pr-10a-radar-frame
crumb_id: rg-svr
document_title: "PR 10a — the radar block's frame: rings, dividers, labels, and FR-70's three scales: Implementation Plan"
created_date: 2026-10-11
synopsis: "The radar block without markers: ring circles, quadrant dividers, ring labels over a halo, quadrant labels, the empty and one-quadrant cases, and FR-70's narrow scaling made explicit — geometry by the radar's scale, labels never below natural size, marks and strokes never below 7/9 (PRD v0.50, rg-6w7)."
status: done
type: Plan
revision: "1.1"
review_date: 2026-10-11
reviewed_by: [rust-engineer, architect-reviewer, qa-expert, ui-designer, accessibility-tester]
completed_date: 2026-10-11
comments: "1.1 folds in the five-seat plan review; see Review rulings."
---
# PR 10a — the radar block's frame: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Crumbs:** `rg-svr` (part 1 of 2; the maintainer split PR 10, 2026-10-11), `rg-6w7` (decided: the floor) · **Phase:** v1.0 · **Then:** PR 10b (markers)

**Goal:** The radar's frame, drawn from the catalogue: a ring circle per declared ring, a divider per quadrant, every ring's name at 12 o'clock over a halo, every quadrant's name outside its sector. Correct with zero quadrants (rings only, FR-42), one (FR-43), and any number above; its names stay at natural size in the narrow arrangement (FR-70).

**Architecture:** A `RadarBlock` in `src/layout/radar.rs`, built from `(&Catalogue, &Plotted, &Fonts)`, implementing PR 09's `Block`. Its `size` is not linear in scale (architecture §4) because the radar has **three scales**, one rule each, held in a `RadarScale` value: **geometry** (radii, positions, line ends) by `s`; **labels** (ring and quadrant names, their halo, the label pad) by `max(s, 1)` — FR-70's counter-scaling, as already written; **marks** (grid strokes now; markers and their numbers in PR 10b) by `max(s, RADAR_FLOOR)` with `RADAR_FLOOR` = 7/9 — the maintainer's decision on `rg-6w7` (2026-10-11), written into FR-70 as PRD v0.50. Primitives take a two-factor `svg::Scale { position, size }`, so a position and a size can scale differently.

**Tech Stack:** Rust 2024, `std` only. `ttf-parser` is already a dev-dependency. No new dependency.

**Spec:** `docs/product/radargen-prd.md` v0.49 → v0.50 — FR-10, FR-19, FR-30–FR-32, FR-42, FR-43, FR-63, FR-70, FR-77, FR-79, FR-84, FR-89, FR-94, D-31, D-69 (new). Design: `docs/design/architecture.md` §3–§5. Reference: `docs/product/mockup.html` `buildRadar` (frame part, l.584–613) and CSS l.126–144.

## Global Constraints

- Lints as strict as `Cargo.toml` says (pedantic, nursery, cargo, `missing_docs`, `indexing_slicing`, `as_conversions`, `unwrap_used`, `float_cmp` …); suppress only with `#[expect(lint, reason)]` whose reason is true; rustqual markers likewise; never reshape code to hide from a lint or rustqual; no silent failures.
- **Scale is applied to coordinates, never as an SVG transform** (architecture §4). `Group` only translates.
- **No hex in a `Node`**; colour is a `Role` or `Paint::None` (architecture §5).
- **Quadrant order is FR-10's** (`Plotted::quadrants`), never `Catalogue::quadrants` order.
- **Labels never wrap or truncate** (FR-79): a long name widens the block.
- **Every advance width comes from `text::Fonts`** (FR-84); vertical text extents come from the pinned `text::TEXT_ASCENT` / `TEXT_DESCENT` (Task 2).
- **Floating-point arithmetic stays `*`/`+`**, with `#[expect(clippy::suboptimal_flops, reason = "NFR-4: …")]` as in `geometry.rs`, never `mul_add` (different bits without FMA). Halves are `f64::midpoint`, not `/ 2.0` (rustqual MAGIC_NUMBER).
- **Tests compare floats with `block::close`** (or a `near(Point, Point)` built on it), never `assert_eq!` on `f64`.
- `cargo fmt`; `cargo clippy --all-targets -- -D warnings`, `just lint`, `just ci` clean at every commit. If rustqual's score rises, `just quality-baseline`; if it falls, find the cause and fix the structure honestly. Helpers take at most 4 parameters (bundle into a struct).
- Commits: `type(scope): description`, ≤ 72 per line, trailer `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`; `git-mit es` first; verify `git log -1 --format=%B`. Implementers and reviewers never run a git command that writes outside their own worktree branch.

## Review Focus

1. **A CSV with only a header** (zero quadrants). Ring circles and ring labels, no dividers, no quadrant labels, no NaN anywhere in the tree (FR-42). *Test: Task 4, `zero_quadrants_draws_rings_only`.*
2. **One quadrant.** One divider at 12 o'clock, its label at 6 o'clock (FR-43). *Test: Task 4, `one_quadrant_has_one_divider`.*
3. **A declared ring nobody uses.** Its circle and label still draw (FR-19). *Test: Task 4, `every_declared_ring_draws`.*
4. **A very long quadrant or ring name.** It does not wrap and stays inside the block (FR-79). *Tests: Task 4, `a_long_quadrant_label_widens_the_block`, `a_long_ring_name_widens_the_block`.*
5. **The narrow arrangement.** Geometry shrinks; names stay at natural size; grid strokes stop at 7/9 (FR-70 v0.50). *Test: Task 4, `three_scales`.*
6. **The ring name over the 12 o'clock divider.** A halo of `Surface1` sits under each ring name so the divider does not strike through it; the halo is hidden from assistive technology by construction and the name itself is not, so it is read once (FR-31). *Test: Task 4, `ring_labels_sit_on_a_halo`.*

## Crumbs this plan incorporates

| Crumb | What it asked | Where |
| --- | --- | --- |
| `rg-svr` | Sectors, bands, labels, FR-70 counter-scaled labels | Tasks 2–4 (markers: PR 10b) |
| `rg-svr` (PR 08 review) | `Geometry::middle`; zero quadrants → rings only; N = 1 | Task 2; Task 4 (dividers are lines and rings are circles, so no sector arc is drawn and N = 1 needs no special case) |
| `rg-svr` (PR 09) | `Node` variants when needed: `Line`, a text halo | Task 2 |
| `rg-6w7` | Marker numbers below legibility in narrow | Task 1 (PRD v0.50, D-69), applied to markers in PR 10b |

## Review rulings (plan review, 2026-10-11)

Five seats reviewed revision 1.0; their reports are in the session scratchpad. Rulings:

- **Labels keep FR-70's counter-scaling; the floor covers marks only.** Revision 1.0 extended the maintainer's 7/9 floor from markers to labels. UI and accessibility showed ring names would fall to 7.8 units, 15–24 % under the mockup, in the weakest ink. The extension was ours, not the maintainer's, so it is withdrawn: labels scale by `max(s, 1)` as FR-70 already says, and only markers, numbers and grid strokes take the floor. This also leaves FR-94 and D-31 consistent without rewording.
- **Two-factor scale** (rust, architect, QA blocker): `svg::Scale { position, size }`; `TextRun::node(Scale)`. `Glyph::draw` moves to `Scale` in PR 10b, where markers first need it.
- **The halo is a paint, not a group** (all five): `Node::Text`'s `ink` becomes `TextPaint::{Fill(Paint), Halo { stroke, width }}`. A halo has no fill, a round line join and is hidden from assistive technology, by construction; PR 13's emitter writes `fill="none" stroke-linejoin="round" aria-hidden="true"`. The ring name itself is a plain, exposed text.
- **`TextRun` gains `anchor`.**
- **Pad inside, minimum box square**: half-extent per axis = `max(MIN_RADAR_WIDTH/2 · s, label extents + RADAR_LABEL_PAD · r.labels)`, so short labels at `s` = 1 give exactly 660 × 660 (the mockup's width; its fixed 650 height was a constant).
- **Ring labels count in the size**, halo included.
- **Monotone means non-decreasing.** Below some `s` a label term can dominate and the size plateaus; PR 12's narrow solve must handle "no scale fits" (note on `rg-ozb`).
- **Bounds helper** becomes `layout::test_support` (Task 3): anchor- and stroke-aware, `Line` arm, hhea metrics; key and title tests move onto it unchanged.
- **Accessible source for names**: the radar draws ring and quadrant names as plain text; the legend (FR-35) and markers' accessible names (FR-50, PR 10b) carry them for assistive technology. No headings in the radar. The radar's root is not `role="img"` (PR 13 note).
- **Contrast of the ring name** is measured against its halo's `Surface1`, the ground it is actually read on (SC-20); no test covers that pair today, so Task 4 adds `ring_names_clear_4_5_on_their_halo` in `palette.rs`'s tests: `contrast(TextMuted, Surface1) ≥ 4.5` in both themes (reviewers measured 6.4 and 8.0). **Find-in-page** matches a ring name twice in HTML (halo and name); accepted, as in the mockup.
- **Out of scope, crumbed:** long quadrant labels crossing the outer ring at 3/9 o'clock and diagonals (mockup-identical; new crumb); SC-17 contrast if quadrants are ever tinted (from `rg-6w7`; new crumb).

## Decisions taken in this plan (flag on review)

- **Quadrant labels**: middle-anchored, baseline on the label point, at `polar(middle(p), QUADRANT_LABEL_RADIUS)` in geometry space, `QUADRANT_LABEL`, ink `TextPrimary` — mockup-identical, including the 6 o'clock label sitting ~29 units from the outer ring against ~40 at 12.
- **Ring labels**: middle-anchored at `(0, −midpoint(inner, outer))`, `RING_LABEL`, ink `TextMuted`, after a `Halo` copy of `LABEL_HALO` in `Surface1`.
- **Draw order**: ring circles, dividers, quadrant labels, ring labels (each halo just before its name). PR 10b's markers draw after all of these.
- **The frame draws inside an inner `Group` at the centre `C`**, so every child is centre-relative and `Point` needs no `Add`.
- **`TEXT_ASCENT` = 1984/2048, `TEXT_DESCENT` = 494/2048** (Inter `hhea`, both faces, line gap 0), in `text.rs`, pinned by a test. They bound the line box, not ink: stacked Vietnamese capitals may exceed the ascent, inside the pad.
- **No version bump** (nothing user-observable yet).

---

### Task 1: PRD v0.50 — the radar's marks floor (rg-6w7)

**Files:** `docs/product/radargen-prd.md`, `docs/design/architecture.md`

- [x] **Step 1: FR-70.** After "Radar-internal labels are counter-scaled so ring and quadrant names survive the reduction." add: "Markers, their numbers and the radar's strokes scale with the radar but **never below 7/9 of their natural size**, so a 9-unit marker number draws at no less than 7 units while the radar's geometry keeps shrinking (D-69). FR-94's hit area is not floored; it is counter-scaled like the labels."
- [x] **Step 2: FR-89.** Add: "Capacity is evaluated at the wide arrangement's scale; in the narrow arrangement markers are relatively larger (D-69), so a band can overlap there before it reaches capacity."
- [x] **Step 3: D-69** after D-68 — *The radar's marks floor* (FR-70, FR-89, FR-94; crumb `rg-6w7`). Text: the narrow arrangement scales the radar to about 0.58 (380 of 660 units), which shrank a 9-unit marker number to about 5.2 units, roughly 4 px on a 320 px screen, though the number is a marker's only link to its legend row. Three options were weighed: scale markers with the radar (numbers illegible); counter-scale them like the labels (legible, but markers grow 1.74× against the geometry and a band holds far fewer before overlapping); or a floor. The maintainer chose the floor at 7/9: numbers draw at 7 units (about 5.3 px at 320 px, 6.5 px at 390 px), and markers grow 1.35× against the geometry at `s` = 0.576 — about a quarter fewer per band before overlap — rising as `s` falls. Labels are unaffected: FR-70 already keeps them at natural size. FR-69's breakpoint is unchanged: it is set by the legend's 12.5 px (SC-6), and at the breakpoint markers are at scale 1. FR-89's capacity stays a wide-arrangement figure. OQ-16's output-pixel marker remains a v1.1 question.
- [x] **Step 4:** Annotate D-31: "*Amended by D-69:* marker numbers are held at 7/9 of their size, since scaling them with the radar left them illegible." Revision row 0.50; bump `Version:`.
- [x] **Step 5: Architecture.** `Implements:` v0.50. §3's `radar.rs` row: "FR-70's three scales". §4's radar paragraph: the radar is not `size(1)·k` because it has three scales — geometry by `s`, labels by `max(s, 1)`, marks by `max(s, 7/9)` (D-69). §5's `Node` listing: add `Line`, and `Text`'s paint as `TextPaint::{Fill, Halo}` (a halo: no fill, round join, hidden from assistive technology).
- [x] **Step 6:** Commit `docs(prd): v0.50 — the radar's marks floor (D-69, rg-6w7)`.

---

### Task 2: primitives — `Scale`, `TextPaint`, `Line`, `TextRun` anchor and halo, `Geometry::middle`, text metrics

**Files:** `src/svg/mod.rs`, `src/text.rs`, `src/layout/geometry.rs`, `src/layout/title.rs` and `src/layout/key.rs` (call sites only)

**Interfaces:**

```rust
// svg
/// How far positions and sizes are multiplied; they differ inside the radar (FR-70).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale { pub position: f64, pub size: f64 }
impl Scale { pub const fn uniform(k: f64) -> Self }

/// How a text is painted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextPaint {
    /// Glyphs filled; no outline.
    Fill(Paint),
    /// An outline only, drawn under a filled copy so lines behind the text
    /// stop short of it. No fill, a round line join, and hidden from
    /// assistive technology: the filled copy is the one that is read.
    Halo { stroke: Paint, width: f64 },
}

Node::Text { origin, style, ink: TextPaint, anchor, lines, leading }
Node::Line { from: Point, to: Point, stroke: Paint, stroke_width: f64 }

pub struct TextRun { origin, style, ink: Neutral, anchor: Anchor, lines, leading }
impl TextRun {
    pub fn node(&self, scale: Scale) -> Node          // origin × position; style, leading × size
    pub fn halo(&self, width: f64, scale: Scale) -> Node  // ink Halo { Role(Neutral(Surface1)), width × size }
}

// text
pub const TEXT_ASCENT: f64 = 1984.0 / 2048.0;   // Inter hhea ascender / upem, both faces
pub const TEXT_DESCENT: f64 = 494.0 / 2048.0;   // −descender / upem; line gap 0

// geometry
impl Geometry {
    /// The angle halfway across `position`'s sector, in degrees. NaN with no quadrants, like `divider`.
    pub fn middle(self, position: usize) -> f64    // f64::midpoint(divider(p), divider(p + 1))
}
```

Title and key build `TextRun { …, anchor: Anchor::Start, … }` and call `node(Scale::uniform(at.scale))`; their output is unchanged.

- [x] **Step 1: Failing tests:**
  - `scale_splits_position_from_size`: a `TextRun` at origin (10, 20), size 12, spacing 0.48, leading 15, anchor `Middle`, `node(Scale { position: 0.5, size: 1.0 })` → origin (5, 10), size 12, spacing 0.48, leading 15, anchor `Middle`; with `Scale::uniform(2.0)` everything doubles.
  - `halo_is_an_outline_of_the_same_text`: `halo(3.0, Scale { position: 0.5, size: 2.0 })` has the same origin, style, anchor, lines and leading as `node` with that scale, and `ink == TextPaint::Halo { stroke: Paint::Role(Role::Neutral(Neutral::Surface1)), width: 6.0 }` (width via `close`).
  - `ordinary_text_is_filled`: `node` gives `TextPaint::Fill(Paint::Role(Role::Neutral(ink)))`.
  - `middle_is_halfway_across_the_sector`: N = 1 → 90; N = 2 → 0, 180; N = 3 → −30, 90, 210; N = 4 → −45, 45, 135, 225; at N = 4 also `close(middle(p), angle(p, 0, 1))`; N = 0 → `is_nan()`.
  - `text_metrics_match_the_vendored_faces` (in `text.rs` tests): for both `assets/Inter-Regular.ttf` and `Inter-SemiBold.ttf` via `ttf_parser::Face`, the integers `units_per_em() == 2048`, `ascender() == 1984`, `descender() == -494`, `line_gap() == 0` (integers, so no `float_cmp`), and the constants are documented as those ratios.
  - Title and key tests pass unchanged. The svg module's own `TextRun` scale test moves to `Scale::uniform`, and `TextRun`'s "start-anchored" doc comment changes to describe `anchor`.
- [x] **Step 2–4:** red, implement, green; lints; rustqual.
- [x] **Step 5:** Commit `feat(svg): two-factor Scale, a text halo, Line, Geometry::middle`.

---

### Task 3: `layout::test_support` — one bounds walker, one fixture builder

**Files:** Create `src/layout/test_support.rs` (`#[cfg(test)]`, declared `#[cfg(test)] pub(crate) mod test_support;` in `layout/mod.rs`); modify `src/layout/key.rs` and `title.rs` tests to use it; remove key.rs's private `Frame`, `assert_arcs_inside`, `assert_text_inside`, `assert_inside`.

**Interfaces:**

```rust
/// The smallest box holding everything `node` paints, in its parent's coordinates.
pub(crate) fn bounds(node: &Node, fonts: &Fonts) -> Option<Rect>   // None for nothing painted
pub(crate) struct Rect { pub left: f64, pub top: f64, pub right: f64, pub bottom: f64 }
/// Panics naming the first node outside `size`.
pub(crate) fn assert_inside(node: &Node, size: Size, fonts: &Fonts)
/// Every f64 in the tree is finite.
pub(crate) fn assert_finite(node: &Node)
/// A catalogue and its FR-10 order from CSV text and an optional `--rings` value.
pub(crate) fn plotted(csv: &str, rings: Option<&str>) -> (Catalogue, Plotted)
```

`bounds` per variant: `Group` → children's union, offset added; `Circle` → centre ± (radius + stroke_width/2); `Line` → both ends ± stroke_width/2; `Path` → as key.rs's arc walker does today (half-circle arcs; keep its assertion), ± stroke_width/2; `Text` → per line, width measured with the node's style (letter-spacing included); horizontal extent by anchor (`Start`: x..x+w; `Middle`: x−w/2..x+w/2); vertical baseline − `TEXT_ASCENT`·size to baseline + `TEXT_DESCENT`·size, baseline stepping by `leading`; a `Halo` widens all four sides by width/2. `plotted` is order.rs's `catalogue_with` (parse, `parse_rings`, `build_catalogue`, each `expect`ed) plus `order::resolve`.

- [x] **Step 1:** Move and extend; switch key and title tests to it. Add tests of the walker itself: a middle-anchored text of known width is centred; a halo widens by width/2; a `Line` from (0,0) to (10,0) of width 2 is (−1,−1,11,1); a `Group` offset shifts the box.
- [x] **Step 2:** All existing tests green (key's last label bottom 142.65 < 146 and title's first ascent 16.5 < 22 under hhea metrics); lints; rustqual.
- [x] **Step 3:** Commit `test(layout): one bounds walker and fixture builder for every block`.

---

### Task 4: `RadarBlock` — the frame (FR-19, FR-30–FR-32, FR-42, FR-43, FR-70)

**Files:** Create `src/layout/radar.rs`; modify `src/layout/mod.rs`; `src/style.rs` (constants).

**Interfaces:**

```rust
// style
/// The fraction of their natural size below which markers, their numbers and
/// the radar's strokes do not shrink (FR-70, D-69).
pub const RADAR_FLOOR: f64 = 7.0 / 9.0;
/// Clear space between the radar's outermost label and the block's edge.
pub const RADAR_LABEL_PAD: f64 = 10.0;

// radar
/// FR-70's three scales for one radar scale `s`.
pub struct RadarScale { pub geometry: f64, pub labels: f64, pub marks: f64 }
impl RadarScale {
    /// `geometry` = s, `labels` = max(s, 1), `marks` = max(s, RADAR_FLOOR).
    /// Debug-asserts `s` is finite and positive (a NaN would otherwise vanish into `max`).
    pub fn of(s: f64) -> Self
}
pub struct RadarBlock { /* geometry, quadrant labels (FR-10 order) with measured widths, ring names with measured widths */ }
impl RadarBlock {
    /// # Panics
    /// If `plotted` names a quadrant `catalogue` lacks (`order::resolve` guarantees it does not).
    pub fn new(catalogue: &Catalogue, plotted: &Plotted, fonts: &Fonts) -> Self
}
impl Block for RadarBlock
```

`new` maps `plotted.quadrants` to labels with `.get(q).expect(…)` and the `// qual:allow(error_handling) reason: …` marker, as `geometry::place` does; widths come from `fonts.face(style.weight).measure(text, size, letter_spacing)` at natural size.

**Size** (`r = RadarScale::of(s)`; quadrant labels `Q` at `p = polar(middle(i), QUADRANT_LABEL_RADIUS)` with width `w`; ring names `R` with width `w` and mid-radius `m = midpoint(inner, outer)`; `A`, `D` = `TEXT_ASCENT`, `TEXT_DESCENT`; `z` = the label's font size; `h` = `LABEL_HALO`):

- `label_half_width` = max over `Q` of `|p.x|·s + w/2·r.labels`, and over `R` of `(w/2 + h/2)·r.labels` (0 with neither).
- `label_half_height` = max over `Q` of `max(p.y·s + D·z·r.labels, A·z·r.labels − p.y·s)`, and over `R` of `m·s + (A·z + h/2)·r.labels`.
- `half_w = max(MIN_RADAR_WIDTH/2 · s, label_half_width + RADAR_LABEL_PAD · r.labels)`; `half_h` likewise with `label_half_height`.
- `size = (2·half_w, 2·half_h)`; centre `C = (half_w, half_h)`.

Write it as named pure operations (`label_half_width(&self, r)`, `label_half_height(&self, r)`, `extent(&self, r) -> Size`) with `size(s)` = `self.extent(RadarScale::of(s))`, so PR 10b adds marker and hit-area reach as one more term each.

**Render** (`at.origin`, `at.scale` = s): `Group::new(at.origin, [Group::new(C, frame)])`, `frame` in this order:
1. Per declared ring `i`: `Circle { centre: ORIGIN, radius: band(i).outer · s, fill: Paint::None, stroke: Role(Neutral(StrokeRing)), stroke_width: GRID_STROKE · r.marks }`.
2. Per quadrant position (none at N = 0): `Line { from: ORIGIN, to: polar(divider(p), DIVIDER_RADIUS · s), stroke: Role(Neutral(StrokeDivider)), stroke_width: GRID_STROKE · r.marks }`.
3. Per quadrant position: `TextRun { origin: polar(middle(p), QUADRANT_LABEL_RADIUS), style: QUADRANT_LABEL, ink: TextPrimary, anchor: Middle, lines: [label], leading: 0 }.node(Scale { position: s, size: r.labels })`.
4. Per declared ring: the `TextRun { origin: (0, −m), style: RING_LABEL, ink: TextMuted, anchor: Middle, … }` → `halo(LABEL_HALO, scale)` then `node(scale)`, `scale = Scale { position: s, size: r.labels }`.

- [x] **Step 1: Failing tests** (fixtures via `test_support::plotted`; all floats via `close`; `C` from `size(s)` halves, plus literal pins):
  - `radar_scale`: `of(1.0)` = (1, 1, 1); `of(0.9)` = (0.9, 1, 0.9); `of(RADAR_FLOOR)` = (7/9, 1, 7/9); `of(0.5)` = (0.5, 1, 7/9); `of(1.5)` = (1.5, 1.5, 1.5).
  - `zero_quadrants_draws_rings_only`: header-only CSV, default rings → `size(1.0)` is exactly 660 × 660; 4 circles, 0 lines, 8 texts (4 halos, 4 names), nothing else; `assert_finite`.
  - `one_quadrant_has_one_divider`: one `Line` to `polar(−90, 262)`; one quadrant label at `polar(90, 300)` (x via `close` — `cos 90°` is ~1e-14).
  - `dividers_follow_fr77`: N = 2 → ends at −90, 90; N = 3 → −90, 30, 150; N = 4 → −90, 0, 90, 180; N = 8 → −90 + 45k — compared as points against `polar(angle, 262)`.
  - `quadrant_labels_follow_fr10`: CSV first-seen order `Tool, platform, Practice` → label texts `platform, Practice, Tool` (FR-10's case-insensitive sort), at `middle(0..3)`; and a CSV whose rows are all in an undeclared ring still draws all dividers and labels (FR-42).
  - `every_declared_ring_draws`: `--rings` of 5 with rows in one → 5 circles at radii 52·k, 5 names at y = −midpoint(inner, outer); also 1 ring (one circle r 260, name at y −130) and 8 rings (band 32.5).
  - `ring_labels_sit_on_a_halo`: each ring name is preceded by a `Text` with the same origin, style, anchor and lines whose ink is `TextPaint::Halo { stroke: Surface1, width: LABEL_HALO }`; the name's ink is `Fill(Role(Neutral(TextMuted)))`; no `Semantics::Decorative` group in the tree; ring texts come after every `Line`.
  - `quadrant_labels_are_primary_and_middle_anchored`.
  - `three_scales`: for `s` ∈ {1.0, 0.9, 7/9, 0.7, 0.5, 0.4}: circle radius = `outer · s`; circle and line stroke = `GRID_STROKE · max(s, 7/9)`; line end = `polar(·, 262 · s)`; quadrant label origin = `polar(·, 300 · s)` and its size, spacing = natural; ring name origin y = `−m · s`, size natural; halo width = `LABEL_HALO`.
  - `size_pins`: PRD example (3 quadrants) at `s` = 1 → 660 × 660; zero quadrants at `s` = 0.5 → 330 × 330; N = 8 (a label above centre: middle −67.5) at `s` = 0.3 → height = `2·(300·0.3·sin 67.5° + A·12 + 10)` ≈ 209.55, above the 198 floor so a wrong ascent fails it (at `s` = 1 every N gives 660), computed in the test from the formula's parts; N = 1 at `s` = 0.4 → height = `max(264, 2·(300·0.4 + D·12 + 10))`.
  - `a_long_quadrant_label_widens_the_block`: N = 4, one name of 40 × "W" → width = `2·(|polar(45,300).x|·s + w/2 + 10)` with `w` from `Fonts`, > 660, at `s` = 1 and 0.576.
  - `a_long_ring_name_widens_the_block`: zero quadrants, `--rings` with one ring named `"W"` × 80 (about 808 units; ordinary 80-character text measures under 450 and would lose to the 660 minimum) → width = `2·(w/2 + 1.5 + 10)` at `s` = 1, with `w` from `Fonts`.
  - `size_is_non_decreasing`: for each fixture, over `s` from 0.30 to 1.00 in steps of 0.05, each component ≥ the previous (tolerance via `close`); and for N = 1 with a long label, `size(0.4).width` equals `size(0.7).width` (the plateau PR 12 must handle).
  - `everything_drawn_lies_inside_the_size_and_touches_it`: for the PRD example, N = 1, 2, 6, 8, zero quadrants, the long-quadrant and long-ring cases, at `s` ∈ {1.0, 0.576, 0.4}: `assert_inside(render(at(0,0,s)), size(s))`, and on any axis the labels dominate, the drawn `bounds` reaches within 1e-6 of `RADAR_LABEL_PAD · r.labels` of the block edge (tightness: catches an oversized `size`).
  - `ring_names_clear_4_5_on_their_halo` (in `src/palette.rs` tests, using its private `contrast`): `TextMuted` against `Surface1` is ≥ 4.5 in the light and dark palettes.
  - `names_are_carried_raw`: a quadrant named `A<&">B` and one in CJK appear in the node's `lines` unchanged (escaping is the serialiser's, NFR-6).
  - `block::assert_translates(&radar)`, and the same check at `s` = 0.5.
- [x] **Step 2–4:** red, implement, green; lints; rustqual (`size`, `render` integration-only; `RadarBlock::new` with `# Panics`).
- [x] **Step 5: Gate** `just ci`.
- [x] **Step 6:** Commit `feat(layout): the radar block's frame (FR-30–FR-32, FR-42, FR-43, FR-70)`.

---

## Finishing

1. **Reviews:** final whole-branch review (most capable model) in Copilot's seat (out of credits until November 2026; say so in the PR body), simplify, final check.
2. **Crumbs:**
   - `rg-svr` stays open for PR 10b. Note: markers by `Glyph::draw(Scale { position: s, size: r.marks })` (move `Glyph::draw` to `Scale`; key passes `Scale::uniform`); numbers in `MARKER_NUMBER` at `r.marks`; markers draw after ring labels; FR-36 `<title>`; FR-94 hit areas at `r.labels` (≥ 24 CSS px); number ink per series entry at bind time (D-54); add marker and hit-area reach terms to `extent`.
   - `rg-ozb` (PR 12) note: the narrow scale is solved from `RadarBlock::size`, which is non-decreasing but plateaus where a label dominates; define the minimum `s` and the behaviour when no `s` fits (a quadrant name over ~360 units at 12 px).
   - PR 13's serialiser crumb note: `TextPaint::Halo` → `fill="none" stroke-linejoin="round" aria-hidden="true"`; the radar's root is not `role="img"`.
   - New crumb (idea, layout): long quadrant labels cross the outer ring at 3/9 o'clock and diagonals (mockup-identical) — consider inner-end anchoring or a radial push-out.
   - New crumb (task, accessibility): SC-17 contrast against what is painted if quadrants are ever tinted (carried from `rg-6w7`).
   - `crumbs close rg-6w7` (decided in PRD v0.50; implemented for markers in PR 10b).
3. Move this plan to `docs/plans/done/`; tick the steps.
4. **No version bump.** Commit, push, PR (state that Task 1 is a PRD amendment shipped with its first consumer), CI; merge when clean, if the maintainer authorises it.
