---
id: rg-v3a
title: Upgrade harfrust and html5ever when their companions catch up
status: open
type: task
priority: 3
tags:
- deps
created: 2026-10-13
updated: 2026-10-15
phase: ''
---

# Upgrade harfrust and html5ever when their companions catch up

From the 2026-10-13 dependency check (rg-9ay). Held back: (1) harfrust 0.12 -> 0.14 (a new API: FontRef, ShaperData and UnicodeBuffer are gone from the root; ShaperFont, Buffer and shape() replace them). It must move with usvg: D-67 measures with the shaper resvg rasterises with, so wrap points agree by construction, and usvg 0.48.1 (the latest) still uses harfrust 0.12. Upgrade both together when usvg moves past 0.48, and rerun agrees_with_resvg and the goldens. (2) html5ever 0.39 -> 0.40 (dev only): markup5ever_rcdom has no 0.40, and 0.39 cannot sink a 0.40 parser. Upgrade both when rcdom 0.40 is published.

[2026-10-15] skrifa (dev, replaced ttf-parser in rg-8im) is pinned to the version usvg uses; move it with usvg.
