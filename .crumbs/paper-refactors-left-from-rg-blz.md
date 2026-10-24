---
id: rg-h5l
title: Paper refactors left from rg-blz
status: closed
type: task
priority: 4
tags:
- pdf
- paper
- refactor
created: 2026-10-23
updated: 2026-10-23
closed_reason: Done in the rg-h5l refactor PR; behaviour-neutral, no release. Items 2 and 5 left with reasons in the crumb.
phase: v1.2
---

# Paper refactors left from rg-blz

Split from rg-blz by the PR 21c plan review (architect): tagging needs none of these. footed() takes all of Sheets but reads title/subtitle/size only (a Footing struct); keep PageKind beside each Document if a consumer appears; PieceSlots::per_page as NonZeroUsize; the title Wrap and piece_width rebuilt several times per page set; the pipeline builds an unused screen LegendBlock on paper. Behaviour-neutral: no bump.

[start] 2026-10-23 17:47:57  Paper refactors

[2026-10-23] Done: Footing struct for footed (sheets.footing()); PieceSlots::per_page is NonZeroUsize; title Wrap and piece width measured once per page set (Measured). Left on purpose: (2) PageKind beside each Document waits for a consumer, as the crumb says, and none exists; (5) the screen LegendBlock that Blocks::new builds is not unused on paper: grow_within measures it to set the radar growth (D-86) that Sheets::radar documents paper as sharing, so dropping it would resize the paper radar. Goldens unchanged.

[stop]  2026-10-23 18:00:53  12m 56s
