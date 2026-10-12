---
tags:
  - plan
  - a11y
  - legend
aliases: []
doc_id: RG-014C
doc_name: pr-14c-legend-text-spacing
crumb_id: rg-mru
document_title: "PR 14c — legend text spacing: Implementation Plan"
created_date: 2026-10-12
synopsis: "A legend row's qualifier and status become a tspan that follows the name inside the name's own text element, so text-spacing overrides (WCAG 1.4.12) push them along instead of overlapping them. Node::Text gains an optional inline tail; the serialiser writes it as a tspan with dx. Rendering is unchanged at default spacing, checked against resvg and in a browser with 1.4.12's spacing applied."
status: draft
type: Plan
revision: "1.0"
review_date:
reviewed_by: []
completed_date:
comments: "For the plan review. Lands before PR 15, which builds selection styling on the legend row's markup."
---
# PR 14c — legend text spacing: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-mru` (P1) · **Phase:** v1.0 · **After:** PR 14b (#60) · **Before:** PR 15

**Goal:** a legend row stays readable when a reader applies WCAG 1.4.12's text spacing: letter spacing 0.12em, word spacing 0.16em, line height 1.5 and paragraph spacing 2em. Today a row draws its name and its meta (qualifier and status, FR-74) as two `<text>` elements. The meta starts at an x computed from the name's measured width, so a wider name runs into it.

**Architecture:**

- `svg::Node::Text` gains `tail: Option<Tail>`: text that continues the last line in its own style and ink, after a gap. `TextRun` carries the same field and scales it with the run.
- The serialiser writes a tail as a `<tspan dx="{gap}" font-size font-weight [letter-spacing] fill>` closing the text's content, inside the same `<text>`. Whatever widens the name, the browser places the meta after the name's rendered end.
- The legend's inline meta becomes the name run's tail. A meta that does not fit takes its own line (FR-74) and stays a separate `<text>` at the row's left edge, where spacing cannot make it collide.
- No other block uses a tail.

**Tech Stack:** Rust 2024; no new dependency.

**Spec:** `docs/product/radargen-prd.md` v0.57:

- NFR-1 (WCAG 2.1 AA, which includes 1.4.12)
- FR-46, FR-74, FR-75, FR-79, FR-84
- NFR-4, NFR-6, NFR-17
- SC-7, SC-17

Design: `docs/design/architecture.md` §5, `Document` and its two per-format differences. A tail adds no third: every format writes it the same way.

## Global Constraints

- **No visible change at default spacing.** The meta's position comes from the same measurement as today (`LEGEND_NAME_X + last_end + META_GAP`), now expressed as the gap after the name's measured end. The goldens change in markup only. A resvg raster of the PRD example's static document matches today's within anti-aliasing.
- **FR-84 holds:** the meta's position still comes from the shaper `resvg` uses. `dx` is relative to the end of the name's last chunk, which the browser and `resvg` lay out with the same face.
- **NFR-6:** the tail's text goes through `xml_text` like every line.
- **NFR-17:** `dx` goes through `fmt_coord`.
- **The accessible name is unchanged** (FR-75). The tail is drawn text inside a control whose label already carries it.
- **No version bump** (the first release is PR 15).
- Lints, rustqual, the commit format, the trailer and `git-mit es` as in every plan.

## Review Focus

1. **With 1.4.12 spacing applied, no legend row's meta overlaps its name**, and wrapped names keep their lines apart. *Test: Task 3, a browser check with a user stylesheet setting the four properties; the meta's box starts at or after the name's last line ends.*
2. **At default spacing the meta draws where it did.** *Test: Task 2, `the_tail_draws_where_the_meta_did`, comparing resvg rasters of the old and new markup.*
3. **The meta never draws outside the page.** With 1.4.12 spacing a name at the column's edge grows past the SVG's right edge, and an inline SVG clips its overflow by default. *Decision: `svg{overflow:visible}` (Task 3), so grown text spills rather than vanishes. A loss of content is the failure 1.4.12 names; a spill is not.*
4. **A meta on its own line** (a long name) is unchanged, and still a separate `<text>`.
5. **Hostile meta text cannot occur,** since the meta is built from the tool's own labels. Escaping is still applied, and tested with a hostile name in the same text element.

## Decisions taken in this plan (flag on review)

- **`Tail { gap: f64, style: TextStyle, ink: Paint, text: String }`.** `gap` is the distance from the end of the last line. `ink` is a `Paint`, because the meta is `TextMuted` against the name's `TextPrimary`. No halo: nothing that halos has a tail.
- **Written only on the last line.** A one-line name with a tail becomes `name<tspan dx…>meta</tspan>`. A multi-line name puts it after the last line's `<tspan>`, with no whitespace between, keeping 14b's rule.
- **The `dx` tspan restates `font-size`, `font-weight` and `fill`,** plus `letter-spacing` when the meta style has one. It inherits `font-family`.
- **`svg{overflow:visible}` on the page** (Review Focus 3).
  - The wide container's cap and centring are unchanged.
  - Grown text can now overlap the page margin, or cause horizontal scroll on the narrow arrangement. Both are acceptable under 1.4.12 and only happen with spacing overridden.
- **The row's right edge** (the crumb's note: "Default" reaching the focus ring on long names) is checked in Task 3. If the meta's measured end exceeds the row target's right edge, the target widens to cover it plus `LEGEND_ROW_RIGHT_OUTSET`. A crumb if it is not a one-line fix.
- **Not in scope:** the other blocks. Title lines, key labels, ring and quadrant labels are single `<text>`s, each positioned on its own, so spacing cannot push one into another of the same row. Label-to-label collisions on the radar under 1.4.12 are a separate question, crumbed if the browser check shows one.

---

### Task 1: `Tail` in the document and the serialiser

**Files:**

- `src/svg/mod.rs`: `Tail`, `Node::Text { tail }`, `TextRun { tail }` and its scaling;
- `src/svg/serialise/text.rs`: `Run { tail }`, written as the closing tspan;
- every `Node::Text`/`TextRun` construction gets `tail: None`;
- `src/layout/test_support/bounds.rs`: text bounds include the tail;
- tests in `src/svg/serialise/tests/text.rs`.

- [ ] **Step 1: Failing tests:**
  - `a_tail_follows_the_last_line`:
    - one line: `<text …>Kafka<tspan dx="6" font-size="11" font-weight="400" fill="var(--text-muted)">Default</tspan></text>`;
    - two lines: the tail after the second `<tspan>`, nothing between;
    - static target: `fill` is the literal.
  - `a_tail_is_escaped`: tail text `<&>` arrives as `&lt;&amp;&gt;`.
  - `a_tail_scales_with_its_run`: `TextRun::node` at a size factor of 2 doubles the gap and the tail's size.
  - `bounds_include_the_tail` (`test_support`): the text's right edge is `last_end + gap + tail width`.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(svg): text with an inline tail, written as a dx tspan`.

---

### Task 2: the legend's inline meta is the name's tail

**Files:** `src/layout/legend/row.rs`, `src/layout/legend/mod.rs` (the render), and tests in each. Re-bless `tests/golden/serialiser/*` and `tests/golden/html/prd.html`, then read the diff.

- [ ] **Step 1: Failing tests:**
  - `an_inline_meta_is_the_names_tail`: the PRD example's Kubernetes row renders one `Text` whose tail is `Default`, gap `META_GAP`, style `LEGEND_META`, ink `TextMuted`, and no separate meta `Text`.
  - `a_meta_on_its_own_line_stays_separate`: a name long enough to push the meta down gives two `Text` nodes, the meta at `LEGEND_NAME_X` on the next baseline, and no tail.
  - `the_tail_draws_where_the_meta_did` (`tests/agrees_with_resvg.rs` or a new oracle test): the PRD example's wide static document, rendered by resvg, against the same document with each tail split back into a separate `<text>` at today's computed x. The rasters differ by no more than anti-aliasing (a per-pixel tolerance and a count bound, as the existing oracle tests use).
- [ ] **Step 2–4:** red, implement, green; re-bless the goldens and read the diff (markup moves, no coordinates change); lints; rustqual.
- [ ] **Step 5: Commit** `fix(legend): the meta follows the name, so text spacing cannot overlap it (rg-mru)`.

---

### Task 3: the page, and the browser check

**Files:** `src/emit/html/css.rs` (`svg{…overflow:visible}`), its tests, `tests/golden/html/prd.html`; possibly `src/layout/legend/mod.rs` (the row's right edge).

- [ ] **Step 1: Failing test:** `the_layout_is_exact` includes `overflow:visible` in the SVG rule.
- [ ] **Step 2–4:** red, implement, green.
- [ ] **Step 5: Browser check** (headless Chromium over CDP, as in 14b), on the PRD example and the template CSV, wide and narrow:
  - Inject `*{letter-spacing:.12em!important;word-spacing:.16em!important;line-height:1.5!important}`.
  - For every legend row, the meta tspan's bounding box starts at or after the name's last tspan ends. Nothing is clipped: the SVG's `getBoundingClientRect` contains each text's box, or overflow is visible.
  - Take screenshots.
  - Also check the default-spacing screenshot against 14b's for the crumb's right-edge note.
- [ ] **Step 6:** `just ci`.
- [ ] **Step 7: Commit** `fix(emit): grown text spills rather than clips (WCAG 1.4.12)`.

---

## Finishing

1. **Reviews:** a final whole-branch review on the most capable model in Copilot's seat (Copilot is out of credits until November 2026; say so), then simplify, then a final check.
2. **Crumbs:** `crumbs close rg-mru`. New crumbs for any radar-label collision the browser check finds, and for the row's right edge if it is not fixed here.
3. Move this plan to `docs/plans/done/`, tick the steps, and set `status: complete` and `completed_date`.
4. **No version bump.** Commit, push, open the PR, check CI on all three platforms, and merge when clean if the maintainer authorises it.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-12 | Initial version, for the plan review. |
