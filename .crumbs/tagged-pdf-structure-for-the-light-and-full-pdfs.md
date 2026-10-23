---
id: rg-ffj
title: 'Tagged PDF: structure for the light and full PDFs'
status: closed
type: feature
priority: 3
tags:
- pdf
- a11y
created: 2026-10-22
updated: 2026-10-23
closed_reason: '21c: every PDF tagged to PDF/UA-1 (1.23.0) and every glyph carries text (1.24.0); just pdfua green on 21 fixtures'
blocked_by:
- rg-4b8
phase: v1.2
---

# Tagged PDF: structure for the light and full PDFs

Maintainer, 2026-10-22 (PR 20 design): the light PDF (rg-5uf) ships untagged but well formed: real text from an embedded Inter subset, a document title, /Lang en, and content in legend reading order. Tag both PDFs together, alongside PR 21 (rg-4b8), whose description table needs structure most. Scope: a structure tree with the title as H1, sector and ring headings as H2/H3, the radar as a Figure whose Alt is D-75 name (RadarBlock::name), legend rows as P (or L/LI), marked content around every text run, a RoleMap, MarkInfo /Marked true, and decorative paint as Artifact. Decide whether to aim at PDF/UA-1 (XMP metadata, veraPDF validation in just ci).

[2026-10-22] From the PR 20 panel (architect): the PDF content walker (src/pdf/content.rs node()) ignores group.semantics and takes an immutable &Canvas, so tagging needs a &mut state (MCID counter, structure tree) threaded through node/shape/show::text and the dispatch split (node() is at its 80-line qual:allow). /Lang (en) is fixed; a non-English CSV is a known limit here.

[start] 2026-10-23 11:24:08

[2026-10-23] 21c-2 (1.23.0): every PDF tagged to PDF/UA-1, just pdfua green on 18 fixtures. 21c-3 remains: FR-113 boxes as outlines with ActualText, ToUnicode for mark glyphs, then the rtl fixture flips to compliant.

[stop]  2026-10-23 14:32:13  3h 8m 5s
