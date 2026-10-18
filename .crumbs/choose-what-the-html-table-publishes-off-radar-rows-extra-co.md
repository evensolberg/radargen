---
id: rg-fgf
title: Choose what the HTML table publishes (off-radar rows, extra columns)
status: in_progress
type: feature
priority: 2
tags:
- html
- table
- privacy
- cli
- prd
created: 2026-10-17
updated: 2026-10-17
phase: ''
---

# Choose what the HTML table publishes (off-radar rows, extra columns)

The page table shows every CSV row and column, including rows not on the radar and columns radargen does not use, so a CSV that doubles as an internal register publishes it all. PNG and SVG already leave these out. Add a choice, e.g. --table radar|all (rows) and a way to name the columns to publish, defaulting to todays behaviour or not (a PRD decision: safer default vs FR-93 and D-27). Needs PRD requirement, decision and SC; README Everything-in-the-CSV note then points to it. From the README rewrite discussion, 2026-10-17.

[2026-10-17] Decided (user, 2026-10-17): default unchanged (every row and column); one switch --table radar drops off-radar rows and unused columns; no per-column list.

[2026-10-18] --table radar should be the default if nothing is specified.

[2026-10-17] Decided (user, 2026-10-17, later), superseding the earlier answer: --table radar is the default (plotted rows, columns radargen uses); --table all publishes every row and column. Breaking change accepted (one user); release as minor with the default change called out in the notes.

[start] 2026-10-17 22:28:00
