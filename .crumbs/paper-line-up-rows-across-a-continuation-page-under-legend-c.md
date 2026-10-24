---
id: rg-jao
title: 'Paper: line up rows across a continuation page under Legend (continued)'
status: open
type: task
priority: 4
tags:
- pdf
- paper
- polish
created: 2026-10-23
updated: 2026-10-23
phase: v1.2
---

# Paper: line up rows across a continuation page under Legend (continued)

Split from rg-73x (UI minor 5 of the PR 21a final review). Only the first column on a continuation page opens with Legend (continued), so columns 2+ start their rows one heading-row higher and rows do not line up across the page. Starting columns 2+ lower would cost each of them a row of room, which changes how LegendBlock::flowed paginates (piece room, PieceSlots), so it is a pagination change, not the cosmetic panel fix rg-73x made.
