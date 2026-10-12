---
id: rg-6h9
title: 'Text-spacing check: harden and widen (PR 14c review minors)'
status: open
type: task
priority: 3
tags:
- tests
- a11y
created: 2026-10-12
updated: 2026-10-12
phase: ''
---

# Text-spacing check: harden and widen (PR 14c review minors)

From the PR 14c final review. (1) tests/tail_oracle.rs covers only one-line names; add a wrapped-name row (a probe passed at 54/64 px). (2) style::spacing_allowance is pub for one in-crate test; make it pub(crate). (3) scripts/text-spacing.mjs: Chromium path is macOS-arm64 only, readdirSync throws without Playwright, fixed sleeps, hard-coded port 9334, a missed breakpoint regex gives NaN widths silently, and digit-only names are skipped (a name like 2048). (4) Firefox and Safari are not covered (Chromium only); and Chromium disables optional ligatures under letter-spacing, unmeasured.
