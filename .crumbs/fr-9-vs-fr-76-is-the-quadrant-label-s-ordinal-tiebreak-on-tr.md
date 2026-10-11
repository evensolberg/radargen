---
id: rg-40l
title: 'FR-9 vs FR-76: is the quadrant label''s ordinal tiebreak on trimmed or raw values?'
status: closed
type: idea
priority: 3
tags:
- prd
- ordering
created: 2026-10-10
updated: 2026-10-11
phase: ''
---

# FR-9 vs FR-76: is the quadrant label's ordinal tiebreak on trimmed or raw values?

From the PR 06 plan review. FR-9 says variants are 'compared trimmed'; FR-76 says the ordinal tiebreaks of FR-9 and FR-34 compare 'the raw values'. For ' tool' vs 'Tool' they pick different labels. PR 06 follows FR-9 (trimmed). Settle one sentence in the PRD, and say the same for FR-34's name tiebreak before PR 07.

[2026-10-10] From the PR 06 plan review (RG-006 rev 1.3, UX): FR-9's ordinal-first rule picks 'TOOL' over 'Tool', so one all-caps cell silently relabels the whole quadrant. FR-11 forbids flagging near-duplicates, but variants equal under FR-76 are not near-duplicates. Consider a warning when a quadrant's variants differ beyond FR-76's trim: "line {line}, quadrant: 'TOOL' and 'Tool' (line {first}) are one quadrant, labelled 'TOOL'; use one spelling to choose the label".

[2026-10-11] Settled by PRD v0.45 (D-63): the ordinal tiebreaks of FR-9 and FR-34 compare trimmed, unfolded values. The relabelling-warning idea moved to its own crumb.
