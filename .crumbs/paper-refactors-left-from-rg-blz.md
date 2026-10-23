---
id: rg-h5l
title: Paper refactors left from rg-blz
status: open
type: task
priority: 4
tags:
- pdf
- paper
- refactor
created: 2026-10-23
updated: 2026-10-23
phase: v1.2
---

# Paper refactors left from rg-blz

Split from rg-blz by the PR 21c plan review (architect): tagging needs none of these. footed() takes all of Sheets but reads title/subtitle/size only (a Footing struct); keep PageKind beside each Document if a consumer appears; PieceSlots::per_page as NonZeroUsize; the title Wrap and piece_width rebuilt several times per page set; the pipeline builds an unused screen LegendBlock on paper. Behaviour-neutral: no bump.
