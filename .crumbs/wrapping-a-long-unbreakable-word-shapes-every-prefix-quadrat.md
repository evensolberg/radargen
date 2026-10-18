---
id: rg-wl7
title: Wrapping a long unbreakable word shapes every prefix (quadratic)
status: open
type: bug
priority: 3
tags:
- text
- performance
created: 2026-10-17
updated: 2026-10-17
phase: ''
---

# Wrapping a long unbreakable word shapes every prefix (quadratic)

From the rg-rv8 efficiency review: Column::fitting_prefix (src/text.rs ~492) measures every char prefix of an unbreakable token, re-shaping each with harfrust, so a long name costs O(n^2) shapes; a 200-char W name dominates layout time (11 s for 141 layouts in a debug test). Bisect the prefix length (widths are monotone in prefix length) for O(n log n). Add a linear-time test like missing_runs_in_linear_time.
