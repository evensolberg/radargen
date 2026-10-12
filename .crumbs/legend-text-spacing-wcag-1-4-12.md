---
id: rg-mru
title: Legend text spacing (WCAG 1.4.12)
status: closed
type: bug
priority: 1
tags:
- a11y
- layout
- legend
created: 2026-10-12
updated: 2026-10-12
closed_reason: 'PR 14c (#63): headroom for WCAG 1.4.12 (D-78) and the legend meta as the name''s tail'
phase: v1.0
---

# Legend text spacing (WCAG 1.4.12)

From the PR 14b accessibility review. A legend row draws the name and its meta (qualifier, status) as separate <text> at fixed x; with 0.12em letter spacing (1.4.12) the name runs into the meta. Emit the meta as a <tspan dx> inside the name text so spacing pushes it along. NFR-1 claims AA: fix before the first release (PR 15). Also seen in PR 14b: on a long name ("Trunk-Based Development"), the meta reaches the focus ring right edge; check LEGEND_ROW_RIGHT_OUTSET when this is done.
