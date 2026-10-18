---
id: rg-the
title: 'Linear-time tests fail under load: test the scaling ratio, not a 5 s limit'
status: open
type: bug
priority: 2
tags:
- tests
- ci
created: 2026-10-18
updated: 2026-10-18
phase: ''
---

# Linear-time tests fail under load: test the scaling ratio, not a 5 s limit

From the rg-dok QA review: src/palette/file/tests.rs (~470, ~485, ~914) and src/text.rs missing_runs_in_linear_time each assert a fixed 5 s wall-clock bound on a debug build that normally takes ~1.8-2 s; under load (avg ~117) all three palette tests failed at 5.8-6.9 s. Time n and 8n in one test, best of 3 each, assert t(8n)/t(n) < 24 (linear ~8, quadratic ~64), shrinking n to stay under a second. Consider [profile.test] opt-level = 1.
