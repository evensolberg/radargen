---
id: rg-mx8
title: NFR-9 omits the --legend hidden warning on PNG and SVG
status: closed
type: task
priority: 3
tags:
- prd
- diagnostics
created: 2026-10-17
updated: 2026-10-18
closed_reason: NFR-9 lists FR-103 and D-87 warnings (PRD 0.95)
phase: ''
---

# NFR-9 omits the --legend hidden warning on PNG and SVG

NFR-9's warning list omits FR-103's warning for --legend hidden on a PNG or SVG. rg-fgf added FR-106's --table all warning to the list beside FR-105's; FR-103's was missing before that. Add it, and check the list against RenderKind for any other gap. Found during rg-fgf, 2026-10-17.

[start] 2026-10-18 10:50:15

[stop]  2026-10-18 10:54:43  4m 28s
