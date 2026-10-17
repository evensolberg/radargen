---
id: rg-0fm
title: 'PR 18b: --create-palette'
status: closed
type: feature
priority: 2
tags:
- impl
- palette
- cli
- docs
created: 2026-10-16
updated: 2026-10-17
closed_reason: Shipped --create-palette on feat/create-palette (1.4.0)
phase: v1.1
---

# PR 18b: --create-palette

FR-101, D-91, SC-28 (PRD v0.78). --create-palette <path> writes the built-in light palette as one :root rule that --palette accepts unchanged, each neutral's dark value in a /* */ comment after the declaration's ;. Bytes are exactly docs/product/palette.sample.css, which PR 18 (rg-cdo) already rewrote along with docs/palette.md; this PR adds the flag and a test that the written bytes equal the sample. Follows FR-99's file rules (alone, never replaces, in place, deterministic, LF, no BOM). Share template::create's create-if-absent helper; generalise the already-exists error to name the flag. Trap: Palette::declarations() orders neutrals then series, FR-101 wants series then neutrals. Release 1.4.0.

[start] 2026-10-17 10:34:48

[stop]  2026-10-17 10:56:58  22m 10s
