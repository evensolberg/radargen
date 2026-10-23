---
id: rg-q1q
title: Paper test gaps from the PR 21a final review
status: open
type: task
priority: 4
tags:
- pdf
- paper
- testing
created: 2026-10-22
updated: 2026-10-22
phase: v1.2
---

# Paper test gaps from the PR 21a final review

From the PR 21a final review (QA minors). Add: a header-only CSV through the CLI on paper (1 page, Page 1 of 1, no Legend); /Title read back for a hostile title on paper; the no-piece-taller-than-its-space loop over every shape with openers; full continuation pages hold exactly per_page columns, Lw + COLUMN_GAP apart; the ground corners equal the dark palette's ground; footer runs disjoint. Fix: the_legend_draws_at_sixteen_fifteenths compares the constant with itself (pin a ring heading at 10 x 16/15); tests/cli.rs's --paper block split help_names_each_default's doc comment from its fn; tests/pdf_matches_png/paper.rs's docstring says each continuation page is full.
