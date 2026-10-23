---
id: rg-3el
title: 'Paper: radar alone on page 1 is drawn left of centre'
status: closed
type: bug
priority: 3
tags:
- pdf
- paper
created: 2026-10-23
updated: 2026-10-23
closed_reason: Not reproducible at c9624ea; reopen if seen again
phase: v1.2
---

# Paper: radar alone on page 1 is drawn left of centre

When a sector label is too wide for the radar to sit beside the legend (first_page falls back to the full-width column, legend starts on page 2), the radar on page 1 is drawn left of centre rather than centred on the page. Repro: a 64-character sector name, --paper letter (fixture in tests/serialise/pdf.rs prd_legend_after_page_one_matches_its_golden). first_page computes x from col.0 of column(sheets, false); check whether the column or the radar width (with its label pad) is off. Found 2026-10-23 during rg-p82.

[start] 2026-10-23 15:00:44  Investigating centring of lone radar on page 1

[2026-10-23] Could not reproduce at HEAD (c9624ea). Probe of first_page over sector-name lengths 30-160 on Letter and A4: whenever the radar sits alone and fits (Letter 64/80, A4 64/80/100) the circle centre and ink box centre are exactly the page centre (offset 0.0). The 64-char fixture rendered via the CLI and rasterised also measures centred (circle, label and key rule all centre on page centre). first_page is unchanged since this crumb was filed (4ad367d). Off-centre cases that do exist are by design: beside=true (legend on the right) and fits=false (radar at its floor, pinned to the left margin, overflowing right, reported as radar_too_wide). The left-aligned title and key under a centred radar may read as left-heavy.

[stop]  2026-10-23 15:02:57  2m 13s  Not reproducible; awaiting maintainer
