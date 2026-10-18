---
id: rg-dok
title: 'HTML: move the breakpoint while the legend is hidden'
status: closed
type: feature
priority: 3
tags:
- html
- layout
- legend
created: 2026-10-17
updated: 2026-10-18
closed_reason: Second breakpoint R-0.2R while the legend is hidden (D-97)
phase: ''
---

# HTML: move the breakpoint while the legend is hidden

From rg-3gc (spec RG-102). FR-69's breakpoint is computed with the legend beside the radar (891 px for the PRD example). While the legend is hidden the wide arrangement would fit down to about 536 px, so readers between those widths get the narrow arrangement when a bigger wide radar would fit. Needs a second media query keyed on data-legend and a state-dependent matchMedia in the script.

[start] 2026-10-18 08:15:51

[stop]  2026-10-18 09:01:59  46m 8s
