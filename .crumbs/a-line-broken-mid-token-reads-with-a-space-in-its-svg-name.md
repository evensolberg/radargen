---
id: rg-t3p
title: A line broken mid-token reads with a space in its SVG name
status: open
type: bug
priority: 3
tags:
- svg
- a11y
created: 2026-10-16
updated: 2026-10-16
phase: ''
---

# A line broken mid-token reads with a space in its SVG name

D-88 names an outlined text by its lines joined with spaces; a token wrap_by broke mid-token reads "Kuber netes". Document holds lines, not the source. Check too whether the HTML multi-line <text> (tspans with no space between) is named without one, the mirror fault. From the PR 17 plan reviews.
