---
id: rg-y1t
title: 'PR 09: Title and key blocks'
status: blocked
type: task
priority: 2
tags:
- impl
- layout
created: 2026-10-08
updated: 2026-10-11
blocks:
- rg-ozb
blocked_by:
- rg-osr
- rg-48z
- rg-7mw
- rg-7vl
- rg-lag
- rg-hz0
phase: v1.0
---

# PR 09: Title and key blocks

FR-65 title block, FR-66 encoding key (markers drawn the same way the radar draws them; MarkerHollow vs Paint::None).

[2026-10-11] [2026-10-11] Maintainer's ruling during the PR 08 plan: PR 09 now also owns layout/block.rs (Block, Size, Placement, using layout::geometry::Point) and a minimal svg::Node carrying only the variants the title and key blocks draw. Later block PRs add variants; PR 13 adds Document and the serialiser.

[2026-10-11] [2026-10-11] From PR 08a's final review: text::FontError(String) drops the source error and does not name the face. PR 09 is the first pipeline caller of Fonts::embedded (NFR-15's font load failure); consider a face field (Regular/SemiBold) when wiring the error.
