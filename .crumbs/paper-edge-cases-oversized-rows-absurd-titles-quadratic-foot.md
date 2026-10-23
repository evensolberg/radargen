---
id: rg-i95
title: 'Paper edge cases: oversized rows, absurd titles, quadratic footer cut'
status: closed
type: bug
priority: 4
tags:
- pdf
- paper
created: 2026-10-22
updated: 2026-10-23
closed_reason: 'PR 21b: title cap and tall legend-row warning'
phase: v1.2
---

# Paper edge cases: oversized rows, absurd titles, quadratic footer cut

From the PR 21a final review (Rust minors). (1) A legend row taller than a page (a 1500-word name) stands alone but runs past the bottom margin over the footer with no warning: warn, like PaperRadarWide. (2) A title tall enough to fill page 1 pushes the radar off the page; PaperRadarWide says it runs past the margins: cap the title block or reword. (3) footer cut() is quadratic in words (2,000-word title 5.8 s debug, 20,000 never finishes): binary-search the word count.

[2026-10-22] Item (3) done in PR 21as simplify pass: cut() now wraps through TextStyle::wrap; a 2,000-word title on paper takes 1.1 s debug (was 5.8 s).
