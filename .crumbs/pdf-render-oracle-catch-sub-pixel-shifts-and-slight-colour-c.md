---
id: rg-ho8
title: 'PDF render oracle: catch sub-pixel shifts and slight colour changes'
status: closed
type: task
priority: 4
tags:
- pdf
- testing
created: 2026-10-22
updated: 2026-10-22
closed_reason: '2x supersampled gate: ceiling 44 + 800 ppm past 16; catches 0.1 px text shifts and 0.08 stroke colour; 10-level colour still passes'
phase: ''
---

# PDF render oracle: catch sub-pixel shifts and slight colour changes

PR 20 gates tests/pdf_matches_png.rs on the worst channel delta (MAX_DELTA 93), because hayro and tiny-skia antialias differently. It misses shifts under ~1/3 px and small colour shifts; the interior-colour and halo structural tests and the text goldens cover some of that. Option: compare at 2x and downsample, or mask edge pixels, then reintroduce a ppm budget.

[start] 2026-10-22 19:10:57

[stop]  2026-10-22 19:24:58  14m 1s
