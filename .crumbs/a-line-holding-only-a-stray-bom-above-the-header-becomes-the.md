---
id: rg-ei7
title: A line holding only a stray BOM above the header becomes the header
status: open
type: idea
priority: 3
tags:
- prd
- csv
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# A line holding only a stray BOM above the header becomes the header

From the PR 06 plan review (RG-006 rev 1.3, QA). FR-3's blank-record test counts only spaces and tabs (D-59), so a first line holding only U+FEFF (concatenated exports) is the header: FR-5 then reports 'no name column' and FR-4 fails every row — the confusion rg-lme closed for header cells. PR 06 pins the same rule for data rows (a row of only U+200B is a row; FR-6 reports it). Decide in the PRD whether FR-3's blank test should also count FR-76's trimmable set.
