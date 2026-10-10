---
id: rg-bkm
title: 'PR 04: style.rs and built-in palette.rs'
status: closed
type: task
priority: 2
tags:
- impl
- layout
- palette
created: 2026-10-08
updated: 2026-10-10
blocks:
- rg-48z
blocked_by:
- rg-osr
phase: v1.0
---

# PR 04: style.rs and built-in palette.rs

All layout constants in one home (type scale, FR-35 indents, marker radius, FR-79 column max, legibility floor). palette.rs: Role, FR-81/FR-82 built-ins, FR-67 cycling at bind time. --palette parsing is PR 18.

[2026-10-09] Plan: docs/plans/pr-04-style-palette.md. Now blocked by rg-osr: style::TextStyle holds text::Weight, which PR 01 introduces.

[start] 2026-10-10 10:36:41

[stop]  2026-10-10 10:49:28  12m 47s
