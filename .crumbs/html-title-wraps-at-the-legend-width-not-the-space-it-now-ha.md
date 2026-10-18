---
id: rg-0ms
title: HTML title wraps at the legend width, not the space it now has
status: closed
type: task
priority: 4
tags:
- html
- title
- layout
created: 2026-10-18
updated: 2026-10-18
closed_reason: Title wraps at the radar part (wide) and narrow document (narrow), less margins; D-99
phase: ''
---

# HTML title wraps at the legend width, not the space it now has

From the rg-dok UI review: since D-97 the title SVG is the radar part wide (670 for the PRD example), but title.rs still wraps at LEGEND_WIDTH_MAX (380), so a long title takes three lines over a 756 px radar with ~390 px empty beside it. title.rs shares the measure with PNG/SVG, so wrapping wider for HTML needs a decision.

[start] 2026-10-18 11:39:15

[stop]  2026-10-18 11:51:02  11m 47s
