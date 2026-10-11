---
id: rg-lme
title: Invisible format characters in header names
status: closed
type: task
priority: 3
tags:
- prd
- csv
created: 2026-10-10
updated: 2026-10-10
closed_reason: 'PRD v0.43, D-61: FR-76 trims leading/trailing Cf; implemented in PR 06 (rg-rnc)'
phase: ''
---

# Invisible format characters in header names

From the PR 05 QA review (#19). A second BOM (U+FEFF) or a zero-width space (U+200B) in a header cell is neither FR-90's undisplayable set nor removed by FR-76's trim, so PR 06's FR-5 will report 'name' missing from a header that visibly has it. The second-BOM case is deliberate and pinned in csv::decode tests. Decide in the PRD whether FR-76 or FR-90 covers Unicode format characters (Cf) in header names.
