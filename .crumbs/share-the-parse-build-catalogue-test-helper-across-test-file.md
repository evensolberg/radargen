---
id: rg-9zd
title: Share the parse→build_catalogue test helper across test files
status: open
type: task
priority: 4
tags:
- test
- tooling
created: 2026-10-11
updated: 2026-10-11
phase: ''
---

# Share the parse→build_catalogue test helper across test files

From the PR 07 simplify review. The chain parse → assert no errors → parse_rings → build_catalogue appears in src/model.rs tests, tests/model.rs, src/order.rs tests and tests/order.rs. A tests/common/mod.rs helper could serve the integration tests (unit tests cannot see it). Check rustqual's score before and after: inlining one-line helpers lowered it in PR 07.

[2026-10-11] [2026-10-11] PR 08 adds a third copy: tests/geometry.rs has csv/rows/build helpers beside src/order.rs's.
