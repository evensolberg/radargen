---
id: rg-lp8
title: 'Docs: unresolved intra-doc link to displayable in cli.rs'
status: open
type: task
priority: 4
tags:
- docs
- lint
created: 2026-10-23
updated: 2026-10-23
phase: ''
---

# Docs: unresolved intra-doc link to displayable in cli.rs

cargo doc warns: src/cli.rs:364 links [`displayable`], which is not in scope there. Found 2026-10-23 during rg-37u; predates it. Fix the path (e.g. crate::...::displayable) and consider adding RUSTDOCFLAGS=-D warnings to just ci so broken links fail the gate.
