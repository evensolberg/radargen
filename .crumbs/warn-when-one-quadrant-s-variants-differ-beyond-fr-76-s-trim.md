---
id: rg-j63
title: Warn when one quadrant's variants differ beyond FR-76's trim
status: open
type: idea
priority: 3
tags:
- prd
- diagnostics
- ordering
created: 2026-10-11
updated: 2026-10-11
phase: ''
---

# Warn when one quadrant's variants differ beyond FR-76's trim

Split from rg-40l. FR-9's ordinal-first rule picks 'TOOL' over 'Tool', so one all-caps cell silently relabels the whole quadrant. FR-11 forbids flagging near-duplicates, but variants equal under FR-76 are not near-duplicates. Consider a warning such as: "line {line}, quadrant: 'TOOL' and 'Tool' (line {first}) are one quadrant, labelled 'TOOL'; use one spelling to choose the label".
