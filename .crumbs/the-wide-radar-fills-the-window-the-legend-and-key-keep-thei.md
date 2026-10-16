---
id: rg-swu
title: The wide radar fills the window; the legend and key keep their size
status: closed
type: feature
priority: 1
tags:
- html
- css
- layout
created: 2026-10-15
updated: 2026-10-15
closed_reason: 'PR #86'
phase: v1.0
---

# The wide radar fills the window; the legend and key keep their size

Maintainer, 2026-10-15: D-85 scales the whole wide SVG, so the legend and key grow with the radar (about 1.6x on their 11-item radar), which is not usable. Split the wide arrangement in HTML into separate SVGs: the radar fills the free space, title, key and legend stay at one pixel per unit. PNG/SVG keep one document.

[start] 2026-10-15 21:52:26

[stop]  2026-10-15 22:16:40  24m 14s
