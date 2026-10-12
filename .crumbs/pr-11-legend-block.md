---
id: rg-67j
title: 'PR 11: Legend block'
status: closed
type: task
priority: 1
tags:
- impl
- layout
created: 2026-10-08
updated: 2026-10-11
blocks:
- rg-ozb
blocked_by:
- rg-osr
- rg-48z
- rg-7mw
phase: v1.0
---

# PR 11: Legend block

FR-35 three levels, FR-74, FR-75, FR-79 wrapping over measured text. Candidate to split if it outgrows one review.

[2026-10-09] D-54: solid legend badges use Palette::number_ink like markers. D-56: LEGEND_WIDTH_MAX is now 380 (NARROW_WIDTH - 2*MARGIN), one wrap for both arrangements.

[2026-10-09] From PR 6 review: FR-35's level separation now relies on ink (LEGEND_QUADRANT TextPrimary vs LEGEND_RING TextMuted), which lives only in style.rs doc comments. Consider a pub ink field on TextStyle (or a legend-level test) so a test sees it.

[2026-10-11] From PRD v0.46 FR-94 (D-66, was rg-pdt): size legend rows (LEGEND_ROW_MIN_HEIGHT, LEGEND_ROW_PAD) for at least 24 CSS px at the smallest scale either arrangement reaches down to 320 px, the narrow arrangement at 320 px included.

[2026-10-11] From PR 10b: legend rows are controls — add Semantics::LegendRow(Control) (data-row only, no data-marker), keyed by svg::ItemKey(item.line), tooltip None; badge numbers use Paint::NumberOn(rank) (TextPrimary on hollow badges); row height from style::MIN_PX_PER_UNIT (≥ MIN_TARGET_PX). Split radar markers into a submodule first or alongside (rg-umt).

[start] 2026-10-11 17:12:24

[stop]  2026-10-11 18:01:48  49m 24s
