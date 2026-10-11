---
id: rg-7mw
title: Simplify text::wrap (cognitive complexity 24)
status: in_progress
type: task
priority: 3
tags:
- text
- complexity
- layout
created: 2026-10-11
updated: 2026-10-11
blocks:
- rg-y1t
- rg-67j
phase: ''
---

# Simplify text::wrap (cognitive complexity 24)

rustqual flags src/text.rs:128 wrap (FR-79 greedy first-fit with FR-46's mid-token break): cognitive complexity 24, cyclomatic 11, the codebase's only complexity finding. It is on the legend's critical path (PR 09, PR 11). Split it when those PRs touch it — e.g. separate the token loop from the mid-token break — keeping tests/wrap.rs and the agrees_with_resvg oracle green, and confirm rustqual's score does not drop.

[start] 2026-10-11 10:41:02
