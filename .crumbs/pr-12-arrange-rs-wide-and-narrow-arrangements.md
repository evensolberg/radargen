---
id: rg-ozb
title: 'PR 12: arrange.rs — wide and narrow arrangements'
status: in_progress
type: task
priority: 2
tags:
- impl
- layout
created: 2026-10-08
updated: 2026-10-11
blocks:
- rg-6ku
blocked_by:
- rg-y1t
- rg-svr
- rg-67j
phase: v1.0
---

# PR 12: arrange.rs — wide and narrow arrangements

FR-68–FR-71. One Blocks, two placements. FR-69 breakpoint computed per document from the wide width and the legibility floor.

[2026-10-11] [2026-10-11] From PR 09: the key has no rule of its own; separators and margins are the arrangement's. Keep the title and key at scale 1 in the narrow arrangement (D-56); check the key's legibility at 320 px against SC-17. Document order: title, radar, key, legend (screen readers read in DOM order). Blocks wrap themselves in a translating Group at Placement.origin; block::assert_translates is the shared test.

[2026-10-11] From PR 10a: solve the narrow scale from RadarBlock::size. It is non-decreasing in s (proved for s >= 0.0034, see radar.rs extent comment) but plateaus where a label dominates; define the minimum s and the behaviour when no s fits (a quadrant name wider than ~360 units at 12 px cannot fit 380).

[2026-10-11] The narrow solve should keep s >= 0.3 (or assert it) so quadrant labels clear the outer ring (D-70): a label box comes no closer to the centre than 300s - 11.625 for s <= 1, so below about 0.3 a 6 o'clock label touches the ring stroke whatever its length.

[2026-10-11] From PR 11: place the legend block at MARGIN − LEGEND_PANEL_INSET_X in both arrangements (content at the margin; in narrow the 404-unit panel spans 8..412 of 420 exactly); the wide document's legend content width is size.width − 2·LEGEND_PANEL_INSET_X; an empty legend (zero quadrants, size zero) takes no space and no gap; the legend is drawn at scale 1 in both arrangements (FR-94's row height assumes it — assert it); DOM order is radar markers, then legend rows.

[2026-10-11] Unblocked: rg-y1t (PR 09), rg-svr (PR 10), rg-67j (PR 11) all closed and merged (#43-#49).

[start] 2026-10-11 22:07:17
