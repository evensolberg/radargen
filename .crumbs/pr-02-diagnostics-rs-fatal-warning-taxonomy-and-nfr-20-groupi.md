---
id: rg-xr3
title: 'PR 02: diagnostics.rs — Fatal/Warning taxonomy and NFR-20 grouping'
status: open
type: task
priority: 2
tags:
- impl
- diagnostics
created: 2026-10-08
updated: 2026-10-09
blocks:
- rg-aqb
- rg-rnc
- rg-tiu
phase: v1.0
---

# PR 02: diagnostics.rs — Fatal/Warning taxonomy and NFR-20 grouping

Architecture §3, §7. thiserror taxonomy; Warnings is a Vec grouped at print time by class and value, first-occurrence order, Option<line> with None first (FR-88).

[2026-10-09] Plan: docs/plans/pr-02-diagnostics.md
