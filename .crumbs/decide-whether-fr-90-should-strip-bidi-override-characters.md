---
id: rg-rta
title: Decide whether FR-90 should strip bidi override characters
status: closed
type: idea
priority: 2
tags:
- a11y
- diagnostics
- prd
created: 2026-10-09
updated: 2026-10-11
phase: ''
---

# Decide whether FR-90 should strip bidi override characters

U+202A-E and U+2066-9 survive FR-90, so visual order can differ from the order aria-label is read. Left in place because they have legitimate uses in RTL names and A-4 keeps input mostly Latin. Revisit if non-Latin input is supported. From the PR 6 accessibility review.

[2026-10-11] Folded into rg-kzl (right-to-left, CJK and Indic support), out of scope by A-4 and D-68.
