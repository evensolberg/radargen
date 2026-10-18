---
id: rg-prs
title: Warn when a column header nearly matches a known column
status: closed
type: idea
priority: 3
tags:
- columns
- diagnostics
- table
created: 2026-10-17
updated: 2026-10-18
closed_reason: 'FR-108: near-miss header warning shipped'
phase: ''
---

# Warn when a column header nearly matches a known column

From the rg-fgf architecture review: before --table radar, a misspelled known column (rationalle, descripton) still showed in the table, so the author noticed. Under the default it is classed Other and dropped silently. A near-miss hint (edit distance 1-2 against known names, once per header) would catch typos without the per-render noise D-95 rejected for every extra column.

[start] 2026-10-18 11:06:18

[stop]  2026-10-18 11:14:16  7m 58s
