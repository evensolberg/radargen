---
id: rg-o25
title: Warn when a PNG draws a glyph Inter lacks (tofu)
status: in_progress
type: idea
priority: 2
tags:
- png
- a11y
created: 2026-10-16
updated: 2026-10-17
phase: v1.x
---

# Warn when a PNG draws a glyph Inter lacks (tofu)

From the PR 16 accessibility plan review: names in scripts the bundled Inter subset lacks (Arabic, Hebrew, CJK, Indic, emoji) draw as missing-glyph boxes in the PNG, unannounced, while the HTML falls back to system fonts, so SC-7 breaks silently. FR-84 already shapes with harfrust; a warning when shaping returns glyph 0 for any drawn string is cheap. README states the limit since 1.1.0.

[2026-10-17] Decided (user, 2026-10-17): one warning per name with a glyph Inter lacks, PNG and SVG only; minor bump.

[start] 2026-10-17 20:02:00
