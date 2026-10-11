---
id: rg-pcd
title: Group repeated FR-4 field-count errors like NFR-20 warnings
status: open
type: task
priority: 3
tags:
- prd
- diagnostics
- csv
created: 2026-10-10
updated: 2026-10-10
blocks:
- rg-f27
phase: ''
---

# Group repeated FR-4 field-count errors like NFR-20 warnings

Raised in the PR 05 plan review (RG-005 rev 1.1). NFR-16 tier two collects every FR-4 field-count error, so one systematic cause — e.g. an extra trailing comma on the header only — prints one identical error per row (200 for a 200-row export). NFR-20 groups warnings only. Decide in the PRD whether hard errors of one class group the way warnings do (one line, positions enumerated, uncapped), then update FR-4/NFR-16/NFR-20 and the Fatal rendering.

[2026-10-10] From the PR 06 plan review (RG-006 rev 1.3): FR-6 has the same shape — an export whose ring column is blank throughout prints one 'empty' error per row. Include FR-6 in the decision; a grouped form could read '{n} rows have an empty {column} — lines 3, 5, 8; every row needs a {column}, so fill them in or delete the rows'.
