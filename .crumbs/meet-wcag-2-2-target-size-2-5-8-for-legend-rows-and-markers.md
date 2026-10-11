---
id: rg-pdt
title: Meet WCAG 2.2 target size (2.5.8) for legend rows and markers
status: closed
type: task
priority: 2
tags:
- a11y
- layout
- prd
created: 2026-10-09
updated: 2026-10-11
phase: ''
---

# Meet WCAG 2.2 target size (2.5.8) for legend rows and markers

Legend rows are max(17, lines*15)+7 = 24 units, so ~19px at the FR-69 breakpoint (scale 0.8); markers are 20 units, ~11px in the narrow arrangement. Size legend rows (LEGEND_ROW_MIN_HEIGHT, LEGEND_ROW_PAD) for >=24px rendered at the breakpoint, and give markers an invisible hit circle >=12px radius rendered, counter-scaled when narrow. Then claim 2.5.8 in NFR-1. Decide before PRs 08-12. From the PR 6 accessibility review.

[2026-10-11] Decided in PRD v0.46 (FR-94, D-66); implementation noted on rg-svr (markers) and rg-67j (legend rows).
