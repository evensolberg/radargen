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
revision: "1.8"
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

1. **Entering the radar.** Tab past the skip link, the legend button and the toggle (on a built-in page; a `--palette` page has no toggle), or browse to the radar.
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
   - *Expect, with the legend hidden (step 9):* "Skip to the table, button", and on Enter, focus on the table's toggle.
8. **The toggle.** Tab to it.
   - *Expect:* "Switch to dark theme, button" ("Switch to light theme" if the system is dark), with no pressed state, and nothing read for the icon.
   - *Expect:* Enter changes the page's colours, and the label becomes the other one.
   - *Expect (sighted):* a moon while the page is light, a sun while it is dark.
   - *Record:* whether the new label is announced after the press, or only when focus returns to the button.
9. **The legend button** (RG-102). Tab to it; it is between the skip link and the theme toggle.
   - *Expect:* "Hide legend, button", with no pressed state, and nothing read for the icon.
   - *Expect:* Enter hides both legends; the key stays; the icon becomes a crossed-out list and the name "Show legend".
   - *Record:* whether the new name is announced after the press, or only when focus returns to the button.
10. **The table** (`interaction.html`). Open *The table* and browse into it.
   - *Expect:* "Every item, region", then a table with its row and column counts.
   - *Expect:* moving across a row reads each cell with its column header; moving down a column reads the row header (the name).
   - *Expect:* the unplotted row's button reads "Legacy Bus, not on the radar".
11. **Two items with one name** (`interaction.html`).
    - *Expect:* the two "Kafka" markers are told apart by their number and position: "1. Kafka, Moved In; Platforms quadrant, Assess ring" and "6. Kafka, New; Tools quadrant, Adopt ring".
12. **Browse mode and navigation lists.**
    - *Expect, NVDA's headings list (Insert+F7) and VoiceOver's rotor (Headings):* level 1 "Technology Radar"; level 2 "Key" and "Legend"; level 3 each quadrant; level 4 each ring under it; level 2 "Every item".
    - *Expect, iOS swipe order:* the same order as Tab.
13. **Scrolling the panel with keys** (crumb `rg-n54`). In desktop Chrome, Firefox and Safari, at a small window (about 320 by 260), open a long item's panel. With focus on the panel heading:
    - *Expect:* ArrowDown and PageDown scroll the panel's content.
    - *If not:* the panel needs to be a focusable scroll region (`tabindex="0"`), and this is a fail.
14. **Real zoom.** In each desktop browser, zoom to 200%, then to 400%.
    - *Expect:* at 200%, text is twice the size. The page switches to the narrow arrangement where the window is narrow enough, and nothing overlaps.
    - *Expect:* at 400% on a 1280 px window, nothing scrolls sideways except the table.
15. **The wide arrangement's parts** (D-85). At a wide window, the arrangement is four SVGs: title, radar, key, legend.
    - *Expect, browsing from the title to the legend:* the title heading, the radar's group, the key and the legend, with no extra unnamed "graphic", "image" or "group" between them.
    - *Expect, in Firefox:* Tab from the last marker goes straight to the first legend row, with no stop on the legend's box.
    - *Expect, with `long-legend.csv`:* Tab down the legend scrolls it within its box, and the radar stays where it is.
    - *Expect, at a narrow window:* the narrow arrangement is two SVGs, its body and its legend (RG-102). Browsing from the key to the legend meets no extra unnamed "graphic", "image" or "group" between them.
16. **The PNG** (D-87, from 1.1.0). Render `three-quadrant.csv` to `light.png`, and again with `--theme dark` to `dark.png`. Open both in an image viewer.
    - *Expect, in both themes:* the title, the radar, the key and the legend are all present.
    - *Expect:* no hover, focus or selection marks anywhere.
    - *Expect:* no missing-glyph boxes for the fixture's names.
    - *Expect, at the default width and at 100% zoom:* every legend name is readable.
17. **The SVG** (D-88, from 1.2.0). Render `three-quadrant.csv` with `--title "Technology Radar"` to `light.svg`, and again with `--theme dark` to `dark.svg`.
    - *Expect, opened in a browser:* each looks like its PNG (step 16), in Inter, with every legend name readable at 100% zoom. Text that cannot be selected is expected, not a fail.
    - *Expect, with NVDA + Firefox, NVDA + Chrome and VoiceOver + Safari, opening the file directly:*
      - the document is named "Technology Radar", and its description reads "Technology radar: 3 quadrants, …";
      - browse mode moves into the drawing rather than stopping at one graphic;
      - the headings list holds: level 1 "Technology Radar"; level 2 "Key" and "Legend"; level 3 each quadrant; level 4 each ring under it, as in step 12;
      - ring labels read outermost first, as in the page;
      - each marker reads once, by its full name ("1. …; … quadrant, … ring"), never its number alone;
      - each legend row reads once;
      - no label reads twice, no halo is read, and nothing reads as an unnamed "graphic" or "group".
      - Record whether a marker's tooltip is read again as a description.
    - *Expect, in PowerPoint (Insert ▸ Pictures), and in Figma or Illustrator if available, in both themes:*
      - the text is the theme's colour, not black;
      - ring labels show no dark smudge behind them, which would be a halo drawn filled;
      - the drawing can be ungrouped, and each glyph is a shape.
      - Record the alt text PowerPoint assigns.
      - A black or smudged result is a fail. The fallback is paint on each `<use>` (plan Ruling 12).
    - *Expect, without `--title`:* the document is named "Technology Radar", and no level-1 heading exists.

18. **A `--palette` page** (rg-cdo, from 1.3.0). Write `dark.css`: copy `docs/product/palette.sample.css` and replace each role's value (the eight below the quadrant colours) with the `dark:` value in its comment. Then render:
    - `radargen tests/fixtures/long-legend.csv -o light-palette.html --palette docs/product/palette.sample.css`
    - `radargen tests/fixtures/long-legend.csv -o dark-palette.html --palette dark.css`

    The long legend makes the legend's box scroll, so there is a scroll bar to judge.
    - *Expect, in Safari and Chrome on macOS with the system set to light and "Show scroll bars: Always":* `dark-palette.html` stays dark, and the page's scroll bar and the legend box's are dark.
    - *Expect, with the system set to dark:* `light-palette.html` stays light, with light scroll bars.
    - *Expect, with NVDA + Firefox and VoiceOver + Safari:* the first Tab says "Skip to the legend, button", and Enter lands on "1. …" in the legend. Reload and press Tab three times: the first stop is the skip link, the second the legend button, and the third the first marker, with nothing between. Browse mode finds no stray button where the toggle would be.
    - *Expect, on the first Tab:* the skip link appears over the page; the title and radar do not move.
    - *Expect, in Windows high-contrast mode with `dark-palette.html`:* the grounds take the system's Canvas colour and focus its Highlight; the quadrant colours keep the palette's values and stay visible.
    - *Expect, with a six-quadrant CSV (a copy of `three-quadrant.csv` with rows in three more quadrants) and a palette of five `--series-` entries (a copy of the sample without `--series-6` to `--series-8`):* the sixth quadrant reuses `--series-1`, and its label and position still tell it apart.

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
| The PNG, light and dark | | | | |
| The SVG: NVDA + Firefox, NVDA + Chrome, VoiceOver + Safari | | | | |
| The SVG: PowerPoint, light and dark | | | | |
| The SVG: Figma or Illustrator, light and dark | | | | |
| A `--palette` page: scroll bars, NVDA + Firefox, VoiceOver + Safari, high contrast | | | | |

## Revisions

| Revision | Date | Change |
| --- | --- | --- |
| 1.0 | 2026-10-13 | First version, for 1.0.0. |
| 1.1 | 2026-10-15 | Step 8: the toggle is named by what a press does, with no pressed state (rg-nuy). |
| 1.2 | 2026-10-15 | Step 14: the wide arrangement drawn in four SVGs, the legend scrolling (D-85, rg-swu). |
| 1.3 | 2026-10-16 | Step 15: the PNG, in both themes (D-87, rg-42m). |
| 1.4 | 2026-10-16 | Step 16: the exported SVG (D-88, rg-m2l). |
| 1.5 | 2026-10-17 | Step 17: a page drawn with `--palette`, light and dark; step 1 notes it has no toggle (rg-cdo, RG-018A R25). |
| 1.6 | 2026-10-17 | Step 8: the toggle shows a moon or a sun, the theme a press switches to, and its icon is not announced (rg-1ru). |
| 1.7 | 2026-10-17 | Step 14: the narrow arrangement's two SVGs, body and legend (RG-102). |
| 1.8 | 2026-10-17 | New step 9: the legend button; step 7 adds the skip link with the legend hidden; later steps renumbered (RG-102). |
