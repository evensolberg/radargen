---
id: rg-42m
title: 'PR 16: PNG export; --theme and --width take effect'
status: blocked
type: feature
priority: 2
tags:
- impl
- png
- output
created: 2026-10-08
updated: 2026-10-12
blocks:
- rg-cdo
blocked_by:
- rg-6ku
phase: v1.1
---

# PR 16: PNG export; --theme and --width take effect

resvg rasterisation through the bundled font (NFR-7), literal colour binding, FR-56/FR-57. PNG compared by decoded pixels.

[2026-10-11] From PR 12: export arrange(Arrangement::Wide, ..) only (FR-71).

[2026-10-12] From PR 13: render svg::serialise(&doc, Target::Static(&palette)) where palette is the run's resolved palette for --theme (and --palette in v1.1), into resvg with the vendored fonts (tests/common/fonts.rs shows the fontdb setup).
