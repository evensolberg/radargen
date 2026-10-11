---
id: rg-mgw
title: Widen the Inter subset to Vietnamese, Greek and Cyrillic (D-68)
status: open
type: task
priority: 2
tags:
- font
- text
created: 2026-10-11
updated: 2026-10-11
phase: ''
---

# Widen the Inter subset to Vietnamese, Greek and Cyrillic (D-68)

PRD v0.46 FR-85/D-68. Add Google Fonts' vietnamese, greek and cyrillic ranges (and -ext where Inter draws them) to scripts/subset-inter.sh, regenerate both WOFF2/TTF pairs and FONTS.sha256, record the measured size in FR-85/D-43, update README's font line, and add fixtures with Greek, Cyrillic and Vietnamese names to the agrees_with_resvg and rasterise-within-measurement tests.
