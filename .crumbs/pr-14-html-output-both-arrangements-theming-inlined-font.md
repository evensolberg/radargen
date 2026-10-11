---
id: rg-f27
title: 'PR 14: HTML output — both arrangements, theming, inlined font'
status: blocked
type: feature
priority: 1
tags:
- impl
- html
- output
created: 2026-10-08
updated: 2026-10-11
blocks:
- rg-6zu
blocked_by:
- rg-rnc
- rg-6ku
- rg-fp5
- rg-3z2
- rg-85c
- rg-pcd
- rg-1uw
- rg-fnu
phase: v1.0
---

# PR 14: HTML output — both arrangements, theming, inlined font

Wire the pipeline end to end in main.rs. FR-69 media-query switch, FR-56 dual palette + prefers-color-scheme + [data-theme] override, inlined WOFF2 (NFR-5). First golden fixtures and the cross-process determinism test. First user-visible change.

[2026-10-10] From PR 06 (#25): the detail panel's status line reads Item::status through Status::label(), not the raw cell (an unrecognised status is Some(NoChange)). Legend rows show Qualifier::label() and Status::label(), which accessible_name contains (WCAG 2.5.3). Table rows: see rg-6ny. The table's and panel's quadrant and plotted-ring text is Quadrant::label and Catalogue::rings[r], not the cell (FR-9, FR-15); decide whether the table shows a raw or canonical qualifier/status cell, so it agrees with the panel. main.rs merges parse's Vec<Fatal> with build_catalogue's in NFR-11's order (PRD v0.44): by line; on the header's line, FR-5 before FR-4's duplicates in column order; on a row's line, FR-4's mismatched row before column errors. Fatal has no position() today and EmptyRequired keeps only the column name, so add a sort key with a line-without-column position (architecture §7); build_catalogue's errors already arrive in line then header-column order, and FR-4-rejected rows never reach FR-6, so a stable sort by line with column-less errors first is enough. Then group under NFR-20 (D-62, rg-pcd) and print as one block after the warnings, so an FR-6 'empty' on a cell that held only U+0001 follows the 'removed U+0001' warning that explains it.

[2026-10-11] From PR 07 (order.rs): the HTML emitter reads Plotted::numbers[i] (aligned with Catalogue::items; None when unplotted) for FR-78's number column and Catalogue::accessible_name, and Plotted::table_rows() for FR-78's row order (plotted in FR-34 order, then unplotted in CSV order).
