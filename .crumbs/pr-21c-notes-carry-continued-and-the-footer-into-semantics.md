---
id: rg-blz
title: 'PR 21c notes: carry continued and the footer into Semantics'
status: open
type: task
priority: 3
tags:
- pdf
- a11y
- tagging
created: 2026-10-22
updated: 2026-10-22
phase: v1.2
---

# PR 21c notes: carry continued and the footer into Semantics

From the PR 21a final review (architecture minors 3-7). entry_node drops Entry::Heading.continued, so on a Document a continued heading looks like a real one: carry it into Semantics in heading_node. The footer is found only as the root's last child: give it its own Semantics in footer(). footed takes all of Sheets but reads title/subtitle/size only (a Footing struct). The title Wrap and piece_width are rebuilt several times per page set; the pipeline builds an unused screen LegendBlock on paper.
