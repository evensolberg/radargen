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
synopsis: "One constant script brings the page to life with no data of its own. A marker, legend row or table row opens one detail panel, built from the item's table row (D-80). Selection, hover and keyboard focus are shown through attributes the script sets, never through per-key CSS, which ends rg-jri's :has cost. Escape and close return focus to the opener's key in whichever arrangement is shown. A theme toggle and a skip link appear only when the script runs."
status: draft
type: Plan
revision: "1.0"
review_date:
reviewed_by: []
completed_date:
comments: "Part two of PR 15 (rg-6zu), with rg-jri. 15a (#70) shipped the table this reads; 15c verifies and releases."
---
# PR 15b — the interactive layer: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumbs:** `rg-6zu` (part two), `rg-jri` · **Phase:** v1.0 · **After:** PR 15a (#70) · **Then:** PR 15c (verification, the first release)

**Goal:**

- **The detail panel (FR-38).** Activating a marker, legend row or table row opens one panel with the item's number, name and every non-empty field. The panel reads these from the item's table row (D-80).
- **Closing it.** Activating the same target again, Escape, or the close button closes the panel and clears the selection. Escape and close return focus to the opener (FR-49).
- **Selection (FR-72, FR-73).** It shows on the marker, the legend row and the table row at once. It looks different from hover, and the marker's selection is not a ring.
- **Hover (FR-37).** Hovering a marker highlights its legend row, and the reverse.
- **Keyboard (FR-48).** Enter and Space activate a control, as a click does.
- **Expanded state (D-66, WCAG 4.1.2).** Every control states it with `aria-expanded` and `aria-controls`.
- **Breakpoints (architecture §5).** On a breakpoint change, focus and selection move to the same item in the arrangement now shown.
- **`rg-jri`.** A marker's focus ring is shown by the script, which ends the per-key `:has` rules. Without script, the native outline returns.
- **Script-only controls.** The theme toggle (FR-56) and a skip link to the legend appear only when the script runs.

**Architecture:**

- **One constant script.** `src/emit/html/script.js` is inlined through `include_str!` into a single `<script>` at the end of `<body>`. It holds **no CSV-derived data**: it reads keys, names and fields from the DOM, so NFR-6's JavaScript context never arises and the script is byte-identical in every page.
- **One configuration value.** The breakpoint query reaches the script as an attribute, `<main data-narrow-query="(max-width: 780px)">`, written from `css::narrow_query`, its one source.
- **State as attributes.** The script sets `data-selected`, `data-hovered` and `data-focused` on the elements carrying an item's key. CSS styles these attributes with **fixed** rules. Nothing is generated per key, so style recalculation no longer grows with the marker count (`rg-jri`).
- **The marker indicator gains a selection outline.** It is a rounded square, not a ring (FR-73), and joins the indicator group (rg-6zu's PR 10b note). The focus ring and the outline are marked `data-focus-ring` and `data-selection`, so one indicator can show either, both or neither.
- **The panel** is `<section id="detail" class="detail" hidden aria-label="Item details">`, written once, outside both arrangements and before the table heading. It is architecture §5's sanctioned exception, and every control's `aria-controls` names it.
- **Behaviour is tested in a browser.** `scripts/interaction.mjs` drives Chromium over CDP, as `scripts/text-spacing.mjs` does, and asserts each behaviour below. The markup is tested in Rust.

**Tech stack:** Rust 2024; vanilla ES2020 with no dependencies, inlined; Playwright's headless Chromium over CDP.

**Spec:**

- `docs/product/radargen-prd.md` v0.61: FR-34, FR-36–FR-38, FR-48–FR-50, FR-56, FR-69, FR-72, FR-73, FR-94, FR-96; NFR-1, NFR-5, NFR-6; D-16, D-66, D-80.
- Architecture §5 (*Identity in the output*, the four radar layers, per-format rules) and §11.
- Crumbs `rg-6zu` (its PR 10b, PR 11, PR 13 and PR 14b notes) and `rg-jri`.

## Global Constraints

- Rust 2024; `cargo clippy --all-targets -- -D warnings` and `cargo lclippy` clean.
- **rustqual IOSP:** every function is either integration or operation.
- **Module size:** about 300 lines.
- **NFR-4:** byte-identical output. The script is a constant, so every page carries the same bytes.
- **NFR-5:** the script fetches nothing. There is no `fetch`, `XMLHttpRequest`, `import`, `WebSocket`, `navigator.sendBeacon` or `src` anywhere in it, and a Rust test asserts this on the bytes.
- **NFR-6:**
  - The script reads text through `textContent` and writes it through `textContent` or `cloneNode`. Nothing goes through `innerHTML`, `insertAdjacentHTML` or `outerHTML`, and a Rust test asserts that too.
  - The one CSV-derived value the script copies, a link, is cloned from the table's already-escaped `<a>` (D-77), never rebuilt.
- **Static outputs are untouched.** The SVG and PNG emitters still drop interactive-only nodes, the new selection outline included (architecture §5).
- **No script, no loss:**
  - the table works as in 15a;
  - focus shows as the native outline;
  - the toggle and skip link stay `hidden`;
  - nothing claims an action it cannot perform.
- **No transitions are added.** `prefers-reduced-motion` therefore has nothing to wrap, and `scrollIntoView` uses `block: 'nearest'` with no smooth behaviour.
- **No version bump.** The maintainer's 2026-10-12 decision: PR 15 releases in 15c.

## Review Focus

1. **A breakpoint change while the panel is open.** Focus is in the panel and the opener's arrangement disappears. Escape must return focus to the same key in the arrangement now shown, never to `<body>`. *Test:* Task 6.
2. **Two rows sharing a name** (FR-28). Selection, hover and the panel follow the key, never the name. *Test:* Task 6's fixture has two `Kafka` rows.
3. **An unplotted row.** It opens from its table button only. The panel says it is not on the radar, and no marker or legend row is selected. *Test:* Task 6.
4. **A hostile value in the panel.** A description holding `<img src=x onerror=alert(1)>` and a refused `javascript:` link appear as text in the panel. Only the table's `https` link is an `<a>`. *Test:* Task 6, with the Rust ban on `innerHTML` in Task 3.
5. **No script.** The page with scripting disabled:
   - has no visible toggle, skip link or panel;
   - shows each marker's native focus outline;
   - still has a working table. *Test:* Task 6, through `Emulation.setScriptExecutionDisabled`.

## Decisions taken in this plan

1. **Focus moves into the panel on open,** to its heading (`tabindex="-1"`), and Escape or Close returns it (FR-49). The heading takes focus, rather than the close button, so a screen reader reads the item's name first, and the page scrolls to the panel with no scripted scroll.
2. **Activating the opener again closes the panel, and focus stays where the activation put it.** A click on the selected marker closes the panel, and focus stays on the marker. Activating a different item swaps the panel's content and moves focus to its heading again.
3. **The opener is remembered as `(key, kind)`,** where `kind` is `marker`, `legend` or `table`, never as an element. On return, the element is looked up in the shown arrangement, or in the table, which is always present. That is architecture §5's rule, and it is what makes Review Focus 1 work.
4. **The panel's content** is built from `tr[data-row="L"]`:
   - **Heading:** the number cell, then the name, as in "`7. Kafka`". An unplotted row's heading is the name followed by FR-41's flag text.
   - **Fields:** a `<dl>` with one `<dt>`/`<dd>` per non-empty cell after the name, in the table's column order. The `<dt>` is the column heading's text, and the `<dd>` holds clones of the cell's child nodes, so a link keeps its escaped `href`, `target`, `rel` and hidden suffix.
   - **Prose:** a prose cell's `<dd>` keeps the `prose` class, so its line breaks show (FR-97).
5. **Selection is drawn without a ring (FR-73):**
   - **Marker:** a rounded square outline, `stroke: var(--text-primary)` at 2 px. It is the new `Node::Rect`, `data-selection`, in the indicator group, at `INDICATOR_REACH` half-width, so it encloses the backing disc.
   - **Legend row:** its backing rect is stroked `var(--text-primary)` at 1.5 px.
   - **Table row:** its cells take `var(--surface-1)` with a 3 px start border in `var(--text-primary)`.
   - All three use `--text-primary`, which already passes 1.4.11 against both surfaces.
6. **Hover is distinct from selection and focus.**
   - **Marker:** its indicator shows the raised copy and backing only, with no outline and no ring.
   - **Legend row:** its backing rect fills `var(--surface-1)`.
   - **Table row:** not highlighted. FR-37 relates the radar and the legend, and a table row is reached by reading, not by matching dots.
7. **Focus (`rg-jri`).**
   - **The handlers:** on `focusin`, the script sets `data-focused` on the focused marker's indicator, but only when `:focus-visible` matches, so a mouse click shows no ring. On `focusout` it clears it.
   - **The CSS:** `[data-marker-indicator][data-focused] [data-focus-ring]{visibility:visible}`.
   - **The no-script fallback:** the rule `[data-marker]:focus-visible{outline:2px solid transparent}` applies only under `.js` on `<html>`, which the script sets first. Without script, the native outline stays.
   - **What goes:** the `@supports selector(:has(*))` block, `marker_rule` and the `markers` argument to `css::indicators`. Legend rows keep their `:has`-free rule.
8. **Keyboard.** On a `role="button"` control:
   - Enter activates on `keydown`;
   - Space activates on `keyup`, and its `keydown` calls `preventDefault` so the page does not scroll.

   That is the native button contract. Table buttons are native `<button>`s and need nothing.
9. **Hit areas.** A `click` or `pointerover` on `[data-hit="L"]` is handled as the marker with the same key in the same `<svg>` (rg-6zu's PR 10b note).
   - **The raised copy:** it already has `pointer-events: none`, so its clicks reach what lies beneath.
   - **The cursor:** hit areas and controls take `cursor: pointer`.
10. **The theme toggle:** a `<button type="button" class="theme-toggle" aria-pressed="false" hidden>Dark theme</button>`, with these rules:
    - the script unhides it and sets `aria-pressed` from the theme in effect: `data-theme` if set, else `prefers-color-scheme`;
    - pressing it sets `data-theme` on `<html>` to `dark` or `light`;
    - nothing is stored, so each open follows the viewer's preference, as FR-56 describes.
11. **The skip link:** a `<button type="button" class="skip" hidden>Skip to the legend</button>`. It comes first in `<main>`, is visually hidden until focused, and focuses the first legend row of the shown arrangement. It is a button because the target has no `id` (§5): two arrangements would duplicate one.
12. **The page controls sit together:** `<div class="page-controls">` holds the skip link and the toggle, at the top of `<main>`, capped like the table. The skip link stays visually hidden unless focused. The toggle sits at the inline end.
13. **Hidden state is kept honest.**
    - **Visibility:** the panel is `hidden` when closed, so it is out of the accessibility tree and the tab order.
    - **Expanded state:** every control's `aria-expanded` is `false` except the opener's, which is `true` on all of its elements (marker, legend row and table button). The relationship is symmetric (FR-72).
14. **`marker_keys` and 15a's `union`** lose their only caller, the per-key rules, and are removed (no dead code). `rg-l0x`'s test of a narrow-only marker is replaced by the interaction check that every marker shows its ring.

## Tasks

### Task 0: close out 15a's plan

- [ ] **Step 1:** set `docs/plans/pr-15a-table.md` to `status: complete` with `completed_date: 2026-10-12`, and move it to `docs/plans/done/`.
- [ ] **Step 2:** commit `docs(plans): RG-015A is complete`.

### Task 1: the markup the script needs

**Files:**
- Modify: `src/svg/serialise/group.rs` (`control_attrs`)
- Modify: `src/layout/radar/markers.rs` (`indicator_node` gains the selection rect)
- Modify: `src/svg/mod.rs` (the selection and focus-ring marks)
- Modify: `src/svg/serialise/*` (writes `data-focus-ring` and `data-selection`)
- Modify: `src/emit/html/table.rs` (the name cell's button)
- Modify: `src/emit/html/mod.rs` (the panel, page controls, `data-narrow-query`)
- Test: `tests/serialise/{page,table,vocabulary,identity}.rs`, and `tests/common/html.rs` allow-lists

**What changes:**
- **Controls:** each interactive control gains `aria-expanded="false" aria-controls="detail"`, after `aria-label` and before `data-row`, in that fixed order.
- **Marker indicators:** each gains a `data-selection` rect. Its focus ring circle is marked `data-focus-ring`.
- **Table rows:** the name cell becomes `<th scope="row"><button type="button" data-row="L" aria-expanded="false" aria-controls="detail">Name <span class="flag">…</span></button></th>`.
- **The page:**
  - `<main data-narrow-query="…">` opens with the page controls;
  - `<section id="detail" class="detail" hidden aria-label="Item details" tabindex="-1">` follows the arrangements;
  - inside the panel, the heading and `<dl>` the script fills: `<h2 class="detail-heading" tabindex="-1"></h2>` and `<dl></dl>`;
  - the panel ends with its `Close` button.

- [ ] **Step 1: failing tests.**
  - **Controls:** every interactive control's attributes, in order (`identity.rs`). The static target has none of the new attributes, and no `data-selection` (`vocabulary.rs`).
  - **Indicators:** each marker indicator holds exactly one `data-selection` rect and one `data-focus-ring` circle.
  - **Table:** each name cell holds one button with the row's key, whose text is the 15a cell's text exactly.
  - **The panel:** there is one `id="detail"`; it is `hidden`; it sits after both arrangements and before the table heading.
  - **The query:** `data-narrow-query` equals `css::narrow_query(wide.width)`.
  - **Allow-lists:**
    - `button`, `section`, `dl`, `dt`, `dd` and `script` become page elements;
    - `type`, `hidden`, `aria-expanded`, `aria-pressed` and `aria-label` become page attributes;
    - `aria-controls` is allowed only with the value `detail`;
    - `id` is allowed only as `table-heading` on the `h2` or `detail` on the `section`;
    - `data-narrow-query` is allowed only on `main`.
- [ ] **Step 2:** run them and see them fail.
- [ ] **Step 3:** implement, then re-bless the goldens: both the serialiser goldens and the HTML one, since the static targets gain nothing and the interactive ones gain attributes. Read the diff by eye against the list above.
- [ ] **Step 4:** commit `feat(html): the markup the interactive layer drives`.

### Task 2: state through attributes, and `rg-jri`

**Files:**
- Modify: `src/emit/html/css.rs` (`indicators` takes no keys; fixed state rules)
- Modify: `src/emit/html/mod.rs` (drop `union`)
- Modify: `src/svg/mod.rs` (drop `marker_keys`)
- Test: `src/emit/html/css/tests.rs`, `tests/serialise/page.rs`

**Rules** (built from `Neutral::as_str()` and the `DATA_*` constants):

```css
.js [data-marker]:focus-visible{outline:2px solid transparent}
[data-marker-indicator] [data-focus-ring],[data-marker-indicator] [data-selection]{visibility:hidden}
[data-marker-indicator][data-focused],[data-marker-indicator][data-selected],[data-marker-indicator][data-hovered]{visibility:visible}
[data-marker-indicator][data-focused] [data-focus-ring]{visibility:visible}
[data-marker-indicator][data-selected] [data-selection]{visibility:visible;stroke:var(--text-primary);stroke-width:2}
[data-legend-row][data-hovered]>rect:first-child{fill:var(--surface-1)}
[data-legend-row][data-selected]>rect:first-child{stroke:var(--text-primary);stroke-width:1.5}
tr[data-selected]>*{background:var(--surface-1)}
tr[data-selected]>:first-child{box-shadow:inset 3px 0 var(--text-primary)}
[data-hit],[role=button]{cursor:pointer}
```

- [ ] **Step 1: failing tests.**
  - The stylesheet holds no `:has(` and no `@supports selector`, and its length does not grow with the marker count: a 5-marker and a 300-marker page give stylesheets of equal length, fonts elided.
  - Each rule above is present.
  - The legend rule (`[data-legend-row]:focus-visible>[data-row-indicator]`) is unchanged.
- [ ] **Step 2:** see them fail.
- [ ] **Step 3:** implement. Delete `marker_rule`, `marker_focus`, `SUPPORTS_HAS`, `HAS_*`, `KEY_*`, `union` and `marker_keys`, together with their tests.
- [ ] **Step 4:** commit `perf(html): focus and selection by attribute, not per-key rules (rg-jri)`.

### Task 3: the script

**Files:**
- Create: `src/emit/html/script.js`
- Modify: `src/emit/html/mod.rs` (`<script>` + the `include_str!` constant before `</body>`)
- Test: `tests/serialise/script.rs` (new)

**The script's shape** is an IIFE in strict mode, about 200 lines.

- **`state`:** `{ open: null | {key, kind}, hovered: null | key }`.
- **`shown()`:** the arrangement whose `getComputedStyle(...).display !== 'none'`.
- **`find(key, kind)`:**
  - `marker`: `[data-marker="L"]` in `shown()`;
  - `legend`: `[data-legend-row="L"]` in `shown()`;
  - `table`: `tr[data-row="L"] button`.
- **`elements(key)`:** every element carrying `data-row="L"`, in both arrangements and the table, plus the marker indicators with `data-marker-indicator="L"`.
- **`open(key, kind)`:**
  - fill the panel from the table row (Decision 4);
  - set `data-selected` and `aria-expanded="true"` on `elements(key)`;
  - unhide the panel and focus its heading.
- **`close(returnFocus)`:**
  - clear `data-selected` and set `aria-expanded="false"`;
  - hide the panel;
  - if `returnFocus`, focus `find(open.key, open.kind)`, falling back to the table button when that kind is not in the shown arrangement.
- **`activate(key, kind)`:** close if `key` is already open, otherwise open.
- **One delegated listener per event on `document`:**
  - `click`: controls, hit areas, table buttons and the close button;
  - `keydown`: Enter, Space and Escape;
  - `keyup`: Space;
  - `pointerover` and `pointerout`: hover;
  - `focusin` and `focusout`: `rg-jri`.
- **`matchMedia(main.dataset.narrowQuery)` change:** if focus was in the arrangement just hidden, focus `find(key, kind)` in the new one.
- **Start-up:**
  - set `document.documentElement.classList.add('js')`;
  - unhide `.theme-toggle` and `.skip`;
  - set the toggle's `aria-pressed`.

- [ ] **Step 1: failing Rust tests** on the bytes of `script.js`:
  - it contains none of `fetch`, `XMLHttpRequest`, `import`, `WebSocket`, `sendBeacon`, `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `eval`, `Function(` or `</script`;
  - the page holds exactly one `<script>`, with no attributes, and its content equals `script.js` byte for byte;
  - two different CSVs give the same script bytes.
- [ ] **Step 2:** see them fail.
- [ ] **Step 3:** write the script.
- [ ] **Step 4:** commit `feat(html): the interactive layer's script`.

### Task 4: the panel and page controls' CSS

**Files:**
- Modify: `src/emit/html/table.rs`, or a new `src/emit/html/chrome.rs` for the panel and controls if `table.rs` passes about 300 lines

**Rules:**
- `.page-controls{display:flex;justify-content:flex-end}`, capped and inset like `.table-view`.
- `.skip:not(:focus){…visually hidden…}`.
- Buttons: `button{font:inherit;padding:.4em .8em;border:1px solid var(--stroke-divider);border-radius:6px;background:var(--surface-2);color:var(--text-primary);cursor:pointer}`. This keeps every button target at least 24 px tall (FR-94).
- `.detail{…capped…;margin-block:1em;padding:1em;background:var(--surface-2);border:1px solid var(--stroke-divider);border-radius:10px}`.
- `.detail dt{font-weight:600;color:var(--text-muted)}`, `.detail dd{margin:0 0 .6em}`, and `.detail dd.prose{white-space:pre-line}`.
- The focus outline rule gains `button:focus-visible` and `.detail-heading:focus-visible`.
- Table name-cell buttons are reset to look like the 15a text, `all:unset` plus a focus outline, so the table looks as before. The underline appears on hover only.

- [ ] **Step 1:** add tests that each rule is present, then implement.
- [ ] **Step 2:** commit `feat(html): the panel and page controls' styles`.

### Task 5: the measurement probe for `rg-jri`

**Files:**
- Create: `scripts/focus-cost.mjs`, generalised from the `rg-min` probe: it Tabs 40 times through a generated 300-marker page and reports `RecalcStyleDuration` per Tab.
- Modify: `justfile` (`focus-cost`)

- [ ] **Step 1:** run it on main's build and on this branch's. Record both in the PR body and in `rg-jri`'s close note. Expected: about 49 ms per Tab before, and under 1 ms after.
- [ ] **Step 2:** commit `test(perf): a probe for focus-ring style cost (rg-jri)`.

### Task 6: behaviour in a browser

**Files:**
- Create: `scripts/interaction.mjs`, reusing `text-spacing.mjs`'s connect, poll, within and close.
- Create: `tests/fixtures/interaction.csv`, with:
  - two `Kafka` rows (FR-28);
  - an unplotted row;
  - a description `<img src=x onerror=alert(1)>`;
  - a refused `javascript:x` link;
  - an `https` link;
  - a multi-line rationale.
- Modify: `justfile`. `interaction` renders the fixture and runs the script; `browser` runs `text-spacing` and `interaction`.

**Scenarios.** Each is asserted at 1400 px (wide) and at 400 px (narrow) unless it names a width.

1. **Tab order (FR-34):**
   - Tab from the top reaches the skip link, then the toggle, then markers 1…n in number order, then legend rows 1…n.
   - Each marker shows its focus ring (`data-focused`, the ring computed visible).
   - A mouse click on a marker shows no ring.
2. **Activation:**
   - Enter on marker 3 opens the panel, whose heading is "3. …" and is focused.
   - Marker 3, legend row 3 and the table button for that key have `aria-expanded="true"` and `data-selected`; no other element has either.
   - The marker's selection outline is visible, and its focus ring is not unless focused.
3. **Closing:**
   - Escape closes the panel; focus returns to marker 3; nothing is selected.
   - Space on legend row 2 opens it, and the page does not scroll (`scrollY` unchanged).
   - The Close button closes it and returns focus to legend row 2.
   - Clicking legend row 2 twice opens it, then closes it.
4. **The table:**
   - Opening the table and pressing Enter on the unplotted row's button opens a panel whose heading ends "not on the radar".
   - No marker or legend row is selected.
   - Escape returns focus to that button.
5. **Duplicate names:** the two `Kafka` rows open different panels, and each selects only its own key.
6. **Hostile content:**
   - The panel for the hostile row holds no `img` element; its text equals the CSV's.
   - The refused link is text; the `https` link is an `<a>` with `target="_blank"` and `rel="noopener noreferrer"`.
   - `window.alert` is stubbed and never called.
7. **Hover:**
   - `pointerover` a marker gives its legend row `data-hovered`, and the reverse.
   - A hovered and a selected item can differ at once, and their styles differ: computed `fill` and `stroke` compared.
8. **Hit area:** a click at a hit area's edge, outside the marker disc, opens that marker's panel.
9. **Breakpoint (Review Focus 1):**
   - Open a marker's panel at 1400 px, then resize to 400 px and press Escape.
   - Focus lands on the same key's marker in the narrow arrangement.
   - The panel stays open across the resize, and the narrow arrangement's elements for the key carry `data-selected`.
10. **Theme:**
    - The toggle's `aria-pressed` matches the emulated `prefers-color-scheme`.
    - Pressing it flips `data-theme` and the page background's computed colour.
11. **Skip link:** focusing it makes it visible; activating it focuses legend row 1 in the shown arrangement.
12. **No script (Review Focus 5):** with `Emulation.setScriptExecutionDisabled`:
    - the toggle, skip link and panel are not rendered;
    - Tab to a marker shows a non-transparent computed outline;
    - the `details` still opens.

- [ ] **Step 1:** write the script and fixture. Run it against Task 1's markup before Task 3's script exists: every scenario must fail.
- [ ] **Step 2:** run it after Tasks 1–4. Every scenario must pass. Mutation-check each, and each must fail:
  - remove the Escape handler;
  - look the opener up by element instead of by key (Scenario 9);
  - use `innerHTML` in the panel (Scenario 6);
  - drop the `.js` gate (Scenario 12).
- [ ] **Step 3:** commit `test(a11y): the interactive layer in a browser`.

### Task 7: text spacing over the panel, and docs

- [ ] **Step 1:** `text-spacing.mjs` opens the hostile row's panel at each width, so its clip and overflow checks cover the panel and the page controls. Expect `ok` everywhere.
- [ ] **Step 2:** update the docs:
  - `docs/csv-format.md` (the panel is live);
  - architecture §5: the per-key rule paragraph becomes the attribute mechanism, and the sanctioned `id`s are `table-heading` and `detail`;
  - architecture §11: the interactive layer is designed here.
- [ ] **Step 3:** commit `docs: the interactive layer is live`.

## Finishing

- Run `just ci` and `just browser`.
- Review seats:
  - code review and simplify;
  - accessibility, on the rendered page with a screen-reader-like walk of the accessibility tree;
  - UI;
  - Copilot's seat, held by a fresh Opus reviewer.
- Close `rg-jri`, with the measured numbers. `rg-6zu` stays open for 15c, with a note appended.
- Note CI's billing block in the PR body if it persists. Raise a crumb to run `just browser` in CI once Actions works.

## Revisions

| Revision | Date | Change |
| --- | --- | --- |
| 1.0 | 2026-10-12 | First draft. |
