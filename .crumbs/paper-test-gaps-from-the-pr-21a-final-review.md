---
id: rg-q1q
title: Paper test gaps from the PR 21a final review
status: closed
type: task
priority: 4
tags:
- pdf
- paper
- testing
created: 2026-10-22
updated: 2026-10-23
closed_reason: PR 21b Task 9
phase: v1.2
---

# Paper test gaps from the PR 21a final review

From the PR 21a final review (QA minors). Add: a header-only CSV through the CLI on paper (1 page, Page 1 of 1, no Legend); /Title read back for a hostile title on paper; the no-piece-taller-than-its-space loop over every shape with openers; full continuation pages hold exactly per_page columns, Lw + COLUMN_GAP apart; the ground corners equal the dark palette's ground; footer runs disjoint. Fix: the_legend_draws_at_sixteen_fifteenths compares the constant with itself (pin a ring heading at 10 x 16/15); tests/cli.rs's --paper block split help_names_each_default's doc comment from its fn; tests/pdf_matches_png/paper.rs's docstring says each continuation page is full.

[2026-10-22] From the PR 21a post-simplify review (minors): a serialiser test for text-anchor="end" beside the middle one (serialise/tests/text.rs); a_trailing_empty_sector_never_stands_alone should also assert the empty sectors heading survives (S4e); continuation()s span fold adds in a new order (width*n + GAP*(n-1) reads clearer); the plan RG-021A still describes FirstPage { title, key } and Ch = H - 2m - t (historical; note PAPER_FOOT_ROOM if kept accurate).
