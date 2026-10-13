---
tags:
  - plan
  - a11y
  - release
aliases: []
doc_id: RG-015C
doc_name: pr-15c-verification
crumb_id: rg-6zu
document_title: "PR 15c — verification and the first release: Implementation Plan"
created_date: 2026-10-12
synopsis: "An automated WCAG 2.1 AA audit of the finished page in headless Chromium, run by just ci, over every criterion a browser can measure; a visual sign-off script; a manual checklist with the exact announcements to expect from NVDA, JAWS and VoiceOver; and a 1.0.0 release checklist, with the bump left to its own PR once the maintainer has tested. The table gains a scroll cue. Two release gates the review found, SC-8 and SC-16, are put to the maintainer."
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
comments: "1.1 folds in the five-seat plan review. Part three of PR 15 (rg-6zu). The maintainer ruled on 2026-10-12: the first release is 1.0.0, after the maintainer's own testing."
---
# PR 15c — verification and the first release: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-6zu` (part three) · **Phase:** v1.0 · **After:** PR 15b-2 (#73) · **Then:** the dependency check (`rg-9ay`), then the 1.0.0 release PR

**Goal:**

- **Back NFR-1's WCAG 2.1 AA claim for the HTML output.** Every AA criterion gets a row, which is one of:
  - proved by an automated check;
  - proved by an earlier PR;
  - left to a named manual check;
  - not applicable, with a reason.
- **Fix the one visual defect the review found:** the table hides columns at desktop width without a cue.
- **Prepare 1.0.0 without cutting it.**

**Open for the maintainer**, before the tag; the release checklist carries both:

1. **SC-8 and SC-16 are v1.0 criteria that need CI** (architecture §10), and Actions is blocked by billing:
   - **SC-8:** HTML byte-identity across macOS, Linux and Windows. `rg-isw` calls it release-gating.
   - **SC-16:** a released binary runs on a clean machine. `just publish` attaches no binaries.

   The choice is to prove both by hand for 1.0.0 (hash outputs on three operating systems; build and attach binaries, then smoke-test one), or to amend the PRD and move them to 1.0.x with a D-entry.
2. **`rg-l4h`, the skeleton CSV, is phase v1.0.** Re-phase it to 1.1, or schedule it before the tag.

**Architecture:**

- **The audit, `scripts/a11y-audit.mjs`,** builds on `scripts/cdp.mjs`. The clip check moves there from `text-spacing.mjs`, so both share it. It joins `just browser` and so `just ci`.
- **What it reads:** Chromium's accessibility tree, joined to the DOM by `backendDOMNodeId`, so controls are found from the DOM and then looked up in the tree. A control that lost its role is caught, not skipped.
- **Contrast stays in Rust,** proved from the palette's values (`src/palette.rs`, FR-82).

**Spec:** `docs/product/radargen-prd.md` v0.62: NFR-1, SC-8, SC-16, SC-17, FR-50, FR-80, FR-94, FR-95, D-66, D-71. Plan RG-015B.

## WCAG 2.1 AA, criterion by criterion

| Criterion | Proved by | Here |
| --- | --- | --- |
| 1.1.1 non-text content | The radar group is named with its quadrants, rings and item count (D-75), and decoration is `aria-hidden` | Audit 8 |
| 1.2.x media | — | n/a: no audio or video |
| 1.3.1 info and relationships | The legend's headings; the table's `th scope`, row headers and name (15a) | Audit 8 |
| 1.3.2 meaningful sequence | Reading order in the tree | Audit 8 |
| 1.3.3 sensory characteristics | Instructions name no shape or colour alone (the key states each encoding) | manual |
| 1.3.4 orientation | Either arrangement fits either orientation | Audit 6 (width) |
| 1.3.5 input purpose | — | n/a: no form inputs |
| 1.4.1 use of colour | Status by shape (FR-40), quadrant by position (FR-39); selection, hover and focus differ in shape | Audit 7 |
| 1.4.2 audio control | — | n/a |
| 1.4.3 contrast, text | `src/palette.rs`, both themes | done |
| 1.4.4 resize text | Page zoom: 200% of a 1280 px window is 640 CSS px. A root-size doubling checks the HTML chrome only, since SVG text is in user units | Audit 6; manual zoom |
| 1.4.5 images of text | — | n/a: the SVG's text is text |
| 1.4.10 reflow | 320 CSS px, which WCAG equates with 400% of 1280 | Audit 6 |
| 1.4.11 non-text contrast | `src/palette.rs`; Task 1 adds TextPrimary on Surface1 | Task 1 |
| 1.4.12 text spacing | `just text-spacing` (D-78), with the table and the panel | done |
| 1.4.13 content on hover or focus | Only the native `<title>` tooltip, which the user agent controls and the criterion exempts | n/a, recorded |
| 2.1.1 keyboard | `interaction.mjs` (Enter, Space, Escape, Tab) | done |
| 2.1.2 no keyboard trap | A full Tab pass leaves the page | Audit 9 |
| 2.1.4 character shortcuts | — | n/a: none |
| 2.2.x timing | — | n/a: none |
| 2.3.1 flashes | — | n/a: nothing moves |
| 2.4.1 bypass blocks | The skip link (FR-98) | done |
| 2.4.2 page titled | FR-65's tab title | done (`page.rs`) |
| 2.4.3 focus order | A full Tab pass in visual order | Audit 9 |
| 2.4.4 link purpose | A link's text is its address, and it says it opens a new tab | done (15a) |
| 2.4.5 multiple ways | — | n/a: a single page |
| 2.4.6 headings and labels | The heading outline from the tree: one level-1, none skipped | Audit 8 |
| 2.4.7 focus visible | After each Tab, a focus mark is drawn over the control | Audit 9 |
| 2.5.1–2.5.4 pointer | No path gestures, activation on release, the visible label in the name, no motion | Audit 1; done |
| 2.5.8 target size (2.2) | Every control, sampled by hit-testing, down to 320 px; D-71 is the recorded exception | Audit 5 |
| 3.1.1 language of page | `lang="en"` (FR-80) | done |
| 3.1.2 language of parts | — | n/a: A-4 bounds text to the pinned font's scripts, none of them right-to-left |
| 3.2.1 on focus | Focus alone never opens the panel or moves focus | Audit 9 |
| 3.2.2 on input | — | n/a: no form inputs |
| 3.2.3 and 3.2.4, consistency | — | n/a: a single page |
| 3.3.x input assistance | — | n/a: no form inputs |
| 4.1.1 parsing | Obsolete in 2.2; met by the HTML5 parse test with no errors (`html5.rs`) | done |
| 4.1.2 name, role, value | Every control's role, name and expanded state, from the tree | Audits 1–4 |
| 4.1.3 status messages | — | n/a: none |

## Global Constraints

- The constraints of RG-015B hold.
- **The audit uses trusted input only.** It waits for `.js` on the root and for `document.fonts.ready`, polls the tree after each action, and fails on any page exception or dialog.
- **Target size has a tolerance of 0.05 px.** It loosens the check, so it is kept small enough to admit rounding and nothing more.
- **No version bump here.** The release PR carries it. This is the stated exception PR 15 has carried since 15a, cited again in the release PR's body.

## Review Focus

1. **A visible label missing from its control's name** (2.5.3).
   - **Markers and legend rows:** the name starts with `N. `, and the visible words appear in order, with punctuation and no-break spaces ignored.
   - **A mutation that must fail:** a legend row's name without its item name.
2. **A state that changes nothing in forced colours.**
   - **Each state's mark:** a screenshot of its box with the state on differs from one with it off.
   - **Its colour:** it differs from a `Canvas` probe.
3. **A hidden arrangement reaching assistive technology,** or a control dropping out of the list unseen. The hidden arrangement's controls are absent or `ignored` in the tree, and none is focusable, at every width including the breakpoint's edges.
4. **Duplicate names.**
   - **Radar controls** never share a name; FR-50's position clause guarantees it.
   - **Table buttons named by a repeated name** (FR-28) share a name by design. They are recorded as expected, not failed: the row's other cells tell them apart, and the panel's heading carries the number.
5. **A target below 24 px outside D-71's exception.** Measured by hit-testing, not by bounding boxes.

## Tasks

### Task 1: the one missing contrast pair

`src/palette.rs`: add TextPrimary on Surface1 at ≥ 3:1 in both themes. This is the selection square on the radar ground, and the selected table row's border on its tint. Cite the existing tests for the other pairs: `text_clears_4_5_on_surface2` and `ring_names_clear_4_5_on_their_halo`.

Commit `test(palette): selection marks clear 3:1 on the radar ground`.

### Task 2: the table's scroll cue

The table is capped at the wide document's width, so at 1400 px some columns sit out of view, and macOS's overlay scrollbars show nothing. `.table-scroll` gains scroll shadows: gradient backgrounds with `background-attachment: local, scroll` in the Surface2 and TextMuted roles, so the cue works in both themes and needs no script. A test asserts the rule.

Commit `feat(html): the table shows when it scrolls`.

### Task 3: the audit

**Files:**
- Create: `scripts/a11y-audit.mjs`
- Modify: `scripts/cdp.mjs`, which gains the clip check, and `scripts/text-spacing.mjs`, which imports it
- Create: `tests/fixtures/d71-wide.csv`, with one quadrant named by 70 `W` characters, as `arrange.rs`'s `beyond()` fixture
- Modify: `justfile`: `a11y-audit`, added to `browser`

**Configurations:**
- **Interaction fixture:** widths 1400, the breakpoint (read from `main[data-narrow-query]`), the breakpoint less one, 640, 400 and 320; in light, dark and forced colours.
- **D-71 fixture:** 400 and 320 only.

**Checks** (numbered as in the WCAG table):

1. **Name and role.**
   - For every `[data-marker]` and `[data-legend-row]` in the shown arrangement, found from the DOM, the tree node is `button`, not ignored.
   - Its name starts with `N. ` and contains its visible words in order.
   - Every table name button is `button`, named by its text.
   - The counts equal the plotted and table rows.
2. **Description:** a marker's `<title>` appears as its description. This is recorded for Task 5's decision, not judged.
3. **Expanded state:**
   - at load, after `.js`, every control has `expanded` present and `false`;
   - after Enter on marker 3, the shown arrangement's item-3 controls and its table button report `true`;
   - after a resize across the breakpoint, the newly shown arrangement's do;
   - the panel is a `region` named "Item details", and `aria-controls` resolves to it.

   The exact five-control set is `interaction.mjs`'s; this check adds what the tree computes.
4. **Names are unique** across radar controls; Review Focus 4 covers the table's repeats.
5. **Target size (2.5.8).**
   - **Radar controls:** a ring of points 12 px from each control's centre (24 px across) must hit that control's own hit area or target, by `elementFromPoint`.
   - **HTML controls:** the toggle, the skip link (focused), the summary, the table buttons and Close are measured by box, within 0.05 px.
   - **The narrow SVG:** its width equals the viewport's, FR-94's `NARROW_HORIZONTAL_INSET_PX = 0` premise.
   - **D-71, interaction fixture:** no shortfall anywhere.
   - **D-71, its own fixture:** the narrow width read from FR-95's warning on stderr is over 491 units. The shortfalls are a non-empty subset of the narrow arrangement's markers and legend rows, and each still measures at least 24 × 320 ÷ width.
6. **Reflow and resize.**
   - At 320 and 640 px, and at 1400 px with the root font size doubled, the document never scrolls sideways, and no visible text in the page chrome spills from its box.
   - The doubled root size is labelled as covering the chrome only.
7. **Forced colours and colour alone.** In forced colours, each of these states' marks, as a clipped screenshot, differs on and off, and its colour differs from a `Canvas` probe:
   - a focused marker's ring;
   - a focused legend row's outline;
   - a selected marker's square;
   - the legend's selection bar;
   - the table's selected-row border;
   - hover's dashes;
   - the toggle's pressed dot;
   - the panel's border.

   Markers keep their status shapes, and the legend names every quadrant.
8. **Structure.**
   - **Headings:** the outline from the tree has exactly one level-1 heading exposed, and no level skipped.
   - **The radar group:** named, with its counts.
   - **The table:** named, its cells reporting their headers.
   - **Reading order:** the legend's headings and rows come in their visual sequence.
   - **The hidden arrangement:** no node of it is exposed.
9. **Keyboard.**
   - A full Tab pass from the top reaches every control of the shown arrangement once, in visual order, then the panel's absence, the table and the end of the page, without a trap.
   - After each Tab, a focus mark (a ring or an outline) is drawn and its box overlaps the control's.
   - Focus alone never opens the panel.

- [ ] **Step 1:** write the audit. Build a release binary in a worktree at `74f95ac` (15b-1) and run the audit there: check 3 must fail on its presence assertion, not crash.
- [ ] **Step 2:** run it on main: every check passes.
- [ ] **Step 3: mutations of the built HTML, not the source.** Each must fail the audit:
  - shrink every `data-hit` circle's `r` with `sed` (check 5);
  - delete the start-up `aria-controls` line from the inlined script (check 3);
  - drop a legend row's name from its `aria-label` (check 1);
  - add CSS that paints the selection square in `Canvas` under forced colours (check 7).
- [ ] **Step 4:** commit `test(a11y): WCAG 2.1 AA's automatable criteria, in a browser`.

### Task 4: visual sign-off

`scripts/screenshots.mjs` and `just screenshots` render the interaction, three-quadrant and D-71 fixtures:
- at 1400 px, the breakpoint, the breakpoint less one, 400 and 320 px;
- in light, dark and forced colours;
- with the panel closed, and open by a trusted click with the mouse then moved off.

PNGs are written to an ignored folder. They are not pixel-diffed, since font rendering varies across machines; the maintainer looks at them against a written list.

Commit `test(ui): screenshots for the release sign-off`.

### Task 5: the manual checklist

**File:** `docs/a11y/manual-checks.md`, with **the expected announcement for each step** and each AT and browser pair:
- NVDA with Firefox and Chrome;
- JAWS with Chrome;
- VoiceOver with Safari, on macOS and iOS.

**The steps**, with what to expect:
- **Entering the radar:** "Technology Radar … group".
- **Tabbing a marker:** "3. Ansible, Moved In; Tools quadrant, Trial ring, button, collapsed". Note whether the `<title>` is read as a description. This records the decision rg-6zu's PR 13 note asked for.
- **Enter:** "expanded", then "Item details, region", then "heading level 2, 3. Ansible".
- **Escape:** back on the same control, "collapsed".
- **The table:** row and column counts, column headers on cells, the row header, and "not on the radar" in the button's name.
- **The skip link and the toggle:** "pressed" or "not pressed".
- **Browse mode:** reading order, NVDA's headings list, VoiceOver's rotor, and iOS swipe order.
- **Keyboard scrolling of the panel** in Chrome, Firefox and Safari (`rg-n54`, which this checklist absorbs).
- **Each browser's own zoom** at 200% and 400%.

**The results table:** pair, versions, date, pass or fail, notes. A failing row becomes a crumb, and open ones gate the tag.

Commit `docs(a11y): the manual checklist for 1.0.0`.

### Task 6: the release checklist

**File:** `docs/release-1.0.0.md`, with:

1. **The open decisions above** (SC-8, SC-16, `rg-l4h`), answered.
2. **The maintainer's tests, the manual checklist, and the screenshots,** looked at against their list.
3. **`rg-9ay`, the dependency check,** done.
4. **`git tag --list v1.0.0`** empty, checked before the PR as well as by `just publish`.
5. **The version:** `Cargo.toml`'s `version` to 1.0.0, then `cargo update -p radargen` for `Cargo.lock`.
6. **The goldens:** the HTML goldens re-blessed locally with `UPDATE_GOLDEN=1`, since the page's generator meta reads `radargen 1.0`. The diff must show only that line.
7. **The changelog:** `just changelog` (`git-cliff --output CHANGELOG.md`, on git-cliff's default config unless a `cliff.toml` is added), from `v0.1.0`.
8. **`just ci` green.**
9. **The PR,** citing PR 15's stated exception to every-PR-is-a-release, then merged.
10. **`just publish`:** the tag on the merge commit and the GitHub release; with binaries attached if SC-16 is kept for 1.0.0.
11. **A line in the release notes** naming what 1.0.0 is not:
    - PNG, SVG and PDF output are v1.1 and later;
    - SC-3 depends on OQ-4;
    - the browser checks run locally only until Actions works (`rg-3wv`);
    - the README half of SC-22.

Commit `docs: the 1.0.0 release checklist`.

## Review rulings

| Seat | Finding | Ruling |
| --- | --- | --- |
| Accessibility | **Critical:** SC-17's eight criteria cannot back an AA claim | The full AA table above; checks 8 and 9 added |
| Accessibility | **Critical:** the hidden arrangement never checked | Check 8; Review Focus 3 |
| Architecture | **Critical:** SC-8 and SC-16 are v1.0 gates needing CI, which is blocked | **For the maintainer** (Open, above); the release checklist holds the tag on the answer |
| Rust | **Critical:** the hit-radius mutation would not compile (a const-assert pins it) | Mutations edit the built HTML |
| Rust, architecture | **Critical:** the named release tooling does not exist | Task 6 names `Cargo.toml`, `just changelog` and `just publish` |
| QA | **Critical:** the forced-colours check would pass on anything | Clipped screenshots on and off, plus a `Canvas` probe; a mutation |
| QA | **Critical:** controls found from the tree let a lost role escape | Found from the DOM, then looked up in the tree |
| UI | **Important:** the table hides columns at 1400 px with no cue | Task 2 |
| QA, accessibility, architecture | 1.4.4 by root size proves the chrome only | Page zoom by viewport (640); the root size labelled as chrome-only |
| QA, accessibility | 2.5.3's substring test is wrong both ways | `N. ` then the words in order |
| QA | Duplicate-name check always passes or always fails | Radar controls must be unique; repeated table names recorded (Review Focus 4) |
| QA | Expanded state could be read before the script ran | Wait for `.js`; require presence; a resize step |
| QA | Bounding boxes are not targets, and 0.5 px admitted 23.6 | Hit-testing; 0.05 px |
| QA, Rust | The D-71 exception unbounded; fixture too small to clear tolerance | A 70-W quadrant, width over 491 read from FR-95, and a lower bound per control |
| Rust | FR-94 already proved in geometry | The audit checks the unmeasured premise (narrow SVG spans the viewport) and HTML controls |
| Rust | Check 3 repeated `interaction.mjs` | Limited to what the tree adds |
| Rust | Only TextPrimary on Surface1 is missing | Task 1 adds just that |
| Architecture | The release PR is the forbidden bump PR | The stated exception, cited in its body |
| Architecture | The bump rewrites the HTML goldens | Task 6 step 6 |
| Architecture | `rg-l4h` is v1.0 | **For the maintainer** (Open, above) |
| Architecture | Run the dependency check before 1.0.0 | Task 6 step 3; the maintainer had already ordered it after 15c |
| Architecture | The manual checklist as a doc; `rg-n54` folds in | Task 5 |
| Accessibility | Expected announcements, not topics | Task 5 |
| Accessibility | 1.4.13, 3.2.1, 3.2.2, 4.1.1 | In the table: exempt, check 9, not applicable, and the HTML5 parse test |
| UI | A visual sign-off step | Task 4 and the checklist |
| UI | The title's size; ragged widths at 400 px | A crumb: neither blocks 1.0.0 |

## Finishing

- Run `just ci`, with the audit included.
- Review seats: code review (holding Copilot's seat), simplify, and accessibility on the audit, since a vacuous audit is the risk.
- `rg-6zu` closes with the release PR.

## Revisions

| Revision | Date | Change |
| --- | --- | --- |
| 1.0 | 2026-10-12 | First draft. |
| 1.1 | 2026-10-12 | Five-seat review folded in (see Review rulings): the full AA table, a DOM-first audit, hit-testing, real forced-colours checks, the table's scroll cue, visual sign-off, real release tooling, and two gates for the maintainer. |
