---
id: rg-v1o
title: Run the manual accessibility checks and screenshot review against 1.14.0
status: open
type: task
priority: 1
tags:
- a11y
- release
created: 2026-10-15
updated: 2026-10-23
phase: v1.23.x
---

# Run the manual accessibility checks and screenshot review against 1.14.0

The maintainer called 1.0.0 on 2026-10-15 before docs/release-1.0.0.md §2's manual checks, and they have not been run since. Run docs/a11y/manual-checks.md (revision 1.9) against the current release, 1.6.0, and fill in its results table, then do the just screenshots review. The checklist now covers what shipped after 1.0.0: the legend button and the skip link to the table (steps 7 and 9, RG-102), the narrow arrangement's two parts (step 15), the PNG and SVG (steps 16 and 17) and --palette pages (step 18). Any failing step is a crumb, fixed in a patch release. Updated 2026-10-17 from 'against 1.0.0', which no longer matched what users install.

[2026-10-18] Retargeted to 1.14.0 (2026-10-18): run docs/a11y/manual-checks.md revision 1.13. Since 1.6.0 the HTML gained --table radar (heading "Items on the radar", the "Nothing is on the radar." row), the lower breakpoint while the legend is hidden with a press switching arrangements (D-97; step 9 now has zoom, scroll, touch, magnifier and screen-reader checks), the scrollbar gutter from the hidden breakpoint, and the title wrapping at the radar width (D-99). PNG/SVG gained --legend-columns.

[2026-10-23] Retargeted to 1.23.0 (PR 21c-2): run docs/a11y/manual-checks.md revision 1.19, which adds step 20, the tagged PDF (NVDA + Acrobat Reader, PAC 2024, Chrome; VoiceOver + Preview as a smoke test), including the row-header verbosity check. Since 1.14.0 the HTML table toggle reads "Table of N items" (1.22.2).
