---
id: rg-tzz
title: breakpoint_px panics on a wide document past MAX_WIDTH
status: closed
type: bug
priority: 2
tags:
- layout
- html
- fr-69
created: 2026-10-15
updated: 2026-10-15
closed_reason: Fixed in v1.0.1
phase: v1.0.x
---

# breakpoint_px panics on a wide document past MAX_WIDTH

From the rg-06b plan review, 2026-10-15: breakpoint_px asserts the wide width is within MAX_WIDTH (10,000 units). Very long quadrant labels or legend rows can make the wide document wider than that, which panics instead of reporting. Reproduce with an extreme label, then decide: clamp, warn, or refuse with a structured error.

[start] 2026-10-15 22:35:51

[stop]  2026-10-15 22:40:34  4m 43s
