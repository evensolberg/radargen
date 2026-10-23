---
id: rg-4b8
title: 'PR 21: PDF output, full (radar plus item descriptions)'
status: closed
type: feature
priority: 3
tags:
- pdf
- output
created: 2026-10-12
updated: 2026-10-23
closed_reason: 'RG-104 complete: paper (1.20.0), the full PDF (1.21.0-1.22.x), tagged PDF/UA-1 (1.23.0, 1.24.0)'
blocks:
- rg-ffj
phase: v1.2
---

# PR 21: PDF output, full (radar plus item descriptions)

The full PDF: the light content plus every item description, likely as a table (FR-78 column and row order), paginated (table across pages, repeated header, wrapped cells). Needs its own PRD section.

First step (maintainer: not now, when the time comes): a Fullbleed spike (~1 day): is inline SVG drawn as vector or rasterised (svg_raster feature); are table headers repeated across page breaks; is output byte-identical across runs (NFR-4); register Inter from the vendored TTFs; check maintainership in its repo. If Fullbleed passes it can serve both light and full (then PR 20 need not add svg2pdf); if not, the fallback is our own table Blocks and page breaking over svg2pdf. Open spec questions: CLI selection (pdf vs pdf-full, or a flag), tagged PDF/PDF-UA for accessibility, page size and orientation.

[2026-10-22] Maintainer, 2026-10-22 (PR 20 design): paper sizes (A4, Letter, and orientation) belong here, in PR 21. PR 20 light PDF is one page sized to the document (CSS px x 0.75 pt, --width as for SVG).

[2026-10-22] From rg-vvh (2026-10-22): /UserUnit pages are searchable in PDFKit, pdf.js and PDFium at any size tried (20k rows, unit 38); only poppler (pdftotext and the viewers built on it) loses text past roughly unit 6-10. Pagination for the full PDF can be decided on its own merits (print, the table); it is not needed to fix search.

[start] 2026-10-22 22:15:03  PR 21a: paper

[stop]  2026-10-22 23:38:01  1h 22m 58s  PR 21a done; 21b next

[start] 2026-10-23 08:04:07  PR 21b: full PDF

[stop]  2026-10-23 10:16:11  2h 12m 4s  PR 21b done; 21c next
