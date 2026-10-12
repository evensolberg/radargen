---
tags:
  - plan
  - layout
  - legend
aliases: []
doc_id: RG-011
doc_name: pr-11-legend
crumb_id: rg-67j
document_title: "PR 11 — the legend block: Implementation Plan"
created_date: 2026-10-11
synopsis: "The legend as a Block: a Surface2 panel, the 'Legend' heading, one group per quadrant in FR-10 order with its colour swatch, ring subheadings in declared order, and one row per plotted item — badge, wrapped name, inline qualifier/status — each row a control keyed by its CSV line, at least 24 CSS px tall at 320 px (FR-35, FR-46, FR-74, FR-75, FR-79, FR-94, D-54, D-56)."
status: draft
type: Plan
revision: "1.2"
review_date: 2026-10-11
reviewed_by: [rust-engineer, architect-reviewer, qa-expert, ui-designer, accessibility-tester]
completed_date:
comments: "1.1 folds in the five-seat plan review; 1.2 the scoped re-review. See Review rulings."
---
# PR 11 — the legend block: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-67j` · **Phase:** v1.0 · **After:** PR 10a/10b, rg-umt, rg-kic · **Then:** PR 12 (arrangements, `rg-ozb`)

**Goal:** A `LegendBlock` implementing `Block`: a `Surface2` panel holding the "Legend" heading, then per quadrant in FR-10 order a swatch in its FR-67 colour and its heading, then per declared ring that has plotted items a subheading, then one row per plotted item in FR-34 order — a numbered badge, the name wrapped greedily to the column (FR-46, FR-79), the qualifier and status inline after it or on their own line when they don't fit (FR-74). Each row is a control keyed by the item's CSV line, carrying FR-75's accessible name (ring included), and its target is at least 24 CSS px square at every rendered width down to 320 px (FR-94).

**Architecture:** `src/layout/legend/` (`mod.rs` and `layout.rs`). `LegendBlock::new(&Catalogue, &Plotted, &Fonts)` measures and wraps everything once into **one ordered list of laid-out entries** — headings, swatches and rows interleaved in reading order — at scale 1, content-relative; `size(s)` is that layout's panel box × `s` (the legend scales linearly; it is drawn at scale 1 in both arrangements, D-31); `render(at)` emits nodes in the list's order, so document order is reading and Tab order. `svg` gains `Node::Rect { origin, width, height, radius, fill, stroke, stroke_width }` (no dependency on `layout`) and `Semantics::LegendRow(Control)` — architecture §5's legend-row control: `data-row`, no `data-marker`.

**Tech Stack:** Rust 2024, `std` only. No new dependency.

**Spec:** `docs/product/radargen-prd.md` v0.52 → v0.53 — FR-10, FR-34, FR-35, FR-42, FR-46, FR-50, FR-67, FR-74, FR-75, FR-79, FR-82, FR-94, SC-20, D-54, D-56, D-66. Design: `docs/design/architecture.md` §3–§5. Reference: `docs/product/mockup.html` `buildLegend` (l.686–760) and CSS (l.160–190).

## Global Constraints

- Lints as strict as `Cargo.toml` says; suppress only with `#[expect(lint, reason)]` whose reason is true; rustqual markers likewise; never reshape code to hide from a lint or rustqual; no silent failures.
- **Scale is applied to coordinates, never as an SVG transform.** `Group` only translates. **No hex in a `Node`**; badge numbers use `Paint::NumberOn(rank)` (D-54).
- **Identity is the CSV line** (`ItemKey(item.line)`), never FR-34's number.
- **Order**: quadrants by `Plotted::quadrants` (FR-10; it keeps quadrants with nothing plotted), rings and rows by `Plotted::runs` (declared ring order, empty rings absent, items in FR-34 order); never re-sorted.
- **Every advance width comes from `text::Fonts`**; wrapping is `text::wrap` through `fonts.face(weight)` (FR-79's greedy first-fit, mid-token break).
- `indexing_slicing` denied; NFR-4 `*`/`+` with the per-site `suboptimal_flops` expect where it fires, never `mul_add`; halves via `f64::midpoint` or a named constant; tests compare floats with `close`/`near`; `crate::trim`, never `str::trim`. Helpers ≤ 4 parameters (bundle into a struct).
- `cargo fmt`; `cargo clippy --all-targets -- -D warnings`, `just lint`, `just ci` clean at every commit. rustqual: rises → `just quality-baseline`; falls → find the cause.
- Commits: `type(scope): description`, ≤ 72 per line, trailer `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`; `git-mit es` first.

## Review Focus

1. **A name longer than the column.** It wraps greedily; a single over-wide token breaks mid-token; nothing is truncated (FR-46, FR-79). *Test: Task 2, `long_names_wrap_and_long_tokens_break`.*
2. **Meta that does not fit after the last name line.** It takes its own line, never truncated or overflowing (FR-74). *Test: Task 2, `meta_moves_to_its_own_line_when_it_does_not_fit`.*
3. **The narrow arrangement at 320 px.** Every row's drawn target is at least 24 px tall and wide at `MIN_PX_PER_UNIT`, and rows tile without gaps or overlaps (FR-94). *Test: Task 3, `row_targets_meet_fr94_and_tile`.*
4. **A row read alone by a screen reader.** Its label carries the ring the visible row omits (FR-75), and otherwise only what the row shows (no "No Change"). *Test: Task 3, `rows_are_controls_with_the_ring_in_the_name`.*
5. **Quadrants and rings with no plotted items.** A quadrant whose rows are all unplotted keeps its heading and swatch; a ring with no items has no subheading; no row for an unplotted item; numbers contiguous (FR-17, FR-18, FR-35). *Test: Task 2, `empty_rings_and_unplotted_rows`.*
6. **The narrow document.** The legend's panel fits it exactly: content at the 20-unit margin, panel reaching into the margin (D-56). *Test: Task 1, const assert.*
7. **Reading order.** Each quadrant's heading comes before its rows, not every heading before every row. *Test: Task 3, `document_order_is_reading_order`.*

## Review rulings (plan review, 2026-10-11)

Five seats reviewed revision 1.0 (reports in the session scratchpad). Rulings:

- **Width (rust blocker, architect, a11y HIGH)**: the content wraps at D-56's 380; the panel adds `LEGEND_PANEL_INSET_X` (12) each side, so the block is up to 404 wide and **PR 12 places it at `MARGIN − LEGEND_PANEL_INSET_X`** (8), content at 20 — the mockup's arrangement. A `const` assert pins `LEGEND_PANEL_INSET_X <= MARGIN` (the panel must not reach past the document's edge; the width then fits by D-56's own arithmetic, 380 + 2·12 = 420 − 2·8). Note on `rg-ozb`.
- **Row targets catch the pointer (all five)**: a `Paint::None` rect gets no pointer events. The row target is filled with `Surface2`, the panel's own colour — invisible, pointer-receiving in every format, no new node kind; PR 15 changes its fill on hover (`Surface1`). **Focus and selection** cannot be a stroke on the target — rows abut and are opaque, so the next row paints over its lower half — so each row also carries an **indicator**: a `Rect` inset by `LEGEND_ROW_INDICATOR_INSET` (1) inside the target, fill and stroke `None`, in a `Semantics::Indicator(key)` group (interactive-only, so static formats drop it), drawn right after the target; PR 15 gives it a `Focus` stroke on focus and selection (FR-48, FR-73: selection is a stroke, since Surface1 on Surface2 is only ~1.1:1).
- **One ordered entry list (rust 3)**: `Layout { entries: Vec<Entry> }` with `Entry::{Heading { run: TextRun, level: NonZeroU8 }, Swatch { centre, series }, Row(Row)}` in reading order; render walks it.
- **Rows laid out from their top edge (rust 4, UI, architect)**: explicit constants, rows abut; see Decisions.
- **"No Change" (rust 6)**: the row shows nothing for `NoChange` (FR-40: it is the plain marker), so `Catalogue::accessible_name` — documented as mirroring the row, and shared with the markers — drops it too. A one-line change in `model.rs` with a test, recorded in PRD v0.53 (FR-50's "whichever the legend row shows" then holds literally).
- **Width rule (rust 5)**: width = the widest laid-out line's right edge, which is ≤ `LEGEND_WIDTH_MAX` because everything wraps there; the capped test asserts `≤ MAX` and equality with the widest line, not equality with `MAX`.
- **Headings wrap at their indent (QA, UI)**: a heading wider than its column (`LEGEND_WIDTH_MAX − its indent`) wraps greedily with a hanging indent at its own indent, `LINE_HEIGHT` leading; the swatch stays on the first line; following offsets hang from the last line. **PRD v0.53 FR-79**: "radar labels do not wrap; the legend's headings wrap at the column like names".
- **Heading levels (a11y)**: "Legend" `Heading(2)` (peer of the key); quadrant headings `Heading(3)`; ring headings `Heading(4)`. Swatches `Decorative`.
- **Zero quadrants (architect)**: empty legend — size zero, empty group. **PRD v0.53 FR-42** gains: with no quadrants the legend is omitted, as the title is when empty.
- **D-66 (architect)**: "the legend gets slightly taller" → one-line rows grow from 24 to 32 units, about a third.
- **SC-20 (architect, a11y)**: the focus indicator is measured against Surface2 for legend rows too; palette tests cover Focus on Surface2.
- **Contrast pairs untested (a11y)**: Task 1 adds palette tests: `TextPrimary` and `TextMuted` on `Surface2` ≥ 4.5, `Focus` on `Surface2` ≥ 3, every built-in series entry on `Surface2` ≥ 3 (the swatch; dark minimum ~3.02), both themes.
- **Small things (rust)**: `Node::Rect` takes `width`/`height` (no `layout` dependency); `NUMBER_CENTRING` moves to `text.rs` (it is a fact about Inter); headings and names are `svg::TextRun`s (no new `Line`); the PRD example fixture moves from `radar::tests` to `test_support` (`pub(crate) const PRD_EXAMPLE`); `scales_linearly` compares with `close`.
- **Not adopted**: a separate interactive-only `RowTarget` node (the Surface2 fill makes it unnecessary); `aria-hidden` on the row's inner text (a `role="button"` has presentational children in HTML; static SVG's double announcement is a PR 13 note).

## Decisions taken in this plan (flag on review)

All in `style.rs`, as named constants, scale 1, content-relative (the panel inset is added at render):

- **Heading**: "Legend", `LEGEND_HEADING`, baseline `LEGEND_HEADING_BASELINE` = 14, `TextPrimary`.
- **Quadrant group**: first quadrant baseline = heading baseline + `LEGEND_HEADING_TO_GROUP` (22) = 36; heading at `INDENT_QUADRANT`, `LEGEND_QUADRANT`, `TextPrimary`; swatch `Circle` centre (`SWATCH_RADIUS`, first baseline − `SWATCH_RAISE` (4)), radius `SWATCH_RADIUS`, fill `Series(rank)`, no stroke.
- **Ring**: baseline = the quadrant heading's last baseline + `LEGEND_QUADRANT_TO_RING` (18); `INDENT_RING`, `LEGEND_RING`, `TextMuted`.
- **Rows**: first row top = the ring heading's last baseline + `LEGEND_RING_TO_ROW` (4); each next row's top = previous top + previous height (rows abut). Row height `h = max(LEGEND_ROW_MIN_HEIGHT, n · LINE_HEIGHT + LEGEND_ROW_PAD)` with `n` = name lines + (meta on its own line ? 1 : 0), `LEGEND_ROW_MIN_HEIGHT` = 32 (const-asserted `≥ MIN_TARGET_PX / MIN_PX_PER_UNIT` = 31.5), `LEGEND_ROW_PAD` = 7. Text block `b = max(LEGEND_TEXT_MIN_HEIGHT (17), n · LINE_HEIGHT)`, centred in the row: block top = row top + (h − b)/2; first name baseline = block top + `LEGEND_FIRST_BASELINE` (9); further lines `LINE_HEIGHT` apart; badge centre y = row top + h/2, x = `INDENT_ITEM + BADGE_RADIUS`.
- **After a ring's last row**: the next ring's baseline = row bottom + `LEGEND_ROW_TO_RING` (12); the next quadrant's baseline = row bottom + `LEGEND_ROW_TO_QUADRANT` (20, the mockup's +8 between groups). A quadrant with no rows: the next quadrant baseline = its heading's last baseline + `LEGEND_EMPTY_GROUP_GAP` (26 = 18 + 8), pinned by a test.
- **Content height** = the bottom of the **last entry**, whatever it is: a row's bottom, or a heading's last baseline + `TEXT_DESCENT · size` (so a last quadrant with only unplotted rows keeps its heading inside the panel); **width** = the widest line's right edge (all ≤ 380). Panel = content + `LEGEND_PANEL_INSET_X` (12) each side, `LEGEND_PANEL_INSET_Y` (8) top and bottom, radius `LEGEND_PANEL_RADIUS` (8), fill `Surface2`.
- **Name and meta**: names at `LEGEND_NAME_X = INDENT_ITEM + 2·BADGE_RADIUS + 8` (68), wrapped at `LEGEND_WIDTH_MAX − LEGEND_NAME_X` (312); meta = `Qualifier::label` and `Status::label` joined by `" · "`, omitting an absent value and `NoChange`; inline at last line's end + `META_GAP` on the last baseline when `end + META_GAP + meta width ≤ 312` (inclusive), else its own line below at `LEGEND_NAME_X`. Meta `LEGEND_META`, `TextMuted`; name `LEGEND_NAME`, `TextPrimary`. Labels as `Qualifier::label`/`Status::label` return them (one vocabulary; the mockup's "Moved in" casing is superseded).
- **Row target**: `Rect` from x = `INDENT_ITEM − 6` to the content width + 4, the row's full height, radius `LEGEND_ROW_RADIUS` (6), fill `Surface2`, no stroke; first child of the row's control group, followed by the row's indicator group (Rulings).
- **Badge**: solid — fill and stroke `Series(rank)`, `BADGE_STROKE`; hollow (`Qualifier::is_hollow`) — fill `MarkerHollow`, stroke `Series(rank)`, `BADGE_HOLLOW_STROKE`. Number `LEGEND_BADGE_NUMBER`, middle-anchored at the badge centre, baseline + `NUMBER_CENTRING · size`, ink `NumberOn(rank)` / `TextPrimary`.
- **No version bump** (nothing user-observable yet).

---

### Task 1: PRD v0.53, `Node::Rect`, `Semantics::LegendRow`, constants, contrast tests, `accessible_name`

**Files:** `docs/product/radargen-prd.md`, `docs/design/architecture.md`, `src/svg/mod.rs`, `src/style.rs`, `src/text.rs` (`NUMBER_CENTRING`), `src/layout/radar/markers.rs` (import it), `src/layout/test_support.rs` (arms, `PRD_EXAMPLE`), `src/layout/radar/mod.rs` (use the moved fixture), `src/palette.rs` (tests), `src/model.rs` (`accessible_name` drops `NoChange`).

- [ ] **Step 1: PRD v0.53**: FR-79 (only radar labels never wrap; legend headings wrap at the column with a hanging indent), FR-42 (no quadrants ⇒ no legend), FR-50 (`NoChange` shown nowhere, so absent from the name), D-66 ("about a third taller", 24 → 32 units), SC-20 (legend focus on Surface2); revision row; `Version:`. Architecture `Implements:` v0.53 and §5 listing (`Rect`, `LegendRow`; the row target's Surface2 fill and inset focus stroke).
- [ ] **Step 2: Failing tests**: `Rect` bounds (origin..origin + (w, h) ± stroke/2) and `assert_finite`; `is_interactive_only` false for `Rect` and a `LegendRow` group; style const asserts (row height, narrow fit); palette: `text_clears_4_5_on_surface2`, `focus_clears_3_on_surface2`, `series_clear_3_on_surface2` (both themes); model: `accessible_name` of a `NoChange` item omits it (and a `New` one keeps it); `NUMBER_CENTRING` value unchanged (1490/4096).
- [ ] **Step 3–4:** implement, green; lints; rustqual.
- [ ] **Step 5:** Commits `docs(prd): v0.53 — the legend's contracts` and `feat(svg): a rectangle and the legend-row control`.

---

### Task 2: the legend's layout (pure data)

**Files:** Create `src/layout/legend/` (`mod.rs` and `layout.rs`); `src/layout/mod.rs`.

`Layout { width, height, entries: Vec<Entry> }`, `Entry::{Heading { run: TextRun, level: NonZeroU8 }, Swatch { centre: Point, series: usize }, Row(Row)}`, `Row { top, height, badge: Point, series, hollow, number: usize, name: TextRun, meta: Option<TextRun>, control: Control }`. The inline test is a pure operation `fits_inline(last_end: f64, meta_width: f64, column: f64) -> bool` (`last_end + META_GAP + meta_width <= column`). Built by an operation `layout(catalogue, plotted, fonts) -> Layout` walking `Plotted::quadrants` and `Plotted::runs` together with a y cursor.

- [ ] **Step 1: Failing tests** (literal pins, citing mockup l.686–760; fixtures from `test_support`):
  - `pins_for_the_prd_example`: heading baseline 14; first quadrant baseline 36, swatch (5, 32); first ring baseline 54; first row top 58, height 32, badge centre y 74, first name baseline 74.5; every value a literal.
  - `rows_of_one_two_and_three_lines`: one ring with 1-, 2- and 3-line names → heights 32, 37, 52 and tops abutting (literals).
  - `groups_follow_fr10_and_rings_follow_the_declaration`: first-seen `Tool, platform, Practice` → `platform, Practice, Tool`; `--rings "Hold,Assess,Trial,Adopt"` with rows in Adopt and Hold → ring headings `Hold, Adopt` in that order; a declared ring with no rows has no heading.
  - `rows_follow_fr34`: numbers 1..=n; keys are CSV lines (a fixture whose line numbers differ from item indices, e.g. with a blank line).
  - `long_names_wrap_and_long_tokens_break`: a name that fits in 312 stays one line; adding one word wraps it; a single token wider than 312 breaks mid-token; every line ≤ 312 (measured). (Exact-fit fixtures are not constructible — advances are multiples of size/2048 — so boundaries are tested on `fits_inline`.)
  - `fits_inline_is_inclusive`: `fits_inline(300.0, 4.0, 312.0)` true, `fits_inline(300.0, 4.0 + 1e-9, 312.0)` false, and literal cases either side.
  - `meta_rides_inline_when_it_fits` (x = last line end + 8, same baseline), `meta_moves_to_its_own_line_when_it_does_not_fit`, `meta_text`: `Default · New`, `Conditional`, `Moved In`, `Default` for (Default, NoChange), nothing for (none, NoChange) or absent values.
  - `headings_wrap_at_the_column`: a 60-character quadrant heading wraps at `380 − 16`, swatch on its first baseline − 4, the ring heading moved by `(lines − 1) · LINE_HEIGHT`; a heading exactly filling its column does not wrap.
  - `empty_rings_and_unplotted_rows`: a quadrant with only unplotted rows keeps heading + swatch and no rows, next group at + `LEGEND_EMPTY_GROUP_GAP`; when that quadrant is last in FR-10 order, the content height reaches its heading's descent (literal); rows Adopt/Nowhere/Adopt → two rows numbered 1, 2.
  - `widths`: a one-row legend "A" is narrow (literal width); a long name gives width ≤ 380 and equal to its widest line's right edge.
  - `zero_quadrants_lays_out_nothing`; `nine_quadrants_use_unreduced_series` (the ninth swatch is `Series(8)`).
  - `names_carried_raw`: `A<&">B` and a non-Latin name appear unchanged.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual (`layout` an integration over small operations; helpers ≤ 4 parameters).
- [ ] **Step 5:** Commit `feat(layout): the legend's layout (FR-35, FR-46, FR-74, FR-79)`.

---

### Task 3: `LegendBlock` — `Block`, controls, rendering

**Files:** `src/layout/legend/` (`mod.rs` and `layout.rs`).

`size(s)` = (width + 2·`LEGEND_PANEL_INSET_X`, height + 2·`LEGEND_PANEL_INSET_Y`) × `s` (zero for an empty layout); `render(at)` = `Group::new(at.origin, [panel Rect, Group::new(insets · s, entries…)])`, each entry × `s` (`Scale::uniform(s)`): headings as `Text` in `Heading(n)` groups, swatches as `Decorative` circles, rows as `Group(ORIGIN, [target Rect, indicator Group(Indicator(key), [inset Rect]), badge Circle, badge number Text, name Text, meta Text?]).with(Semantics::LegendRow(control))`.

- [ ] **Step 1: Failing tests:**
  - `rows_are_controls_with_the_ring_in_the_name`: one `LegendRow` per plotted item, `key == ItemKey(line)`, `label == accessible_name(item, Some(n))` (contains the ring; no "No Change"), `tooltip == None`; no `Marker` semantics.
  - `row_indicators_are_inset_and_interactive_only`: each row's second child is an `Indicator(key)` group holding one `Rect` inset by 1 on every side of the target, fill and stroke `None`; `is_interactive_only` true for it.
  - `row_targets_meet_fr94_and_tile`: for 1-, 2-, 3-line and own-line-meta rows, the drawn target `Rect` is the first child, `height · MIN_PX_PER_UNIT ≥ 24` and `width · MIN_PX_PER_UNIT ≥ 24`, fill `Surface2`; consecutive targets abut (`close`); each target contains its badge and text bounds.
  - `document_order_is_reading_order`: walking the tree, each quadrant heading precedes its own rows and follows the previous quadrant's rows.
  - `badges_match_the_markers`: solid/hollow fills, strokes and widths; number ink `NumberOn(rank)` / `TextPrimary`; number baseline offset `NUMBER_CENTRING · 9`; a three-digit number fits inside a hollow badge (or is recorded as the OQ-16 limit, as on markers).
  - `headings_and_inks`: `Heading(2/3/4)` levels; inks per level; swatches `Decorative`, fill `Series(rank)`.
  - `the_panel_is_surface2_and_holds_everything`: `test_support::assert_inside(render(at(s)), size(s))` at s ∈ {1, 0.7, 0.5}; panel `Rect` equals the size.
  - `scales_linearly`: `size(s)` = `size(1) · s` (`close`) for s ∈ {0.5, 0.7, 1.3}; `assert_translates` at 1 and 0.5.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Gate** `just ci`.
- [ ] **Step 6:** Commit `feat(layout): the legend block (FR-35, FR-50, FR-74, FR-75, FR-94)`.

---

## Finishing

1. **Reviews:** final whole-branch review (most capable model) in Copilot's seat (out of credits until November 2026; say so), simplify, final check.
2. **Crumbs:** `crumbs close rg-67j`. Notes: `rg-ozb` (PR 12) — place the legend block at `MARGIN − LEGEND_PANEL_INSET_X` in both arrangements (content at the margin; in narrow the 404-unit panel fills 8..412 exactly; in wide it sits after the radar with the same offset); an empty legend (zero quadrants) takes no space; the legend is drawn at scale 1 in both arrangements (FR-94's row height assumes it — assert it); DOM order is radar markers, then legend rows. `rg-6ku` (PR 13) — `Rect` → `<rect rx>`; `LegendRow` → HTML `role="button" tabindex="0" aria-label data-row data-legend-row` (a positive attribute, so PR 15 needs no `:not([data-marker])`); static formats: `aria-label` only, and consider `aria-hidden` on the row's inner text so a static SVG does not read number and name twice. `rg-6zu` (PR 15) — hover: target fill `Surface1`; focus and selection: a `Focus` stroke on the row's inset indicator `Rect` (rows abut, so a stroke on the target would be covered), selection distinct from hover (FR-73); FR-37 pairing by `data-row`.
3. Move this plan to `docs/plans/done/`; tick the steps.
4. **No version bump.** Commit, push, PR, CI; merge when clean if the maintainer authorises it.
