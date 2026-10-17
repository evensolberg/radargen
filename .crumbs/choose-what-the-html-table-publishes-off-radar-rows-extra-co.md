---
id: rg-fgf
title: Choose what the HTML table publishes (off-radar rows, extra columns)
status: open
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
