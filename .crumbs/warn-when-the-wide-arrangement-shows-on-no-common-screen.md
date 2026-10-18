---
id: rg-rv8
title: Warn when the wide arrangement shows on no common screen
status: closed
type: idea
priority: 3
tags:
- html
- diagnostics
- fr-69
created: 2026-10-15
updated: 2026-10-17
closed_reason: 'FR-105/D-94: HTML warns when the wide breakpoint passes 1440 px'
phase: v1.x
---

# Warn when the wide arrangement shows on no common screen

From the rg-tzz review: a quadrant name long enough to put the wide breakpoint past any screen (say past WIDE_MIN_VIEWPORT_PX) only gets FR-95's narrow-target warning, which talks about 320 px phones. A warning that the wide arrangement will not show on common screens would tell the author the bigger effect.

[2026-10-17] Decided (user, 2026-10-17): fire past WIDE_MIN_VIEWPORT_PX (1440 px); minor bump.

[start] 2026-10-17 21:29:14

[stop]  2026-10-17 21:48:42  19m 28s
