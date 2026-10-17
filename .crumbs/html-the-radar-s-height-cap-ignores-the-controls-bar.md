---
id: rg-247
title: 'HTML: the radar''s height cap ignores the controls bar'
status: in_progress
type: bug
priority: 3
tags:
- html
- layout
created: 2026-10-17
updated: 2026-10-17
phase: ''
---

# HTML: the radar's height cap ignores the controls bar

From the RG-102 UI review. Wide::radar_cap subtracts only the title's height from 100vh, not the page controls bar (about 51 px), so a radar at its cap ends about 40-50 px below the fold (1400x800: radar bottom 851). Pre-existing; hiding the legend (rg-3gc) lets the radar reach its cap in more windows (900x900: bottom 939, a quadrant label cut at the fold). Fix: add the bar's height to the cap's subtraction.

[start] 2026-10-17 16:23:15
