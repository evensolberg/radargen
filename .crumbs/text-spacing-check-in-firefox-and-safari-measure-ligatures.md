---
id: rg-497
title: Text-spacing check in Firefox and Safari; measure ligatures
status: closed
type: task
priority: 4
tags:
- tests
- a11y
created: 2026-10-12
updated: 2026-10-17
closed_reason: text-spacing check runs in Chromium, Firefox and WebKit via Playwright (1.3.4)
phase: ''
---

# Text-spacing check in Firefox and Safari; measure ligatures

Deferred from rg-6h9 item 4. scripts/text-spacing.mjs drives Chromium only (CDP). Firefox (WebDriver BiDi) and WebKit are not covered, and Chromium turns off optional ligatures under letter-spacing, so the browsers that keep them may draw names wider than measured. Playwright ships webkit and firefox builds; a BiDi client or Playwright as a dev dependency would cover both.

[start] 2026-10-17 08:56:27

[stop]  2026-10-17 10:04:30  1h 8m 3s
