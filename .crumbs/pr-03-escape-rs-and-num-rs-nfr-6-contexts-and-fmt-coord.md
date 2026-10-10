---
id: rg-vgm
title: 'PR 03: escape.rs and num.rs — NFR-6 contexts and fmt_coord'
status: open
type: task
priority: 2
tags:
- impl
- determinism
- security
created: 2026-10-08
updated: 2026-10-09
blocks:
- rg-6ku
phase: v1.0
---

# PR 03: escape.rs and num.rs — NFR-6 contexts and fmt_coord

NFR-6: five output contexts, one function each. NFR-17: fmt_coord is the only path a float takes to output (3 dp, half away from zero, no exponent).

[2026-10-09] Plan: docs/plans/pr-03-escape-num.md
