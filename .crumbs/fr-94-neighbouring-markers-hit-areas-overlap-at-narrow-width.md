---
id: rg-frk
title: 'FR-94: neighbouring markers hit areas overlap at narrow widths'
status: closed
type: bug
priority: 2
tags:
- a11y
- layout
- fr-94
created: 2026-10-13
updated: 2026-10-15
closed_reason: 'Maintainer ruling 2026-10-15: markers meet 2.5.8 through the Equivalent exception; recorded in PRD D-81, FR-94, D-66.'
phase: ''
---

# FR-94: neighbouring markers hit areas overlap at narrow widths

Found by the PR 15c audit (scripts/a11y-audit.mjs). At 320 px on tests/fixtures/interaction.csv, markers 8/9 and 2/4 sit 24.14 px apart while each hit area is 27.43 px across, so their hit areas overlap; WCAG 2.5.8 does not count overlapped area, and the clear radius on the shared side is about 10.4 px, not 12. FR-94 sizes each hit area alone; D-66 did not address overlap. Capping each hit radius at half the nearest-neighbour distance fixes this fixture (24.14 px each) but not the worst case: at MIN_PX_PER_UNIT (2/3) these markers 31.7-unit spacing gives 21 px. A guarantee needs layout to keep marker centres at least 2*HIT_RADIUS (36 units) apart, or a PRD ruling accepting the overlap. Maintainer decision before 1.0.0 (docs/release-1.0.0.md).

[2026-10-13] PR 15c review: the markers meet 2.5.8 through its Equivalent exception (each item legend row and table button open the same panel and meet 24 px; the audit checks both), and 2.5.8 is a WCAG 2.2 criterion beside the 2.1 AA claim. No longer a 1.0.0 blocker: the maintainer decides whether to accept Equivalent (record it in FR-94/D-66) or also make markers pass alone (layout spacing of 36 units).
