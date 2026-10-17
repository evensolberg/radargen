---
id: rg-nsp
title: 'PRD: record the CSV and --palette read caps as runtime failures'
status: open
type: task
priority: 3
tags:
- prd
- diagnostics
created: 2026-10-17
updated: 2026-10-17
phase: ''
---

# PRD: record the CSV and --palette read caps as runtime failures

The CSV cap (64 MiB, rg-w5k) and the --palette cap (1 MiB, RG-018A R4) are user-visible refusals no requirement lists. Add both to NFR-15 runtime failures (and NFR-16 tier one), citing FR-61 and FR-102. From the rg-w5k review.
