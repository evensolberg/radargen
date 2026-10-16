---
id: rg-j7x
title: 'A bigger radar holds more items: a maximum size, and growth beyond overlap'
status: open
type: idea
priority: 2
tags:
- layout
- radar
- html
created: 2026-10-15
updated: 2026-10-15
phase: v1.x
---

# A bigger radar holds more items: a maximum size, and growth beyond overlap

Maintainer, 2026-10-15, at the 1.0.0 cut: "The radar does not really fit more items, though... I do wonder if we should consider a max size like we discussed initially." Window-filling (D-85) enlarges the radar without adding capacity, and growth (D-86) only engages once markers overlap, capped at 2x and a 1440 px breakpoint. Questions: a maximum on-screen size for the radar column; whether growth should take spare window room into account (capacity from the viewport rather than the data), which D-48 and D-86 bear on; how the narrow arrangement and PNG/SVG exports relate.
