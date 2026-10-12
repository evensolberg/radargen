---
id: rg-w39
title: Split Fatal by tier; render errors once
status: open
type: task
priority: 3
tags:
- diagnostics
- pipeline
created: 2026-10-12
updated: 2026-10-12
phase: ''
---

# Split Fatal by tier; render errors once

From the PR 14a simplify (altitude). Tier-one variants (MissingDirectory, OutputIsInput, Font, and PR 19 --config keys) must be listed in errors.rs classify/line_of/rank_of though they never reach row grouping, and pipeline.rs renders Fatal to strings at each stage (alone(), map_err). Split Fatal into a row-level enum render_errors takes and a tier-one enum that renders alone; let stages return one failure type with From<Fatal> so ? works throughout and run renders once. Best done before PR 19 adds more tier-one errors.
