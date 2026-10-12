---
id: rg-pti
title: Emit FR-95 when the narrow document passes 480 units
status: open
type: feature
priority: 2
tags:
- diagnostics
- layout
created: 2026-10-11
updated: 2026-10-11
phase: ''
---

# Emit FR-95 when the narrow document passes 480 units

PR 12 (D-71): placements(Narrow, ..).width > NARROW_MAX_WIDTH is the signal. Emit one render-level warning (RenderKind variant), once per render, exit 0, before row warnings (NFR-11), naming the quadrant whose label sets the radar width (the binding term in RadarBlock half_extent; not always the longest name — a 6 o clock label reaches less far than a diagonal one). Belongs with the pipeline wiring (PR 14 main.rs).
