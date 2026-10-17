---
id: rg-ksn
title: Long-legend check cannot catch a scrollbar narrowing the legend
status: open
type: bug
priority: 3
tags:
- test-infra
- browser
created: 2026-10-16
updated: 2026-10-16
phase: ''
---

# Long-legend check cannot catch a scrollbar narrowing the legend

scripts/interaction.mjs, "a long legend scrolls beside the radar": the check `|drawn - own| <= 0.5` cannot fail. The legend svg has a fixed CSS px width and .wide-legend clips with overflow-x:hidden, so getBoundingClientRect().width equals the viewBox width whether or not the gutter is reserved; a lost gutter clips the right edge rather than narrowing it. Assert the visible area instead: box.clientWidth >= own - 0.5. Found by the rg-u57 review; it predates that fix.
