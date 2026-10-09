---
id: rg-efx
title: 'PR 07: order.rs — quadrant and item comparators'
status: blocked
type: task
priority: 2
tags:
- impl
- ordering
- determinism
created: 2026-10-08
updated: 2026-10-08
blocks:
- rg-48z
blocked_by:
- rg-rnc
- rg-wld
phase: v1.0
---

# PR 07: order.rs — quadrant and item comparators

FR-10, FR-34, compare_names. Every comparator total; asserted in tests. Waits on rg-wld (non-ASCII ordering).

[2026-10-08] PRD v0.37 (D-53): fold with str::to_lowercase, order by code point (str::cmp). Return the quadrant order as a list so a later --quadrants (OQ-17) replaces only the computation.
