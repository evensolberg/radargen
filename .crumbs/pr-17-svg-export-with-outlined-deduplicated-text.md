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
updated: 2026-10-08
blocked_by:
- rg-6ku
phase: v1.1
---

# PR 17: SVG export with outlined, deduplicated text

Architecture §11: a converter, not a serialiser. Glyph outlines via ttf-parser, one path per distinct glyph in defs, placed with <use>. Literal colours.
