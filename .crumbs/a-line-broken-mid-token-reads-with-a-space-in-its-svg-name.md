---
id: rg-t3p
title: A line broken mid-token reads with a space in its SVG name
status: closed
type: bug
priority: 3
tags:
- svg
- a11y
created: 2026-10-16
updated: 2026-10-16
closed_reason: 'Fixed: text nodes carry their words; the SVG names outlines from them, not the joined lines. HTML mirror fault filed as rg-g0c.'
phase: ''
---

# A line broken mid-token reads with a space in its SVG name

D-88 names an outlined text by its lines joined with spaces; a token wrap_by broke mid-token reads "Kuber netes". Document holds lines, not the source. Check too whether the HTML multi-line <text> (tspans with no space between) is named without one, the mirror fault. From the PR 17 plan reviews.

[start] 2026-10-16 17:04:52

[stop]  2026-10-16 17:14:37  9m 45s
