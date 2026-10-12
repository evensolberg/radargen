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
revision: "1.1"
review_date: 2026-10-12
reviewed_by:
  - Rust seat
  - QA seat
  - architecture seat
  - accessibility seat
  - UI seat
completed_date:
comments: "1.1 folds in the five-seat plan review. See Review rulings. Part two of PR 14 (rg-f27)."
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
- `html5ever` and `markup5ever_rcdom` 0.39 are new dev-dependencies (MIT/Apache-2.0), for the HTML5-parser round trip. Both licences are on `deny.toml`'s allow list.

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
- **The narrow SVG spans the viewport up to its own width** (`NARROW_HORIZONTAL_INSET_PX = 0`): `body` has no margin and no padding, and the narrow SVG is `width: 100%` of a container capped at its document width. FR-94's hit radius rests on the spanning below that width.
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

## Decisions taken in this plan

- **Containers:** `<div class="arrangement wide">` and `<div class="arrangement narrow">`, the wide one first in document order. Each holds one SVG.
- **Sizing (FR-51):**
  - every SVG is `display:block;width:100%;height:auto`;
  - **each container is capped at its own document's width and centred**: `.wide{max-width:{W}px;margin:0 auto}` and `.narrow{max-width:{N}px;margin:0 auto}`, with `W` and `N` through `fmt_coord`. A large screen then draws each at one CSS pixel per unit rather than enlarging it. At or below `N` the narrow SVG still spans the viewport, which is where FR-94's `NARROW_HORIZONTAL_INSET_PX = 0` binds.
- **The breakpoint rule:** `.narrow{display:none}`, then `@media {narrow_query}{.wide{display:none}.narrow{display:block}}`, where `fn narrow_query(wide_width) -> String` gives `(max-width: {B - 1}px)` and `B = style::breakpoint_px(wide.width)`, with `saturating_sub(1)`.
  - One query and a default, so there is no width where both or neither show.
  - Range syntax `(width < Bpx)` would close the fractional gap between `B - 1` and `B`, but Safari before 16.4 drops the whole rule. Keep `max-width`, and say so in a comment.
  - A classic scrollbar counts towards the query width, so at exactly `B` the wide legend draws near 9.8 px rather than 10. Accepted, and noted in a comment.
  - `narrow_query` is the one source of the query. PR 15's `matchMedia` reads it (note on `rg-6zu`).
- **The palettes (§5):**
  - `:root{color-scheme:light;…light}`;
  - `@media (prefers-color-scheme: dark){:root:not([data-theme="light"]){color-scheme:dark;…dark}}`;
  - `:root[data-theme="dark"]{color-scheme:dark;…dark}`.

  `data-theme` sits on `<html>`. Values are written as `Palette` holds them: PR 13 ruled palette literals validated by construction (crumb `rg-cdo`, PR 18: every new constructor checks FR-86's `#RRGGBB`). That one check guards attributes and the stylesheet alike.
- **The page background:** `body{margin:0;background:var(--surface-1);color:var(--text-primary)}`.
- **The fonts:**
  - two `@font-face` rules, family `text::FAMILY`, weights `Weight::Regular.css()` and `Weight::SemiBold.css()`;
  - `font-style:normal;font-display:block`;
  - `src:url(data:font/woff2;base64,…) format("woff2")`.

  The WOFF2 bytes are `text::REGULAR_WOFF2` and `text::SEMIBOLD_WOFF2`, beside the TTFs, and are encoded with `base64` at run time.
- **The focus indicators** (architecture §5). Each rule that removes the browser outline has the same selector as the rule that replaces it, so both fail together in an older browser. The outline is made transparent, not removed, so forced-colours mode turns it into a visible system-colour outline:
  - **Legend rows:**
    - `[data-legend-row]:focus-visible{outline:2px solid transparent}`;
    - `[data-legend-row]:focus-visible>[data-row-indicator]{visibility:visible}`;
    - `[data-legend-row]:focus-visible>[data-row-indicator]>rect{stroke:var(--focus);stroke-width:{fmt_coord(FOCUS_RING_STROKE)}px}`.

    The stroke is scoped to focus, so PR 15's selection (FR-73) can style the same rect differently.
  - **Markers,** inside one `@supports selector(:has(*)){…}`:
    - `[data-marker]:focus-visible{outline:2px solid transparent}`;
    - one rule per marker key, `svg:has([data-marker="L"]:focus-visible) [data-marker-indicator="L"]{visibility:visible}`.

    The block is omitted when there are no markers.
  - **A guard:** a `const _: () = assert!(FOCUS_RING_STROKE <= 2.0 * LEGEND_ROW_INDICATOR_INSET)` in `style.rs`. Otherwise the next row cuts the ring off. Today the margin is exactly zero.
- **Marker keys:** `svg::marker_keys(&Document) -> Vec<ItemKey>` walks `Semantics::Marker` controls only, in document order, by recursive `flat_map`. It lives in `svg` so PR 15's selection CSS can reuse it. `page` debug-asserts that the narrow document has the same keys.
- **The stylesheet is assembled by `page`** from `css.rs` sections (`font_faces`, `palettes`, `layout`, `indicators`, `VISUALLY_HIDDEN`). PR 15 can then add the table's CSS beside `table.rs` without widening one function. Every literal rule fragment is a module-level `const`; clippy's `literal_string_with_formatting_args` fires on `{…}` literals inside function bodies.
- **The stylesheet holds no `<`** (asserted), so `</style>` cannot appear even if a later change puts a string in it.
- **The fallback `<h1>`, `<main>` and `lang`** stay from 14a.
- **Interactive controls without handlers:** markers and legend rows are focusable `role="button"`s before PR 15 gives them behaviour. Acceptable only because nothing is released before PR 15. The PR body says so.
- **Not in this PR, with crumbs:**
  - forced-colours styling of the rings (PR 15's assistive-technology pass), and a browser check of `:has` cost on a 300-item radar there;
  - WCAG 1.4.12 text spacing: the legend's name and qualifier are separate `<text>`s at fixed x, so added letter spacing overlaps them. Before the first release (NFR-1);
  - a CR in a name reaches HTML as `&#13;`, which an HTML5 parser reports as a parse error;
  - SC-8's cross-platform comparison of hashes between CI jobs (architecture §10). This PR covers the cross-process half;
  - selection (FR-73), the toggle, and moving focus across a breakpoint change: all PR 15.
- **Narrow placements are computed twice,** once for FR-95 and once by `arrange`. That costs microseconds, so I'm keeping the simpler interface.
- **The golden:** `tests/golden/html/prd.html`, with each font payload elided to `…`. The test asserts that exactly two payloads were elided, so the golden cannot hide a missing font face.

## Review rulings

Five seats reviewed revision 1.0: Rust, QA (with injection), architecture, accessibility, and UI in place of security. The Rust and QA seats probed a scratch copy, including an `html5ever` crate on hostile input. There were no blockers. Rulings:

- **Outline removal:** a browser without `:focus-visible` would get `outline:none` and no indicator (accessibility, architecture, UI). Each removal now shares its replacement's selector and uses a transparent outline, which forced colours make visible.
- **Narrow cap** (UI, architecture, accessibility): the narrow arrangement filled tablets at almost 2x. Both containers are now capped at their own width.
- **Print** (UI): declined. FR-58 makes printing the HTML unsupported, and PNG and SVG are the print path.
- **Range syntax** (QA, Rust): declined, for Safari before 16.4. Kept `max-width` with no gap, because the wide arrangement is the default.
- **`narrow_query`** (architecture) and the **stroke-in-inset guard** (architecture, from a PR 11 note) are adopted.
- **`css_value` on palette values** (architecture): declined at implementation. PR 13 already ruled palette literals validated at construction (`rg-cdo`), which guards every consumer. A second check here would be an error path no input can reach. `rg-cdo` gains a note that the stylesheet is the second consumer.
- **The row stroke is scoped to focus** (accessibility), so selection can differ (FR-73).
- **`html5ever` pinned to 0.39** (Rust, QA): 0.40 does not compile against `markup5ever_rcdom` 0.39. Counts are per namespace, since SVG tooltips are `title` elements too, and `dom.errors` must be empty.
- **Hostile strings are compared** with `hostile.rs`'s document-against-parsed `Strings`, not by substring (Rust).
- **Tests that bite** (QA):
  - an exact `layout` string, plus the parsed stylesheet's `display` declarations;
  - the selector structure checked on the parsed SVG;
  - outline placement;
  - fonts tied to `Weight::css()` and compared with the asset files on disk;
  - interactive paints are `var()`, `none` or a number ink, and the series are equal in both themes;
  - determinism on two fixtures;
  - a zero-marker page and a 200-quadrant page.
- **The golden helper** takes a directory (Rust). The `src:` check is relaxed together with `url(` (Rust, QA).
- **Declined:**
  - `width`/`height` attributes on the SVG root (UI), which would change the serialiser shared with the SVG goldens;
  - `vector-effect: non-scaling-stroke` (accessibility), which only matters for an AAA criterion NFR-1 does not claim;
  - a `Page` input struct now (architecture), which is for PR 15 once `Catalogue` and `Plotted` join. Noted on `rg-6zu`.

---

### Task 1: the stylesheet

**Files:**

- create `src/emit/html/css.rs` (`mod css;`);
- `src/text.rs` (`REGULAR_WOFF2`, `SEMIBOLD_WOFF2`);
- `src/style.rs` (the stroke guard);
- `Cargo.toml` (`base64 = "0.23"` to `[dependencies]`);
- tests in `css.rs`, or in `css/tests.rs` if the file passes about 300 lines.

**Interfaces** (all `pub(super)`):

- `font_faces() -> String`;
- `palettes() -> String`;
- `layout(wide_width: f64, narrow_width: f64) -> String`;
- `narrow_query(wide_width: f64) -> String`;
- `indicators(markers: &[ItemKey]) -> String`;
- `VISUALLY_HIDDEN: &str`.

- [ ] **Step 1: Failing tests:**
  - `the_font_faces_are_the_vendored_fonts`: two `@font-face` rules, family `text::FAMILY`, weights `Weight::Regular.css()` and `Weight::SemiBold.css()`, each `base64,…)` payload decoding to `std::fs::read("assets/Inter-Regular.woff2")` and `…SemiBold.woff2`.
  - `palettes_override_in_order`:
    - the three blocks in order, each declaring exactly its palette's `declarations()` as `name:value;` with its `color-scheme`;
    - the dark selectors are `:root:not([data-theme="light"])` inside the media query, and `:root[data-theme="dark"]`;
    - the series are equal in both built-in themes.
  - `the_layout_is_exact`: `layout(976.817, 420.0)` equals one expected string held as a test `const`, covering the body rule, the SVG rule, both caps and the breakpoint rule. `narrow_query(976.817)` is `(max-width: {B-1}px)`.
  - `only_the_breakpoint_rule_sets_display`: in `layout`'s output, the only `display` declarations on `.wide`, `.narrow` or `.arrangement` are `.narrow{display:none}`, then `.wide{display:none}` and `.narrow{display:block}` inside the query. None uses `visibility` or `opacity`.
  - `one_rule_per_marker_key`:
    - keys `[6, 7, 2]` give three rules in that order inside one `@supports selector(:has(*))`;
    - the marker outline rule is inside it;
    - no keys gives no `@supports` and no empty `{}`.
  - `outlines_go_only_with_their_indicator`: every `outline` declaration's selector ends in `:focus-visible`, and the `data-marker` one is inside the `@supports` block.
  - `the_stylesheet_requests_nothing_but_fonts`: `url(` exactly twice, each `url(data:font/woff2;base64,`; `src:` exactly twice; no `@import`, no `http`, no `<`.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(emit): the page's stylesheet: fonts, palettes, breakpoint, focus`.

---

### Task 2: both arrangements on the page

**Files:**

- `src/svg/mod.rs` (`marker_keys`);
- `src/emit/html/mod.rs` (`page(wide, narrow, title)`);
- `tests/common/html.rs`: `svgs(page) -> Vec<&str>` replaces `split`. The page allowlist adds `div`; the stylesheet check allows the two font `url(data:` and the two `src:`;
- `tests/serialise/page.rs`.

**Interfaces:**

- Consumes: Task 1.
- Produces: `page(wide: &Document, narrow: &Document, title: Option<&str>) -> String`, and `svg::marker_keys(&Document) -> Vec<ItemKey>`.

- [ ] **Step 1: Failing tests** (the PRD example's wide and narrow documents):
  - `marker_keys_walk_controls_only`: the PRD example's keys are its six `data-marker` lines in document order, with no hit areas or indicators among them.
  - `one_arrangement_at_a_time`: two SVGs, the wide first, in their containers, and the stylesheet contains `layout(W, N)` exactly.
  - `both_are_interactive`: in each SVG, every `fill` and `stroke` is `var(--…)`, `none` or one of the two number inks. Both SVGs carry the same `data-marker` keys.
  - `the_indicator_selectors_match_the_tree`: on each parsed SVG,
    - every `[data-row-indicator]` is a direct child of a `[data-legend-row]` with the same key, and has a direct `rect` child;
    - every `[data-marker-indicator]` key is a `[data-marker]` key in the same SVG;
    - every key in the `:has` rules is a wide `data-marker`, and every wide `data-marker` has a rule.
  - The 14a tests carry over through `svgs`.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(emit): both arrangements, interactive, one shown at a time`.

---

### Task 3: the pipeline writes the full page

**Files:** `src/pipeline.rs` (`render` arranges Narrow too); `tests/run.rs`.

- [ ] **Step 1: Failing tests:**
  - `renders_the_prd_example` asserts two `<svg` elements and two `@font-face` rules;
  - `realistic_inputs` adds a zero-marker CSV, every ring unknown: a valid page with no `@supports`;
  - a generated 200-quadrant CSV renders with exit 0 and one `:has` rule per marker.
- [ ] **Step 2–4:** red, implement, green; lints; rustqual.
- [ ] **Step 5: Commit** `feat(pipeline): write both arrangements`.

---

### Task 4: the golden, the HTML5 round trip, and determinism

**Files:**

- `tests/serialise/golden.rs`: `golden(dir, name, got)`;
- `tests/golden/html/prd.html`;
- `tests/serialise/page.rs`;
- `tests/serialise/hostile.rs` (`Strings` and `HOSTILE` `pub(super)`);
- create `tests/determinism.rs`;
- `Cargo.toml` dev-dependencies `html5ever = "0.39"`, `markup5ever_rcdom = "0.39"`.

- [ ] **Step 1: Tests:**
  - `prd_page_matches_its_golden`: the page with its two payloads elided (asserting two were) equals `tests/golden/html/prd.html`.
  - `hostile_text_survives_an_html5_parser`: the hostile catalogue and a hostile title, rendered, then parsed by `html5ever` into an `RcDom`. Then:
    - `dom.errors` is empty;
    - HTML-namespace elements are only the page's (`html head meta title style body main h1 div`);
    - SVG-namespace elements are only `SVG_ELEMENTS`;
    - there is exactly one HTML `title` and one HTML `style`;
    - `Strings` read back from the parsed SVGs equal the document's.

    The doc comment says why this is stronger than the XML check: SVG `<title>` is an HTML integration point, whose content an HTML parser reads as HTML.
  - `two_processes_write_the_same_bytes`: the built binary, two separate processes, on the PRD example and on a hostile CSV with a title; the bytes are equal for each.
- [ ] **Step 2:** bless the golden and read it.
- [ ] **Step 3:** `just ci`.
- [ ] **Step 4: Look at it.** Render the PRD example and the template. In a browser, check viewport widths `B - 1`, `B` and `B + 1`, zoom 110% around `B`, light and dark, and Tab through both arrangements. Record what was checked in the PR body.
- [ ] **Step 5: Commit** `test: the page's golden, an HTML5 round trip, determinism across processes`.

---

## Finishing

1. **Reviews:** a final whole-branch review on the most capable model in Copilot's seat (Copilot is out of credits until November 2026; say so), then simplify, then a final check.
2. **Crumbs:**
   - `crumbs close rg-f27`.
   - New crumbs:
     - forced-colours indicators, and the `:has` cost check;
     - WCAG 1.4.12 legend text spacing, before release;
     - CR in names in HTML;
     - SC-8's cross-job hash comparison.
   - A note on `rg-6zu`:
     - the toggle sets `data-theme` on `<html>`;
     - `narrow_query` feeds `matchMedia`;
     - on a breakpoint change, focus moves to the same key in the newly shown arrangement;
     - selection styling (FR-73) goes beside the generated focus rules, reusing `svg::marker_keys`;
     - a `Page` input struct when `Catalogue` and `Plotted` join;
     - reduced motion for any transition.
3. Move this plan to `docs/plans/done/`, tick the steps, and set `status: complete` and `completed_date`.
4. **No version bump.** Commit, push, open the PR, check CI on all three platforms, and merge when clean if the maintainer authorises it.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-12 | Initial version, for the plan review. |
| 1.1 | 2026-10-12 | The five-seat review: both containers capped; outline removal paired with its indicator, and transparent; `css_value` on palette values; `narrow_query`; the stroke guard; `html5ever` 0.39; tests that check structure, not substrings; print and range syntax declined. |
