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
updated: 2026-10-11
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

[2026-10-09] From PR 6 simplify: Palette::number_ink returns a hex literal. Bind it through Paint (e.g. Paint::NumberInk(series index), resolved by the emitter via number_ink) so architecture §5's 'no node carries a hex value' holds.

[2026-10-11] [2026-10-11] svg::Node now arrives with PR 09's first block and grows variants per block PR; PR 13 adds Document, the serialiser and emit/write.rs.
