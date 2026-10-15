---
id: rg-zb9
title: 'A horizontal key: several entries per row'
status: open
type: feature
priority: 1
tags:
- layout
- key
created: 2026-10-15
updated: 2026-10-15
phase: v1.0
---

# A horizontal key: several entries per row

From rg-xe7. The key (src/layout/key.rs) stacks its five entries vertically below the radar. Lay them out in rows of as many as fit the width the arrangement gives (wide: 2 rows; narrow: likely 2 per row, check the longest label), so the key takes less height. FR-66 fixes content only. Touches arrange.rs (the key needs its width), D-78 headroom, the text-spacing check and the goldens. Maintainer: in 1.0.0, its own PR.
