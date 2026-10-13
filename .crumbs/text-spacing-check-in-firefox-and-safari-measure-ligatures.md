---
id: rg-497
title: Text-spacing check in Firefox and Safari; measure ligatures
status: open
type: task
priority: 4
tags:
- tests
- a11y
created: 2026-10-12
updated: 2026-10-12
phase: ''
---

# Text-spacing check in Firefox and Safari; measure ligatures

Deferred from rg-6h9 item 4. scripts/text-spacing.mjs drives Chromium only (CDP). Firefox (WebDriver BiDi) and WebKit are not covered, and Chromium turns off optional ligatures under letter-spacing, so the browsers that keep them may draw names wider than measured. Playwright ships webkit and firefox builds; a BiDi client or Playwright as a dev dependency would cover both.
