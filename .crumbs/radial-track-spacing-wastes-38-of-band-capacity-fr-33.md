---
id: rg-cbx
title: Radial track spacing wastes ~38% of band capacity (FR-33)
status: open
type: task
priority: 2
tags:
- prd
- layout
- geometry
created: 2026-08-29
updated: 2026-08-29
phase: ''
---

# Radial track spacing wastes ~38% of band capacity (FR-33)

Raised while investigating whether ring *widths* should flex with content
(conversation, 2026-08-29). They should not — see below — but the investigation
found a real defect in FR-33's radial placement.

## The finding

FR-33 places items on two radial tracks at fixed fractions of band width,
0.38 and 0.68. In the **innermost** band those tracks are only
`0.3 x 65 = 19.5px` apart, which is **less than a marker diameter**
(26px, or 28px with FR-40's status ring and a 2px gap). Adjacent items on
different tracks therefore nearly collide, and the innermost band holds
**one item at any quadrant count** — including three.

Under the default `--rings "Retire,Adopt,Trial,Assess"` the innermost ring is
**Retire**, so the ring most likely to accumulate a backlog is the one with the
least room.

Replacing the fixed fractions with a guaranteed separation — tracks centred in
the band, `min(marker_diameter + gap, 0.6 x width)` apart — modelled as:

| quadrants | today [r1,r2,r3,r4] | total | separated | total |
| --- | --- | --- | --- | --- |
| 3 | 1, 7, 13, 19 | 40 | 2, 9, 17, 25 | 53 |
| 4 | 1, 5, 9, 14 | 29 | 2, 6, 12, 19 | 39 |
| 6 | 1, 3, 6, 9 | 19 | 2, 4, 8, 12 | 26 |
| 8 | 1, 2, 4, 6 | 13 | 2, 2, 5, 9 | 18 |

About +38%, and it doubles the worst ring. **FR-32's equal-width bands are
untouched** — this is an FR-33 change only.

## Why flexible ring widths were rejected

**Content-proportional widths do not work.** Capacity is *angular*, not radial.
Widening Adopt from 65px to 131px on an adopt-heavy radar left its capacity at
2 — the tracks land at r=80 and r=119, whose usable arcs inside a 45° sector
are 53px and 79px, i.e. 1.9 and 2.8 markers. No radial redistribution
manufactures angular room.

They would also be **data-dependent**, so ring boundaries move between editions
and an item that did not move renders at a different radius. That is exactly
what D-7 refused for quadrant ordering, on a published, dated,
side-by-side-comparable artefact.

**Equal-area (or a power family) is data-independent** and does help a little:
caps go [1,2,4,6] -> [2,3,4,5] at 8 quadrants, total 13 -> 14. `p=0.7` gets the
same gain as equal-area at half the width distortion (2.1x vs 3.7x). Worth
considering *after* the FR-33 fix, and probably not needed once it lands.

## Before speccing

The numbers above come from a model — circular markers, all-pairs distance,
an assumed 2px gap. **Validate in `mockup.html` first.** D-31 and the
text-metrics spike are both cases where building it changed the answer, and
the spike's first oracle was wrong in a way only measurement caught.

No flag either way: the separated-track rule is strictly better and
data-independent, so it should be the behaviour rather than an option. Two
layout modes would mean two sets of golden files (SC-8, SC-13) for a choice
that has a right answer.
