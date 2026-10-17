---
id: rg-dok
title: 'HTML: move the breakpoint while the legend is hidden'
status: open
type: feature
priority: 3
tags:
- html
- layout
- legend
created: 2026-10-17
updated: 2026-10-17
phase: ''
---

# HTML: move the breakpoint while the legend is hidden

From rg-3gc (spec RG-102). FR-69's breakpoint is computed with the legend beside the radar (891 px for the PRD example). While the legend is hidden the wide arrangement would fit down to about 536 px, so readers between those widths get the narrow arrangement when a bigger wide radar would fit. Needs a second media query keyed on data-legend and a state-dependent matchMedia in the script.
