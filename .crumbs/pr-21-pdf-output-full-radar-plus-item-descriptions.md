---
id: rg-4b8
title: 'PR 21: PDF output, full (radar plus item descriptions)'
status: open
type: feature
priority: 3
tags:
- pdf
- output
created: 2026-10-12
updated: 2026-10-12
phase: v1.2
---

# PR 21: PDF output, full (radar plus item descriptions)

The full PDF: the light content plus every item description, likely as a table (FR-78 column and row order), paginated (table across pages, repeated header, wrapped cells). Needs its own PRD section.

First step (maintainer: not now, when the time comes): a Fullbleed spike (~1 day): is inline SVG drawn as vector or rasterised (svg_raster feature); are table headers repeated across page breaks; is output byte-identical across runs (NFR-4); register Inter from the vendored TTFs; check maintainership in its repo. If Fullbleed passes it can serve both light and full (then PR 20 need not add svg2pdf); if not, the fallback is our own table Blocks and page breaking over svg2pdf. Open spec questions: CLI selection (pdf vs pdf-full, or a flag), tagged PDF/PDF-UA for accessibility, page size and orientation.
