---
id: rg-lp8
title: 'Docs: fix cargo doc warnings (broken and private intra-doc links)'
status: closed
type: task
priority: 4
tags:
- docs
- lint
created: 2026-10-23
updated: 2026-10-23
closed_reason: fixed all 12 cargo doc warnings; just ci gates cargo doc with -D warnings
phase: ''
---

# Docs: fix cargo doc warnings (broken and private intra-doc links)

cargo doc warns: src/cli.rs:364 links [`displayable`], which is not in scope there. Found 2026-10-23 during rg-37u; predates it. Fix the path (e.g. crate::...::displayable) and consider adding RUSTDOCFLAGS=-D warnings to just ci so broken links fail the gate.

[2026-10-23] Also pre-existing: diagnostics.rs SectorTwice doc links the private SECTOR_ALIAS; layout/table/mod.rs module doc links the private columns. Fix all, then gate cargo doc with RUSTDOCFLAGS=-D warnings in just ci.

[start] 2026-10-23 20:05:39

[stop]  2026-10-23 20:13:08  7m 29s
