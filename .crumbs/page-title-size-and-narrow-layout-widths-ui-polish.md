---
id: rg-zch
title: Page title size and narrow-layout widths (UI polish)
status: closed
type: task
priority: 3
tags:
- html
- ui
created: 2026-10-12
updated: 2026-10-15
closed_reason: Narrow legend panel spans the column (LegendBlock::spanning). The visible title is the SVG's; the only HTML h1 is visually hidden, so nothing to size.
phase: ''
---

# Page title size and narrow-layout widths (UI polish)

From the PR 15c UI seat. (1) The HTML h1 for an untitled page computes to 16px, the size of body text; the visible title is the SVG title, but the h1 (visually hidden) is fine — check whether any visible title sits in HTML and size it ~1.5rem. (2) At 400 px the legend card shrinks to its content while the panel spans the viewport and the table is inset: three widths stacked. Consider the legend at full width in the narrow arrangement.

[start] 2026-10-15 16:56:44

[stop]  2026-10-15 17:04:33  7m 49s
