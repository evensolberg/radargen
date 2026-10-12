---
id: rg-hs9
title: 'PRD vs code: name is trimmed for display'
status: open
type: task
priority: 3
tags:
- prd
- csv
created: 2026-10-12
updated: 2026-10-12
phase: ''
---

# PRD vs code: name is trimmed for display

Found by the CSV guide review (docs/csv-format.md). FR-76 says normalisation never changes displayed text except the quadrant label (FR-9) and ring name (FR-15), but src/layout/legend/row.rs:62 and src/layout/radar/markers.rs:82 show name trimmed (terraform followed by a space shows as terraform). Decide which is right and align the PRD or the code; the guide currently describes the code.
