---
id: rg-p82
title: Full PDF table polish from the PR 21b final review
status: in_progress
type: task
priority: 4
tags:
- pdf
- table
- polish
created: 2026-10-23
updated: 2026-10-23
phase: v1.2
---

# Full PDF table polish from the PR 21b final review

Minors deferred from the PR 21b final review: (1) a wrapped word can leave trailing punctuation alone on a line when a column is very narrow (UI 7); (2) TABLE_HEADING_TO_HEAD 10 -> 14 or a muted "(continued): ..." run to strengthen the heading hierarchy (UI 9); (3) TABLE_POINTER_ABOVE 18 -> 22 to set the pointer apart from the footer (UI 10); (4) tests: a sector change and the unplotted boundary falling at a page top (QA 9), a paper golden with the legend wholly after page 1 (PieceSlots on_first false, QA 11), a prd-full A4 golden (QA 9); (5) table test long functions and everything_inside_its_column complexity (rustqual).

[2026-10-23] Sector/ring stack (1.22.0): the stacked header caps each heading at 3 lines, so a long user-written ring header can make the header 4 lines (Sector + 3). Cap the combined list or update HEADING_LINES doc.

[start] 2026-10-23 11:06:38
