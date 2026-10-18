---
tags: [spec, html, legend, a11y]
aliases: []
doc_id: RG-102
doc_name: legend-toggle
crumb_id: rg-3gc
document_title: "Showing and hiding the legend on the HTML page"
created_date: 2026-10-17
synopsis: "An icon button on the HTML page shows and hides the legend in both arrangements, and --legend shown|hidden sets where it starts. The narrow arrangement is cut into a body and a legend part so CSS can hide the legend; the key always shows. PNG and SVG are unchanged."
status: active
type: Spec
revision: "1.4"
review_date:
reviewed_by: []
completed_date:
comments: "From rg-ek8, split into rg-3gc. Design agreed in conversation on 2026-10-17."
---

# Showing and hiding the legend on the HTML page

## Purpose

On the HTML page the legend is not strictly needed: hovering or opening a
marker names its item, and the table view lists every item. A reader who
does not need it should be able to put it away, for a cleaner view and so
the radar can grow where the legend's width was holding it back. The page's
author chooses where the page starts.

## What the reader sees

- An **icon button** in the page controls, beside the theme toggle and the
  same size. Its icon shows the legend's state: a list while the legend
  shows, a crossed-out list while it is hidden, swapped by CSS as the moon
  and sun are. That is the press's visible feedback, which matters in the
  narrow arrangement, where the legend is usually below the fold and a
  press would otherwise change nothing on screen. Its name, in visually
  hidden text, says what a press does: **Hide legend** while the legend
  shows, **Show legend** while it is hidden. No pressed state (the rg-nuy
  ruling).
- A press hides or shows the legend in **both arrangements at once**, so
  crossing FR-69's breakpoint keeps the state.
- **The key always shows.** Only the legend panel, which maps numbers to
  names, is hidden. The title, radar and key are unaffected.
- The choice is **not remembered** between visits, as the theme choice is
  not: every visit starts where the author set it. Storage is also
  unreliable on `file://` pages, which is how many radars are opened.
- **Printing always shows the legend**: a printout has no button to bring
  it back.
- **Without script** there is no button and the legend always shows,
  whatever the page was set to (FR-98's rule: nothing claims an action it
  cannot perform).
- With **nothing plotted** there is no legend, so there is no button, as the
  skip link is already hidden then.
- A **`--palette` page** has the button too: it does not depend on the
  built-in themes.

## The flag

`--legend <shown|hidden>`, default `shown`, matched case-insensitively,
with **no short form**: a page's start is set once per edition, and FR-100's
planned `--legend-columns` should not find `-l` taken by a flag for a
different format. The values `show` and `hide` are accepted as hidden
aliases: they work but the help lists only `shown|hidden`.

It sets where the HTML page starts. PNG and SVG always draw the legend, so
**`--legend hidden` with a PNG or SVG output is one warning**, and the run
still succeeds: `--legend hidden applies to HTML pages; PNG and SVG always
draw the legend`. Unlike `--theme` on HTML, which costs the reader nothing,
ignoring it silently would hand the author the opposite of what was asked.
`--legend shown` matches what is drawn, so it is silent.

With **nothing plotted** the page is written without `data-legend`, whatever
the flag says: there is no legend and no button to bring one back.

## Layout

### The narrow arrangement in parts

The narrow arrangement is one document today. HTML now draws it as two,
the way the wide arrangement is drawn as four (D-85):

- **body**: the title, radar, key rule and key, and the gap under the key;
- **legend**: the legend panel.

The cut runs at the legend panel's top edge, so the body keeps the gap
under the key and the page does not butt against the key when the legend
is hidden. Each part is
the narrow document's own drawing translated to its own origin, as
`wide_parts` does; nothing is laid out again. PNG and SVG keep drawing the
whole narrow document and do not change.

### Hiding

The page writes `data-legend="hidden"` on `<html>` when started hidden; the
button sets and removes it. The hiding rules apply only under the `js`
class the script adds, so without script the legend shows, and only on
screen, so print shows it. The narrow legend, last in its column, is removed;
the wide one is made invisible in place (below). A page started hidden may paint its legend
before the script runs and then hide it: a brief shift, accepted as the
cost of FR-98's rule.

### The wide radar while the legend is hidden

The wide legend turns invisible in place, which also takes its rows out of
the tab order and the accessibility tree, so the grid's columns, the
legend's scrollbar gutter included, never change. The radar spans both
columns, its SVG keeping its existing cap (as tall as the viewport below
the title). It stays left-aligned with the title, controls and table, and
grows only where the window was too narrow for radar and legend side by
side; on a short, wide window it keeps its size and the space to its right
is empty, under the button that brings the legend back. (A one-column grid
as wide as radar and legend was tried first; it missed by the scrollbar
gutter, which CSS cannot know, and moved everything a few pixels.) The title, controls bar
and table keep their positions and widths, so nothing moves sideways or
shrinks when the legend is toggled. The key spans both columns with the
radar, never wider than it (rg-dok), so it keeps its width too except where
the radar grows, and then grows with it, as it does on a resize.

### The breakpoint moves with the legend

*Since rg-dok (PRD D-97).* FR-69's breakpoint is computed with the legend
beside the radar (891 px for the PRD example). While the legend is hidden
the page switches at a second, lower one, the same rule with the legend's
width taken out: `R − 0.2 × R` for a radar part `R` wide, 536 px for the PRD
example. The title part is cut as wide as the radar part so it never draws
smaller than the radar, and the key spans the columns with the radar, so
all three reach 0.8 together; the invisible legend's column always fits
inside that width. The second query applies on screen under the script's
class only, so a page without script, or printed, keeps FR-69's breakpoint,
as it keeps its legend. The script's breakpoint focus handling listens to
both queries and acts on the one for the legend's state.

On a window between the two, a press switches the arrangement: hiding the
legend shows the wide one and showing it brings back the narrow one, once
each, since the switch is the stylesheet's and the queries do not change.
The button is in the page controls, outside both arrangements, so it keeps
focus; where a click does not focus it and the switch hides what had focus,
focus moves to it, as for a hidden row. An open panel stays open, its item
selected in the arrangement now shown, and closes to that item there. A
page started with `--legend hidden` is drawn in the right arrangement as
soon as the script marks the page, the moment the legend hides.

## Script and accessibility

- The button is written `hidden` and revealed by the script, like the theme
  toggle. A press toggles `data-legend` and rewrites the hidden name.
- **Skip link** (FR-98): it keeps the bypass in both states. While the
  legend shows it reads **Skip to the legend** and moves focus to the first
  legend row; while it is hidden it reads **Skip to the table** and moves
  focus to the table view's toggle.
- **Focus**: if what has focus is hidden by a press, focus moves to the
  button, never left on a hidden element. In WebKit and in Firefox on
  macOS a click does not focus a button, and in WebKit the click first
  blurs whatever had focus, so the press can leave focus on a row it hid or
  on the page itself; either way it goes to the button.
- **Detail panel**: an open panel stays open; it belongs to the item, not
  the legend. If a legend row opened it, closing it returns focus to that
  item's marker.
- **Hover** (FR-37): a marker hovered while the legend is hidden has no row
  to light. No special case.
- **Quadrant colours** (FR-35): the legend is where quadrant colours are
  named in words. While it is hidden, each item's quadrant is still given
  by its sector and the sector-edge quadrant labels (FR-39), and in words by
  the table view's quadrant column. Finding an item by name is through the
  table view. The PRD records this as sufficient.
- The icon is drawn inline, stroked in `currentColor`, `aria-hidden`, and
  the button is at least 24 px square (WCAG 2.5.8), as the theme toggle is.

## Testing

Tests first.

- **CLI**: `--legend` takes `shown` and `hidden` in any case, and `show` and
  `hide`; the help lists only `shown|hidden`; PNG and SVG output is
  byte-identical with and without it.
- **Narrow parts**: the two parts together draw exactly what the narrow
  document draws, as `wide_parts` is held. PNG and SVG goldens unchanged.
- **Page**: the button's markup (icon, hidden name, `hidden`);
  `data-legend="hidden"` on `<html>` only with `--legend hidden`; the hiding
  rule only under `.js`; the button on a `--palette` page; none with nothing
  plotted; the script's names block held to the page's constants; the HTML
  golden, HTML5-parser and requests-nothing checks over the new markup.
- **Browser** (`interaction.mjs` and `a11y-audit.mjs` in headless
  Chromium, as the existing checks are; a small Playwright check of the
  focus-on-click path in Firefox and WebKit):
  - a press hides both legends and the wide radar widens on a
    window too narrow for both, and nothing else moves or changes width;
  - the key stays visible;
  - the skip link hides and returns;
  - the skip link is renamed and retargeted with the state;
  - focus on a legend row moves to the button when hidden, including when
    the click does not focus the button;
  - the state survives crossing the breakpoint;
  - `--legend hidden` starts hidden; without script the legend shows and
    there is no button;
  - the button is named by what a press does, has no pressed state, is at
    least 24 px, and is painted under forced colours.

## Documentation

- PRD: a new FR-103 for the button and the flag; FR-35 amended (sector
  labels and the table name quadrants while the legend is hidden); FR-51
  amended (the radar's growth while hidden); FR-68 noting HTML's narrow
  parts; FR-98 amended (the skip link retargets to the table); §6.6's flag
  table gains `--legend`; FR-91's key list says `legend` will be a key; a
  revision row.
- Manual accessibility checklist: a step for the legend button.
- Architecture: D-85's note on HTML's parts gains the narrow cut; §4's
  breakpoint note says hiding the legend does not move it (rg-dok).
- README: the flag in the reference, a line in the HTML section.
- A crumb for moving the breakpoint while the legend is hidden (rg-dok,
  since done: D-97).

## Delivery

Two PRs. First the narrow arrangement in parts, which no reader can tell
apart: no version bump. Then the feature: a minor bump, 1.6.0.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-17 | Initial version, from the design agreed in conversation (rg-3gc). |
| 1.1 | 2026-10-17 | After the five-agent review: no short form; a warning for PNG and SVG; the icon shows the state; the skip link retargets to the table; the wide box keeps its width; the narrow cut keeps the key's gap; print shows the legend; nothing plotted writes no state; Chromium plus a Firefox and WebKit focus check; two PRs. |
| 1.2 | 2026-10-17 | As built: the wide legend is hidden in place, not by a one-column grid, which missed by the scrollbar gutter; focus left on the page by a WebKit click goes to the button too. |
| 1.3 | 2026-10-17 | Review fixes: the skip link is drawn over the page on every page; the invisible wide legend takes no pointer events; the crossed-out list's X is larger; one testing bullet reworded. |
| 1.4 | 2026-10-18 | rg-dok (PRD D-97): the breakpoint moves while the legend is hidden, to `R − 0.2 × R`; the title part is cut as wide as the radar; the key spans the columns with the radar; a press between the two breakpoints switches the arrangement and leaves focus on the button. Replaces the "breakpoint does not move" limitation. |
