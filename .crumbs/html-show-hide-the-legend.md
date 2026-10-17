---
id: rg-3gc
title: 'HTML: show/hide the legend'
status: in_progress
type: feature
priority: 2
tags:
- html
- legend
- layout
- a11y
created: 2026-10-17
updated: 2026-10-17
phase: ''
---

# HTML: show/hide the legend

From rg-ek8. The legend is not strictly needed in HTML: hover, the panel and the table carry everything. Add a control to show/hide it on the page, and an option for the default (shown or hidden).

Layout:
- Wide (D-85): the legend is its own grid part; hiding it should give its width back to the radar, which css::Wide sizes from radar_width + legend_width (rg-swu).
- Narrow: the legend is inside the single serialised narrow Document. Hiding it needs narrow split into parts like WideParts, or the toggle is wide-only. This is the bulk of the work.

Open questions (settle as PRD rulings before building):
1. No-script: controls are hidden until the script runs (FR-98 rule). Default-hidden can only apply under the js class; without script the legend must show.
2. Flag shape, e.g. --legend shown|hidden. HTML-only: refused or ignored for PNG/SVG (cf --theme ignored for HTML)?
3. Not rendering the legend at all: recommend no. Static outputs need it; HTML hidden-by-default gets nearly all the benefit.

Knock-ons:
- FR-35: the legend is the only words-mapping of quadrant colour (FR-66 leaves it out of the key). The table names quadrants; the PRD should say that suffices.
- FR-98: the skip link targets the first legend row; hide it or retarget while the legend is hidden.
- FR-37: marker hover lights the legend row; must not break hidden, and resume when shown.
- Button label says what a press does (rg-nuy): Hide legend / Show legend. Both arrangements hold a legend, so no single id for aria-controls (as the skip link).

[start] 2026-10-17 13:21:14
