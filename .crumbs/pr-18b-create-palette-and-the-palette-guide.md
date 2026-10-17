---
id: rg-0fm
title: 'PR 18b: --create-palette and the palette guide'
status: blocked
type: feature
priority: 2
tags:
- impl
- palette
- cli
- docs
created: 2026-10-16
updated: 2026-10-16
blocked_by:
- rg-cdo
phase: v1.1
---

# PR 18b: --create-palette and the palette guide

FR-101, D-91, SC-28 (PRD v0.78). --create-palette <path> writes the built-in light palette as one :root rule that --palette accepts unchanged, each neutral's dark value in a line comment; follows FR-99's file rules (alone, never replaces, in place, deterministic). docs/product/palette.sample.css becomes those bytes (rewrite tests/palette.rs drift check). Share template::create's create-if-absent helper; generalise the already-exists error to name the flag. Write docs/palette.md: each role, where it is drawn (annotated illustration), built-in light/dark values, contrast each needs; link from README. Release 1.4.0.
