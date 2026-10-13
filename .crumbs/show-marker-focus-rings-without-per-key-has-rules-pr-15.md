---
id: rg-jri
title: Show marker focus rings without per-key :has rules (PR 15)
status: open
type: task
priority: 2
tags:
- a11y
- html
- performance
created: 2026-10-12
updated: 2026-10-12
phase: v1.0
---

# Show marker focus rings without per-key :has rules (PR 15)

From rg-min (measured in headless Chromium, 2026-10-12): the per-key rules svg:has([data-marker="L"]:focus-visible) [data-marker-indicator="L"] cost 2.4 ms of style recalc per Tab at 60 markers, 8.6 ms at 120, 49 ms at 300 (all of it the :has block; 0.01 ms without). PR 15 adds JavaScript: show the focused marker indicator from a focusin/focusout handler (set its visibility) and drop the per-key rules, keeping a no-JS fallback (the native outline when no script runs, e.g. a class set by the script gates the outline:transparent rule). Re-measure with the same probe.

[2026-10-12] Planned into PR 15b (docs/plans/pr-15a-table.md, Split of PR 15).
