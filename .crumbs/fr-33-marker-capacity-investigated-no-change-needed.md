---
id: rg-cbx
title: "FR-33 marker capacity: investigated, no change needed"
status: closed
type: task
priority: 3
tags:
- prd
- layout
- geometry
created: 2026-08-29
updated: 2026-08-29
phase: ''
---

# FR-33 marker capacity: investigated, no change needed

Opened claiming FR-33's radial track spacing wasted ~38% of band capacity and
that the innermost ring held one item at any quadrant count. **Both claims were
wrong.** Corrected the same day, before anything was built on them.

## The error

The model gave every marker a footprint of radius 13 — "marker plus status
ring" — and added an invented 2px gap, demanding **28px** of separation.

- FR-40 rings only **New** and **Moved** items. The common marker is plain
  `r=10`, needing **20px**.
- The ring size was wrong regardless: `statusRingMarkup` uses `markerR + 4`
  with a 2px stroke, so a ringed marker is **15**, not 13.

At 4 rings and 3 quadrants two markers sit 27.3px apart. The threshold was 28.
The entire finding turned on a 0.7px margin from invented assumptions.

## Corrected capacity — plain markers (r=10), 4 rings

| quadrants | ring 1 | ring 2 | ring 3 | ring 4 | per quadrant | whole radar |
| --- | --- | --- | --- | --- | --- | --- |
| 3 | 3 | 14 | 26 | 37 | 80 | 240 |
| 4 | 2 | 10 | 19 | 27 | 58 | 232 |
| 6 | 2 | 6 | 12 | 18 | 38 | 228 |
| 8 | 2 | 4 | 9 | 13 | 28 | 224 |

Innermost ring at 3 quadrants: **4 items at 3 rings, 3 at 4 rings, 2 at 5
rings** — matching what a reader eyeballing `mockup.html` reported, which is
what prompted the recheck.

Where every marker is ringed (all New/Moved) capacity roughly halves: 10 per
quadrant at 8 quadrants against 28.

## Conclusions

- **FR-33 needs no change.** Guaranteed track separation gives 0% at 4, 6 and
  8 quadrants and -3% at 3 once the threshold is right. It only ever helped
  against the inflated problem.
- **Flexible ring widths stay rejected**, on the reasoning that does hold:
  capacity is *angular*, not radial, so widening a band cannot manufacture
  angular room — and content-dependent boundaries would move a stationary item
  between editions, which D-7 refused for quadrant ordering.
- **A-5 and FR-88 are unaffected.** Both were argued from sector width and
  label crowding (OQ-14), never from marker capacity.

## Lesson worth keeping

The same failure as the text-metrics spike's first oracle: a plausible model,
never validated, whose answer was decided entirely by an assumption nobody
checked. Both were caught by comparing against something real — there by
`usvg`'s own layout, here by a reader looking at the prototype. Geometry claims
in this project get checked against `mockup.html` before they are written down.
