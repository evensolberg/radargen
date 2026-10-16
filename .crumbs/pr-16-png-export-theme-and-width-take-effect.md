---
id: rg-42m
title: 'PR 16: PNG export; --theme and --width take effect'
status: closed
type: feature
priority: 2
tags:
- impl
- png
- output
created: 2026-10-08
updated: 2026-10-16
closed_reason: PNG export shipped in 1.1.0 (PR 16)
blocks:
- rg-cdo
phase: v1.1
---

# PR 16: PNG export; --theme and --width take effect

resvg rasterisation through the bundled font (NFR-7), literal colour binding, FR-56/FR-57. PNG compared by decoded pixels.

[2026-10-11] From PR 12: export arrange(Arrangement::Wide, ..) only (FR-71).

[2026-10-12] From PR 13: render svg::serialise(&doc, Target::Static(&palette)) where palette is the run's resolved palette for --theme (and --palette in v1.1), into resvg with the vendored fonts (tests/common/fonts.rs shows the fontdb setup).

[2026-10-15] From rg-tzz review (2026-10-16): the wide document has no upper bound in layout units (a long quadrant name makes it 18,000+ units). D-86 says PNG and SVG default to the document width, so a default export of such a radar would skip FR-64's 10,000-px ceiling, which guards against a multi-gigabyte rasterisation. Clamp or refuse the default width at FR-64's ceiling.

[start] 2026-10-16 08:25:00

[stop]  2026-10-16 09:07:25  42m 25s
