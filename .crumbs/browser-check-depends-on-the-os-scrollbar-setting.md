---
id: rg-u57
title: Browser check depends on the OS scrollbar setting
status: closed
type: bug
priority: 2
tags:
- test-infra
- browser
created: 2026-10-16
updated: 2026-10-16
closed_reason: 'Fixed: the scenario styles the legend scrollbar, so Chromium lays out a classic gutter on any machine'
phase: ''
---

# Browser check depends on the OS scrollbar setting

scripts/interaction.mjs "a long legend scrolls beside the radar" asserts a classic scrollbar takes room (gutter > 0). On this Mac it now measures 0 px on main as well as on feat/pr-16-png (2026-10-16), so just ci fails for an environment reason: Chromium follows the macOS "show scroll bars" setting and draws overlay scrollbars. Force classic scrollbars in the check (a ::-webkit-scrollbar width in the test page, or a Chromium flag) so it does not depend on the machine.

[start] 2026-10-16 16:40:19

[stop]  2026-10-16 16:46:28  6m 9s
