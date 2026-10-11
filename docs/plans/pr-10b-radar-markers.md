---
tags:
  - plan
  - layout
  - radar
aliases: []
doc_id: RG-010B
doc_name: pr-10b-radar-markers
crumb_id: rg-svr
document_title: "PR 10b — the radar block's markers: glyphs, numbers, controls, hit areas and indicators: Implementation Plan"
created_date: 2026-10-11
synopsis: "Every plotted item drawn on the radar as FR-20/FR-40's glyph with its FR-34 number in per-entry ink (D-54), exposed as a control keyed by its CSV row with FR-50's accessible name and FR-36's tooltip; an FR-94 hit layer below the markers and an FR-48 indicator layer above them; markers obey D-69's 7/9 floor."
status: draft
type: Plan
revision: "1.2"
review_date: 2026-10-11
reviewed_by: [rust-engineer, architect-reviewer, qa-expert, ui-designer, accessibility-tester]
completed_date:
comments: "1.1 folds in the five-seat plan review; 1.2 the scoped re-review. See Review rulings."
---
# PR 10b — the radar block's markers: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumbs:** `rg-svr` (part 2 of 2; closes it) · **Phase:** v1.0 · **After:** PR 10a (#44) · **Then:** PR 11 (legend)

**Goal:** The radar block draws every plotted item: FR-20/FR-40's glyph at FR-33's position, coloured by its quadrant's FR-67 rank, with its FR-34 number on it in D-54's ink; each marker is a control keyed by its CSV row (architecture §5), carrying FR-50's accessible name and FR-36's tooltip; an invisible FR-94 hit layer sits **below** every marker and an FR-48 indicator layer **above** every marker. Markers and numbers scale with the radar but not below 7/9 (D-69); hit areas stay at natural size.

**Architecture:** The radar becomes four layers in paint order: **frame** (PR 10a) → **hit layer** → **marker controls** → **indicator layer**. The tree stays emitter-neutral: `svg` gains the vocabulary (an item key, a control, an indicator, a hit area, a number's paint) and the contracts are written into architecture §5 now (Task 1), so PR 11, 13 and 15 build to the design rather than to crumb notes. `Glyph::draw` takes `svg::Scale` (position by `s`, size by `marks`).

**Tech Stack:** Rust 2024, `std` only. No new dependency.

**Spec:** `docs/product/radargen-prd.md` v0.50 → v0.51 — FR-20, FR-33, FR-34, FR-36, FR-37, FR-40, FR-48, FR-50, FR-67, FR-70, FR-82, FR-89, FR-94, SC-20, D-54, D-66, D-69. Design: `docs/design/architecture.md` §4 (px per unit), §5 (late-bound colour; "Identity in the output" — `data-row`, not FR-34's number). Reference: `docs/product/mockup.html` marker markup (l.630–651), CSS (l.147–166).

## Global Constraints

- Lints as strict as `Cargo.toml` says; suppress only with `#[expect(lint, reason)]` whose reason is true; rustqual markers likewise; never reshape code to hide from a lint or rustqual; no silent failures.
- **Scale is applied to coordinates, never as an SVG transform.** `Group` only translates.
- **No hex in a `Node`**, number ink included (architecture §5, D-54).
- **Identity is the item's CSV line** (`Item::line`, architecture §5), never FR-34's number.
- **A quadrant's colour is its FR-10 rank** (`Plotted::rank`), never `Item::quadrant` (FR-67).
- **Marker order is FR-34's** (`Plotted::items`); document order is reading and tab order.
- `indexing_slicing` is denied: zip, `enumerate`, `.get(..).expect(..)` under the `qual:allow(complexity, error_handling)` form `RadarBlock::new` already uses; never `[i]`.
- Floating-point arithmetic stays `*`/`+` with `#[expect(clippy::suboptimal_flops, reason = "NFR-4: …")]` where the lint fires (and only there — an unfulfilled `expect` fails the build), never `mul_add`; halves via `f64::midpoint` or a named constant. Tests compare floats with `close` / `near`.
- `str::trim` is banned (clippy.toml): names are trimmed with `crate::trim`.
- `cargo fmt`; `cargo clippy --all-targets -- -D warnings`, `just lint`, `just ci` clean at every commit. rustqual: rises → `just quality-baseline`; falls → find the cause.
- Commits: `type(scope): description`, ≤ 72 per line, trailer `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`; `git-mit es` first.

## Review Focus

1. **Neighbouring markers within capacity.** A visible disc always wins the pointer over a neighbour's hit area, because every hit area is in a layer below every marker (FR-36, FR-37, FR-38). *Test: Task 3, `hit_layer_paints_before_every_marker`.*
2. **A focused marker covered by a later one, or crossed by a neighbour.** Its indicator is in a layer after every marker and paints a Surface1 backing, a raised copy of the marker, then the ring — so the ring is measured against Surface1 (FR-48, SC-20, `FOCUS_RING_GAP`'s contract). *Test: Task 3, `indicators_raise_the_marker_over_surface`.*
3. **The narrow arrangement.** Positions by `s`; disc, ring, number and indicator by `max(s, 7/9)`; hit area at natural size (D-69, FR-94). *Test: Task 3, `marker_scales`.*
4. **The green series entry.** Its number names the entry; the bind picks `#16191F` (D-54). *Test: Task 3, `numbers_name_the_entry_they_sit_on`.*
5. **A hollow (Conditional) marker.** Its number is `TextPrimary` on `MarkerHollow`. *Tests: Task 3, `hollow_markers_number_in_primary_text`; Task 2, `primary_text_clears_4_5_on_marker_hollow`.*
6. **Unplotted rows among plotted ones.** No control, no hit area, no indicator for them; numbers stay contiguous (FR-17, FR-18). *Test: Task 3, `unplotted_rows_draw_nothing`.*
7. **SVG and PNG export.** Hit areas and indicators are interactive-only and say so structurally, so a static emitter can omit them (Task 1, §5). *Test: Task 2, `interactive_only_nodes_say_so`.*

## Review rulings (plan review, 2026-10-11)

Five seats reviewed revision 1.0 (reports in the session scratchpad). Rulings:

- **Key by CSV line, not number** (architect F1, HIGH): architecture §5 says identity is `data-row`, because unplotted rows have no number. New `svg::ItemKey(usize)` holding `Item::line`. `Control { key, label, tooltip: Option<String> }` — the number already lives in the label and tooltip; `tooltip` is optional because PR 11's legend rows are controls without one (F11).
- **`Semantics::Marker`, not `Control`** (final review I1): the variant is `Semantics::Marker(Control)` (named `Control` in rev 1.2), a radar marker as an interactive control. PR 11 adds `Semantics::LegendRow(Control)`, which carries `data-row` but not `data-marker`, so the focus rule cannot light a marker from its legend row.
- **Three layers, not hit areas inside markers** (architect F2, a11y, rust 2 — all HIGH): a later marker's hit area painted over an earlier marker's visible disc even within capacity. The hit layer is a `Group` (semantics `None`) of `Node::HitArea { centre, radius, key }`, painted before every marker; PR 15 forwards its pointer events to the control with the same key (the same delegated handler FR-37 needs).
- **Indicator layer with a Surface1 backing** (architect F5, rust 1): `style.rs`'s `FOCUS_RING_GAP` says the layout must raise the focused marker over Surface1. Each indicator is a `Group` with `Semantics::Indicator(ItemKey)` holding: a Surface1 disc of radius `INDICATOR_REACH · marks` (= `MARKER_RADIUS + FOCUS_RING_GAP + FOCUS_RING_STROKE/2` = 19), a copy of the marker's glyph and number, then the `Focus` ring of radius `(MARKER_RADIUS + FOCUS_RING_GAP) · marks`, width `FOCUS_RING_STROKE · marks`. Named for its job (an indicator), so PR 15's selection mark is one more child (F8). The layer group is `Semantics::None`.
- **Linking focus to its indicator in HTML** (architect F6, a11y): generated per-key `:has()` rules qualified to the marker control (`svg:has([data-marker="L"]:focus-visible) [data-indicator="L"] { visibility: visible }`), so a legend row for the same item (PR 11, same `data-row`) does not light the marker's ring (re-review 3); a marker control emits both `data-row` and `data-marker`, a hit area `data-hit`, so focus visibility does not depend on JavaScript. Recorded in architecture §5 (Task 1); emitted by PR 13/14.
- **Static formats omit interactive-only nodes** (architect F4, a11y): `Node::is_interactive_only()` is true for a `HitArea`, for a `Group` with `Semantics::Indicator`, and for a non-empty plain (`Semantics::None`) `Group` all of whose children are interactive-only (so the two layer groups drop whole, re-review 7). SVG and PNG skip them, and emit a `Control` as `<g aria-label><title>` without `role`/`tabindex`. HTML emits indicators hidden by a presentation attribute (`visibility="hidden" aria-hidden="true" pointer-events="none"`), and hit areas as `fill="none" pointer-events="all"` (no CSS colour keyword). The second per-format difference after colour binding; recorded in §5.
- **`HIT_RADIUS` is derived, with its premise named** (architect F3, rust 12): `MIN_TARGET_PX` 24, `NARROW_MIN_VIEWPORT_PX` 320, `NARROW_HORIZONTAL_INSET_PX` 0 (the narrow SVG spans the viewport; PR 14 must keep that or change this constant), and `MIN_PX_PER_UNIT = min((320 − inset)/NARROW_WIDTH, MIN_LEGIBLE_PX/LEGEND_NAME.size)` = 0.762 (a `pub const`; `f64::min` is const on 1.99); `HIT_RADIUS` = 16 with a `const { assert!(2.0 * HIT_RADIUS * MIN_PX_PER_UNIT >= MIN_TARGET_PX) }`. The breakpoint (`breakpoint_px`) should subtract the same inset — noted on the PR 14 crumb. Architecture §4 states that px per unit is the SVG's rendered width. Note on the PR 14 crumb.
- **`Paint::NumberOn(usize)`** — an unreduced series index, like `Role::Series` (architect F7, rust 6). Architecture §5's binding table gains a row: number ink is a literal in every format, HTML included (series values do not change with theme).
- **One extent term, the hit reach, inside `half_extent`, per axis, without the label pad** (rust 3, architect F10; re-review 2, 5): `half = max(330·s, label_reach + pad, hit_reach)` with `hit_reach = markers.map(|m| |m.at.x|·s + HIT_RADIUS·labels).fold(0.0, f64::max)` (likewise `y`), so a radar without markers gets no term. The indicator never sets the size: for `s < 7/9` its reach `|p|s + 19·7/9` is below the hit reach `|p|s + 16`, and for `s ≥ 7/9`, `|p| + 19 < 279 < 330` puts it under the floor — written as the doc-comment proof and guarded by `const` asserts `INDICATOR_REACH * RADAR_FLOOR <= HIT_RADIUS` and `BAND_OUTER_RADIUS + INDICATOR_REACH <= MIN_RADAR_HALF`. The hit term binds only for `s < 16/(330 − |p|)` ≲ 0.23. Non-decreasing in `s`, extending radar.rs's proof.
- **The number is a hand-built `Node::Text`** (rust 8, QA): `TextRun`'s ink is a `Neutral`. Baseline at `centre.y · s + NUMBER_CENTRING · MARKER_NUMBER.size · marks` with `NUMBER_CENTRING = midpoint(TEXT_ASCENT, −TEXT_DESCENT)` = 1490/4096 (Inter's cap height over two: lining digits sit exactly centred; equivalent to the mockup's `dominant-baseline: central`, UI review).
- **`centreward` from `atan2`** (architect F13, rust 15): accepted. The code already relies on platform `sin_cos` for every position; exposing FR-33's angle from `place` would change a stable API for last-bit determinism nothing else has. Cost if wrong: last-bit differences in half-ring endpoints across platforms.
- **The tooltip trims** (rust 7, a11y): `format!("{n}. {}", crate::trim(&item.name))`; label and tooltip built in one helper.
- **The Semantics doc** widens from "what a group means to assistive technology" to "what a group is for: its accessibility role, or its part in the interactive layer" (architect F14).
- **Not adopted** (low severity, re-review 9): splitting explicit `Default` from an absent qualifier in the glyph matrix (they draw identically, FR-20); an optional group around the markers (a11y 9; PR 15 may add one); the raised copy taking the pointer (re-review 10; PR 15 note); Surface1 outside the ring as well as inside (re-review 11: 1.4.11 is met against the inner Surface1).
- **PRD v0.51** (architect F9, F5): FR-94 says hit areas overlap whenever centres are closer than `2 · HIT_RADIUS` — most bands below capacity, the narrow arrangement nearly always — and that a visible marker always wins over a neighbour's hit area; D-66's element count; SC-20: the indicator is measured against Surface1 because it paints its own backing.
- **Known limits, recorded not fixed:** a three-digit number touches the disc's stroke (UI; OQ-16 territory); the `<title>` becomes the accessible description and repeats the name (a11y, F11; FR-36 mandates it — verify with screen readers in PR 15); a number wider than `2 · INDICATOR_REACH` (~6 digits) would leave the reach terms.

## Decisions taken in this plan (flag on review)

- **Marker colour**: `Role::Series(rank)` with `rank = plotted.rank.get(item.quadrant)`, unreduced (FR-67 cycles at bind).
- **Paint order inside a marker control**: status ring, disc, number. Markers in `Plotted::items` order.
- **No hit or indicator layer when there are no markers** (keeps PR 10a's node-count pins).
- **Where the layers sit**: children of the centring group, after the frame (`frame()` expects exactly one centring group under the root). Hit areas are drawn at `HIT_RADIUS · labels` (16 for `s ≤ 1`, 24 at 1.5). The indicator's Surface1 backing disc has `stroke: Paint::None, stroke_width: 0.0`.
- **`Glyph::draw(Scale)`** term-wise: `radius · size + STATUS_RING_GAP · size` (the form the NFR-4 `expect` fires on), replacing today's `mul_add`. Values unchanged at the tested scales.
- **No version bump.**

---

### Task 1: PRD v0.51 and architecture — the marker contracts

**Files:** `docs/product/radargen-prd.md`, `docs/design/architecture.md`

- [ ] **Step 1: FR-94.** Replace "In a band over FR-89's capacity, where markers overlap, their hit areas overlap too;" with "Hit areas overlap wherever two markers' centres are closer than twice the hit radius — in most bands below FR-89's capacity, and in the narrow arrangement nearly always. They are drawn below every marker, so a visible marker always takes its own pointer events, and a hit area only fills empty space; where hit areas overlap each other," (keep the rest: "a marker there relies on 2.5.8's *equivalent* exception…").
- [ ] **Step 2: D-66**: the cost note "each marker carries one more element" → "each marker carries a hit area and an indicator beside its control"; the Focus sentence "drawing focus in a top layer makes SC-20's surface contrast the contrast that is actually seen" → "drawing focus in a top layer, on its own Surface1 backing, makes SC-20's surface contrast the contrast that is actually seen". **SC-20**: "the indicator against the surfaces only, since FR-48 draws it above every marker" → "the indicator against Surface1, since FR-48 draws it above every marker on its own Surface1 backing".
- [ ] **Step 3:** Revision row 0.51; bump `Version:`.
- [ ] **Step 4: Architecture.** `Implements:` v0.51. §4: one sentence — px per unit is the narrow SVG's rendered width, which `NARROW_HORIZONTAL_INSET_PX` (0) ties to the viewport; FR-94's hit radius and PR 11's row height depend on it. §5 listing: `ItemKey`, `Control`, `Semantics::{Marker, Indicator}`, `Node::HitArea`, `Paint::NumberOn(usize)`; the radar's four layers (frame → hit → controls → indicators) and why; the binding table's number-ink row (a literal in every format); the per-format rule (static emitters skip `is_interactive_only` nodes and drop `role`/`tabindex`; HTML hides indicators by presentation attribute and shows one by a generated per-key `:has()` rule on `data-marker`). Amend l.~347 ("the one place a single `Document` legitimately serialises differently per format" → one of two, colour binding and interactive-only omission, with the reason: a static image has no pointer or focus) and the §12 risk row that rejects a second per-format difference (now admits exactly two, the second being this omission).
- [ ] **Step 5:** Commit `docs(prd): v0.51 — the radar's marker contracts`.

---

### Task 2: svg vocabulary, `HIT_RADIUS`, `Glyph::draw(Scale)`

**Files:** `src/svg/mod.rs`, `src/style.rs`, `src/layout/marker.rs`, `src/layout/key.rs` (call sites), `src/layout/test_support.rs` (new arms), `src/palette.rs` (one test)

**Interfaces:**

```rust
// svg
/// An item's identity in the output: its CSV line (architecture §5, `data-row`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemKey(pub usize);

/// An interactive item (FR-50).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Control { pub key: ItemKey, pub label: String, pub tooltip: Option<String> }

pub enum Semantics { None, Decorative, Heading(NonZeroU8), Marker(Control), Indicator(ItemKey) }
pub enum Paint { Role(Role), NumberOn(usize), None }
Node::HitArea { centre: Point, radius: f64, key: ItemKey }
impl Node { #[must_use] pub fn is_interactive_only(&self) -> bool }   // HitArea; Group with Indicator; non-empty Group of only such (const fn if the recursion allows)

// style
pub const MIN_TARGET_PX: f64 = 24.0;
pub const NARROW_MIN_VIEWPORT_PX: f64 = 320.0;
pub const NARROW_HORIZONTAL_INSET_PX: f64 = 0.0;
pub const MIN_PX_PER_UNIT: f64;                             // min((320 − inset)/NARROW_WIDTH, MIN_LEGIBLE_PX/LEGEND_NAME.size)
pub const HIT_RADIUS: f64 = 16.0;
pub const INDICATOR_REACH: f64 = MARKER_RADIUS + FOCUS_RING_GAP + FOCUS_RING_STROKE / 2.0;  // 19

// marker
impl Glyph { pub fn draw(&self, scale: Scale) -> Vec<Node> }
```

- [ ] **Step 1: Failing tests:**
  - `glyph_scales_position_and_size_apart`: `Scale { position: 0.5, size: 7/9 }` → centre × 0.5; radius 7.7778, status ring radius 10.8889, strokes × 7/9; `Scale::uniform(1.0)` values equal today's (existing marker tests, call sites moved to `Scale::uniform`).
  - `interactive_only_nodes_say_so`: `HitArea` → true; `Group` with `Indicator` → true; a plain group of two hit areas → true; an empty group → false; a plain group of a hit area and a circle → false; `Control` group, text, circle → false.
  - `hit_radius_meets_fr94`: the `const` assert in style.rs, plus a test that `MIN_PX_PER_UNIT` is `320/420` within 1e-12.
  - `indicator_reach_is_19`.
  - `primary_text_clears_4_5_on_marker_hollow` (palette.rs tests): light and dark.
  - `number_ink_clears_4_5_on_every_built_in_entry` extended to the dark palette.
  - test_support: `bounds` of a `HitArea` is its circle; `assert_finite` covers it.
- [ ] **Step 2–4:** red, implement (Semantics drops `Copy`; call sites), green; lints; rustqual.
- [ ] **Step 5:** Commit `feat(svg): item keys, controls, indicators, hit areas, number ink`.

---

### Task 3: the markers in `RadarBlock`

**Files:** `src/layout/radar.rs`

**Interfaces:** a private `fn markers(catalogue: &Catalogue, plotted: &Plotted) -> Vec<Marker>` (operation), `Marker { at: Point /* scale-1, centre-relative */, qualifier: Option<Qualifier>, status: Option<Status>, series: usize, number: usize, control: Control }`, built by zipping `place(..)` with `plotted.items.iter().enumerate()` (number = index + 1, FR-34). `render` appends, after the frame: the hit layer (if any markers), one control group per marker, the indicator layer (if any markers); helpers `glyph_nodes`, `number_node`, `hit_node`, `indicator_node` are operations and `render` integrates. `half_extent` gains the two per-axis reach terms (no pad). Update the module doc, `RadarScale::marks`' doc and the `half_extent` proof comment.

- [ ] **Step 1: Failing tests** (fixtures via `test_support::plotted`, all floats via `close`/`near`):
  - `every_plotted_item_is_a_control`: PRD example → one `Control` group per plotted item, in `Plotted::items` order; `key == ItemKey(item.line)`; `label == catalogue.accessible_name(item, Some(n))`; `tooltip == Some("{n}. {trimmed name}")`; a row named `"  Kafka  "` gives tooltip `"k. Kafka"`.
  - `markers_sit_where_fr33_puts_them`: independent anchor — compute `geometry.marker(position, ring, j, n)` from `Geometry::of` for named items, and compare with each control's disc centre ÷ `s`.
  - `controls_carry_names_raw`: a row named `A<&">B` → its label and tooltip contain it unchanged (escaping is the serialiser's, NFR-6).
  - `marker_colour_is_the_quadrant_rank`: `Tool, platform, Practice` rows in Adopt (first-seen 0, 1, 2; `rank = [2, 0, 1]`) → by item name: platform `Series(0)`, Practice `Series(1)`, Tool `Series(2)`; and nine quadrants → the ninth's disc is `Series(8)`, unreduced.
  - `qualifier_and_status_draw_per_fr20_fr40`: solid × {none, New, MovedIn, MovedOut} and hollow × the same four. MovedIn's half-ring ends equal `c + polar(centreward ∓ 90, 14 · marks)` with `centreward = atan2(−c.y, −c.x)` in degrees, and `c + polar(centreward, 14 · marks)` is nearer the origin than `c`; MovedOut the reverse; checked at every quadrant position of N = 4.
  - `numbers_name_the_entry_they_sit_on`: solid marker → `Fill(Paint::NumberOn(rank))` — including a quadrant whose FR-10 rank is 5 (the green entry, so its number binds to `#16191F` via `Palette::number_ink(5)`, asserted alongside), `lines == [n]`, `MARKER_NUMBER` at `marks`, anchor `Middle`, baseline offset 3.27393 at `s` = 1 and 2.54639 at `s` = 0.4.
  - `hollow_markers_number_in_primary_text`.
  - `marker_scales`: at `s` = 0.4: disc 7.7778, number 7.0, hit 16, indicator ring 14.0, number offset 2.54639; at `s` = 1: 10, 9, 16, 18, 3.27393; at `s` = 7/9 exactly and 1.5 likewise from the formulas.
  - `hit_layer_paints_before_every_marker`: the hit layer's index is below every control's; one `HitArea` per marker, same order, `radius == HIT_RADIUS`, centre on its marker, `key` matching.
  - `indicators_raise_the_marker_over_surface`: the indicator layer is the last child; per marker an `Indicator(key)` group of [Surface1 disc r = 19·marks, the marker's glyph nodes and number (equal to the control's), the `Focus` ring r = 18·marks, width 2·marks, fill `None`]; layer group semantics `None`.
  - `a_crowded_band_draws_every_marker`: 30 items in one band → 30 controls, hit areas and indicators; numbers 1..=30.
  - `unplotted_rows_draw_nothing`: rows `Zed`/Adopt, `Mid`/Nowhere, `Alpha`/Adopt (one quadrant) → 2 controls, numbers [1, 2] in FR-34 order (`Alpha` is 1 though it is the later row), keys the two Adopt lines.
  - `zero_quadrants_draws_rings_only` (extended): still 12 nodes — no hit or indicator layer.
  - `size_counts_markers`: three `Q0` rows in **Assess** (the outermost default band; N = 1, default rings): points (±212.80, 54.64), (0, 239.2); at `s` = 0.1 the width is 66 (the floor) before implementation and `2 · (212.80 · 0.1 + 16)` ≈ 74.5595 after (compute from `place`), height stays label-set (≈ 87.875); a header-only CSV at `s` = 0.02 stays 13.2 (no marker term without markers).
  - `everything_drawn_lies_inside_the_size_and_touches_it` (extended): marker fixtures added and `s` = 0.1 added; "touches" asserted as `gap == pad` only where the label term strictly beats the floor and both marker terms, and as `gap == 0` (to the hit area's edge) where the hit term strictly wins; a non-vacuity count for each of the two kinds (label, hit).
  - `size_is_non_decreasing` over the marker fixtures, adding a low sweep 0.02–0.30 in steps of 0.02 (the hit term overtakes the floor near 0.135 for the Assess fixture).
  - `assert_translates` at 1 and 0.5 with markers.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Gate** `just ci`.
- [ ] **Step 6:** Commit `feat(layout): the radar's markers (FR-20, FR-33–FR-36, FR-40, FR-48, FR-50, FR-94)`.

---

## Finishing

1. **Reviews:** final whole-branch review (most capable model) in Copilot's seat (out of credits until November 2026; say so), simplify, final check.
2. **Crumbs:** `crumbs close rg-svr`. Notes: `rg-6ku` (PR 13) — emit `Control` as `role="button" tabindex="0" aria-label data-row` + `<title>` (HTML) or `aria-label` + `<title>` only (static); skip `is_interactive_only` nodes in SVG/PNG; `Indicator(key)` as `data-indicator`, hidden by presentation attribute; `HitArea` as `fill="none" pointer-events="all" data-hit`; marker controls carry `data-row` and `data-marker`, legend-row controls (`Semantics::LegendRow`, PR 11) `data-row` only; `NumberOn(i)` via `Palette::number_ink`, a literal in every format; the number text gets `pointer-events: none`. PR 14 crumb — keep the narrow SVG edge to edge (`NARROW_HORIZONTAL_INSET_PX`) or change it and re-derive `HIT_RADIUS`; generate the per-key `:has()` rules. PR 15 crumb — forward hit-area pointer events (`data-hit`) to the marker control with the same key (`data-marker`); selection is one more child of the indicator; `aria-expanded`/`aria-controls`; verify the `<title>` description with NVDA, JAWS and VoiceOver; cursor on hit areas; the raised copy in an indicator is `pointer-events: none`, so a click on it reaches what lies beneath — route it to the focused control if that confuses. PR 11 (`rg-67j`) — legend rows are `Semantics::LegendRow(Control)` with `tooltip: None`, keyed by `ItemKey`; badge numbers use `NumberOn`; row height from `MIN_PX_PER_UNIT`.
3. Move this plan to `docs/plans/done/`; tick the steps.
4. **No version bump.** Commit, push, PR, CI; merge when clean if the maintainer authorises it.
