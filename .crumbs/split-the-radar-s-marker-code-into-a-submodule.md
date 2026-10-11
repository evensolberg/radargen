---
id: rg-umt
title: Split the radar's marker code into a submodule
status: open
type: task
priority: 2
tags:
- layout
- quality
created: 2026-10-11
updated: 2026-10-11
phase: ''
---

# Split the radar's marker code into a submodule

rustqual flags src/layout/radar.rs SRP_MODULE (378 production lines after PR 10b). Move Marker, markers, marker_nodes, glyph_nodes, number_node, hit_node, indicator_node, layer and hit_reach into a layout/radar/markers.rs submodule. Natural moment: PR 11, whose legend controls reuse NumberOn and Control. Also: define 'hollow' once (Item::hollow, radar number_node and Glyph::draw each decide it).

[2026-10-11] Also from the final review: M5 (render clones the drawn markers twice, once for the controls and once for the indicators; iterate by value for one consumer) and M7's invariant once split (markers() asserts place() and plotted.items have equal lengths; keep it in the submodule).
