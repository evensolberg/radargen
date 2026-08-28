---
id: rg-m59
title: Decide whether warnings need a volume control
status: open
type: task
priority: 3
tags:
- prd
- cli
- diagnostics
created: 2026-08-27
updated: 2026-08-27
phase: ''
---

# Decide whether warnings need a volume control

PRD v0.13 warns on ordinary content: empty description (FR-7), blank qualifier (FR-20), unrecognised status (FR-26), unparsable dates (FR-24/25), duplicate name (FR-28). The document's own seven-row example CSV emits at least three blank-qualifier warnings plus an unrecognised-ring warning. At A-2's 'tens of items' every healthy run prints a page of stderr.

There is no --quiet, no -v, no severity filter, and no cap on repetition. NFR-8 routes warnings to stderr so nothing breaks — but a channel nobody reads is worth less than one that fires selectively.

Two questions: (a) should there be a verbosity flag, and (b) should blank qualifier warn at all, given FR-29 treats every other absent optional column as normal? Note FR-20's blank warning was scoped in v0.13 to fire only where the column is present, which removes the worst case but not the general one.
