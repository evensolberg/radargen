---
id: rg-min
title: Focus rings in forced-colours mode; :has cost on large radars
status: closed
type: task
priority: 2
tags:
- a11y
- html
created: 2026-10-12
updated: 2026-10-12
closed_reason: 'PR E: forced colours paint both focus rings Highlight (verified in Chromium); the :has cost is measured and moved to its own crumb for PR 15'
phase: v1.0
---

# Focus rings in forced-colours mode; :has cost on large radars

From PR 14b. The rings stroke var(--focus); forced colours may not remap SVG strokes. Under @media (forced-colors: active) set stroke:Highlight on [data-marker-indicator] ring circle and [data-row-indicator]>rect. Also, in the PR 15 AT pass, Tab through a ~300-item radar in Safari and Chrome and check style recalc per focus move stays small (one :has rule per marker). Check Firefox <121 shows its native outline on a focused SVG g[tabindex] (the no-:has fallback).
