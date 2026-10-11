---
id: rg-bvl
title: 'Default-ring warning: list the rings and say how to plot the row'
status: open
type: task
priority: 3
tags:
- prd
- diagnostics
- rings
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# Default-ring warning: list the rings and say how to plot the row

From the PR 06 plan review. With RingSource::Default, "'Rejected' is not declared by the default ring set" names nothing the author declared and gives no next step. Proposed: "'Rejected' is not one of the default rings (Assess, Trial, Adopt, Retire); … to plot it, list it in --rings". Listing the rings also shows a typo like Adpot. FR-17 and D-61 pin the current wording, so this is a PRD change first.
