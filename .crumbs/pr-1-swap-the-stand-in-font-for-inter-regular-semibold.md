---
id: rg-osr
title: 'PR 1: Swap the stand-in font for Inter (Regular + SemiBold)'
status: open
type: task
priority: 1
tags:
- impl
- font
- text
created: 2026-10-08
updated: 2026-10-08
blocks:
- rg-y1t
- rg-svr
- rg-67j
phase: v1.0
---

# PR 1: Swap the stand-in font for Inter (Regular + SemiBold)

FR-85. Replace Libertinus with Inter, Latin subset, TTF + WOFF2 subset from one source in one step. text.rs gains a weight parameter (two faces, loaded once). Must land before the layout blocks (architecture §9) so golden files are never generated against the wrong face. Record the measured subset size (R-7).
