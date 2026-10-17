---
id: rg-v1o
title: Run the manual accessibility checks and screenshot review against 1.6.0
status: open
type: task
priority: 1
tags:
- a11y
- release
created: 2026-10-15
updated: 2026-10-17
phase: v1.6.x
---

# Run the manual accessibility checks and screenshot review against 1.6.0

The maintainer called 1.0.0 on 2026-10-15 before docs/release-1.0.0.md §2's manual checks, and they have not been run since. Run docs/a11y/manual-checks.md (revision 1.9) against the current release, 1.6.0, and fill in its results table, then do the just screenshots review. The checklist now covers what shipped after 1.0.0: the legend button and the skip link to the table (steps 7 and 9, RG-102), the narrow arrangement's two parts (step 15), the PNG and SVG (steps 16 and 17) and --palette pages (step 18). Any failing step is a crumb, fixed in a patch release. Updated 2026-10-17 from 'against 1.0.0', which no longer matched what users install.
