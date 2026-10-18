---
id: rg-the
title: 'Linear-time tests fail under load: test the scaling ratio, not a 5 s limit'
status: closed
type: bug
priority: 2
tags:
- tests
- ci
created: 2026-10-18
updated: 2026-10-18
closed_reason: linear-time tests compare n vs 8n (test_timing::assert_linear)
phase: ''
---

# Linear-time tests fail under load: test the scaling ratio, not a 5 s limit

From the rg-dok QA review: src/palette/file/tests.rs (~470, ~485, ~914) and src/text.rs missing_runs_in_linear_time each assert a fixed 5 s wall-clock bound on a debug build that normally takes ~1.8-2 s; under load (avg ~117) all three palette tests failed at 5.8-6.9 s. Time n and 8n in one test, best of 3 each, assert t(8n)/t(n) < 24 (linear ~8, quadratic ~64), shrinking n to stay under a second. Consider [profile.test] opt-level = 1.

[start] 2026-10-18 10:50:15

[stop]  2026-10-18 10:54:43  4m 28s
