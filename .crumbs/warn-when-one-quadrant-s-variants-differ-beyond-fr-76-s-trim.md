---
id: rg-j63
title: Warn when one quadrant's variants differ beyond FR-76's trim
status: closed
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

[2026-10-11] Won't do (decided 2026-10-11 with the maintainer). Casing variants group correctly and the label is deterministic (D-29); which spelling labels the quadrant does not matter. The quadrant label is the most visible text on the radar, so a surprising choice shows at once without a diagnostic, and a warning on every run of a mixed-case export would be noise NFR-20 exists to reduce. FR-11 also keeps the tool out of judging spelling. Do not reopen without a case where the label choice causes real harm.
