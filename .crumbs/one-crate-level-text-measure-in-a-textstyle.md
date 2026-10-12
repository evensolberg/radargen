---
id: rg-5r8
title: One crate-level text measure in a TextStyle
status: open
type: task
priority: 3
tags:
- layout
- quality
created: 2026-10-11
updated: 2026-10-11
phase: ''
---

# One crate-level text measure in a TextStyle

From the rg-rvh review: the face/size/letter-spacing measure is hand-written in src/layout/radar/mod.rs (~l.96), src/layout/title.rs (~l.51 and its test helper ~l.345), src/layout/key.rs (~l.131), src/layout/test_support.rs (~l.201), and legend/mod.rs (measure, wrapped). Lift one helper into crate::layout (or a TextStyle method in style.rs). Not crate::text: style.rs imports text::Weight, so text taking a TextStyle would make a style<->text cycle.
