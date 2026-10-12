---
id: rg-aea
title: Split layout/test_support.rs so shared test helpers can grow
status: open
type: task
priority: 3
tags:
- test
- quality
- rustqual
created: 2026-10-11
updated: 2026-10-11
phase: ''
---

# Split layout/test_support.rs so shared test helpers can grow

From the PR 12b simplify pass: promoting radar tests csv_of (radar/mod.rs) and legend tests groups (legend/mod.rs) into test_support pushed it past rustqual SRP_MODULE (316 lines), so arrange.rs keeps local copies (quadrants, controls). Split test_support by concern (fixtures and CSV builders vs. node walkers and bounds), then share csv_of, groups and HEAD (radar keeps a HEADER copy).
