---
id: rg-73x
title: 'Paper: equal-height legend panels on a continuation page'
status: closed
type: task
priority: 4
tags:
- pdf
- paper
- polish
created: 2026-10-22
updated: 2026-10-23
closed_reason: 'Fixed in 1.25.2: continuation panels end level, capped at the room; row alignment split out as rg-jao'
phase: v1.2
---

# Paper: equal-height legend panels on a continuation page

From the PR 21a final review (UI, Important 4, deferred as cosmetic). Columns side by side end at different heights (up to ~4 mm on A4), which looks accidental. Draw every panel on a page to the tallest one's height (a height counterpart to LegendBlock::spanning), perhaps exempting the last page's final column. Also UI minor 5: start columns 2+ one heading-row lower so rows line up across the page under Legend (continued).

[start] 2026-10-23 17:13:34  Equal-height panels on continuation pages

[stop]  2026-10-23 17:43:18  29m 44s
