---
tags:
  - plan
  - layout
  - svg
aliases: []
doc_id: RG-009
doc_name: pr-09-title-key
crumb_id: rg-y1t
document_title: "PR 09 — the Block trait, a first svg::Node, and the title and key blocks: Implementation Plan"
created_date: 2026-10-11
synopsis: "The first blocks that draw: layout/block.rs (Block, Size, Placement), a minimal svg::Node and Paint, one marker-drawing helper the radar will reuse, FR-65's title block and FR-66's key."
status: done
type: Plan
revision: "1.1"
review_date: 2026-10-11
reviewed_by:
  - rust-engineer
  - qa-expert
  - architect-reviewer
  - ui-designer
  - accessibility-tester
completed_date: 2026-10-11
comments:
---
# PR 09 — Block, a first svg::Node, and the title and key blocks: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-y1t` · **Phase:** v1.0 · **Blocks:** `rg-ozb` (PR 12)

**Goal:** The first blocks that draw. A title and subtitle sit above the radar or vanish entirely when omitted (FR-65), and a fixed key explains hollow, solid and the three status treatments with sample markers drawn the way the radar draws them (FR-66).

**Architecture:** Four pieces, in dependency order (architecture §4, §5; maintainer's ruling of 2026-10-11 that `Block` and `Node` arrive with their first user). Revised after five plan reviews (rev 1.1).
- `src/svg/mod.rs`: `Point` (moved here from `layout::geometry`, which re-exports it, so `layout` depends on `svg` and never the reverse — the plan review found the cycle), `Node`, `Group`, `Semantics`, `Paint`, `Anchor` and `PathCommand`, carrying only what the title and key draw. Coordinates stay `f64`; PR 13's serialiser formats them through `num::fmt_coord` (NFR-17). Colour is a `palette::Role`, never a hex value (architecture §5).
- `src/layout/block.rs`: `Size`, `Placement` and the `Block` trait. **Scale is applied to coordinates, never as an SVG transform**: a block draws at the scale it is asked for, so FR-70's radar can shrink its positions while its labels keep their size. A `Group` only translates.
- `src/layout/marker.rs`: one function that draws a marker glyph — fill, stroke, status ring — with its qualifier and status treatment (FR-20, FR-40). The key uses it now and the radar block (PR 10) will, wrapping it with its number, tooltip and hit area; FR-66's "drawn the same way the radar draws it" is then one code path.
- `src/layout/title.rs` and `src/layout/key.rs`: the two blocks.

**Tech Stack:** Rust 2024, `std` only. No new dependency.

**Spec:** `docs/product/radargen-prd.md` v0.49 — FR-20, FR-39, FR-40, FR-44, FR-45, FR-47, FR-50, FR-65, FR-66, FR-70, FR-79, FR-82, NFR-1, NFR-4, NFR-6, NFR-17, D-24, D-25, D-56. Design: `docs/design/architecture.md` §3 (`layout/`, `svg/`), §4 (`Block`, `Size`, `Point`, `Placement`), §5 (`Node`, `Paint`, late colour binding). Reference: `docs/product/mockup.html` `buildTitle`, `buildKey`, `halfRingPath`, `statusRingMarkup`.

## Global Constraints

- **Lints are as strict as `Cargo.toml` says** — pedantic, nursery, cargo, `missing_docs`, `indexing_slicing`, `as_conversions`, `unwrap_used`, `string_slice` and more, all deny. Fix findings; suppress only with `#[expect(lint, reason = "…")]` whose reason is true; rustqual markers likewise. Never reshape code to hide from a lint or rustqual. No silent failures.
- **No hex in a `Node`** (architecture §5): fills and strokes are `Paint::Role(Role)` or `Paint::None`. `Paint::None` is SVG's `none` (stroked-but-unfilled shapes); FR-20's hollow marker fills with `Role::Neutral(Neutral::MarkerHollow)`, not `Paint::None`.
- **Block-local coordinates:** a block draws from its own top-left at (0, 0); `Placement` moves and scales it. Margins belong to the arrangement (PR 12), not to blocks.
- **Every measured size comes from `text::Fonts`** (FR-84): never a guess or a character count.
- **No `<foreignObject>`** (FR-47): `Node` has no variant for it.
- **Determinism** (NFR-4): no `HashMap`, no randomness; `f64` stays unformatted in the tree until PR 13.
- `cargo fmt`; `cargo clippy --all-targets -- -D warnings`, `just lint` and `just ci` clean at every commit. If rustqual's score rises, `just quality-baseline`; if it falls, find the cause, do not reshape to recover it.
- Commits: `type(scope): description`, subject and body lines ≤ 72, ending `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`. Run `git-mit es` first; check `git log -1 --format=%B` (the hook may not be installed in a worktree).

## Review Focus

1. **No title and no subtitle** (the default). The title block is empty: zero size, renders nothing, and the block below moves up (FR-65). *Test: Task 3, `an_omitted_title_block_is_empty`.*
2. **A title or subtitle of only whitespace.** It counts as omitted (FR-65). *Test: Task 3, `whitespace_only_counts_as_omitted`.*
3. **A long title.** It wraps at the legend's column, the same width in both arrangements (D-56), rather than running past a phone-width document. *Test: Task 3, `a_long_title_wraps_at_the_legend_column`.*
4. **A key in the dark theme.** The hollow sample's fill is `MarkerHollow`, which matches the radar ground, not `none`, which would show the key's own background (architecture §5). *Test: Task 2, `the_hollow_marker_fills_with_marker_hollow`.*
5. **A key read without colour.** Every encoding is named in words beside its sample (FR-66, FR-39), and every row is drawn whatever the dataset uses (D-25). *Test: Task 4, `the_key_draws_every_row_always`.*
6. **A title with markup characters** (`<`, `&`, `"`). The words are stored as written (runs of whitespace collapse, as FR-79's wrap splits on whitespace); escaping is the serialiser's (NFR-6, PR 13). *Tests: Task 3, `title_text_is_stored_as_written`, `interior_whitespace_collapses`.*
7. **A wrapped title followed by a subtitle.** The subtitle follows the title's LAST line, and a screen reader hears the title as one heading, not as fragments. *Tests: Task 3, `subtitle_follows_last_title_line`, `the_title_is_one_heading`.*
8. **A blank title with a real subtitle.** The subtitle takes the subtitle-alone position. *Test: Task 3, `blank_title_with_subtitle_is_subtitle_alone`.*
9. **The last key row.** Its ring and label sit inside the block's height, so nothing placed below clips them. *Test: Task 4, `everything_drawn_lies_inside_the_size`.*

## Crumbs this plan incorporates

| Crumb | What it asked | Where |
| --- | --- | --- |
| `rg-y1t` | FR-65 title block; FR-66 key, markers drawn as the radar draws them (`MarkerHollow` vs `Paint::None`) | Tasks 2–4 |
| `rg-y1t` (PR 08 ruling) | `layout/block.rs` (`Block`, `Size`, `Placement`, using `geometry::Point`) and a minimal `svg::Node` | Task 1 |
| `rg-y1t` (PR 08a review) | `FontError` names no face | Not here: PR 14 is the first caller of `Fonts::embedded`; noted on `rg-f27` (Finishing) |

## Decisions taken in this plan (flag on review)

- **`Point` lives in `svg`** and `layout::geometry` re-exports it (`pub use crate::svg::Point;`), so the dependency runs one way, `layout → svg`. Keeping it in `geometry` made `svg` depend on `layout` while `layout` returns `svg::Node`: a module cycle rustqual reports (the plan review measured it).
- **Scale is baked into coordinates.** `Block::render(&self, at: Placement)` draws at `at.scale` itself — positions, radii, stroke widths and text sizes are multiplied for the title and key, which scale linearly — and wraps the result in a translating `Group` at `at.origin`. No `Node` carries a scale transform: an SVG `scale()` would shrink the radar's labels, which FR-70 forbids. Architecture §4 records the rule.
- **`Group` has a semantics slot now** (`Semantics::{None, Decorative, Heading(u8)}`, default `None`, built through `Group::new`), so PRs 10–15 add roles, labels and identity without editing every group earlier PRs built (the architecture and accessibility reviews). This PR uses three values: the title as a level-1 heading, the key's "Key" as a level-2 heading, and the key's samples as decorative (hidden from assistive technology; the words beside them carry the meaning). PR 13 writes them out; PR 15 adds `Button`. FR-65 puts the title inside the shared document so PNG and SVG carry it, so it is the SVG text, not an HTML `<h1>`, that is the heading.
- **A wrapped text is one `Node::Text` with several lines** (`lines: Vec<String>`, `leading`), serialised as one `<text>` with `<tspan>`s, so a wrapped title is one heading, not several fragments.
- **One marker helper for key and radar, glyph only.** `layout::marker::draw(centre, radius, qualifier, mark, ink)` returns the marker's fill, stroke and status ring; the radar (PR 10) wraps it with its number, `<title>` and hit area. `ink` is a `Role` (a marker inked `none` is meaningless), and the direction toward the centre belongs to the moved statuses (`StatusMark::{New, MovedIn { centreward }, MovedOut { centreward }}`), not a free argument that New and No Change ignore.
- **The key's samples are smaller than the radar's markers**, as `mockup.html` draws them (D-36): radius 7 for the qualifier rows; radius 5 with the radar's `STATUS_RING_GAP` of 4 (ring at 9) for the status rows. In the narrow arrangement the radar's own markers render at about 5.8 with a ring near 8.1 (scale ≈ 0.58), so on a phone the key matches what is on screen (UI review). The shared helper gives solid samples the radar's 2-unit `Surface1` separating stroke and hollow ones `MARKER_HOLLOW_STROKE` (3, against the mockup's 2.5): intended — one code path, the radar's encoding exactly.
- **Key rows are 24 units apart** (mockup: 22): 4 units between neighbouring status rings, the same as a marker's gap to its ring, and the legend's single-line row height (`max(17, 15) + 7`). The height is computed from the last sample's extent, not a magic bottom margin.
- **Key samples are inked `TextMuted`.** FR-39 keeps colour off the correctness path and FR-66 excludes quadrant colour; `TextMuted` clears 3:1 non-text and 4.5:1 text contrast on both surfaces in both themes (accessibility review measured it).
- **Status direction in the key is fixed** (left toward the centre, right toward the edge), and the labels say so in words a key row can carry: "half-ring on the side toward the radar's centre" / "…toward the radar's outer edge".
- **The key has no full-width rule**; separators, margins and whether the key scales in narrow (it should not; D-56) are the arrangement's (PR 12).
- **The title wraps at `LEGEND_WIDTH_MAX`** (FR-79's greedy wrap via `text::wrap`), the legend's column in both arrangements (D-56). FR-65 says nothing about long titles; the maintainer decided no PRD change (Finishing).
- **`PathCommand::Arc` carries `large_arc` and `sweep`** rather than fixing them in a comment: the half-ring uses small arc, sweep 1; PR 10's ring bands need the other sweep.
- **No version bump.** Nothing calls the blocks until PR 12 and PR 14.

---

### Task 1: `svg::Node`, `Point`, `Paint`, and `layout::block`

**Files:**
- Create: `src/svg/mod.rs`, `src/layout/block.rs`
- Modify: `src/lib.rs` (`pub mod svg;` between `pub mod style;` and `pub mod text;`), `src/layout/mod.rs` (`pub mod block;`), `src/layout/geometry.rs` (move `Point` out; `pub use crate::svg::Point;`), `docs/design/architecture.md` (§4, §5)

**Interfaces:**

```rust
// src/svg/mod.rs
pub struct Point { pub x: f64, pub y: f64 }                    // moved from layout::geometry; Debug, Clone, Copy, PartialEq
pub enum Paint { Role(Role), None }                            // Debug, Clone, Copy, PartialEq, Eq
pub enum Anchor { Start, Middle }                              // Debug, Clone, Copy, PartialEq, Eq
pub enum Semantics { None, Decorative, Heading(u8) }           // Debug, Clone, Copy, PartialEq, Eq, Default (None)
pub enum PathCommand {                                         // Debug, Clone, Copy, PartialEq
    MoveTo(Point),
    Arc { radius: f64, large_arc: bool, sweep: bool, end: Point },
}
pub struct Group { pub offset: Point, pub semantics: Semantics, pub children: Vec<Node> }  // Debug, Clone, PartialEq
impl Group { pub fn new(offset: Point, children: Vec<Node>) -> Self /* semantics None */;
             pub fn with(self, semantics: Semantics) -> Self }
pub enum Node {                                                // Debug, Clone, PartialEq
    Group(Group),
    Text { origin: Point, style: TextStyle, ink: Paint, anchor: Anchor, lines: Vec<String>, leading: f64 },
    Circle { centre: Point, radius: f64, fill: Paint, stroke: Paint, stroke_width: f64 },
    Path { commands: Vec<PathCommand>, fill: Paint, stroke: Paint, stroke_width: f64 },
}
// src/layout/block.rs
pub struct Size { pub width: f64, pub height: f64 }            // Debug, Clone, Copy, PartialEq, Default
pub struct Placement { pub origin: Point, pub scale: f64 }     // Debug, Clone, Copy, PartialEq
impl Size { pub fn scaled(self, scale: f64) -> Self; pub fn is_empty(self) -> bool /* self == Self::default() */ }
impl Placement { pub const fn at(origin: Point) -> Self /* scale 1 */ }
pub trait Block {
    /// Size when drawn at `scale`.
    fn size(&self, scale: f64) -> Size;
    /// The block drawn at `at.scale`, translated to `at.origin`. Scale is
    /// applied to coordinates, never as an SVG transform (FR-70).
    fn render(&self, at: Placement) -> Node;
}
```

`Text`'s first line's baseline is at `origin`; each further line is `leading` below. Every public item carries a doc comment saying what it means and the requirement it serves (`missing_docs` is denied). `Node` is not `Eq` (it holds `f64`); `Paint`, `Anchor` and `Semantics` must derive `Eq` (nursery's `derive_partial_eq_without_eq`).

- [x] **Step 1: Write the failing tests.** `src/layout/block.rs`: `Size::scaled` multiplies both sides; `Size::default().is_empty()`; a non-zero size is not; `Placement::at(p)` has scale 1. `src/svg/mod.rs`: `Paint::None` ≠ `Paint::Role(Role::Neutral(Neutral::MarkerHollow))` (architecture §5's distinction); `Group::new` defaults to `Semantics::None` and `with` sets it; a `Group` keeps its children in order. Existing geometry tests still compile through the re-export.
- [x] **Step 2: Run** `cargo test --lib svg:: layout::` — expect compile failure.
- [x] **Step 3: Implement** the types with their docs; move `Point` and re-export it.
- [x] **Step 4: Run** the tests, `cargo clippy --all-targets -- -D warnings`, `/opt/homebrew/bin/just lint`, and `rustqual .` — confirm no `CYCLE` and no new SDP finding.
- [x] **Step 5: Update architecture.** §4: the code block becomes the trait as built; `Size { width, height }` (renamed from `w`, `h`); add "Scale is applied to coordinates, never as an SVG transform: a block draws at the scale it is asked for, so the radar can shrink positions while labels keep their size (FR-70); a `Group` only translates." and "`Point` is `svg::Point`, re-exported by `layout::geometry`, so `layout` depends on `svg` and not the reverse." §5: replace the `Node` sketch with the variants above, noting that variants and `Semantics` values are added by the PR that first needs them.
- [x] **Step 6: Commit** `feat(svg): Node, Point and Paint; Block, Size and Placement`.

---

### Task 2: `layout::marker` — one way to draw a marker glyph

**Files:**
- Create: `src/layout/marker.rs`; modify `src/layout/mod.rs`

**Interfaces:**
- Consumes: Task 1's types; `model::{Qualifier, Status}`; `layout::geometry::polar`; `style::{STATUS_RING_GAP, STATUS_RING_STROKE, MARKER_STROKE, MARKER_HOLLOW_STROKE}`; `palette::{Role, Neutral}`.
- Produces:

```rust
pub enum StatusMark { New, MovedIn { centreward: f64 }, MovedOut { centreward: f64 } }  // Debug, Clone, Copy, PartialEq
impl StatusMark { pub fn of(status: Status, centreward: f64) -> Option<Self> /* NoChange → None */ }
pub fn draw(centre: Point, radius: f64, qualifier: Option<Qualifier>, mark: Option<StatusMark>, ink: Role) -> Vec<Node>
```

`centreward` is the direction, in degrees (SVG frame), from the marker toward the radar's centre.

Behaviour (FR-20, FR-40, mockup `statusRingMarkup`):
- Solid (`Some(Qualifier::Default)` or `None`): `Circle { fill: Role(ink), stroke: Role(Neutral(Surface1)), stroke_width: MARKER_STROKE }`.
- Hollow (`Some(Qualifier::Conditional)`): `Circle { fill: Role(Neutral(MarkerHollow)), stroke: Role(ink), stroke_width: MARKER_HOLLOW_STROKE }` — never `Paint::None` as fill.
- `New`: a full ring `Circle { radius: radius + STATUS_RING_GAP, fill: Paint::None, stroke: Role(ink), stroke_width: STATUS_RING_STROKE }`, before the marker.
- `MovedIn { c }` / `MovedOut { c }`: a half-ring `Path [MoveTo(centre + polar(d − 90, R)), Arc { radius: R, large_arc: false, sweep: true, end: centre + polar(d + 90, R) }]`, `R = radius + STATUS_RING_GAP`, `fill: Paint::None`, before the marker, where `d = c` (In) or `c + 180` (Out).
- No mark: the marker alone.

- [x] **Step 1: Failing tests:**
  - `the_hollow_marker_fills_with_marker_hollow`; `a_solid_marker_fills_with_its_ink` (Default and `None`).
  - `new_draws_a_full_ring_under_the_marker`: two nodes, ring first, radius `r + STATUS_RING_GAP`, fill `Paint::None`.
  - `half_ring_endpoints_follow_centreward`: centre (100, 50), radius 7, `centreward` ∈ {0, 90, 225}: Moved In starts at centre + `polar(c − 90, 11)` and ends at centre + `polar(c + 90, 11)`; Moved Out the same with `c + 180`. Literal expectations computed in the test with `polar`, compared within 1e-9.
  - `the_half_rings_are_mirror_images`: at `centreward` 180, Moved In's start equals Moved Out's end and vice versa, within 1e-9 (exact equality fails at x = 10.000000000000004).
  - `arc_flags`: the half-ring's Arc has `large_arc: false, sweep: true`.
  - `status_mark_of_maps_every_status`: New → New; MovedIn/MovedOut carry the angle; NoChange → None.
  - `no_mark_draws_the_marker_alone`.
- [x] **Step 2–4:** red, implement, green; clippy, `just lint`.
- [x] **Step 5: Commit** `feat(layout): one marker glyph for key and radar (FR-20, FR-40)`.

---

### Task 3: the title block (FR-65)

**Files:**
- Create: `src/layout/title.rs`; modify `src/layout/mod.rs`, `src/style.rs`

**Interfaces:**
- Consumes: Tasks 1–2; `text::{Fonts, wrap}`; `style::{TITLE, SUBTITLE, LEGEND_WIDTH_MAX}`; `crate::trim`.
- Produces: `pub struct TitleBlock`; `impl TitleBlock { pub fn new(title: Option<&str>, subtitle: Option<&str>, fonts: &Fonts) -> Self }`; `impl Block for TitleBlock`.

Layout, from the mockup's `buildTitle`, with named constants in `style.rs`: `TITLE_TOP` 4, `TITLE_FIRST_BASELINE` 18, `TITLE_LEADING` 21, `SUBTITLE_AFTER_TITLE` 17, `SUBTITLE_ALONE` 15, `SUBTITLE_LEADING` 15, `TITLE_BOTTOM` 14.
- A value is omitted when it is `None` or empty after `crate::trim` (FR-65; FR-76's trimmable set, so a lone zero-width space is as empty as spaces). Both omitted → `size` is `Size::default()` and `render` returns an empty `Group` (no reserved space; whether an empty group is emitted is PR 12/13's).
- Title: one `Node::Text` with `lines` from `text::wrap(title, LEGEND_WIDTH_MAX, …TITLE…)`, first baseline `TITLE_TOP + TITLE_FIRST_BASELINE` (22), `leading` `TITLE_LEADING`, ink `TextPrimary`, x 0, inside a `Group` with `Semantics::Heading(1)`.
- Subtitle: one `Node::Text` the same way with `SUBTITLE`, first baseline `+ SUBTITLE_AFTER_TITLE` after the title's LAST line, or `TITLE_TOP + SUBTITLE_ALONE` (19) with no title; leading `SUBTITLE_LEADING`; ink `TextMuted`; no heading.
- Height: last baseline `+ TITLE_BOTTOM`. Width: the widest measured line.
- At `scale` ≠ 1, every coordinate, the leading and the text styles' size and letter-spacing are multiplied by `scale`.
- Words are stored as written; runs of whitespace collapse, since FR-79's wrap splits on whitespace. Escaping is PR 13's (NFR-6).
- rustqual: keep `new` an integration function — pure helpers (e.g. `omitted`, `subtitle_baseline`, `widest`) and a small builder taking a `Part { style, ink, leading }` const per value — so IOSP and the parameter count stay clean (the plan review saw a violation from the obvious shape).

- [x] **Step 1: Failing tests:**
  - `an_omitted_title_block_is_empty`; `empty_string_equals_none` (`Some("")` ≡ `None`); `whitespace_only_counts_as_omitted` (`Some("  ")`, `Some("\u{200B}")`, `Some("\u{FEFF}\u{200B}")`).
  - `blank_title_with_subtitle_is_subtitle_alone`: `(Some(" \t"), Some("Sub"))` → subtitle baseline 19, height 33.
  - `title_alone` (baseline 22, height 36), `subtitle_alone` (19, 33), `title_and_subtitle` (22 and 39, height 53).
  - `subtitle_follows_last_title_line`: a title that wraps to two lines plus a subtitle → title baselines 22 and 43, subtitle 60, height 74.
  - `subtitle_wraps_with_its_leading`: a long subtitle alone → baselines 19 and 34.
  - `the_title_is_one_heading`: a wrapped title is one `Text` node with ≥ 2 `lines`, inside a `Group` with `Semantics::Heading(1)`; the subtitle is not a heading.
  - `a_long_title_wraps_at_the_legend_column`: every line measures ≤ `LEGEND_WIDTH_MAX`.
  - `unbreakable_token_is_split_not_overflowed`: a 200-character token with no spaces → every line ≤ `LEGEND_WIDTH_MAX`, lines concatenated equal the token.
  - `title_text_is_stored_as_written` (`"R&D <2026>"`); `interior_whitespace_collapses` (`"Q3\n2026\tEdition"` → one line `"Q3 2026 Edition"`).
  - `title_width_is_widest_line`.
  - `size_scales_linearly` and `render_matches_size_at_half_scale` (baselines and text size halve at scale 0.5).
- [x] **Step 2–4:** red, implement, green; clippy, `just lint`, `rustqual .` (no IOSP violation).
- [x] **Step 5: Commit** `feat(layout): the title block (FR-65)`.

---

### Task 4: the key (FR-66)

**Files:**
- Create: `src/layout/key.rs`; modify `src/layout/mod.rs`, `src/style.rs`

**Interfaces:**
- Consumes: Tasks 1–3; `layout::marker::{draw, StatusMark}`; `style::{KEY_HEADING, KEY_LABEL, STATUS_RING_GAP, STATUS_RING_STROKE}`.
- Produces: `pub struct KeyBlock`; `impl KeyBlock { pub fn new(fonts: &Fonts) -> Self }`; `impl Block for KeyBlock`.

Content, fixed whatever the dataset uses (D-25), from the mockup's `buildKey` with the UI review's row spacing. Constants in `style.rs`: `KEY_HEADING_BASELINE` 20, `KEY_FIRST_ROW` 40, `KEY_ROW` 24, `KEY_SAMPLE_X` 10, `KEY_LABEL_X` 28, `KEY_LABEL_DROP` 4, `KEY_MARKER_RADIUS` 7, `KEY_STATUS_MARKER_RADIUS` 5, and `KEY_BOTTOM` = `KEY_STATUS_MARKER_RADIUS + STATUS_RING_GAP + STATUS_RING_STROKE / 2` (10), a const expression so it cannot drift from the samples.
- Heading `Key`: a `Node::Text` in `KEY_HEADING`, ink `TextPrimary`, baseline 20, inside a `Group` with `Semantics::Heading(2)`.
- Five rows at `y` = 40, 64, 88, 112, 136; the sample centred at `(KEY_SAMPLE_X, y)` inside a `Group` with `Semantics::Decorative`; the label a `Node::Text` at `(KEY_LABEL_X, y + KEY_LABEL_DROP)` in `KEY_LABEL`, ink `TextMuted`:
  1. `Default, or no qualifier — solid marker`: `draw(c, 7, Some(Qualifier::Default), None, Role::Neutral(Neutral::TextMuted))`.
  2. `Conditional — hollow marker`: radius 7, `Some(Qualifier::Conditional)`.
  3. `New this edition — full ring`: radius 5, `Some(StatusMark::New)`.
  4. `Moved in — half-ring on the side toward the radar's centre`: radius 5, `Some(StatusMark::MovedIn { centreward: 180.0 })`.
  5. `Moved out — half-ring on the side toward the radar's outer edge`: radius 5, `Some(StatusMark::MovedOut { centreward: 180.0 })`.
- Height: last row + `KEY_BOTTOM` = 146. Width: `KEY_LABEL_X` + the widest measured label, or the heading's width if wider.
- At `scale` ≠ 1, everything scales linearly as for the title.

- [x] **Step 1: Failing tests:**
  - `the_key_draws_every_row_always`: heading then five rows, labels exactly as above, in order.
  - `the_key_samples_are_the_radar_s_markers`: each row's sample equals `draw(...)` with that row's arguments (one code path).
  - `key_status_rows_literal_geometry` (not circular): row 4's half-ring runs from (10, 121) to (10, 103) and bulges left; row 5's from (10, 127) to (10, 145) and bulges right; row 3's ring is a circle of radius 9 at (10, 88). Literal numbers, within 1e-9.
  - `key_solid_sample_fill_is_text_muted`: row 1's fill is exactly `Role(Neutral(TextMuted))`; row 2's stroke likewise; row 2's fill `MarkerHollow`.
  - `the_samples_are_decorative_and_the_heading_is_a_heading`.
  - `key_geometry`: heading baseline 20; rows 40, 64, 88, 112, 136; labels 4 below; height 146; width 28 + the widest label as `Fonts` measures it.
  - `everything_drawn_lies_inside_the_size`: a test helper walks the rendered `Node` tree (translating through groups) and checks every `Circle` (centre ± radius + stroke/2), every `Path` endpoint ± the arc radius, and every text line's baseline + 0.3 × size lie within `size(1.0)`; run it for the key and the title.
  - `size_scales_linearly`.
- [x] **Step 2–4:** red, implement, green; clippy, `just lint`.
- [x] **Step 5: Gate:** `just ci` (if rustqual rises, `just quality-baseline`; if it falls, find the cause — e.g. an IOSP mix or a parameter count — and fix the structure honestly).
- [x] **Step 6: Commit** `feat(layout): the key (FR-66)`.

---

## Finishing

1. **Reviews:** final whole-branch review (most capable model) in Copilot's seat (out of credits until November 2026; say so), simplify, final check.
2. **Crumb notes:**
   - `rg-svr` (PR 10): draw markers through `layout::marker::draw` with `MARKER_RADIUS` and `StatusMark::of(status, centreward)`; wrap each glyph in a `Group` for its number, `<title>` (FR-36) and hit area. Add the `Node` variants the radar needs when it needs them: `Line`, `PathCommand::{LineTo, Close}`, `Anchor::End`, a text halo, `Node::Title`, a hit-area paint that still takes pointer events (`Paint::None` does not — FR-94), and a paint for marker numbers chosen per series entry at bind time (FR-82, D-54).
   - `rg-ozb` (PR 12): the key has no rule of its own; separators and margins are the arrangement's; the key and title stay at scale 1 in narrow (D-56), and the key's legibility at 320 px is checked against SC-17; keep document order title, radar, key, legend (screen readers read in DOM order).
   - `rg-6ku` (PR 13): serialise `Semantics` (`Heading(n)` → `role="heading" aria-level`, `Decorative` → `aria-hidden="true"`), `Text` lines as `<tspan>`s, `PathCommand::Arc`'s flags, every coordinate through `fmt_coord`, text through `escape::xml_text`; decide whether an empty `Group` is emitted; the exported standalone SVG gets a root `<title>` and `<desc>`.
   - `rg-f27` (PR 14): the pipeline is `Fonts::embedded`'s first caller — consider naming the face in `FontError` (PR 08a review).
3. **Settled with the maintainer.** FR-65 says nothing about long titles; the maintainer decided no PRD change (this PR wraps at the legend column). FR-50's marker accessible name omitting qualifier and status is resolved by PRD v0.49 (bb360b2).
4. `crumbs close rg-y1t`; move this plan to `docs/plans/done/`; tick the steps.
5. **No version bump.** Commit, push, PR, CI; merge when clean (the maintainer authorised merging this PR when ready).
