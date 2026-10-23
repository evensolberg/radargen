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
revision: "1.20"
review_date:
reviewed_by: []
completed_date:
comments: "Part of RG-015C, Task 5. Absorbs crumb rg-n54 (keyboard scrolling of the panel)."
---
# Manual accessibility checks for 1.0.0

`just ci` already runs the automated audit (`scripts/a11y-audit.mjs`) over every WCAG 2.1 AA criterion a browser can measure. What it cannot measure is what a screen reader says, how a real browser's zoom behaves, and whether keys scroll a panel. Those are below.

**The page to test:** `just screenshots` writes the fixtures' pages to its output folder. Use `three-sector.html` for the radar and `interaction.html` for the panel, the table and duplicate names. You can also render your own: `radargen tests/fixtures/three-sector.csv -o radar.html --title "Technology Radar"`.

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

The expected names come from `three-sector.csv`.

1. **Entering the radar.** Tab past the skip link, the legend button and the toggle (on a built-in page; a `--palette` page has no toggle), or browse to the radar.
   - *Expect:* "Technology radar: 3 sectors, 4 rings, 6 items, group".
   - *Expect:* nothing else of the drawing (rings, dividers and labels) is read as content.
2. **A marker.** Tab to the first marker.
   - *Expect:* "1. Kubernetes, Default; Platform sector, Adopt ring, button, collapsed".
   - *Record:* whether a description follows that repeats "1. Kubernetes" (the marker's `<title>`). Decide whether to keep the `<title>`: it is the hover tooltip FR-36 requires, but if it is read as a description it repeats the name. This is the decision crumb `rg-6zu`'s PR 13 note left open.
3. **Opening the panel.** Press Enter.
   - *Expect:* "expanded", or the state change read in some form.
   - *Expect:* focus moves to "Item details, region", then "1. Kubernetes, heading level 2".
4. **Reading the panel.** Read on. For the link, use `interaction.csv`'s first Kafka, which has one; the three-sector fixture has no links.
   - *Expect:* "Close, button".
   - *Expect:* then a list of terms and values (sector, ring, description and so on), each term read before its value.
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
   - *Expect:* "Switch to dark theme, button" ("Switch to light theme" if the system is dark), with no pressed state, and nothing read for the icon.
   - *Expect:* Enter changes the page's colours, and the label becomes the other one.
   - *Expect (sighted):* a moon while the page is light, a sun while it is dark.
   - *Expect (sighted, mouse):* hovering shows the name as a tooltip. After a press, move the pointer off the button and back on: it shows the other name (browsers hide a tooltip on a click until the pointer moves).
   - *Expect:* the name is read once, with no description or delayed hint repeating it.
   - *Record:* whether the new label is announced after the press, or only when focus returns to the button.
9. **The legend button** (RG-102). Tab to it; it is between the skip link and the theme toggle (alone at the end on a `--palette` page, step 18).
   - *Expect:* "Hide legend, button", with no pressed state, and nothing read for the icon.
   - *Expect:* Enter hides both legends; the key stays; the icon becomes a crossed-out list and the name "Show legend".
   - *Record:* whether the new name is announced after the press, or only when focus returns to the button. Silent in two or more pairs is a fail: raise a crumb for a polite status message.
   - *Expect, with the legend still hidden:* browse mode and the headings list no longer find the "Legend" heading or any legend row.
   - *Expect, with the legend still hidden:* reload and press Tab once: "Skip to the legend, button" again, since nothing is remembered. Hide the legend, Shift+Tab to the skip link: "Skip to the table, button", and on Enter, focus on the table's toggle.
   - *Expect, with `--legend hidden`:* on the first visit the button reads "Show legend, button" and the skip link "Skip to the table, button".
   - *Expect (sighted, narrow window):* with the legend below the fold, the icon changes on each press.
   - *Expect (sighted, mouse):* hovering shows "Hide legend" as a tooltip. After a press, move the pointer off the button and back on: it shows "Show legend".
   - *Expect:* the name is read once, with no description or delayed hint repeating it.
   - *The two breakpoints:* `<main>` carries them as media queries, `data-narrow-query` with the legend shown and `data-narrow-query-hidden` with it hidden; each is one pixel under its breakpoint (read them with the browser's inspector, or `grep -o 'data-narrow-query[^>]*' page.html`). "Between the breakpoints" is any window width from the hidden one up to, but not including, the shown one.
   - *Expect, at a window between the two breakpoints* (it shows the narrow arrangement)*:* Enter on the button hides the legend and the page switches to the wide arrangement, once, with no flicker back; focus stays on the button, which is read again as "Show legend, button" or not at all, never as a marker or the page. Enter again shows the legend and switches back to narrow, focus still on the button (D-97).
   - *Expect, at that width, with a marker's panel open:* the switch leaves the panel open; Close returns focus to the same marker in the arrangement now shown.
   - *Expect (sighted, mouse, zoom):* in a 1280 px window at 150%, 175% and 200% zoom, where the page's width in CSS pixels falls between the breakpoints, a mouse press switches the arrangement and the button stays under the pointer, so a second press without moving the mouse switches back.
   - *Expect (sighted, scrolled):* scroll until the button is near the top of the window, press it twice: the button stays where it was, and the page comes back to where it was scrolled.
   - *Expect (touch, landscape):* on a phone or tablet held landscape at a width between the breakpoints, a tap switches the arrangement and the button stays under the finger.
   - *Expect (magnifier, ZoomText or macOS Zoom following focus):* after a press that switches the arrangement, the magnified view stays on the button.
   - *Expect (screen reader):* a press that switches the arrangement announces nothing about the layout, only the button's new name if anything, and the reading order afterwards is the same: title, radar, key, then the legend when shown.
10. **The table** (`interaction.html`). Open *The table* and browse into it.
    - *Expect:* "Items on the radar, region", then a table with its row and column counts. A page rendered with `--table all` reads "Every item, region" instead.
    - *Expect:* moving across a row reads each cell with its column header; moving down a column reads the row header (the name).
    - *Expect, on a page rendered with `--table all`:* the unplotted row's button reads "Legacy Bus, not on the radar". The default page leaves that row out.
11. **Two items with one name** (`interaction.html`).
    - *Expect:* the two "Kafka" markers are told apart by their number and position: "1. Kafka, Moved In; Platforms sector, Assess ring" and "6. Kafka, New; Tools sector, Adopt ring".
12. **Browse mode and navigation lists.**
    - *Expect, NVDA's headings list (Insert+F7) and VoiceOver's rotor (Headings):* level 1 "Technology Radar"; level 2 "Key" and "Legend" (with the legend shown); level 3 each sector; level 4 each ring under it; level 2 "Items on the radar" ("Every item" with `--table all`).
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
16. **The PNG** (D-87, from 1.1.0). Render `three-sector.csv` to `light.png`, and again with `--theme dark` to `dark.png`. Open both in an image viewer.
    - *Expect, in both themes:* the title, the radar, the key and the legend are all present.
    - *Expect:* no hover, focus or selection marks anywhere.
    - *Expect:* no missing-glyph boxes for the fixture's names.
    - *Expect, at the default width and at 100% zoom:* every legend name is readable.
17. **The SVG** (D-88, from 1.2.0). Render `three-sector.csv` with `--title "Technology Radar"` to `light.svg`, and again with `--theme dark` to `dark.svg`.
    - *Expect, opened in a browser:* each looks like its PNG (step 16), in Inter, with every legend name readable at 100% zoom. Text that cannot be selected is expected, not a fail.
    - *Expect, with NVDA + Firefox, NVDA + Chrome and VoiceOver + Safari, opening the file directly:*
      - the document is named "Technology Radar", and its description reads "Technology radar: 3 sectors, …";
      - browse mode moves into the drawing rather than stopping at one graphic;
      - the headings list holds: level 1 "Technology Radar"; level 2 "Key" and "Legend"; level 3 each sector; level 4 each ring under it, as in step 12;
      - ring labels read outermost first, as in the page;
      - each marker reads once, by its full name ("1. …; … sector, … ring"), never its number alone;
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

18. **A `--palette` page** (rg-cdo, from 1.3.0). Write `dark.css`: copy `docs/product/palette.sample.css` and replace each role's value (the eight below the `--series-` entries) with the `dark:` value in its comment. Then render:
    - `radargen tests/fixtures/long-legend.csv -o light-palette.html --palette docs/product/palette.sample.css`
    - `radargen tests/fixtures/long-legend.csv -o dark-palette.html --palette dark.css`

    The long legend makes the legend's box scroll, so there is a scroll bar to judge.
    - *Expect, in Safari and Chrome on macOS with the system set to light and "Show scroll bars: Always":* `dark-palette.html` stays dark, and the page's scroll bar and the legend box's are dark.
    - *Expect, with the system set to dark:* `light-palette.html` stays light, with light scroll bars.
    - *Expect, with NVDA + Firefox and VoiceOver + Safari:* the first Tab says "Skip to the legend, button", and Enter lands on "1. …" in the legend. Reload and press Tab three times: the first stop is the skip link, the second the legend button, and the third the first marker, with nothing between. Browse mode finds no stray button where the toggle would be.
    - *Expect, on the first Tab:* the skip link appears over the page; the title and radar do not move.
    - *Expect, in Windows high-contrast mode with `dark-palette.html`:* the grounds take the system's Canvas colour and focus its Highlight; the sector colours keep the palette's values and stay visible.
    - *Expect, with a six-sector CSV (a copy of `three-sector.csv` with rows in three more sectors) and a palette of five `--series-` entries (a copy of the sample without `--series-6` to `--series-8`):* the sixth sector reuses `--series-1`, and its label and position still tell it apart.

19. **The PDF** (FR-110, from 1.19.0). Render `three-sector.csv` with `--title "Technology Radar"` to `light.pdf`, again with `--theme dark` (keeping `--title`) to `dark.pdf`, again with `--palette dark.css` (step 18, keeping `--title`) to `palette.pdf`, again without `--title` to `untitled.pdf`, and a copy of `three-sector.csv` with one item renamed `中文` to `cjk.pdf`. Render `long-legend.csv` to `long.pdf`, and `long-legend.csv --width 10000` to `huge.pdf`. Render a PNG of each of the first four with the same flags (`light.png`, `dark.png`, `palette.png`, `untitled.png`): step 16's PNGs have no `--title`, so they are not the comparison. `palette.pdf` compares to the PNG rendered with the same `--palette`. Open each PDF in Preview, Acrobat Reader, Chrome (PDFium) and Firefox (pdf.js).
    - *Expect:* each page looks like its PNG; at 400% zoom a ring name's halo is smooth; the dark page shows no light seam at its edges; the window title is "Technology Radar", for `untitled.pdf` too.
    - *Expect, selecting:* drag-selecting a legend name and pasting it gives the name as written; a letter-spaced legend heading and a kerned name ("AVATAR" if the fixture has none, add one) paste as words, not spaced letters; a selection across a wrapped name pastes with a space or line break, not run together.
    - *Expect, searching:* searching for an item name and for a ring name finds each, once: a ring name has no second hit from its halo.
    - *Expect, Select All, copy, paste into a text editor:* the title, key, ring labels and legend in reading order, each once. This is the copy order, the content stream's, which Preview and VoiceOver read; record any disorder or repeat. The screen-reader order follows the tags: step 20.
    - *Expect, `cjk.pdf`:* the run prints a warning naming the line and saying the PDF draws the characters as boxes, though they still read and copy as themselves; the name draws as boxes, the file opens without a warning from the viewer, and copying it gives the characters (from 1.24.0; Acrobat and Chrome honour the box's `/ActualText`, Preview may not).
    - *Expect, printing `light.pdf` to A4 or Letter with Fit to page:* the legend text is readable.
    - *Expect, printing `long.pdf` to Letter or A4 with Fit to page, rendered with and without `--legend-columns 2`:* the single column prints small; two columns print larger. Record the smallest text you can read in each.
    - *Expect, `huge.pdf`:* each viewer shows it at full size, not at 1/n: Document Properties gives the page size (about 104 inches wide). A viewer that ignores `/UserUnit` shows it small or refuses it; record that as a finding, and FR-110's "keeps its size" is then softened for that viewer.
    - *On paper (FR-111, from 1.20.0):* render `long-legend.csv` with `--paper letter` to `letter.pdf` and with `--paper a4 --theme dark` to `a4-dark.pdf`, and a 300-item CSV (one sector, one ring) with `--paper a4` to `a4-long.pdf`. In the four viewers: every page is landscape; page 1 holds only the title, radar, key and the start of the legend; each continuation page opens with *Legend (continued)*, and a column that starts inside a sector or ring repeats its heading marked *(continued)*; every footer reads the title and *Page n of m*.
    - *Expect, printing on paper:* print `letter.pdf` on Letter and on A4, and `a4-long.pdf` on A4 and on Letter, each at Actual size and at Fit to page: on its own paper at Actual size nothing is cut; on the other paper Fit to page loses nothing, as the README says. On a laser print, two-digit white marker numbers and the radar's 7.5 pt ring labels still read. A thin white unprinted frame around `a4-dark.pdf` is the printer's margin, not a defect.
    - *The full PDF (FR-112, from 1.21.0):* render a 300-item CSV with a description and a rationale on every row, in two sectors, with `--pdf full` to `full-letter.pdf` and with `--pdf full --paper a4 --theme dark` to `full-a4-dark.pdf`. In the four viewers: page 1 names the page the table starts on, and the table starts on a page of its own; the header row repeats on every table page; each table page after the first is headed with its sector and item numbers, and a number on the radar finds its row; no row is split; the table's text is searchable and copies in reading order, row by row. Expect about 7 rows to a Letter page when rows run to five lines.
    - *Expect, printing the full PDF:* print `full-letter.pdf` on Letter at Actual size: the 9 pt cells and the muted header row read on a laser print, and on `full-a4-dark.pdf`'s dark pages the muted text still reads.
    - If installed, `qpdf --check light.pdf` reports no errors and `pdffonts light.pdf` lists two embedded subset fonts with ToUnicode.
    - Small text on Windows may hint slightly differently from the PNG; that is expected. From 1.23.0 `untitled.pdf` draws *Technology Radar* as its title, so compare it with `light.png`, not an untitled PNG. Screen-reader navigation is step 20.

20. **The tagged PDF** (FR-114, from 1.23.0). `PDFUA_KEEP=pdfua just pdfua` writes the gate's PDFs to `pdfua/`: `light.pdf`, `untitled.pdf`, `subtitled.pdf`, `letter.pdf`, `full.pdf`, `full-dark.pdf`, `split.pdf` (a row taller than a page), `nothing.pdf` (nothing plotted) and `headers.pdf` (a headers-only CSV under `--table all`), among others. Also use `full-letter.pdf` from step 19. The main pair is **NVDA + Acrobat Reader** on Windows; also check **PAC 2024** (its Matterhorn report and Logical Structure view) and **Chrome's PDF viewer** with NVDA. **VoiceOver + Preview** is a smoke test only: Preview reads content order and largely ignores tags, so record only that text is read.
    - *Expect, Acrobat's Tags pane on `full-letter.pdf`:* `H1` (title), `P` (subtitle if any), `P` (the pointer), `Figure`, `H2` Key and its `L`, `H2` Legend, then sector `H3`s and ring `H4`s each followed by one `L`, then `H2` (the table's heading) and one `Table`.
    - *Expect, NVDA browse mode, H and Shift+H:* every heading in order, the title first; no *(continued)* heading, no *Legend (continued)*; footers and *Page n of m* are never read.
    - *Expect, G (graphics) on the radar in `light.pdf`:* "Technology radar: 3 sectors, 4 rings, 6 items" once, and none of the marker numbers or ring labels inside it.
    - *Expect, L (lists) in the legend:* each ring's rows announced as one list with its full count, even where the ring breaks across a page (`letter.pdf`); each item's number then its name and meta.
    - *Expect, T then Ctrl+Alt+arrows in the table:* the column headers announced as you move across, the first as *Number*, not "number sign"; moving down announces the item's name as the row header. **Record how verbose vertical moves are:** the name cell also holds the folded line (domain · qualifier · status · dates) and is read with it; if that is too much to bear, file a crumb to make the number cell the row header instead.
    - *Expect, `split.pdf`:* the long row reads as one row across the page break; *continues on the next page*, the repeated name and *(continued)* are silent.
    - *Expect, `nothing.pdf` and `headers.pdf`:* the table is announced with its column count; the empty one's single row reads *Nothing is on the radar.*
    - *Expect, `full-dark.pdf`:* the same as light.
    - *Expect, in the table:* a wrapped heading such as "Sector ring" and a wrapped description are read as words, not run together across the line break; the legend reads an item's name, then its qualifier and status after a pause ("Kubernetes, Default").
    - *Expect:* the legend's ` · ` separator is not read as "dot" at default punctuation settings.
    - *Expect, PAC 2024:* no Matterhorn failures but the human-checked ones it lists for review; record any it flags.

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
| The PDF: Preview, Acrobat Reader, Chrome, Firefox; light, dark, palette | | | | |
| The PDF, `long.pdf` printed with and without `--legend-columns 2`, and `huge.pdf` at full size: each of the four viewers | | | | |
| The tagged PDF (step 20): NVDA + Acrobat Reader, NVDA + Chrome, PAC 2024; VoiceOver + Preview as a smoke test | | | | |

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
| 1.9 | 2026-10-17 | Step 9 covers browse mode, a hidden start, the narrow icon change and a fail rule for silent renames; the skip-link-to-table check moves there from step 7 (RG-102 review). |
| 1.10 | 2026-10-17 | Steps 8 and 9: the icon buttons' hover tooltips (rg-7gd). |
| 1.11 | 2026-10-17 | Steps 10 and 12: the default table's heading is "Items on the radar", "Every item" under `--table all`, which alone lists the unplotted row (FR-106, rg-fgf). |
| 1.12 | 2026-10-18 | Step 9: a press between the two breakpoints switches the arrangement, focus staying on the button, and an open panel survives it (D-97, rg-dok). |
| 1.13 | 2026-10-18 | Step 9: how to read the two breakpoints from `<main>`; checks for zoom with the mouse, a scrolled page, touch in landscape, a magnifier and a screen reader across a switch (rg-dok review). |
| 1.14 | 2026-10-22 | Sectors, not quadrants: the expected announcements, headings and the `three-sector` fixture (D-101; rg-k16). |
| 1.15 | 2026-10-22 | New step 19: the PDF, selecting, searching and reading order in four viewers (FR-110; rg-5uf). |
| 1.16 | 2026-10-22 | Step 19: `long.pdf` printed Fit to page with and without two legend columns, `huge.pdf` at full size under `/UserUnit`, matching PNGs, the `cjk.pdf` warning and each name once (FR-110, FR-104). |
| 1.17 | 2026-10-22 | Step 19: PDFs on paper (`--paper letter`, `--paper a4`, dark, a 300-item legend): landscape pages, page 1's parts, *Legend (continued)* and *(continued)* headings, footers, and printing at Actual size and Fit to page on both papers (FR-111; rg-4b8). |
| 1.18 | 2026-10-23 | Step 19: the full PDF (`--pdf full`): the page-1 pointer, the table on its own pages, the repeated header row, later headings with sector and item numbers, no split rows, reading order, and printing at 9 pt in both themes (FR-112; rg-4b8). |
| 1.19 | 2026-10-23 | New step 20: the tagged PDF in NVDA + Acrobat, PAC 2024 and Chrome, VoiceOver + Preview as a smoke test: tag order, headings, the figure's name, lists across pages, table headers and row-header verbosity, a split row, empty tables, the separator; `untitled.pdf` now draws its title (FR-114, rg-ffj, rg-v1o). |
| 1.20 | 2026-10-23 | Step 19: `cjk.pdf`'s boxes copy as their characters, and the warning says so (FR-113, rg-ffj); step 20's `PDFUA_KEEP` set includes `rtl.pdf` and `boxes.pdf`. |
