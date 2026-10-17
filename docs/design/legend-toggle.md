---
tags: [spec, html, legend, a11y]
aliases: []
doc_id: RG-102
doc_name: legend-toggle
crumb_id: rg-3gc
document_title: "Showing and hiding the legend on the HTML page"
created_date: 2026-10-17
synopsis: "An icon button on the HTML page shows and hides the legend in both arrangements, and --legend shown|hidden sets where it starts. The narrow arrangement is cut into a body and a legend part so CSS can hide the legend; the key always shows. PNG and SVG are unchanged."
status: draft
type: Spec
revision: "1.0"
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
  same size: one list icon in both states. Its name, in visually hidden
  text, says what a press does: **Hide legend** while the legend shows,
  **Show legend** while it is hidden. No pressed state (the rg-nuy ruling).
  A sighted reader sees the state by whether the legend is there.
- A press hides or shows the legend in **both arrangements at once**, so
  crossing FR-69's breakpoint keeps the state.
- **The key always shows.** Only the legend panel, which maps numbers to
  names, is hidden. The title, radar and key are unaffected.
- The choice is **not remembered** between visits, as the theme choice is
  not: every visit starts where the author set it.
- **Without script** there is no button and the legend always shows,
  whatever the page was set to (FR-98's rule: nothing claims an action it
  cannot perform).
- With **nothing plotted** there is no legend, so there is no button, as the
  skip link is already hidden then.
- A **`--palette` page** has the button too: it does not depend on the
  built-in themes.

## The flag

`--legend <shown|hidden>`, default `shown`, matched case-insensitively. The
values `show` and `hide` are accepted as hidden aliases: they work but the
help lists only `shown|hidden`. It sets where the HTML page starts and is
**ignored, not an error, for PNG and SVG**, as `--theme` is ignored for
HTML. Its help says so.

## Layout

### The narrow arrangement in parts

The narrow arrangement is one document today. HTML now draws it as two,
the way the wide arrangement is drawn as four (D-85):

- **body**: the title, radar, key rule and key;
- **legend**: the legend panel.

The cut runs through the gap between the key and the legend. Each part is
the narrow document's own drawing translated to its own origin, as
`wide_parts` does; nothing is laid out again. PNG and SVG keep drawing the
whole narrow document and do not change.

### Hiding

The page writes `data-legend="hidden"` on `<html>` when started hidden; the
button sets and removes it. One rule hides both arrangements' legend parts,
and applies only under the `js` class the script adds, so without script
the legend shows.

### The wide radar while the legend is hidden

The wide grid falls to one column, so the radar's column may use the width
the legend had. The radar's existing cap still applies: it grows until it
is as tall as the viewport below the title. So it grows only where the
window was too narrow for radar and legend side by side; on a short, wide
window it keeps its size and the page is simply less busy. The title, key,
controls bar and table keep their widths, so nothing moves sideways when
the legend is toggled.

### The breakpoint does not move

FR-69's breakpoint is still computed with the legend beside the radar (891
px for the PRD example). While the legend is hidden the wide arrangement
would fit down to about 536 px, but moving the breakpoint would need a
second media query and a state-dependent `matchMedia` in the script. Not
in this change; recorded as a follow-up crumb.

## Script and accessibility

- The button is written `hidden` and revealed by the script, like the theme
  toggle. A press toggles `data-legend` and rewrites the hidden name.
- **Skip link** (FR-98): hidden while the legend is, shown again with it.
- **Focus**: if a legend row has focus when the legend is hidden, focus
  moves to the button, never left on a hidden element.
- **Detail panel**: an open panel stays open; it belongs to the item, not
  the legend. If a legend row opened it, closing it returns focus to that
  item's marker.
- **Hover** (FR-37): a marker hovered while the legend is hidden has no row
  to light. No special case.
- **Quadrant colours** (FR-35): the legend is where quadrant colours are
  named in words. While it is hidden, the table view's quadrant column names
  each item's quadrant. The PRD records this as sufficient.
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
- **Browser** (`interaction.mjs`, `a11y-audit.mjs`, three engines):
  - a press hides both legends and the wide radar's column widens on a
    window too narrow for both;
  - the key stays visible;
  - the skip link hides and returns;
  - focus on a legend row moves to the button when hidden;
  - the state survives crossing the breakpoint;
  - `--legend hidden` starts hidden; without script the legend shows and
    there is no button;
  - the button is named by what a press does, has no pressed state, is at
    least 24 px, and is painted under forced colours.

## Documentation

- PRD: a new FR-103 for the button and the flag; FR-35 amended (the table
  names quadrants while the legend is hidden); FR-98 amended (the skip link
  hides with the legend); FR-68 noting HTML's narrow parts; a revision row.
- Manual accessibility checklist: a step for the legend button.
- Architecture: D-85's note on HTML's parts gains the narrow cut.
- README: the flag in the reference, a line in the HTML section.
- A crumb for moving the breakpoint while the legend is hidden.

## Release

A feature: minor bump, 1.6.0.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-17 | Initial version, from the design agreed in conversation (rg-3gc). |
