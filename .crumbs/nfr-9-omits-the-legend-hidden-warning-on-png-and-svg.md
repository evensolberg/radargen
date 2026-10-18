---
id: rg-mx8
title: NFR-9 omits the --legend hidden warning on PNG and SVG
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

# NFR-9 omits the --legend hidden warning on PNG and SVG

NFR-9's warning list omits FR-103's warning for --legend hidden on a PNG or SVG. rg-fgf added FR-106's --table all warning to the list beside FR-105's; FR-103's was missing before that. Add it, and check the list against RenderKind for any other gap. Found during rg-fgf, 2026-10-17.
