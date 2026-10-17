---
id: rg-sw0
title: Test FontMetrics::place applies y_offset
status: open
type: task
priority: 4
tags:
- test
- text
created: 2026-10-17
updated: 2026-10-17
phase: ''
---

# Test FontMetrics::place applies y_offset

No test catches a flipped y_offset sign in FontMetrics::place (src/text.rs): with the Inter subset, even combining-mark corpora shape with y_offset 0, so the spike test and agrees_with_resvg pass under the mutation. Find text the subset shapes with y_offset != 0 and add a unit test that placed.y == -y_offset * scale; if none exists, record that. From the rg-3i4 test-quality review.
