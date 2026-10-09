---
id: rg-6ku
title: 'PR 13: svg/ Document and serialiser, emit/write.rs'
status: blocked
type: task
priority: 1
tags:
- impl
- svg
- output
created: 2026-10-08
updated: 2026-10-08
blocks:
- rg-f27
- rg-42m
- rg-m2l
blocked_by:
- rg-ozb
- rg-vgm
phase: v1.0
---

# PR 13: svg/ Document and serialiser, emit/write.rs

Node, Document, late colour binding via Role, no ids, data-row identity. NFR-14 atomic write with a drop guard that removes the temp file (SC-18).

[2026-10-08] PRD v0.37 (D-52, SC-25): the HTML and SVG emitters write the output version, major.minor only, from CARGO_PKG_VERSION_MAJOR/_MINOR. Not a Document node.
