---
id: rg-i4a
title: 'Legend: sector heading with nothing under it when nothing is plotted'
status: closed
type: bug
priority: 3
tags:
- legend
- a11y
created: 2026-10-23
updated: 2026-10-23
closed_reason: 'Fixed in 1.24.1: legend omits a sector with no plotted rows'
phase: v1.2
---

# Legend: sector heading with nothing under it when nothing is plotted

From the 21c-2 review (a11y 5): nothing.pdf outlines H2 Legend, H3 Tools, then the next H2, with no list under the sector, so a screen reader lands on an empty heading. The drawing matches. Do not draw a sector heading with no rows, or draw "Nothing plotted" under it.

[start] 2026-10-23 15:11:25  Empty sector heading in the legend

[stop]  2026-10-23 15:50:17  38m 52s
