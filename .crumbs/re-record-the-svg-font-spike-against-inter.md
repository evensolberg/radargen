---
id: rg-35i
title: Re-record the svg-font spike against Inter
status: closed
type: task
priority: 4
tags:
- docs
- svg
created: 2026-10-17
updated: 2026-10-17
closed_reason: svg-font spike re-recorded against Inter
phase: ''
---

# Re-record the svg-font spike against Inter

docs/spikes/svg-font.md still shows Libertinus-era figures (4,150 / 645,673 / 73,508 / 216,658 bytes; 82 subpixels, 20 ppm; "nearly nine times"; the unsubset Libertinus embed caveat). Current: 3,710 / 366,806 / 53,322 / 112,454; 152 subpixels (38 ppm); ~6.9x. Re-record the size table, gzip column, ratio wording, embed caveat and precision sweep. Stale since d424bf8 (Inter swap). From the rg-3i4 review.

[start] 2026-10-17 17:15:08

[stop]  2026-10-17 17:17:11  2m 3s
