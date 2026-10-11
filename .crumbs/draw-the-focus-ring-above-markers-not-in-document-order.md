---
id: rg-r48
title: Draw the focus ring above markers, not in document order
status: closed
type: task
priority: 2
tags:
- a11y
- html
created: 2026-10-09
updated: 2026-10-11
phase: ''
---

# Draw the focus ring above markers, not in document order

Focus #0B5FA5 / #7FB8E6 is 1.25-2.49:1 against the series colours, and SVG paints in document order, so later markers can cover a focused marker's outline (WCAG 2.4.11, 1.4.11). Draw focus in a top-layer overlay in PR 13/14, and note in SC-20 that focus is only guaranteed against surfaces. From the PR 6 accessibility review.

[2026-10-11] Decided in PRD v0.46 (D-66); implementation noted on rg-6zu (PR 15).
