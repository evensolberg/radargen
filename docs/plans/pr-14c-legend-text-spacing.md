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
synopsis: "Headroom for WCAG 1.4.12 (maintainer ruling): every wrap point and block size uses the text width with 1.4.12 spacing added, so spaced text fits by construction. A legend row's qualifier and status become a tspan that follows the name inside the name's own text element, so text-spacing overrides (WCAG 1.4.12) push them along instead of overlapping them. Node::Text gains an optional inline tail; the serialiser writes it as a tspan with dx. Rendering is unchanged at default spacing, checked against resvg and in a browser with 1.4.12's spacing applied."
status: draft
type: Plan
revision: "1.2"
review_date: 2026-10-12
reviewed_by:
  - Rust seat
  - QA seat
  - architecture seat
  - accessibility seat
  - UI seat
completed_date:
comments: "1.1 folds in the five-seat review and the maintainer's headroom ruling. Lands before PR 15."
---
# PR 14c — legend text spacing: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-mru` (P1) · **Phase:** v1.0 · **After:** PR 14b (#60) · **Before:** PR 15

**Goal:** the radar and legend stay readable, with nothing lost or overlapping, when a reader applies WCAG 1.4.12's text spacing: letter spacing 0.12em, word spacing 0.16em, line height 1.5 and paragraph spacing 2em. Two defects today:

- **Overlap.** A legend row draws its name and its meta (qualifier and status, FR-74) as two `<text>` elements, the meta at an x computed from the name's measured width. A wider name runs into it.
- **Loss.** Spaced text is about 15–25% wider. Where it passes the SVG's edge it is clipped. Where the SVG fills the viewport (the narrow arrangement on a phone), overflowing text cannot be scrolled to even with `overflow: visible`, because Chromium treats SVG overflow as ink, not scrollable overflow (verified by the QA and UI seats).

**The maintainer's ruling (2026-10-12): headroom.** Layout reserves the room spaced text needs, so it fits by construction. The visible cost is accepted: names wrap sooner, panels are a little wider, and radar labels reserve more room.

**Architecture:**

- **Layout width.**
  - `TextStyle::measure` and `TextStyle::wrap`, the only ways a block measures or wraps text, return and wrap against the *layout width*: the shaper's advance (FR-84) plus 1.4.12's allowance of `0.12 × size` per character and `0.16 × size` per word separator (U+0020, U+00A0).
  - Every block's size, every wrap point, every row target and every document extent then holds spaced text.
  - `FontMetrics::measure` stays the pure shaper advance, and the FR-84 oracle tests keep checking it against `resvg`.
- **Positions still come from the natural advance where text continues other text.** That is the legend's meta.
  - `svg::Node::Text` gains `tail: Option<Tail>`, text continuing the last line in its own style, after a gap. The serialiser writes it as `<tspan dx>` inside the same `<text>`, so the browser places the meta after the name's rendered end, whatever the spacing.
  - A meta that does not fit inline takes its own line (FR-74) and stays a separate `<text>`.
- **Centred labels** (quadrant and ring names) grow in both directions under spacing. Their reach uses the layout width, so the radar block holds them.

**Tech Stack:** Rust 2024; no new dependency.

**Spec:** `docs/product/radargen-prd.md`, v0.59 after Task 0:

- NFR-1 (WCAG 2.1 AA, which includes 1.4.12)
- FR-46, FR-70, FR-74, FR-79, FR-84, FR-94, FR-95
- D-71, D-78
- SC-17

Design: `docs/design/architecture.md` §4 (blocks size from measured text), §5 (`Document`), §11 (the SVG outliner).

## Global Constraints

- **Spaced text fits.** With 1.4.12's four properties applied, no text overlaps another and none passes the edge of its arrangement's viewBox. This is asserted in a browser (Task 4) and structurally by `test_support::bounds`, whose text bounds use the layout width.
- **Text Inter covers renders where the layout says, at default spacing.** The tail's `dx` is a gap from the name's rendered end. For text Inter covers, that matches the shaper to a fraction of a unit. For fallback-font text (scripts outside FR-85), the browser's width wins, which is still correct and no worse.
- **FR-84 is unchanged:** the shaper `resvg` uses measures every advance. The allowance is added on top, by one function.
- **The tail has a leading space** (`dx = META_GAP − the space's advance at LEGEND_META`, asserted ≥ 0), so copy, find-in-page and assistive technology read "Kubernetes Default", not "KubernetesDefault". Nothing else goes between the name and the tspan: a newline would collapse to an extra space.
- **NFR-6:** the tail's text goes through `xml_text`. **NFR-17:** `dx` goes through `fmt_coord`.
- **The accessible name is unchanged** (FR-75), checked by comparing the accessibility tree before and after.
- **No version bump** (the first release is PR 15).
- Lints, rustqual, the commit format, the trailer and `git-mit es` as in every plan.

## Review Focus

1. **Under 1.4.12 spacing, every text's box sits inside its arrangement's viewBox and intersects no other text**, at viewport widths 320, 420, B − 1, B and 1400, on the PRD example, the template and the named fixtures. *Test: Task 4's committed browser check.*
2. **The tail continues the name** in one-line and wrapped names, with the space between them. *Tests: Task 1 and Task 3.*
3. **The headroom's cost is what was accepted.** Names wrap sooner, panels widen, and narrow documents widen past 480 more often (FR-95 warns). The goldens and D-71's figures are updated deliberately. *Test: the re-blessed goldens are read, and D-71's figures are re-measured.*
4. **The allowance is computed in one place**, and every sizing site uses it. *Test: Task 2, `layout_width_is_advance_plus_allowance`, and the bounds checks that already run over every block.*
5. **Nothing in the static outputs changes except layout:** PNG and SVG lay out with the same headroom. That costs a little room and keeps one `Document` (§5).

## Decisions taken in this plan

- **The allowance (D-78):** `0.12 × size × chars + 0.16 × size × separators`, where `chars` is the `char` count, a slight over-estimate for combining marks, which is the safe side. Letter spacing in a style is already inside the shaper's advance; the allowance is added on top. 1.4.12's line height and paragraph spacing have no effect on SVG text placed at absolute `y`, so lines cannot collide vertically by construction.
- **Every block uses it**, through `TextStyle::measure` and `wrap`:
  - title lines;
  - key labels;
  - legend headings, names and meta;
  - quadrant and ring labels.

  One rule, no list of exemptions to maintain.
- **`Tail { gap, style, ink: Paint, text }`.** `Node::tail_x(fonts)` is the one place the tail's x is computed (bounds, the oracle test, PR 17's outliner). `halo()` drops the tail. A tail requires `Anchor::Start` (debug-asserted). The tail tspan writes `letter-spacing` whenever it differs from its text's, `0` included, so it never inherits the name's.
- **No `svg{overflow:visible}`.** The QA and UI seats showed it cannot prevent loss where the SVG fills the viewport, and headroom makes it unnecessary. The default clipping now guards against any regression the browser check would catch.
- **The row target covers the meta's layout width,** so the focus ring encloses the label under spacing. This also resolves the crumb's default-spacing note (the meta reaching the ring's edge).
- **The tail fix stays** even with headroom: without it, a wider name still runs into the meta's fixed x.
- **Rejected:**
  - `overflow:visible` alone, which loses content at narrow widths;
  - scoping NFR-1's claim, which leaves an AA exception;
  - an HTML legend, which is a redesign that breaks the shared `Document`;
  - deferring past PR 15.

  The first three were put to the maintainer, who chose headroom.

## Review rulings

**From the Rust seat** (its report arrived after 1.1):

- **Cross-boundary kerning** (blocker): `usvg` shapes a whole text chunk, so a kern pair between the name's last glyph and the meta's first is applied across the tspan. Probed: names ending `A L T V Y .` before `Conditional`, and `r v y` before `Default`/`New`/`Moved In`, drew sub-pixel differently from today's separate `<text>`. Browsers likely split shaping at the size change, so HTML and PNG could disagree (SC-7).
  - **Ruling:** the tail's leading space makes the pair at the boundary *letter–space*, which Inter does not kern.
  - **The oracle's corpus must include names ending in each of `A L T V Y r v y .`** before each meta label, at 2x, with the 0.5 px-shift case as the discrimination check.
  - If any pair still drifts, the gap is corrected by the kern `resvg` applies, and the residual between engines is bounded in the browser check. A ruling either way is recorded at Task 3.
- **Shape:** the tail lives on `Node::Text` only, not on `TextRun`. `Row`'s meta becomes `enum Meta { Inline(Tail), OwnLine(TextRun) }`, so "inline and on its own line" cannot be built, and `TextRun::node_with_tail` writes the inline form. That saves `tail: None` at eight `TextRun` sites. `Tail::scaled` holds the arithmetic; `serialise::text::Run` borrows `Option<&Tail>`; `text()` appends the tail after `content()`.
- **Sites that compile but would skip a tail:** `bounds.rs` `assert_finite`, `tests/serialise/hostile.rs` `Strings::walk` (the tail's text joins the line's), and `tests/serialise/vocabulary.rs`'s every-kind fixture (gains a tail, covering `dx`).
- **The oracle has no existing pixel-diff helper:** about 25 lines in a new test, building the old form by walking the `Document`, with a count bound of about 64 px at 2x.

**From the other seats:**

Five seats reviewed revision 1.0 (Rust, QA, architecture, accessibility, UI). The QA seat's blocker reversed the overflow decision: `overflow:visible` does not create scrollable overflow for SVG content. The maintainer ruled headroom. Folded in:

- the tail's leading space (accessibility);
- `tail_x` as the one source (architecture);
- the halo and anchor guards and explicit `letter-spacing` on the tail (architecture, UI);
- a committed browser check over every text pair, with named fixtures and a non-zero count of tails (QA, accessibility);
- an independent oracle, against the committed pre-change golden rendered by `resvg` (QA);
- the accessibility-tree comparison (accessibility);
- SC-17 gaining 1.4.12 (accessibility);
- the claim limited to text Inter covers (UI);
- the design and spec docs updated in this PR (architecture).

---

### Task 0: the spec and the design

**Files:** `docs/product/radargen-prd.md` (v0.59), `docs/design/architecture.md`.

- [ ] **Step 1:** In the PRD:
  - **D-78** (headroom for 1.4.12: the allowance, why layout reserves it, the accepted cost, and the rejected alternatives);
  - **NFR-1** names 1.4.12 as met by construction (D-78);
  - **FR-79 and FR-84:** wrap points and block sizes use the layout width, while the shaper still gives every advance;
  - **FR-74:** the inline meta continues the name's own text;
  - **SC-17** adds a text-spacing check, with the four values, both arrangements and both themes;
  - **D-71's figures** re-measured (Task 2 gives them).
- [ ] **Step 2:** In the architecture:
  - §4: blocks size from the layout width;
  - §5: `Node::Text` gains `tail`;
  - §11: the outliner places a tail through `tail_x`.
- [ ] **Step 3: Commit** `docs: headroom for text spacing (D-78)`.

---

### Task 1: `Tail` in the document and the serialiser

**Files:**

- `src/svg/mod.rs`: `Tail` (with `scaled`), `Node::Text { tail }`, `Node::tail_x`, and `TextRun::node_with_tail`;
- `src/svg/serialise/text.rs`;
- every `Node::Text` construction gets `tail: None` (seven sites); `TextRun` is unchanged;
- `src/layout/test_support/bounds.rs`, through `tail_x`.

- [ ] **Step 1: Failing tests** (`src/svg/serialise/tests/text.rs`, `svg/mod.rs`):
  - `a_tail_follows_the_last_line`:
    - one line: `<text …>Kafka<tspan dx="…" font-size="10.5" font-weight="400" letter-spacing="0" fill="var(--text-muted)"> Default</tspan></text>`, with no text node between;
    - two lines: after the last `<tspan>`;
    - static target: a literal fill.
  - `a_tail_is_escaped`: `<&>` arrives as `&lt;&amp;&gt;`.
  - `a_tail_scales_with_its_run`.
  - `a_halo_has_no_tail`: `TextRun::halo` never carries one.
  - `bounds_include_the_tail`, on a two-line name, where the tail extends the last line, not the widest.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(svg): text with an inline tail, written as a dx tspan`.

---

### Task 2: the layout width

**Files:** `src/style.rs` (`TextStyle::measure`, `wrap`, and `fn allowance`); `src/text.rs` (`wrap` takes a width function or an allowance); the tests whose figures move; the D-71 figures for Task 0.

- [ ] **Step 1: Failing tests:**
  - `layout_width_is_advance_plus_allowance`: for `"Kafka Streams"` at `LEGEND_NAME`, `measure` equals `FontMetrics::measure + 0.12·12.5·13 + 0.16·12.5·1`.
  - `wrap_holds_spaced_lines`: every line `wrap` returns has a layout width ≤ the column.
  - The existing `test_support::bounds` and arrangement checks run unchanged, and now hold spaced text.
- [ ] **Step 2–4:** red, implement, green. Update the tests whose figures were measured at the natural advance (the arrange fixtures' widths, and the legend's wrap thresholds), each with its new measured value in the comment. Lints; rustqual.
- [ ] **Step 5: Commit** `fix(layout): reserve room for WCAG 1.4.12 text spacing (D-78)`.

---

### Task 3: the legend's meta is the name's tail

**Files:** `src/layout/legend/row.rs`, `src/layout/legend/mod.rs`; re-bless `tests/golden/**` and read the diffs.

- [ ] **Step 1: Failing tests:**
  - `an_inline_meta_is_the_names_tail`: one `Text` with a tail of ` Default`, and no separate meta `Text`.
  - `a_meta_on_its_own_line_stays_separate`.
  - `the_row_target_covers_the_meta`: the target's right edge is at least the meta's layout end plus the outset.
  - `the_tail_draws_where_the_meta_did` (oracle): the committed pre-change static golden and the new static output, both rendered by `resvg`, agree on the meta's ink position for the PRD example's rows. Measured as each row's leftmost meta-ink column, within one pixel at 2x. The layout change moves the rest, so only the meta's offset from its name is compared.
- [ ] **Step 2–4:** red, implement, green; re-bless and read the diffs; lints; rustqual.
- [ ] **Step 5: Commit** `fix(legend): the meta follows the name, with a space (rg-mru)`.

---

### Task 4: the committed browser check

**Files:** create `scripts/text-spacing.mjs` (CDP against the Playwright headless Chromium, as in 14b), add a `just text-spacing` recipe, and add fixtures under `tests/fixtures/` named for what they hold.

- [ ] **Step 1: The check.** For each fixture, at widths 320, 420, B − 1, B and 1400, inject `*{letter-spacing:.12em!important;word-spacing:.16em!important;line-height:1.5!important} p{margin-bottom:2em!important}`. Then:
  - every displayed `<text>`'s extent, from `getExtentOfChar` over its characters, lies inside its SVG's box;
  - no two displayed texts' extents intersect, excluding a halo and its fill, and a marker number and its own disc;
  - every tail starts at or after its name's last character's end;
  - the number of tails found is non-zero and as expected per fixture;
  - the accessibility tree's names and roles are unchanged from a run without the stylesheet.

  The script exits non-zero on failure.
- [ ] **Step 2: Fixtures:**
  - qualifier only; status only; both;
  - a wrapped name with an inline meta; a name at the inline threshold; an own-line meta; no meta;
  - a hostile name with a tail;
  - one right-to-left name, for a screenshot only;
  - the PRD example and the template.
- [ ] **Step 3:** run it, read the screenshots, record the results in the PR body. Firefox and Safari are not automated; that is a known limitation, crumbed.
- [ ] **Step 4:** `just ci`.
- [ ] **Step 5: Commit** `test: the text-spacing browser check (WCAG 1.4.12)`.

---

## Finishing

1. **Reviews:** a final whole-branch review on the most capable model in Copilot's seat (Copilot is out of credits until November 2026; say so), then simplify, then a final check.
2. **Crumbs:**
   - `crumbs close rg-mru`;
   - a note on `rg-m2l` (PR 17): the outliner places tails through `tail_x`;
   - a note on `rg-6zu` (PR 15): its panel and table layout under 1.4.12 is checked with the same script;
   - a crumb for Firefox and Safari coverage of the text-spacing check.
3. Move this plan to `docs/plans/done/`, tick the steps, and set `status: complete` and `completed_date`.
4. **No version bump.** Commit, push, open the PR, check CI on all three platforms, and merge when clean if the maintainer authorises it.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-12 | Initial version, for the plan review. |
| 1.1 | 2026-10-12 | The five-seat review: `overflow:visible` cannot prevent loss (QA, UI). The maintainer ruled headroom: the layout width adds 1.4.12's allowance everywhere (D-78). The tail gains a leading space, `tail_x`, and guards. A committed browser check over every text pair; an independent oracle. |
| 1.2 | 2026-10-12 | The Rust seat: cross-boundary kerning answered by the leading space and pinned by an oracle corpus; the tail on `Node::Text` only, with `Meta::{Inline, OwnLine}`; sites that would skip a tail. |
