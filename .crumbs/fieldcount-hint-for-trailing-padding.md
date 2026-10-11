---
id: rg-w18
title: FieldCount hint for trailing padding
status: closed
type: task
priority: 3
tags:
- prd
- diagnostics
- csv
created: 2026-10-10
updated: 2026-10-11
blocks:
- rg-f27
phase: ''
---

# FieldCount hint for trailing padding

From the PR 05 QA review (#19). Fatal::FieldCount with found > expected always says 'a comma inside a value needs the value in double quotes'. When data rows carry trailing ,, padding the header lacks (an Excel used-range artefact), that sends the author to the wrong cause. Decide the wording (e.g. mention trailing empty cells when the extra fields are all empty) together with rg-pcd's error grouping.

[2026-10-10] Decided in PRD v0.44 (D-62, #24): FR-4's message, for one row or many, ends with the likely cause — the header first when no data row matches it; fewer values: a missing value or unquoted line break; more: a comma that needs quoting, or trailing empty cells when every value past the header's last column, in every such row, is empty under FR-76. It also counts the header's unnamed columns. Implement with rg-pcd.

[start] 2026-10-11 12:11:00

[stop]  2026-10-11 12:23:36  12m 36s
