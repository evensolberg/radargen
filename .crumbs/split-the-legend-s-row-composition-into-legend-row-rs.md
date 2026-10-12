---
id: rg-rvh
title: Split the legend's row composition into legend/row.rs
status: in_progress
type: task
priority: 2
tags:
- layout
- quality
created: 2026-10-11
updated: 2026-10-11
phase: ''
---

# Split the legend's row composition into legend/row.rs

rustqual SRP_MODULE flags src/layout/legend/layout.rs (404 production lines; it was flagged and baselined as legend.rs before the PR 11 split). Cohesive seam from the PR 11 Task 3 review: row composition (FR-74 meta placement, FR-94 frame) vs the walk (quadrant/ring/row rhythm). Move Row (with bottom), fits_inline, meta_text, Meta, measured, line_count, Frame, frame, name_run, meta_run (~130 lines) and their tests (fits_inline_is_inclusive, meta_*, rows_of_one_two_and_three_lines) into legend/row.rs; Context::row stays the caller. Own refactor PR.

[start] 2026-10-11 21:21:33
