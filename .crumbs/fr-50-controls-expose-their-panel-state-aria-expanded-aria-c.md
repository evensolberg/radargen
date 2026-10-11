---
id: rg-1uw
title: FR-50 controls expose their panel state (aria-expanded, aria-controls)
status: closed
type: task
priority: 2
tags:
- prd
- a11y
- html
created: 2026-10-10
updated: 2026-10-11
blocks:
- rg-f27
phase: ''
---

# FR-50 controls expose their panel state (aria-expanded, aria-controls)

From the PR 06 plan accessibility review (RG-006 rev 1.4). FR-50's controls (markers, legend rows, table rows) open the detail panel (FR-38) and show a selection (FR-72), but nothing in the PRD or design says how that state reaches assistive technology (WCAG 4.1.2). Proposed: the activated control carries aria-expanded="true" and aria-controls pointing at the panel; every other control carries aria-expanded="false". State stays out of the accessible name. Decide in the PRD (FR-50) before PR 14 (rg-f27) builds the interactive layer.

[2026-10-11] Decided in PRD v0.46 (D-66); implementation noted on rg-6zu (PR 15).
