---
id: rg-73h
title: 'Static outputs: wrap a long legend into several columns'
status: open
type: idea
priority: 3
tags:
- png
- svg
- layout
- legend
created: 2026-10-16
updated: 2026-10-16
phase: ''
---

# Static outputs: wrap a long legend into several columns

In the wide arrangement the legend is one column beside the radar (LEGEND_WIDTH_MAX), so a long legend sets the image height (a_tall_legend_sets_the_wide_height, src/layout/arrange.rs). PNG (and SVG in 1.2) then comes out tall and narrow, with the radar at the top of a mostly empty left side: awkward on a slide. A very long legend at --width 10000 can also hit the 100 MP cap (refused cleanly, not a crash).

Idea: for static outputs, flow legend rows into extra columns when the legend is much taller than radar + key, keeping reading order (FR-68) and quadrant grouping. Needs a PRD decision (opt-in flag vs automatic), and must keep HTML unchanged or decide it too.

[2026-10-16] Decided (user, 2026-10-16): opt-in flag, not automatic. Default output stays one column; the flag (name and column count/threshold TBD in the PRD) wraps the legend for static outputs.
