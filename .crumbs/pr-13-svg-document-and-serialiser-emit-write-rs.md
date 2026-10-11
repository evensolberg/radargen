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
- rg-f4o
phase: v1.0
---

# PR 13: svg/ Document and serialiser, emit/write.rs

Node, Document, late colour binding via Role, no ids, data-row identity. NFR-14 atomic write with a drop guard that removes the temp file (SC-18).

[2026-10-08] PRD v0.37 (D-52, SC-25): the HTML and SVG emitters write the output version, major.minor only, from CARGO_PKG_VERSION_MAJOR/_MINOR. Not a Document node.

[2026-10-09] From PR 6 simplify: Palette::number_ink returns a hex literal. Bind it through Paint (e.g. Paint::NumberInk(series index), resolved by the emitter via number_ink) so architecture §5's 'no node carries a hex value' holds.

[2026-10-11] [2026-10-11] svg::Node now arrives with PR 09's first block and grows variants per block PR; PR 13 adds Document, the serialiser and emit/write.rs.

[2026-10-11] [2026-10-11] From PR 09: serialise svg::Semantics (Heading(n) → role="heading" aria-level=n; Decorative → aria-hidden="true"), Text.lines as <tspan>s at Text.leading, PathCommand::Arc's large_arc/sweep flags, every coordinate through num::fmt_coord, text through escape::xml_text; decide whether an empty Group is emitted. The exported standalone SVG gets a root <title> (the edition title) and <desc>.

[2026-10-11] From PR 10a: TextPaint::Halo serialises as fill="none" stroke=<role> stroke-width=<w> stroke-linejoin="round" aria-hidden="true". The radar's root must not be role="img" (markers are its controls). Node::Line -> <line>.

[2026-10-11] From PR 10b (architecture §5 is the contract): Semantics::Marker(Control) → HTML: role="button" tabindex="0" aria-label data-row data-marker + <title> (when tooltip is Some); static SVG/PNG: aria-label + <title>, no role/tabindex. Skip Node::is_interactive_only nodes in SVG/PNG (hit layer and indicator layer drop whole). Semantics::Indicator(key) → data-indicator, visibility="hidden" aria-hidden="true" pointer-events="none". Node::HitArea → fill="none" pointer-events="all" data-hit. Paint::NumberOn(i) → Palette::number_ink(i), a literal in every format. Number text gets pointer-events: none.
