---
tags:
  - a11y
  - release
aliases: []
doc_id: RG-015C-M
doc_name: manual-checks
crumb_id: rg-6zu
document_title: "Manual accessibility checks for 1.0.0"
created_date: 2026-10-13
synopsis: "What only a person with a screen reader, a real browser's zoom or a keyboard on a real machine can judge, step by step, with the announcement to expect at each step and a results table to fill in before tagging 1.0.0. The automated audit (scripts/a11y-audit.mjs) covers everything a browser can measure; this covers the rest."
status: draft
type: Guide
revision: "1.2"
review_date:
reviewed_by: []
completed_date:
comments: "Part of RG-015C, Task 5. Absorbs crumb rg-n54 (keyboard scrolling of the panel)."
---
# Manual accessibility checks for 1.0.0

`just ci` already runs the automated audit (`scripts/a11y-audit.mjs`) over every WCAG 2.1 AA criterion a browser can measure. What it cannot measure is what a screen reader says, how a real browser's zoom behaves, and whether keys scroll a panel. Those are below.

**The page to test:** `just screenshots` writes the fixtures' pages to its output folder. Use `three-quadrant.html` for the radar and `interaction.html` for the panel, the table and duplicate names. You can also render your own: `radargen tests/fixtures/three-quadrant.csv -o radar.html --title "Technology Radar"`.

**How to record:** for each pair below, mark each step **pass** or **fail** in the results table, with a note. Expect the substance of each announcement, not its exact wording: screen readers word things differently. A failing step becomes a crumb, and open ones hold the tag.

## The pairs

| Pair | Platform |
| --- | --- |
| NVDA + Firefox | Windows |
| NVDA + Chrome | Windows |
| JAWS + Chrome | Windows |
| VoiceOver + Safari | macOS |
| VoiceOver + Safari | iOS |

## Steps, with what to expect

The expected names come from `three-quadrant.csv`.

1. **Entering the radar.** Tab past the skip link and the toggle, or browse to the radar.
   - *Expect:* "Technology radar: 3 quadrants, 4 rings, 6 items, group".
   - *Expect:* nothing else of the drawing (rings, dividers and labels) is read as content.
2. **A marker.** Tab to the first marker.
   - *Expect:* "1. Kubernetes, Default; Platform quadrant, Adopt ring, button, collapsed".
   - *Record:* whether a description follows that repeats "1. Kubernetes" (the marker's `<title>`). Decide whether to keep the `<title>`: it is the hover tooltip FR-36 requires, but if it is read as a description it repeats the name. This is the decision crumb `rg-6zu`'s PR 13 note left open.
3. **Opening the panel.** Press Enter.
   - *Expect:* "expanded", or the state change read in some form.
   - *Expect:* focus moves to "Item details, region", then "1. Kubernetes, heading level 2".
4. **Reading the panel.** Read on. For the link, use `interaction.csv`'s first Kafka, which has one; the three-quadrant fixture has no links.
   - *Expect:* "Close, button".
   - *Expect:* then a list of terms and values (quadrant, ring, description and so on), each term read before its value.
   - *Expect:* a link announced with its address and "opens in a new tab".
5. **Closing.** Press Escape.
   - *Expect:* focus returns to the marker: "1. Kubernetes, …, button, collapsed".
   - *Repeat with Close:* the same.
6. **A legend row.** Tab on past the markers.
   - *Expect:* the legend rows in number order, each "N. Name …, button, collapsed".
7. **The skip link.** Reload and press Tab once.
   - *Expect:* "Skip to the legend, button".
   - *Expect:* on Enter, focus on "1. …" in the legend.
8. **The toggle.** Tab to it.
   - *Expect:* "Switch to dark theme, button" ("Switch to light theme" if the system is dark), with no pressed state.
   - *Expect:* Enter changes the page's colours, and the label becomes the other one.
   - *Record:* whether the new label is announced after the press, or only when focus returns to the button.
9. **The table** (`interaction.html`). Open *The table* and browse into it.
   - *Expect:* "Every item, region", then a table with its row and column counts.
   - *Expect:* moving across a row reads each cell with its column header; moving down a column reads the row header (the name).
   - *Expect:* the unplotted row's button reads "Legacy Bus, not on the radar".
10. **Two items with one name** (`interaction.html`).
    - *Expect:* the two "Kafka" markers are told apart by their number and position: "1. Kafka, Moved In; Platforms quadrant, Assess ring" and "6. Kafka, New; Tools quadrant, Adopt ring".
11. **Browse mode and navigation lists.**
    - *Expect, NVDA's headings list (Insert+F7) and VoiceOver's rotor (Headings):* level 1 "Technology Radar"; level 2 "Key" and "Legend"; level 3 each quadrant; level 4 each ring under it; level 2 "Every item".
    - *Expect, iOS swipe order:* the same order as Tab.
12. **Scrolling the panel with keys** (crumb `rg-n54`). In desktop Chrome, Firefox and Safari, at a small window (about 320 by 260), open a long item's panel. With focus on the panel heading:
    - *Expect:* ArrowDown and PageDown scroll the panel's content.
    - *If not:* the panel needs to be a focusable scroll region (`tabindex="0"`), and this is a fail.
13. **Real zoom.** In each desktop browser, zoom to 200%, then to 400%.
    - *Expect:* at 200%, text is twice the size. The page switches to the narrow arrangement where the window is narrow enough, and nothing overlaps.
    - *Expect:* at 400% on a 1280 px window, nothing scrolls sideways except the table.
14. **The wide arrangement's parts** (D-85). At a wide window, the arrangement is four SVGs: title, radar, key, legend.
    - *Expect, browsing from the title to the legend:* the title heading, the radar's group, the key and the legend, with no extra unnamed "graphic", "image" or "group" between them.
    - *Expect, in Firefox:* Tab from the last marker goes straight to the first legend row, with no stop on the legend's box.
    - *Expect, with `long-legend.csv`:* Tab down the legend scrolls it within its box, and the radar stays where it is.

## Results

| Pair | Version (AT, browser, OS) | Date | Steps passed | Failed steps and notes |
| --- | --- | --- | --- | --- |
| NVDA + Firefox | | | | |
| NVDA + Chrome | | | | |
| JAWS + Chrome | | | | |
| VoiceOver + Safari, macOS | | | | |
| VoiceOver + Safari, iOS | | | | |
| Keys scroll the panel: Chrome, Firefox, Safari | | | | |
| Real zoom 200% and 400%: Chrome, Firefox, Safari | | | | |
| The wide arrangement's parts: NVDA + Firefox, VoiceOver + Safari | | | | |

## Revisions

| Revision | Date | Change |
| --- | --- | --- |
| 1.0 | 2026-10-13 | First version, for 1.0.0. |
| 1.1 | 2026-10-15 | Step 8: the toggle is named by what a press does, with no pressed state (rg-nuy). |
| 1.2 | 2026-10-15 | Step 14: the wide arrangement drawn in four SVGs, the legend scrolling (D-85, rg-swu). |
