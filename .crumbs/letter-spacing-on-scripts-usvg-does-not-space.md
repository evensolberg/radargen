---
id: rg-rqe
title: Letter-spacing on scripts usvg does not space
status: closed
type: bug
priority: 4
tags:
- png
- svg
- layout
created: 2026-10-16
updated: 2026-10-16
closed_reason: 'Folded into rg-kzl: only observable once a font draws those scripts'
phase: ''
---

# Letter-spacing on scripts usvg does not space

usvg skips letter-spacing for some scripts (Devanagari and others); measure and place add it, so layout and the SVG differ from the PNG there. Inter draws none of those scripts (.notdef), so the effect is box spacing. From the PR 17 plan QA review.
