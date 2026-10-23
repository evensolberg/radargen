---
id: rg-ho8
title: 'PDF render oracle: catch sub-pixel shifts and slight colour changes'
status: open
type: task
priority: 4
tags:
- pdf
- testing
created: 2026-10-22
updated: 2026-10-22
phase: ''
---

# PDF render oracle: catch sub-pixel shifts and slight colour changes

PR 20 gates tests/pdf_matches_png.rs on the worst channel delta (MAX_DELTA 93), because hayro and tiny-skia antialias differently. It misses shifts under ~1/3 px and small colour shifts; the interior-colour and halo structural tests and the text goldens cover some of that. Option: compare at 2x and downsample, or mask edge pixels, then reintroduce a ppm budget.
