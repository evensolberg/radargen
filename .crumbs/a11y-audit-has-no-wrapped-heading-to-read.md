---
id: rg-cuj
title: A11y audit has no wrapped heading to read
status: open
type: task
priority: 4
tags:
- test-infra
- a11y
created: 2026-10-16
updated: 2026-10-16
phase: ''
---

# A11y audit has no wrapped heading to read

scripts/a11y-audit.mjs reads a level-3/4 heading by aria-label ?? textContent (rg-g0c), but its fixtures (interaction.csv, d71-wide.csv) have no quadrant or ring name long enough to wrap, so the aria-label branch never runs. Add one. Also note: a quadrant name with doubled internal spaces is collapsed in a wrapped heading (words) but kept in the table td and the row label, so the audit would report it misplaced; pre-existing, decide whether names should be collapsed at parse time.
