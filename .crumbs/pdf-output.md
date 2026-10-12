---
id: rg-5uf
title: 'PDF output: light (radar + legend) and full (with descriptions)'
status: open
type: idea
priority: 3
tags: []
created: 2026-10-09
updated: 2026-10-12
phase: ''
---

# PDF output: light (radar + legend) and full (with descriptions)

Two PDF products (maintainer, 2026-10-12).

LIGHT: radar, key and legend, i.e. the wide arrangement. A format option on the SVG export: arrange(Wide) -> svg::Document -> PDF via an SVG-to-PDF crate (svg2pdf, Typst project, on usvg like the resvg PNG path; check version and licence). One page sized to the document, vector, same Document as SVG/PNG, reproducible. ~1 PR after PR 17 (v1.1). Needs an FR beside FR-71 and a format value.

FULL: light plus every item description, likely as a table (FR-78 column and row order). Long, unbounded text means pagination: a table flowing across pages, repeated header, wrapped cells.
  Route 1: Fullbleed (https://docs.fullbleed.dev/, MIT, pure Rust, no JS) fed a print-only HTML page (wide SVG on page 1, then the table, @page rules). Spike first (~1 day): is inline SVG drawn as vector or rasterised (its Rust SVG support sits behind an svg_raster feature); does it repeat table headers across page breaks; is output byte-identical across runs (NFR-4); fonts register from TTF (we vendor Inter TTFs). Verify maintainership in its repo (fullbleed-engine/fullbleed-official).
  Route 2 (fallback): our own table Blocks and page breaking, then a multi-page PDF from our SVG pages. Fully ours and deterministic, but several PRs.

Open questions for the spec: CLI selection (--format pdf vs pdf-full, or a --pdf-detail flag); whether the full PDF is accessible (tagged PDF / PDF-UA, which neither route gives for free); page size (A4/Letter) and orientation (landscape radar page).
