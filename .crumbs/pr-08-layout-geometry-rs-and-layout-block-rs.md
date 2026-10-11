---
id: rg-48z
title: 'PR 08: layout/geometry.rs and layout/block.rs'
status: closed
type: task
priority: 1
tags:
- impl
- layout
created: 2026-10-08
updated: 2026-10-11
blocks:
- rg-y1t
- rg-svr
- rg-67j
blocked_by:
- rg-rnc
- rg-bkm
- rg-efx
phase: v1.0
---

# PR 08: layout/geometry.rs and layout/block.rs

FR-30–FR-33, FR-43, FR-77, FR-89 capacity. Block trait, Size, Placement (with scale, for FR-70). Start of the critical path.

[2026-10-11] From PR 07 (order.rs): layout reads Plotted::quadrants for sector order (FR-10) and Plotted::items for markers in FR-34 order — already grouped by (quadrant, ring), so FR-33's per-band j and FR-35's legend groups are runs of it. A quadrant's series entry is Plotted::rank[q] (FR-67), never Item::quadrant. A band-grouping helper, if needed, belongs in order.rs.

[2026-10-11] [2026-10-11] Scope: geometry only (FR-30–FR-33, FR-43, FR-77, FR-88, FR-89, Plotted::runs). Block and Node moved to PR 09 (rg-y1t) at the maintainer's ruling. Plan: docs/plans/pr-08-geometry.md.

[start] 2026-10-11 10:19:56

[stop]  2026-10-11 10:33:29  13m 33s
