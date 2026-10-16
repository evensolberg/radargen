---
id: rg-rqe
title: Letter-spacing on scripts usvg does not space
status: open
type: bug
priority: 4
tags:
- png
- svg
- layout
created: 2026-10-16
updated: 2026-10-16
phase: ''
---

# Letter-spacing on scripts usvg does not space

usvg skips letter-spacing for some scripts (Devanagari and others); measure and place add it, so layout and the SVG differ from the PNG there. Inter draws none of those scripts (.notdef), so the effect is box spacing. From the PR 17 plan QA review.
