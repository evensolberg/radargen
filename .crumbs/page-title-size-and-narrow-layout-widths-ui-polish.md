---
id: rg-zch
title: Page title size and narrow-layout widths (UI polish)
status: open
type: task
priority: 3
tags:
- html
- ui
created: 2026-10-12
updated: 2026-10-12
phase: ''
---

# Page title size and narrow-layout widths (UI polish)

From the PR 15c UI seat. (1) The HTML h1 for an untitled page computes to 16px, the size of body text; the visible title is the SVG title, but the h1 (visually hidden) is fine — check whether any visible title sits in HTML and size it ~1.5rem. (2) At 400 px the legend card shrinks to its content while the panel spans the viewport and the table is inset: three widths stacked. Consider the legend at full width in the narrow arrangement.
