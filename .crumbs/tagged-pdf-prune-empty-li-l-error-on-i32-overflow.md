---
id: rg-969
title: 'Tagged PDF: prune empty LI/L; error on i32 overflow'
status: closed
type: task
priority: 4
tags:
- pdf
- a11y
created: 2026-10-23
updated: 2026-10-23
closed_reason: 'Fixed in 1.24.2: empty structure elements pruned; integer overflow is Fatal::PdfTooLarge'
phase: v1.2
---

# Tagged PDF: prune empty LI/L; error on i32 overflow

From the 21c-2 review (Rust 4, 7): elements with no kids (a legend LI or L whose content drew nothing) are written; prune them in Tagger::into_tree except TD/TH, which keep the cell count. MCID, StructParents, ParentTreeNextKey and ColSpan clamp to i32::MAX via unwrap_or, which would write duplicates silently; unreachable in practice, but return a Fatal or debug_assert with a reason.

[start] 2026-10-23 16:02:18  Prune empty L/LI; no silent i32 clamp

[stop]  2026-10-23 16:19:11  16m 53s
