---
id: rg-3el
title: 'Paper: radar alone on page 1 is drawn left of centre'
status: open
type: bug
priority: 3
tags:
- pdf
- paper
created: 2026-10-23
updated: 2026-10-23
phase: v1.2
---

# Paper: radar alone on page 1 is drawn left of centre

When a sector label is too wide for the radar to sit beside the legend (first_page falls back to the full-width column, legend starts on page 2), the radar on page 1 is drawn left of centre rather than centred on the page. Repro: a 64-character sector name, --paper letter (fixture in tests/serialise/pdf.rs prd_legend_after_page_one_matches_its_golden). first_page computes x from col.0 of column(sheets, false); check whether the column or the radar width (with its label pad) is off. Found 2026-10-23 during rg-p82.
