---
id: rg-svr
title: 'PR 10: Radar block'
status: blocked
type: task
priority: 1
tags:
- impl
- layout
created: 2026-10-08
updated: 2026-10-11
blocks:
- rg-ozb
blocked_by:
- rg-osr
- rg-48z
- rg-6w7
phase: v1.0
---

# PR 10: Radar block

Sectors, bands, markers, labels, FR-40 status rings, FR-70 counter-scaled labels. Candidate to split if it outgrows one review.

[2026-10-09] D-54: numbers on solid markers use Palette::number_ink(series index), not a neutral role.

[2026-10-11] From PRD v0.46 FR-94 (D-66, was rg-pdt): each marker gets an invisible hit area giving a target of at least 24 CSS px at every width down to 320 px, counter-scaled in the narrow arrangement; overlapping markers in a band over FR-89's capacity rely on 2.5.8's equivalent exception.

[2026-10-11] [2026-10-11] From PR 08's final review (recommendations): add Geometry::middle(position) for quadrant-label angles (angle(p,0,1) equals it only by coincidence); consider Geometry::bands() and Band::mid() for ring circles and band labels; at N=1 the full-circle sector is a degenerate SVG arc (draw it as a circle or two arcs); with zero quadrants (no rows) draw ring circles only — sector() is infinite then.

[2026-10-11] [2026-10-11] From PR 09: draw markers as layout::marker::Glyph { centre, radius: MARKER_RADIUS, qualifier, mark: StatusMark::of(status, centreward), ink: Role::Series(rank) }.draw(scale) — the key uses the same path (FR-66). Wrap each glyph in a Group for its number, FR-36 <title> and FR-94 hit area. Add Node variants when needed: Line, PathCommand::{LineTo, Close}, Anchor::End, a text halo, Node::Title, a hit-area paint that still takes pointer events (Paint::None does not), a paint for marker numbers chosen per series entry at bind time (FR-82, D-54). Scale goes into coordinates (FR-70); labels keep their size. Markers' accessible names come from Catalogue::accessible_name (FR-50 as amended in PRD v0.49).
