---
id: rg-n54
title: Keyboard-scroll the detail panel in Chrome, Firefox and Safari (15c manual)
status: open
type: task
priority: 2
tags:
- a11y
- html
created: 2026-10-12
updated: 2026-10-12
phase: ''
---

# Keyboard-scroll the detail panel in Chrome, Firefox and Safari (15c manual)

From the PR 15b-2 accessibility seat (WCAG 2.1.1). At 320x256 the panel content is taller than its box, and an item with no link has nothing focusable below Close. Check by hand that with focus on the panel heading, ArrowDown and PageDown scroll the panel in desktop Chrome, Firefox and Safari; if any fails, make the panel a focusable scroll region. The CDP harness could not drive keyboard scrolling of any inner scroller.
