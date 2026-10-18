---
id: rg-ft5
title: 'HTML: no controls bar without script, but the radar cap still allows for it'
status: open
type: bug
priority: 4
tags:
- html
- layout
created: 2026-10-17
updated: 2026-10-17
phase: ''
---

# HTML: no controls bar without script, but the radar cap still allows for it

From the rg-247 review. Without JavaScript (and on a --palette page with nothing plotted) every control stays hidden, so the bar is 0 px, yet the radar cap still subtracts 3.05rem + 2px: the radar stops about 51 px short of the window's bottom. Safe direction, cosmetic. A fix would apply the allowance only under :root.js, duplicating every rule built from Wide::cap().

[2026-10-17] Print too (rg-kw7 review): the controls bar is not printed, but the cap still subtracts its height, so a printed radar is about 50 px smaller than it could be. Same fix shape: apply the allowance only where the bar shows.
