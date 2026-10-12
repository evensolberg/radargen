---
id: rg-iqm
title: FR-95 names the ring source, not always --rings
status: closed
type: task
priority: 3
tags:
- diagnostics
- config
created: 2026-10-12
updated: 2026-10-12
closed_reason: 'PR A: Binding::Ring carries RingSource; Catalogue keeps ring_source'
phase: v1.1
---

# FR-95 names the ring source, not always --rings

From the PR 14a final review. Binding::Ring carries no RingSource and the FR-95 message always says "in --rings". Correct today (default rings are too short to bind), wrong once PR 19 (--config) supplies rings: D-73 says the warning names the ring and its source. Carry RingSource in Binding::Ring and word the fix per source. Do with PR 19 (rg-u6u).
