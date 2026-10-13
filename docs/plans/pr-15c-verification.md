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
synopsis: "SC-17's audit of the finished page, automated where a browser can measure it: every control's name, role, description and expanded state from Chromium's accessibility tree (4.1.2); target sizes at every width down to 320 px, with D-71's widened narrow arrangement as the recorded exception (2.5.8); reflow and resize (1.4.10, 1.4.4); all under both themes and forced colours. What a browser cannot judge goes in a manual checklist for NVDA, JAWS and VoiceOver. The release is 1.0.0, cut in its own PR once the maintainer has tested."
status: draft
type: Plan
revision: "1.0"
review_date:
reviewed_by: []
completed_date:
comments: "Part three of PR 15 (rg-6zu). The maintainer ruled on 2026-10-12: the first release is 1.0.0, after the maintainer's own testing."
---
# PR 15c — verification and the first release: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-6zu` (part three) · **Phase:** v1.0 · **After:** PR 15b-2 (#73) · **Then:** the 1.0.0 release PR, and the dependency check (crumb)

**Goal:**

- **Close SC-17 for HTML** in two ways:
  - an automated audit in headless Chromium of every criterion a browser can measure;
  - a manual checklist for what only a person with a screen reader can judge.
- **Prepare the 1.0.0 release, without cutting it.** The maintainer tests first. The release PR then carries the version bump, the changelog and the tag (the "every PR is a release" rule, applied to the release itself).

**Architecture:**

- **`scripts/a11y-audit.mjs`** builds on `scripts/cdp.mjs` and joins `just browser`, so `just ci` runs it.
- **What it reads:** Chromium's accessibility tree (`Accessibility.getFullAXTree`), computed boxes and computed styles.
- **What it runs over:**
  - widths 1400, the breakpoint, the breakpoint less one, 400 and 320;
  - light and dark (emulated `prefers-color-scheme`), and forced colours;
  - the interaction fixture and a new D-71 fixture.
- **Contrast stays in Rust**, where the palette tests (`src/palette.rs`, FR-82) already prove every text and non-text pair from the palette's values in both themes. Reading colours back from a browser would add nothing but flakiness. This PR adds only the pairs the interactive layer introduced, as unit tests.

**Spec:** `docs/product/radargen-prd.md` v0.62: SC-17, NFR-1, FR-50, FR-80, FR-94, D-66, D-71. Plan RG-015B for the controls under test.

## What SC-17 asks, and where each part is proved

| SC-17 criterion | Proved by | Status before 15c |
| --- | --- | --- |
| 1.4.3 text contrast | `src/palette.rs` tests (FR-82, D-54), both themes | done |
| 1.4.11 non-text contrast: markers, ring strokes, focus | `src/palette.rs`; the selection and hover marks are added here | partly; Task 1 |
| 2.5.8 target size at the breakpoint and at 320 px | FR-94's geometry tests in Rust; **in the browser, Task 2** | geometry only |
| 4.1.2 name, role, expanded state | **Task 2**, from the accessibility tree | none in a browser |
| 1.4.10 reflow at 320 px and at 400% zoom | `text-spacing.mjs` at 320 px; Task 2 adds the zoom case | partly |
| 1.4.4 resize text to 200% | **Task 2**, at half the width with the text checks | none |
| 1.4.12 text spacing, both arrangements | `just text-spacing` (D-78), with the table and the panel | done |
| 3.1.1 page language | `tests/serialise/page.rs` (FR-80) | done |
| both themes | Task 2 runs light and dark | — |
| the D-71 widened narrow arrangement, as the recorded 2.5.8 exception | **Task 2**, with a fixture whose narrow width passes 480 units | none |
| screen-reader behaviour: the `<title>` description, announcements, panel scrolling | **Task 3**, the manual checklist | none |

## Global Constraints

- The constraints of RG-015B hold: Rust 2024, clippy and lclippy clean, rustqual IOSP, and NFR-4.
- **Trusted input only.** The audit reloads the page per configuration, polls instead of sleeping, and fails on any page exception or dialog.
- **The tolerance for 2.5.8 is 0.5 px**, so subpixel rounding cannot hide a real failure.
- **No version bump in this PR.** 1.0.0 is its own PR, after the maintainer's tests.

## Review Focus

1. **A control whose accessible name loses its visible text** (2.5.3) at some width or theme. *Test:* Task 2 compares each control's computed name with its `aria-label`, and checks that the visible text is a substring of it.
2. **A target under 24 px at 320 px that is not D-71's recorded exception.** *Test:* Task 2 fails on any control below 24×24, except the controls of the D-71 fixture's narrow arrangement, which must instead be reported, and must be exactly those.
3. **Expanded state out of step across arrangements.** After opening an item, exactly that item's controls report expanded in the tree, in whichever arrangement is shown. *Test:* Task 2.
4. **Forced colours hiding a state.** Focus, selection and the panel's border must still be painted in forced colours. *Test:* Task 2 checks that each state's mark computes to a system colour and is visible.
5. **Two controls with the same accessible name.** That would happen when two rows share a name (FR-28), which is legal, so FR-50's position clause has to make them differ. *Test:* Task 2 on the fixture's two `Kafka` rows.

## Tasks

### Task 1: the interactive layer's colours, proved from the palette

**Files:** `src/palette.rs` (tests)

- [ ] **Step 1: failing tests**, in both themes:
  - **Selection square and legend bar:** `--text-primary` at ≥ 3:1 against Surface1 and Surface2 (1.4.11).
  - **Hover dashes:** `--text-muted` at ≥ 3:1 against Surface1 and Surface2.
  - **Table selection:** the border (`--text-primary`) at ≥ 3:1 against Surface1, which is the selected row's tint, and against Surface2, the table.

  If one of these already exists under another name, cite it rather than duplicating it.
- [ ] **Step 2: implement** (tests only), and **commit** `test(palette): the interactive layer's marks clear 3:1`.

### Task 2: the browser audit

**Files:**
- Create: `scripts/a11y-audit.mjs`
- Create: `tests/fixtures/d71-wide-narrow.csv`, whose narrow arrangement passes 480 units through one long quadrant name, as in `tests/diagnostics.rs`'s D-71 case
- Modify: `justfile` (`a11y-audit`; `browser` gains it)

**Checks**, for each configuration:

1. **Name and role (4.1.2, FR-50, 2.5.3):**
   - every marker and legend row in the shown arrangement is `button` in the tree;
   - its name equals its `aria-label`;
   - its visible text, the number and name, is contained in that name;
   - every table name button is `button`, named by its text.
2. **Description:** where a marker carries a `<title>`, the tree reports it as the control's description. This is recorded, not judged: whether it should be read is the manual check's question (Task 3).
3. **Expanded state:**
   - every control reports `expanded: false` at load;
   - after Enter on marker 3, exactly item 3's controls in the shown arrangement and its table button report `expanded: true`;
   - the panel is a `region` named "Item details";
   - the controls' `aria-controls` resolves to it.
4. **Unique names (Review Focus 5):** no two controls in the shown arrangement share a name.
5. **Target size (2.5.8, FR-94):**
   - **what is measured:** every marker's hit area, legend row, table name button, `summary`, theme toggle, skip link (while focused) and Close;
   - **the threshold:** each box is at least 24×24 CSS px, within 0.5 px, at every width;
   - **the exception:** on the D-71 fixture at 320 px, the shortfalls are reported as D-71's exception, and the run fails if any control outside the narrow arrangement falls short, or if no control does (the fixture would then not be testing D-71).
6. **Reflow and resize (1.4.10, 1.4.4):**
   - at 320 px, and at 640 px with the root font size doubled (200%), the document never scrolls sideways;
   - no text in the page chrome (headings, summary, panel, table header) spills out of its box, by the visible-text clip check that `text-spacing.mjs` uses, shared through `cdp.mjs`.
7. **Forced colours** (`forced-colors: active`):
   - a focused marker's ring;
   - a selected marker's square;
   - a selected legend row's bar;
   - the panel's border.

   Each computes to a system colour and has computed `visibility: visible`.

- [ ] **Step 1:** write the audit. Run it on the 15b-1 build (`git worktree` of `74f95ac`): check 3 fails, since nothing there carries expanded state.
- [ ] **Step 2:** run it on main. Every check passes, and the D-71 exception is reported and bounded.
- [ ] **Step 3: mutation-check**, each of which must fail the audit:
  - drop `aria-controls` from the start-up loop (check 3);
  - set a legend row's `aria-label` to omit its number (check 1);
  - shrink the hit radius in the build (check 5).
- [ ] **Step 4:** commit `test(a11y): SC-17's audit in a browser`.

### Task 3: the manual checklist

**Files:** Create `docs/a11y/manual-checks.md` (RG-015C-M), with these sections:

- **Screen readers:** NVDA with Firefox and Chrome, JAWS with Chrome, VoiceOver with Safari on macOS and iOS. For each, step by step:
  - reaching the radar group, its name and size (D-75);
  - Tabbing a marker: its name, its `<title>` description (decide whether to keep the `<title>`; rg-6zu's PR 13 note), and its expanded state;
  - opening the panel and hearing the region and the heading;
  - reading the fields; Close; Escape and where focus lands;
  - the table: rows, row headers, the flag;
  - the skip link and the toggle.
- **Keyboard scrolling of the panel** (crumb `rg-…`): in desktop Chrome, Firefox and Safari, with focus on the heading, ArrowDown and PageDown scroll the panel.
- **Zoom:** 200% and 400% in each browser's own zoom, not emulated.
- **Results table:** browser, AT, version, date, pass or fail, notes. The maintainer fills it in before tagging 1.0.0.

- [ ] **Step 1:** write it. **Step 2:** commit `docs(a11y): the manual checklist for 1.0.0`.

### Task 4: release preparation (no bump)

- [ ] **Step 1:** check the changelog tooling. If `git-cliff` is configured, preview the 1.0.0 section with `--unreleased`; otherwise draft `CHANGELOG.md`'s 1.0.0 section by hand from PRs #47–#73. Do not commit the version.
- [ ] **Step 2:** write `docs/release-1.0.0.md`, the release checklist:
  1. the maintainer's tests, and the manual checklist filled in;
  2. `just ci` green;
  3. `mise run bump major`, or the project's equivalent, to 1.0.0 in every released artifact;
  4. the changelog regenerated;
  5. the PR, merged;
  6. `git tag --list v1.0.0` empty, then the tag on the merge commit;
  7. the GitHub release;
  8. a note if Actions is still blocked.
- [ ] **Step 3:** commit `docs: the 1.0.0 release checklist`.

## Finishing

- Run `just ci`, with the audit included.
- Review seats: code review (holding Copilot's seat), simplify, and accessibility on the audit itself, since a vacuous audit is the failure to fear.
- `rg-6zu` closes with the release PR, not here.
- Then the dependency check (crumb), in its own PR.

## Revisions

| Revision | Date | Change |
| --- | --- | --- |
| 1.0 | 2026-10-12 | First draft. |
