---
id: rg-7vl
title: Extend just lint to --all-targets and clear the test-target warnings
status: open
type: task
priority: 3
tags:
- lint
- tooling
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# Extend just lint to --all-targets and clear the test-target warnings

From PR 14 review. just lint runs cargo lclippy on the library only. Over --all-targets with pedantic + nursery, test code gives 10 warnings (9 suboptimal_flops in tests/measure.rs, tests/outlines_match_text.rs and lib tests; 1 missing_const_for_fn in tests/diagnostics.rs), plus unwrap_used hits in src/escape.rs tests with -W clippy::unwrap_used. Decide whether tests should meet nursery/unwrap_used (likely allow unwrap_used in tests via clippy.toml allow-unwrap-in-tests = true) and add --all-targets to the recipe.
