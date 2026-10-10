---
id: rg-w1v
title: Rasterise-within-measurement check for the SemiBold face
status: open
type: task
priority: 3
tags:
- font
- text
- test
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# Rasterise-within-measurement check for the SemiBold face

From PR 01 (#10) review. rasterises_within_measurement.rs covers Regular only. agrees_with_resvg pins SemiBold's advance to resvg's layout, but nothing checks that SemiBold's ink stays inside its measured advance. Matters once legend SemiBold headings sit near a wrap boundary (PR 04 onward).
