---
id: rg-6w7
title: Marker numbers shrink below legibility in the narrow arrangement
status: closed
type: task
priority: 2
tags:
- layout
- accessibility
- oq-16
created: 2026-10-10
updated: 2026-10-11
blocks:
- rg-svr
phase: ''
---

# Marker numbers shrink below legibility in the narrow arrangement

From PR 13 (PR 04) accessibility review. MARKER_NUMBER is 9px and is the only link from a marker to its legend entry. At FR-69's breakpoint the wide document renders at 0.8x (numbers 7.2px). The narrow arrangement scales the radar by (420 - 40) / 660 = 0.576, so numbers fall to ~5.2px (~4px at a 320px viewport), while FR-70 counter-scales only ring and quadrant names. Decide with OQ-16 when PR 10 builds the radar block: likely extend FR-70's counter-scaling to the marker radius and number in the narrow arrangement (PRD amendment), and consider computing the breakpoint against an ~8px number floor as well (max(W*10/12.5, W*8/9)). Also confirm SC-17 contrast is measured against what is actually painted if quadrants are ever tinted. Maintainer chose to defer this to the layout PRs (2026-10-10).
