---
id: rg-3i4
title: outlines_match_text drives harfrust and skrifa by hand
status: closed
type: task
priority: 4
tags:
- test
- svg
created: 2026-10-16
updated: 2026-10-17
closed_reason: Spike test runs through FontMetrics; offsets and worst channel now checked (1.3.3)
phase: ''
---

# outlines_match_text drives harfrust and skrifa by hand

tests/outlines_match_text.rs (the svg-font spike) shapes and outlines with its own PathSink and mul_add; FontMetrics::shape/place/outline now cover it. Moving it onto them may shift the spike recorded numbers, so re-record the spike if done. From the PR 17 simplify reuse review.

[start] 2026-10-17 08:56:28

[stop]  2026-10-17 09:47:21  50m 53s
