---
id: rg-m2l
title: 'PR 17: SVG export with outlined, deduplicated text'
status: blocked
type: feature
priority: 2
tags:
- impl
- svg
- output
created: 2026-10-08
updated: 2026-10-12
blocks:
- rg-5uf
blocked_by:
- rg-6ku
phase: v1.1
---

# PR 17: SVG export with outlined, deduplicated text

Architecture §11: a converter, not a serialiser. Glyph outlines via ttf-parser, one path per distinct glyph in defs, placed with <use>. Literal colours.

[2026-10-11] From PR 12: export arrange(Arrangement::Wide, ..) only (FR-71).

[2026-10-12] From PR 13: extend svg::serialise rather than splicing its string: a text-emission seam (outlined vs <text>) and a root-prelude hook (<title>, <desc>, <defs>); <!-- radargen X.Y --> before the root; the root <title>/<desc> via escape::xml_text; no role="img" on the root (it would flatten headings and labels); consider xml:lang; outlined text leaves heading groups nameless, so give them an aria-label.

[2026-10-12] From PR 14c (D-78): Node::Text may carry a tail (the legend inline meta). The HTML writes it as a dx tspan; the outliner has no browser, so place the tail at Node::tail_x (last line advance + gap), and shape the tail text (it opens with U+00A0) in its own style.
