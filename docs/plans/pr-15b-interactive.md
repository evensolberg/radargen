---
tags:
  - plan
  - html
  - a11y
aliases: []
doc_id: RG-015B
doc_name: pr-15b-interactive
crumb_id: rg-6zu
document_title: "PR 15b — the interactive layer: Implementation Plan"
created_date: 2026-10-12
synopsis: "One constant script brings the page to life, with no data of its own. Part 1 (15b-1) lays the script's skeleton, shows marker focus rings by attribute instead of per-key CSS (rg-jri), and adds the theme toggle and skip link. Part 2 (15b-2) adds one sticky detail panel built from the item's table row (D-80), selection, hover, keyboard activation, and focus return keyed on the item in whichever arrangement is shown."
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
comments: "1.1 folds in the five-seat plan review and the maintainer's two rulings (a sticky panel; 15b split in two). See Review rulings. Part two of PR 15 (rg-6zu), with rg-jri."
---
# PR 15b — the interactive layer: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumbs:** `rg-6zu` (part two), `rg-jri` · **Phase:** v1.0 · **After:** PR 15a (#70) · **Then:** PR 15c (verification, the first release)

This plan ships as **two PRs**, by the maintainer's ruling of 2026-10-12:

| PR | Delivers |
| --- | --- |
| **15b-1** (Part 1) | <ul><li>The script's skeleton: the `.js` gate, the shared names block, and the delegated listeners with nothing yet to activate.</li><li>**`rg-jri`:** marker focus shown by a script-set attribute, the per-key `:has` rules gone, and the cost measured.</li><li>The theme toggle (FR-56) and the skip link.</li><li>The browser harness, `scripts/interaction.mjs`.</li></ul> |
| **15b-2** (Part 2) | <ul><li>The detail panel (FR-38), sticky at the viewport's bottom and built from the item's table row (D-80).</li><li>Selection (FR-72, FR-73), hover (FR-37) and keyboard activation (FR-48).</li><li>Escape and close with focus return (FR-49), and `aria-expanded`/`aria-controls` (D-66).</li><li>Breakpoint handling (architecture §5) and hit-area forwarding.</li></ul> |

Each PR is complete in itself: every handler lands with the state it manages, and no PR ships a control that does nothing.

**Architecture:**

- **One constant script.** `src/emit/html/script.js` is inlined via `include_str!` into a single `<script>` at the end of `<body>`.
  - **No data:** it holds **no CSV-derived data** and reads everything from the DOM, so NFR-6's JavaScript context never arises.
  - **Same bytes everywhere:** it is byte-identical in every page (NFR-4).
- **One configuration value.** The breakpoint query reaches the script as `<main data-narrow-query="…">`, from `css::narrow_query`, its one source.
- **One names block.** Every attribute, class and id the script uses sits in one `const N = {…}` at the top of `script.js`. A Rust test parses it, asserts each value equals the Rust constant of the same role (`DATA_ROW`, `DATA_MARKER`, …), and asserts the script holds no other `data-` literal. A renamed constant then fails `cargo test`, even with the browser checks out of CI.
- **State as attributes.** The script sets `data-focused` (15b-1), and `data-selected` and `data-hovered` (15b-2), on elements carrying an item's key. CSS styles these attributes with **fixed** rules, so style recalculation no longer grows with the marker count (`rg-jri`).
- **Marks, not positions.** The serialiser finds the focus ring and the selection marks through two new unit variants, `Semantics::FocusRing` and `Semantics::Selection`. Each wraps one child in a `<g data-focus-ring="L">` or `<g data-selection="L">` inside an indicator.
  - **Static outputs:** both are `is_interactive_only`, so the SVG and PNG emitters drop them with their indicator. Static goldens and the resvg tests do not change.
  - **Geometry:** stroke widths are in the tree, scaled by `r.marks`. CSS only toggles visibility and swaps paint.

**Tech stack:** Rust 2024; vanilla ES2020, no dependencies, inlined; Playwright's headless Chromium over CDP.

**Spec:**

- `docs/product/radargen-prd.md`, v0.61 → v0.62 (Part 1 Task 0): FR-34, FR-36–FR-38, FR-48–FR-50, FR-56, FR-69, FR-72, FR-73, FR-94, FR-96; NFR-1, NFR-5, NFR-6; D-16, D-66, D-80.
- Architecture §5 and §11.
- Crumbs `rg-6zu` and `rg-jri`.

## Global Constraints

- Rust 2024; `cargo clippy --all-targets -- -D warnings` and `cargo lclippy` clean. CSS rule text is built from **module-constant fragments**, as `css.rs` does, never from `{…}` literals in function bodies.
- **rustqual IOSP:** every function is integration or operation. **Module size** is about 300 lines: the page chrome (controls, panel and their CSS) goes in a new `src/emit/html/chrome.rs` from the start, since `table.rs` is already 262 lines.
- NFR-4: byte-identical output.
- **NFR-5:** the script fetches nothing. This is tested two ways:
  - **On the bytes:** a word-boundary test rejects `fetch`, `XMLHttpRequest`, `import`, `WebSocket`, `EventSource`, `Worker`, `sendBeacon`, `navigator.`, `window.open`, `document.write`, `new Image`, `src`, `srcdoc` and `<!--`.
  - **Behaviourally:** with `Network.enable` and a panel open, the document is the only request.
- **NFR-6:**
  - **The DOM:** the script reads with `textContent` and writes with `textContent`, `setAttribute` on its own names, or `cloneNode`. There is no `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `DOMParser`, `createContextualFragment`, `eval` or `Function(`, and the word-boundary test asserts it.
  - **Links:** a link is cloned from the table's escaped `<a>`, never rebuilt (D-77).
- **No script, no loss:**
  - the table works as in 15a;
  - markers show the native outline;
  - the toggle, skip link and panel stay `hidden=""`, guarded by `[hidden]{display:none!important}`.
  - **Static `aria-*` attributes:** the name-cell buttons, which are static markup, carry no `aria-expanded` or `aria-controls`; the script adds both at start-up. A native button without its handler is still a button that does nothing. That is accepted for those buttons alone, as it already is for the `role=button` markers. Architecture §5 records it.
- **No transitions or animations:** a Rust test bans `transition`, `animation` and `scroll-behavior` in the stylesheet, so `prefers-reduced-motion` has nothing to wrap.
- **Every hidden element is written `hidden=""`,** which the tag scanner can read.
- **No version bump.** By the maintainer's decision of 2026-10-12, PR 15 releases in 15c.

## Review Focus

1. **Focus return when the opener cannot take focus.** Two cases:
   - the opener was a table button, and the reader has since closed the table's `<details>`;
   - a breakpoint hid the opener's arrangement.

   Escape must land on a focusable element, never `<body>`:
   - **the closed table:** the table's `summary`;
   - **the breakpoint:** the same key in the shown arrangement.

   *Test:* Part 2 Scenarios 4b and 9.
2. **Swapping items without closing.** Opening item 3 and then activating item 5 leaves:
   - only key 5 selected and expanded;
   - a `<dl>` holding only item 5's fields;
   - Escape returning focus to 5's opener.

   *Test:* Scenario 2b.
3. **A breakpoint change with focus on a marker and no panel open.** Focus moves to the same key's marker in the new arrangement, with its ring shown. *Test:* Scenario 9b.
4. **A hostile value in the panel.** A description `<img src=x onerror=alert(1)>` and a refused `javascript:` link are text, only the `https` link is an `<a>`, and `alert` is never called. *Test:* Scenario 6.
5. **No script.** No toggle, skip link or panel renders, a marker shows its native outline, and the table opens with Enter on its summary. *Test:* Part 1 Scenario 12.

## Decisions taken in this plan

1. **The panel is a disclosure whose target is a named region.**
   - **Markup:** `<section id="detail" class="detail" aria-label="Item details" hidden="">`. It is not a dialog: its content is for reading, and a dialog implies containment.
   - **Placement:** it sits after both arrangements, before the table heading.
   - **Sticky (the maintainer's ruling):** `.detail{position:sticky;bottom:0;max-height:min(45vh,28em);overflow:auto}`. While it is open, the radar stays in view.
   - **No hidden focus:** on open and close the script sets `document.documentElement.style.scrollPaddingBottom` to the panel's height, so the panel never covers a focused legend row (WCAG 2.4.11).
2. **Focus moves to the panel's heading on open,** `<h2 class="detail-heading" tabindex="-1">`, with `focus({preventScroll:true})`. The heading is on screen, since the panel is sticky. Screen readers announce "Item details region, 3. Kafka, heading 2". Escape or Close returns focus to the opener (FR-49).
3. **The panel's head row:** `<div class="detail-head">` holds the heading and a `Close` button, so Close sits beside the name rather than after long prose.
4. **Activating the open item again closes the panel; focus stays where the activation put it.** Activating a different item swaps the content and moves focus to the heading.
5. **The opener is remembered as `(key, kind)`,** where `kind` is `marker`, `legend` or `table`, never as an element. `state.focus` records the last `(key, kind)` on every `focusin`, and `focusout` never clears it, so a breakpoint change can restore focus after Chromium's focus fixup has moved focus to `<body>`.
   - **Return order:** focus goes to the opener in the shown arrangement, else that key's table button. If the table's `<details>` is closed, it goes to its `summary`.
6. **The panel's content is built from `tr[data-row="L"]`.**
   - **Heading:** the number cell, then the name, as "`7. Kafka`". An unplotted row's heading is the name followed by FR-41's flag text.
   - **Fields:** one `<dt>`/`<dd>` per non-empty cell after the name, in column order. Each `<dt>` is the column heading's visible text. Each `<dd>` holds clones of the cell's children, with `prose` kept.
   - **Empty state:** when no cell after the name has a value, the panel shows `<p class="detail-empty">No further details.</p>`.
7. **Selection, hover and focus differ in shape, not only in colour (FR-73, 1.4.1).** On a marker:

   | State | What shows |
   | --- | --- |
   | Selection | a rounded square, `data-selection`, stroked `--text-primary` in the tree at `FOCUS_RING_STROKE · r.marks`. Its half-width is `(INDICATOR_REACH − FOCUS_RING_STROKE/2) · r.marks`, so its outer edge stays at `INDICATOR_REACH` and the reach and frame invariants hold. Its corner radius is `MARKER_RADIUS / 4`, below a quarter of its side, so it never reads as a ring. |
   | Hover | the same square, dashed (`stroke-dasharray:3 2`) in `--text-muted`, through CSS paint overrides. Selection's rule comes later, so it wins. |
   | Focus | the existing ring, drawn after the square, so it is never broken. Where both show, the two cross at four points, and only on a marker that is both focused and selected. Accepted: keeping the square inside the reach budget forbids clearing the ring. |

   On a legend row (architecture §5: never on the target rect, which the next row covers):

   | State | What shows |
   | --- | --- |
   | Selection | a new **start bar**, `data-selection`, inside the inset band: 4 units wide, `--text-primary`, interactive-only. |
   | Hover | the inset indicator rect, dashed 1 unit in `--text-muted`. |
   | Focus | the same rect, solid in `--focus` at `FOCUS_RING_STROKE`, as today. Focus's rule comes after hover's. |

   A focused, selected row therefore shows a ring and a bar, never two overlapping outlines.

   On a table row: selection is `border-inline-start:3px solid var(--text-primary)` on the first cell, which survives forced colours, where a `box-shadow` would not. Table rows are not hover-highlighted.
8. **Keeping focus on top (FR-48, D-66).** Hover and selection now show indicators too, and a later indicator's opaque backing could cover an earlier focused ring. So on `focusin` the script moves the focused marker's indicator to the end of its layer (`parentNode.append`).
9. **Focus (`rg-jri`).**
   - **Setting the ring:** `data-focused` is set on `focusin` when `:focus-visible` matches. It is checked again on `keydown`, so the first key after a mouse focus shows the ring. It is cleared on `focusout`.
   - **The outline:** removed only under `.js` (`.js [data-marker]:focus-visible{outline:2px solid transparent}`).
   - **What goes:** the `@supports selector(:has(*))` block, `marker_rule`, `css::indicators`' keys, `svg::marker_keys` and 15a's `union`. `marker_keys` is public API, and removing it is acceptable before 1.0. This supersedes rg-6zu's PR 14b note to reuse it.
10. **Keyboard.** The key handlers act only on `[role=button]`, so a table's native button is never activated twice.
    - **Enter:** activates on `keydown`, ignoring `event.repeat`.
    - **Space:** activates on `keyup`; its `keydown` calls `preventDefault`.
    - **Escape:** acts only while the panel is open and focus is inside it or on an element with `aria-expanded="true"`. It skips events that are `defaultPrevented` or `isComposing`.
11. **Hover is driven by the mouse:** `pointerover`/`pointerout` with `pointerType !== 'touch'`, so a tap leaves no hover behind. A hit area's `click` and `pointerover` act as the marker with that key in the same `<svg>`. Hit areas and controls take `cursor:pointer`.
12. **The theme toggle** is `<button type="button" class="theme-toggle" aria-pressed="false" hidden="">Dark theme</button>`, with a dot before it that fills when pressed.
    - **Its state:** `aria-pressed` follows `data-theme` if set, else `prefers-color-scheme`, including a live change of that preference while `data-theme` is unset.
    - **Storage:** nothing is stored.
13. **The skip link** is a button, `Skip to the legend`. It comes first in `.page-controls`, is visually hidden until focused, and appears at the start when focused (`margin-inline-end:auto`). It focuses the shown arrangement's first legend row. It is a button because the target has no `id` (§5).
14. **Name-cell buttons:** `all:unset; display:block; min-height:24px; cursor:pointer; text-decoration:underline dotted`, solid on hover.
    - The rule comes before the focus rule, and the focus selector `th button:focus-visible` is the more specific, so the outline always wins. A test asserts the computed outline.
    - The dotted underline shows on touch and with the keyboard, and differs from a link's solid one.
15. **Shared classes are scoped:** `td.prose` and `td.link` keep 15a's minimum widths, and `.detail dd.prose{white-space:pre-line;max-width:70ch}`.

## Review rulings

| Seat | Finding | Ruling |
| --- | --- | --- |
| Maintainer | Panel placement | **Sticky at the viewport's bottom** (Decision 1) |
| Maintainer, architecture | Split 15b | **Two PRs**, Part 1 and Part 2 |
| Rust | **Critical:** `tags()` splits on every `<`, so it cannot read a script; `run.rs` bans `{}` in the whole page | `outside()` also removes `<script>…</script>`; the `{}` check is narrowed to the stylesheet (Part 1 Task 1) |
| Rust | **Critical:** a bare `hidden` fails the scanner | `hidden=""` everywhere (Global Constraints) |
| QA | **Critical:** focus return into a closed `<details>` lands on `<body>` | Falls back to the `summary` (Decision 5); Scenario 4b |
| QA | **Critical:** item swaps are untested | Scenario 2b |
| QA | **Critical:** "no other element has either" is ambiguous, and the `tr` would get `aria-expanded` | Exact sets named in Scenario 2; `aria-expanded` only on `[role=button]` and `button` |
| UI, accessibility | **Critical:** marker hover invisible (FR-37) | A dashed, muted square (Decision 7) |
| Architecture | Hover and selection indicators can cover a focused ring | The focused indicator moves last (Decision 8); a scenario |
| Architecture, Rust, accessibility | Legend selection stroked on the target rect | A start bar in the inset band; hover is a dashed inset outline (Decision 7) |
| Architecture | The script's names are untied to Rust | The names block plus its Rust test |
| Architecture, Rust, UI | Selection square overshoots the reach, or touches the ring | Outer edge at `INDICATOR_REACH`, stroke in the tree; the ring crossing is accepted and recorded (Decision 7) |
| Architecture, Rust | How the serialiser finds the ring and the selection | `Semantics::FocusRing` and `Semantics::Selection` |
| Architecture, QA, accessibility | `activeElement` is `<body>` after a breakpoint | `state.focus` from `focusin` (Decision 5); Scenario 9b |
| Architecture | "No script, no loss" contradicts the static table buttons | The script adds `aria-*` at start-up; native buttons without script accepted, as for markers (Global Constraints) |
| Architecture | Architecture amendments missing | Part 2 Task 7 lists them |
| Architecture | The skip link has no FR | PRD 0.62 adds it (Part 1 Task 0) |
| Architecture | A no-transitions test | Global Constraints |
| Accessibility | A focused, selected legend row is told apart by colour only | A bar for selection, the outline for focus (Decision 7) |
| UI, accessibility | Table selection vanishes in forced colours | `border-inline-start` (Decision 7) |
| UI | The panel below the radar scrolls the item away | Sticky (maintainer); `preventScroll` (Decision 2) |
| UI | A selected row is weaker than a hovered one | Selection is a solid bar, hover a dashed outline |
| UI, accessibility | The name-cell button has no affordance; `all:unset` can drop the outline and size | Decision 14, with a test |
| UI | Close at the panel's end | Beside the heading (Decision 3) |
| UI | The toggle's visible state; the skip link's placement; shared class scope; panel type; empty state | Decisions 12, 13, 15, 6 and Part 2 Task 4 |
| Accessibility | Escape caught everywhere | Scoped (Decision 10) |
| Accessibility | Space activating a table button twice | Key handlers on `[role=button]` only; Space on a table button in Scenario 4 |
| Accessibility | The section's `tabindex` | Dropped |
| Accessibility | `[hidden]` overridable | `display:none!important` |
| Accessibility | `event.repeat`; the ring after a mouse click; the theme following a live preference; persistent underline | Decisions 9, 10, 12, 14 |
| QA | Input fidelity (trusted events, focus emulation, a reload per scenario, exceptions) | The harness's rules (Part 1 Task 4) |
| QA | Missing negatives | Scenario 13 |
| QA | Ban-list brittleness | Word-boundary regexes plus the network check |
| QA | The equal-length CSS test | Replaced: no attribute selector carries a value (`\[data-[a-z-]+="`) and no `:has(` |
| QA | `just browser` outside every gate | **`just ci` runs `just browser`** and fails when Chromium is missing |
| QA | Scenario 12 passes without the script | Its mutation is the `.js` gate, as Part 1 Task 4 says |
| Architecture | The `<title>` description check | 15c's, recorded there |
| Rust | The tests Task 2 breaks | Listed in Part 1 Task 2 |

## Part 1 — PR 15b-1: the skeleton, `rg-jri`, the toggle and skip link

### Task 0: close out 15a's plan; the skip link in the PRD

- [ ] **Step 1:** set `docs/plans/pr-15a-table.md` to `status: complete` with `completed_date: 2026-10-12`, and move it to `docs/plans/done/`.
- [ ] **Step 2:** PRD 0.62.
  - **FR-98:** with script, a skip link before the radar moves focus to the shown arrangement's first legend row (WCAG 2.4.1). It is absent without script.
  - **FR-56's toggle:** the in-page toggle is a pressed/unpressed **Dark theme** button.
  - Add a revisions row.
- [ ] **Step 3:** commit `docs: RG-015A complete; FR-98, the skip link`.

### Task 1: the script, the names block, and the page controls

**Files:**
- Create: `src/emit/html/script.js`, `src/emit/html/chrome.rs`
- Modify: `src/emit/html/mod.rs` (`<main data-narrow-query>`, the page controls, `<script>`)
- Modify: `tests/common/html.rs`:
  - `outside()` drops `<script>…</script>`;
  - the allow-lists add `button`, `script`, `type`, `hidden`, `aria-pressed` and `data-narrow-query` (on `main` only);
  - `id` stays limited to `table-heading`.
- Modify: `tests/run.rs` (the `{}` check is limited to the stylesheet)
- Test: `tests/serialise/script.rs` (new)

**The script in Part 1:**
- the names block;
- `state.focus`;
- start-up: add `js`, unhide the toggle and skip link, set `aria-pressed`;
- `focusin`, `focusout` and `keydown` for the ring;
- the toggle's click and its `prefers-color-scheme` listener;
- the skip link's click;
- the `matchMedia` change, which moves focus to `state.focus`'s key in the shown arrangement.

- [ ] **Step 1: failing tests.**
  - **The script's bytes:**
    - the page holds exactly one attribute-less `<script>`, whose content equals `include_str!("../../src/emit/html/script.js")`;
    - the names block's values equal the Rust constants;
    - there is no other `data-` literal;
    - the word-boundary bans hold.
  - **The markup:**
    - `.page-controls` is first in `<main>`;
    - both controls carry `hidden=""`;
    - `data-narrow-query` equals `css::narrow_query(wide.width)`.
- [ ] **Step 2:** see them fail. **Step 3:** implement. **Step 4:** commit `feat(html): the script's skeleton, toggle and skip link`.

### Task 2: focus by attribute (`rg-jri`)

**Files:**
- Modify: `src/svg/mod.rs` (`Semantics::FocusRing`; drop `marker_keys`)
- Modify: `src/layout/radar/markers.rs` (wrap the ring)
- Modify: `src/svg/serialise/group.rs`
- Modify: `src/emit/html/css.rs` (`indicators()` takes no keys; fixed rules)
- Modify: `src/emit/html/mod.rs` (drop `union`)
- **Tests that change:**
  - `markers.rs` `probe`, and the slice pattern at about line 650;
  - `svg/serialise/tests/semantics.rs`;
  - `tests/serialise/vocabulary.rs` (`data-focus-ring` in `INTERACTIVE_ONLY`);
  - `tests/run.rs` `the_page_at_its_extremes` (no `svg:has(`);
  - `tests/serialise/page.rs` (the three indicator tests);
  - `css/tests.rs`, at about lines 123–180.

**Rules:**
- `.js [data-marker]:focus-visible{outline:2px solid transparent}`
- `[data-focus-ring]{visibility:hidden}`
- `[data-marker-indicator][data-focused]{visibility:visible}`
- `[data-marker-indicator][data-focused] [data-focus-ring]{visibility:visible}`
- `[hidden]{display:none!important}`

- [ ] **Step 1: failing tests.**
  - The stylesheet has no `:has(`, no `@supports selector`, and no attribute selector with a value.
  - Each marker indicator holds one `data-focus-ring` group.
  - The legend's focus rule is unchanged.
  - Static goldens are unchanged.
- [ ] **Step 2:** see them fail. **Step 3:** implement, and re-bless the HTML golden only. **Step 4:** commit `perf(html): marker focus by attribute, not per-key rules (rg-jri)`.

### Task 3: the cost, measured

- [ ] **Step 1:** create `scripts/focus-cost.mjs`, from the `rg-min` probe: it generates a 300-marker page, Tabs 40 times, and reports `RecalcStyleDuration` per Tab. Add `just focus-cost`.
- [ ] **Step 2:** run it on main and on the branch, and record both numbers. Expected: about 49 ms before, under 1 ms after.
- [ ] **Step 3:** commit `test(perf): a probe for focus-ring style cost (rg-jri)`.

### Task 4: the browser harness

**Files:**
- Create: `scripts/interaction.mjs`, with `connect`, `poll`, `within` and `close` taken from `text-spacing.mjs` into a shared `scripts/cdp.mjs`, which both import
- Create: `tests/fixtures/interaction.csv`
- Modify: `justfile`:
  - `interaction`;
  - `browser`, which runs `text-spacing` and `interaction`;
  - `ci` gains `just browser`, which fails if Chromium is missing.

**The harness's rules:**
- **Trusted input only:** `Input.dispatchKeyEvent` (Enter with `windowsVirtualKeyCode:13` and `text:'\r'`; Space with code 32 down and up) and `Input.dispatchMouseEvent`. Never `.focus()` or `dispatchEvent` for input.
- **Setup:** `Emulation.setFocusEmulationEnabled({enabled:true})`. Colour scheme and script disabling are set **before** navigation.
- **Isolation:** the page is reloaded for each scenario.
- **Exceptions:** any `Runtime.exceptionThrown` fails the run.
- **Waiting:** by polling, never by sleeping.

**Part 1 scenarios**, run at 1400 and 400 px:

1. **Tab order:** the skip link, the toggle, markers 1…n in number order (FR-34), then legend rows. Each marker shows its ring. A mouse click shows none until the next key press.
2. **Focus on top:** a focused marker's ring stays visible when its later neighbour is hovered, a check that applies once Part 2 adds hover, and when the focused indicator is moved last.
3. **Breakpoint (9b):** focus marker 3 at 1400 px and resize to 400 px. Focus reaches narrow marker 3, with its ring.
4. **Theme:**
   - `aria-pressed` matches an emulated `dark` and an emulated `light`, and follows a live change while `data-theme` is unset;
   - pressing it flips `data-theme` and the body's computed background.
5. **Skip link:** visible when focused; activating it focuses the shown first legend row.
6. **No script:**
   - the toggle and skip link compute `display:none`;
   - a marker's computed outline is not transparent;
   - Enter on the summary opens the table.
7. **NFR-5:** with `Network.enable`, the document is the only request.

- [ ] **Step 1:** write the harness. Run it on main's build: Scenarios 1 and 3–5 fail.
- [ ] **Step 2:** run it on the branch: all pass. Mutation-check:
  - the `.js` gate removed (Scenario 6);
  - `focusin` reading `activeElement` instead of `state.focus` (Scenario 3).
- [ ] **Step 3:** commit `test(a11y): the interactive layer in a browser`.

### Part 1 finishing

- Run `just ci`, which now includes `just browser`.
- Review seats: code review (holding Copilot's seat), simplify, and accessibility on the rendered page.
- Close `rg-jri` with the measured numbers. `rg-6zu` stays open, with a note appended.

## Part 2 — PR 15b-2: the panel, selection, hover and keyboard

### Task 1: the markup

**Files:**
- Modify: `src/svg/mod.rs` (`Semantics::Selection`)
- Modify: `src/layout/radar/markers.rs`: the square, with `MARKER_RADIUS / 4` corners and its stroke in the tree. A const-assert keeps its reach at `INDICATOR_REACH`.
- Modify: `src/layout/legend/mod.rs`: the start bar in the inset band, interactive-only, with a const-assert that it fits the inset.
- Modify: `src/svg/serialise/group.rs`
- Modify: `src/emit/html/chrome.rs`: the panel `<section>` with its head row and `<dl>`, and the empty state written by the script.
- Modify: `src/emit/html/table.rs`: the name cell's `<button type="button" data-row="L">`.

- [ ] **Step 1: failing tests.**
  - Each marker indicator has one `data-selection`, and each legend row has one bar.
  - The square's bounds equal the backing disc's.
  - Static goldens are unchanged.
  - The table button's text equals 15a's cell text.
  - The panel is the only `id="detail"`: it is `hidden=""` and sits between the narrow arrangement and the table heading.
  - The allow-lists gain `section`, `dl`, `dt`, `dd`, `p` and `aria-label`; `id` is allowed only as `detail` on `section`.
- [ ] **Step 2:** see them fail. **Step 3:** implement and re-bless the HTML golden. **Step 4:** commit `feat(html): the panel and selection markup`.

### Task 2: the state rules

**What the rules cover**, as module-constant fragments, with Rust tests that each is present:
- the selection and hover visibility and paint overrides of Decision 7;
- `tr[data-selected]>:first-child{border-inline-start:3px solid var(--text-primary)}`;
- `[data-hit],[role=button]{cursor:pointer}`;
- the panel, head row, `dt`/`dd`, empty state, buttons, the toggle's dot, the name-cell button, and the focus outlines.

Commit `feat(html): selection, hover and panel styles`.

### Task 3: the script's behaviour

Add the following, all through the names block:
- `open`, `close`, `activate` and `elements`. `elements` gives `aria-expanded` to `[role=button]` and `button` only, and `data-selected` to those, the `tr` and the indicators.
- The panel built from the row.
- The click, key and pointer handlers.
- The scroll padding.
- The table buttons' `aria-*`, added at start-up.

The word-boundary bans still hold. Commit `feat(html): the panel, selection and keyboard`.

### Task 4: the scenarios

Run at 1400 and 400 px; each must fail on Part 1's build and pass after Task 3.

2. **Open:**
   - Enter on marker 3 opens the panel; its heading reads "3. …" and is focused and inside the viewport.
   - `aria-expanded="true"` is on exactly the five controls for key 3: two markers, two legend rows and the table button. `data-selected` is on those five, the `tr` and both indicators.
   - The square is visible and the ring hidden, unless focused.
   - **2b:** activating legend row 5 without closing leaves only key 5's sets, gives a `<dl>` with exactly 5's non-empty fields, and Escape returns focus to legend row 5.
3. **Close:**
   - Escape returns focus to marker 3, now inside the viewport, with nothing selected.
   - Space on legend row 2 opens it without scrolling the page.
   - Close returns focus to legend row 2.
   - Clicking legend row 2 twice opens it, then closes it.
4. **The table:**
   - Enter, and separately Space, on the unplotted row's button opens a panel whose heading ends "not on the radar". No marker or legend row is selected. Escape returns focus to the button.
   - **4b:** open from a table button, close the `details` with Enter on its summary, then press Escape: focus is on the summary.
5. **Duplicate names:** the two `Kafka` rows each open their own panel and select only their own key.
6. **Hostile content:** no `img` in the panel; text equal to the CSV's; the refused link is text; the `https` link is an `<a>` with its `target` and `rel`; `alert` is stubbed and never called.
7. **Hover:**
   - real `mouseMoved` over a marker gives its legend row `data-hovered`, and the reverse;
   - moving to empty space clears it;
   - a tap (`Input.dispatchTouchEvent`) leaves no hover;
   - hovered and selected items differ in computed `stroke` and `stroke-dasharray`.
8. **Hit area:** a click at a hit area's edge, outside the disc, opens that marker.
9. **Breakpoint with the panel open:** open at 1400 px, resize to 400 px, press Escape. Focus lands on the narrow marker for the key; the wide arrangement holds no `data-selected` once closed; while open, the narrow elements carry it.
10. **Sticky panel:**
    - with the panel open, the selected marker stays inside the viewport;
    - `scroll-padding-bottom` equals the panel's height;
    - Tabbing through legend rows never leaves one hidden under the panel.
11. **A focused, selected legend row:** the computed outline is `--focus` and the bar is visible, so two marks show, not one merged edge.
13. **Negatives:**
    - Escape with no panel open changes nothing;
    - Escape in the toggle with a panel open does nothing;
    - an Enter with `autoRepeat` does not toggle twice;
    - a click on the heading or the `<dl>` does not close the panel.

Mutation-check:
- the Escape handler removed;
- the opener looked up by element (Scenario 9);
- `innerHTML` in the panel (Scenario 6);
- no `append` on `focusin` (Part 1's Scenario 2, now that hover exists);
- `aria-expanded` set on the `tr` (Scenario 2).

Commit `test(a11y): the panel, selection and keyboard in a browser`.

### Task 5: text spacing over the panel

`text-spacing.mjs` opens the hostile row's panel at each width, so the clip and overflow checks cover the panel and its head row. Expect `ok` everywhere. Commit `test(a11y): text spacing reaches the panel`.

### Task 6: docs

- **`docs/csv-format.md`:** the panel is live.
- **Architecture §5:**
  - item 4 of the layer list gains the selection square and the legend bar;
  - the per-format paragraph describes the attribute mechanism and the marks, in place of the per-key rules;
  - the sanctioned `id`s are `table-heading` and `detail`;
  - native table buttons without script are accepted.
- **Architecture §10:** the behaviour row names `scripts/interaction.mjs`.
- **Architecture §11:** the interactive layer is designed here.
- **The 15a split table:** text spacing over the panel moved into 15b.

Commit `docs: the interactive layer is live`.

### Part 2 finishing

- Run `just ci`.
- Review seats: code review (holding Copilot's seat), simplify, accessibility, and UI on the rendered page.
- Append a note to `rg-6zu`; 15c remains.

## Revisions

| Revision | Date | Change |
| --- | --- | --- |
| 1.0 | 2026-10-12 | First draft. |
| 1.1 | 2026-10-12 | Five-seat review folded in (see Review rulings). The maintainer rules on a sticky panel and a split into 15b-1 and 15b-2. |
