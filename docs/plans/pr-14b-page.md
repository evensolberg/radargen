---
tags:
  - plan
  - html
  - output
aliases: []
doc_id: RG-014B
doc_name: pr-14b-page
crumb_id: rg-f27
document_title: "PR 14b — the full page: Implementation Plan"
created_date: 2026-10-12
synopsis: "The HTML page carries both arrangements, drawn interactive, and a media query at the per-document breakpoint shows exactly one (FR-69). Both palettes are declared at once: light by default, dark under prefers-color-scheme, and a [data-theme] override for PR 15's toggle (FR-56). Inter is inlined as WOFF2 (NFR-5). Focus indicators are shown by generated CSS, with no JavaScript. Tests add an HTML golden, an HTML5-parser round trip of hostile input, and a cross-process determinism check (NFR-4)."
status: draft
type: Plan
revision: "1.0"
review_date:
reviewed_by: []
completed_date:
comments: "For the plan review. Part two of PR 14 (rg-f27)."
---
# PR 14b — the full page: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-f27` (part two) · **Phase:** v1.0 · **After:** PR 14a (#59) · **Then:** PR 15 (the interactive layer, table view, theme toggle, first release)

**Goal:** the page `radargen` writes is the real one:

- both arrangements, with exactly one displayed at any viewport width (FR-69);
- both palettes, chosen by the viewer's colour scheme and overridable by `[data-theme]` (FR-56);
- Inter inlined, so the page looks the same on every machine and makes no request (NFR-5, NFR-7);
- a visible focus indicator on every marker and legend row, with no JavaScript (FR-48).

**Architecture:**

- `emit::html::page(wide, narrow, title)` serialises both documents `Target::Interactive` and wraps each in a container. The stylesheet comes from a new `emit/html/css.rs`, which builds:
  - the font faces;
  - the palettes;
  - the layout and the breakpoint rule;
  - the indicator rules.
- **The stylesheet is generated, not templated.** Every value comes from a constant or the `Document`: `text::FAMILY`, the palettes' `declarations()`, `style::breakpoint_px(wide.width)`, `style::FOCUS_RING_STROKE`, the `DATA_*` attribute names, and the marker keys found by walking the wide `Document`.
- The pipeline arranges the one `Blocks` twice and passes both documents to `page`. No layout code changes.

**Tech Stack:**

- Rust 2024.
- `base64` moves from dev-dependency to dependency (MIT/Apache-2.0) to encode the vendored WOFF2.
- `html5ever` and `markup5ever_rcdom` are new dev-dependencies (MIT/Apache-2.0), for the HTML5-parser round trip. Both licences are on `deny.toml`'s allow list.

**Spec:** `docs/product/radargen-prd.md` v0.56:

- FR-48, FR-56, FR-68, FR-69, FR-70, FR-80, FR-85, FR-94
- NFR-1, NFR-4, NFR-5, NFR-6, NFR-7, NFR-12, NFR-13, NFR-17
- SC-6, SC-8, SC-15
- D-16, D-43, D-66, D-71, D-75

Design: `docs/design/architecture.md`:

- §4, the computed breakpoint;
- §5, late binding, identity, the indicator rules;
- §6, determinism;
- §10, testing (golden, determinism across processes).

## Global Constraints

- **FR-69:** the hidden arrangement is `display: none`. Never `visibility` or `opacity`, so it leaves the tab order and the accessibility tree; one radar group and one heading outline reach assistive technology.
- **No window with both or neither arrangement shown.** The wide container shows by default; one `max-width` query swaps them below the breakpoint.
- **The narrow SVG spans the viewport** (`NARROW_HORIZONTAL_INSET_PX = 0`): `body` has no margin and no padding, and the narrow SVG is `width: 100%`. FR-94's hit radius rests on this.
- **NFR-5:** the only `url(` anywhere is `url(data:font/woff2;base64,…)` inside `@font-face`, exactly two of them. Every other check from 14a stays.
- **NFR-4 and NFR-17:** byte-identical across runs and platforms. The stylesheet is LF, and every number in it is an integer or goes through `num::fmt_coord`. Marker keys are in document order, never from a hash map.
- **NFR-6:** no CSV text enters the stylesheet. Keys are CSV line numbers, written as integers.
- **Zoom is never limited** (viewport meta unchanged from 14a).
- **No version bump** (maintainer, 2026-10-12). The first release is PR 15.
- Lints, rustqual, the commit format, the `Co-Authored-By` trailer and `git-mit es` as in every plan.

## Review Focus

1. **Exactly one arrangement is displayed at every viewport width**, including fractional widths at browser zoom, and the narrow one below the breakpoint. *Test: Task 2, `one_arrangement_at_a_time`.*
2. **Both palettes are present and the overrides win in the right order:**
   - light by default;
   - dark under `prefers-color-scheme: dark`;
   - `[data-theme="light"]` beats the dark media query;
   - `[data-theme="dark"]` beats a light preference.

   *Test: Task 1, `palettes_override_in_order`.*
3. **Hostile text survives an HTML5 parser**, not only an XML one. A page with two SVGs and a stylesheet, parsed as a browser parses it, has no element, attribute or text outside what the tool wrote. *Test: Task 4, `hostile_text_survives_an_html5_parser`.*
4. **The inlined font is the vendored font.** The base64 in each `@font-face` decodes to `assets/Inter-*.woff2` byte for byte, under `text::FAMILY` and the weights the SVG text uses. *Test: Task 1, `the_font_faces_are_the_vendored_fonts`.*
5. **Two processes write the same bytes** (NFR-4), and the page matches its golden apart from the font payload. *Tests: Task 4, `two_processes_write_the_same_bytes` and `prd_page_matches_its_golden`.*

## Decisions taken in this plan (flag on review)

- **Containers:** `<div class="arrangement wide">` and `<div class="arrangement narrow">`, the wide one first in document order. Each holds one SVG.
- **Sizing:**
  - every SVG is `display: block; width: 100%; height: auto`;
  - the wide container is capped at the wide document's own width (`max-width: {W}px`, `W` through `fmt_coord`) and centred, so a large screen draws it at one CSS pixel per unit rather than blowing it up;
  - the narrow container is not capped, so it spans the viewport.
- **The breakpoint rule:** `.narrow{display:none}` and `@media (max-width: {B - 1}px){.wide{display:none}.narrow{display:block}}`, where `B = style::breakpoint_px(wide.width)`. One query, so there is no gap. At fractional widths between `B - 1` and `B`, the wide one shows, which is the safe side.
- **The palettes:**
  - `:root{color-scheme:light;…light declarations}`;
  - `@media (prefers-color-scheme: dark){:root:not([data-theme="light"]){color-scheme:dark;…dark}}`;
  - `:root[data-theme="dark"]{color-scheme:dark;…dark}`.

  `data-theme` sits on `<html>`, which PR 15's toggle sets. Declarations come from `Palette::declarations()` in its order. The series are the same in both themes but are declared in each block anyway, so each block is complete on its own.
- **The page background:** `body{margin:0;background:var(--surface-1);color:var(--text-primary)}`. The radar ground is Surface1 (FR-82), so the page and the SVG agree in both themes.
- **The fonts:**
  - two `@font-face` rules, `font-family:"Inter"` from `text::FAMILY`;
  - `font-weight:400` and `600`, `font-style:normal`, `font-display:block`;
  - `src:url(data:font/woff2;base64,…) format("woff2")`.

  `block` is the right choice because the data is inline: there is no load to wait for, and no fallback flash. A locally installed Inter is never used, since `@font-face` takes precedence for the family name.
- **The focus indicators** (architecture §5), shown with no JavaScript:
  - **Legend rows:** `[data-legend-row]:focus-visible>[data-row-indicator]{visibility:visible}`, and `[data-row-indicator]>rect{stroke:var(--focus);stroke-width:{FOCUS_RING_STROKE}px}`. The row's rect is drawn with `stroke="none"` for the interactive layer to paint.
  - **Markers:** inside `@supports selector(:has(*))`, one rule per marker key in the wide document's order: `svg:has([data-marker="L"]:focus-visible) [data-marker-indicator="L"]{visibility:visible}`. The ring is already stroked.
  - **Default outlines:** the browser's outline is removed where the indicator replaces it. For legend rows that is always. For markers it is only inside the same `@supports` block, so a browser without `:has` keeps its default outline rather than showing nothing.
- **The fallback `<h1>`, `<main>` and `lang`** stay from 14a. The visually-hidden rule moves into `css.rs`.
- **Not in this PR:**
  - **Forced-colours styling for the indicators:** the PR 15 assistive-technology pass, crumb.
  - **Selection (FR-73)**, the toggle, and the JavaScript that moves focus across a breakpoint change: all PR 15.
- **Narrow placements are computed twice,** once for FR-95 and once by `arrange`. That costs microseconds, so I'm keeping the simpler interface.
- **The golden** is `tests/golden/html/prd.html`, with each font payload replaced by `…` before the comparison and the blessing. Otherwise every edit would diff 220 kB of base64; Review Focus 4 checks the payload separately.

---

### Task 1: the stylesheet

**Files:** Create `src/emit/html/css.rs` (`mod css;` in `src/emit/html/mod.rs`); `Cargo.toml` (`base64` to `[dependencies]`); tests in `css.rs`.

**Interfaces:** produces `pub(super) fn stylesheet(wide_width: f64, markers: &[ItemKey]) -> String`, built from small functions:

- `font_faces()`;
- `palettes()`;
- `layout(wide_width)`;
- `indicators(markers)`;
- `VISUALLY_HIDDEN`.

- [ ] **Step 1: Failing tests:**
  - `the_font_faces_are_the_vendored_fonts`: two `@font-face` rules, family `text::FAMILY`, weights `400` and `600`. Each `base64,…)` payload decodes (`base64` engine `STANDARD`) to `include_bytes!("../../../assets/Inter-Regular.woff2")` and `…SemiBold.woff2` respectively.
  - `palettes_override_in_order`:
    - the `:root{…}` block holds exactly `Palette::built_in(Light).declarations()` as `name:value;`, with `color-scheme:light`;
    - the `prefers-color-scheme: dark` block's selector is `:root:not([data-theme="light"])` and holds the dark declarations;
    - a `:root[data-theme="dark"]` block holds the same dark declarations;
    - the order in the stylesheet is light, media, `[data-theme]`.
  - `the_breakpoint_is_the_documents`: for widths 976.817 and 1200, the rule is `@media (max-width: {B-1}px){.wide{display:none}.narrow{display:block}}` with `B = breakpoint_px(w)`; `.narrow{display:none}` precedes it; the wide cap is `max-width:{fmt_coord(w)}px`.
  - `one_rule_per_marker_key`: for keys `[6, 7, 2]`, three rules in that order, each `svg:has([data-marker="6"]:focus-visible) [data-marker-indicator="6"]{visibility:visible}`, all inside one `@supports selector(:has(*)){…}`. No keys gives no `@supports` block.
  - `the_stylesheet_requests_nothing_but_fonts`: `url(` appears exactly twice, each followed by `data:font/woff2;base64,`; no `@import`; no `http`.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(emit): the page's stylesheet: fonts, palettes, breakpoint, focus`.

---

### Task 2: both arrangements on the page

**Files:**

- `src/emit/html/mod.rs`: `page(wide: &Document, narrow: &Document, title: Option<&str>) -> String`; `fn marker_keys(&Document) -> Vec<ItemKey>`.
- `tests/serialise/page.rs`.
- `tests/common/html.rs`: `split` becomes `svgs(page) -> Vec<&str>`, and the allowlist adds `div`. The stylesheet check allows exactly the two font `url(data:font/woff2;base64,`.

**Interfaces:**

- Consumes: Task 1's `stylesheet`.
- Produces: the new `page` signature for Task 3.

- [ ] **Step 1: Failing tests** (in `tests/serialise/page.rs`, the PRD example's wide and narrow documents):
  - `one_arrangement_at_a_time` (Review Focus 1):
    - the page holds exactly two SVGs, the wide first, in `<div class="arrangement wide">` and `<div class="arrangement narrow">`;
    - the stylesheet hides `.narrow` by default and swaps both inside the one `max-width` query;
    - no rule uses `visibility` or `opacity` on `.wide` or `.narrow`.
  - `both_are_interactive`: each SVG has `role="button"` controls with `data-row`, and the same set of `data-marker` keys.
  - `markers_have_indicator_rules`: every `data-marker` value in the wide SVG has its `:has` rule, and no rule names a key absent from it.
  - The 14a tests carry over: `the_page_is_minimal_and_valid` (now with two SVGs inside `<main>`), `the_page_requests_nothing`, `the_title_falls_back`, `a_hostile_title_is_escaped`.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(emit): both arrangements, interactive, one shown at a time`.

---

### Task 3: the pipeline writes the full page

**Files:** `src/pipeline.rs` (`render` arranges Narrow too); `tests/run.rs`.

- [ ] **Step 1: Failing test:** `renders_the_prd_example` asserts two `<svg` elements and the `@font-face` rules. The rest of `tests/run.rs` stands. The hostile and allowlist checks now cover both SVGs and the stylesheet through Task 2's helpers.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(pipeline): write both arrangements`.

---

### Task 4: the golden, the HTML5 round trip, and determinism

**Files:**

- `tests/golden/html/prd.html`;
- `tests/serialise/page.rs`, for the golden and the round trip;
- create `tests/determinism.rs`;
- `Cargo.toml` dev-dependencies `html5ever`, `markup5ever_rcdom`.

- [ ] **Step 1: Tests:**
  - `prd_page_matches_its_golden`:
    - the PRD example's page, with each `base64,…)` payload replaced by `base64,…)`, equals the golden;
    - blessing follows §10: `UPDATE_GOLDEN=1` writes it and fails, and it refuses under `CI`, as the serialiser goldens do.
  - `hostile_text_survives_an_html5_parser` (Review Focus 3):
    - the hostile catalogue from `tests/serialise/hostile.rs`, with a hostile title, rendered and parsed with `html5ever`;
    - the tree's element names are only the page's and the SVG vocabulary, with no `script` anywhere;
    - there is exactly one `style` and one `title`;
    - every hostile string appears as text or as an `aria-label` value, unchanged.
  - `two_processes_write_the_same_bytes` (NFR-4): the built binary run twice in separate processes on `three-quadrant.csv`, into two files; the bytes are equal.
- [ ] **Step 2:** bless the golden and read it: the head, the stylesheet with the payloads elided, both SVGs.
- [ ] **Step 3:** `just ci`. Render the PRD example and the template CSV; look at both arrangements, by viewport width, in light and dark.
- [ ] **Step 4: Commit** `test: the page's golden, an HTML5 round trip, determinism across processes`.

---

## Finishing

1. **Reviews:** a final whole-branch review on the most capable model in Copilot's seat (Copilot is out of credits until November 2026; say so), then simplify, then a final check.
2. **Crumbs:**
   - `crumbs close rg-f27`.
   - A new crumb for forced-colours indicators, for the PR 15 assistive-technology pass.
   - A note on `rg-6zu` (PR 15):
     - the toggle sets `data-theme` on `<html>`;
     - on a breakpoint change, focus moves to the same key in the newly shown arrangement;
     - selection styling (FR-73) goes beside the generated focus rules.
3. Move this plan to `docs/plans/done/`, tick the steps, and set `status: complete` and `completed_date`.
4. **No version bump.** Commit, push, open the PR, check CI on all three platforms, and merge when clean if the maintainer authorises it.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-12 | Initial version, for the plan review. |
