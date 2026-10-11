---
id: rg-kzl
title: Right-to-left, CJK and Indic script support
status: open
type: idea
priority: 4
tags:
- text
- a11y
- prd
created: 2026-10-11
updated: 2026-10-11
phase: ''
---

# Right-to-left, CJK and Indic script support

Out of scope by A-4 and D-68 (PRD v0.46). Needs a second font with fallback in measurement and resvg; bidi for mixed-direction names (wrap and draw), dir handling; CJK per-render glyph subsetting and UAX #14 line breaking. Carries rg-q2b (Cf prepended concatenation marks trimmed by FR-76: exempt Grapheme_Cluster_Break=Prepend?) and rg-rta (should FR-90 strip bidi overrides U+202A-E, U+2066-9?).
