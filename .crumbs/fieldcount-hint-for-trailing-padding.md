---
id: rg-w18
title: FieldCount hint for trailing padding
status: open
type: task
priority: 3
tags:
- prd
- diagnostics
- csv
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# FieldCount hint for trailing padding

From the PR 05 QA review (#19). Fatal::FieldCount with found > expected always says 'a comma inside a value needs the value in double quotes'. When data rows carry trailing ,, padding the header lacks (an Excel used-range artefact), that sends the author to the wrong cause. Decide the wording (e.g. mention trailing empty cells when the extra fields are all empty) together with rg-pcd's error grouping.
