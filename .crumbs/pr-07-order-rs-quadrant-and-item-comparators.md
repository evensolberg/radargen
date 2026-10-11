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
updated: 2026-10-11
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

[2026-10-10] From PR 06 (#25): PR 07 reads Item::name_key, Quadrant::key and Item::line (FR-34's last tiebreak). It returns FR-10's order as a list of indices into Catalogue::quadrants, plus a rank, and does not reorder or mutate the Catalogue. Item::quadrant is first-appearance order and never a palette index. Settle whether FR-34's ordinal name tiebreak compares raw or trimmed names (rg-40l). PR 07 also owns resolve's output: FR-18's numbers, contiguous over plotted items in FR-34 order and None for unplotted ones, keyed by Item::line. PR 14's table (FR-78) and accessible_name read them from there.

[2026-10-11] [2026-10-11] rg-40l settled by PRD v0.45 (D-63): FR-34's ordinal name tiebreak compares lib::trim(name), not folded and not raw; names equal after the trim fall through to Item::line.
