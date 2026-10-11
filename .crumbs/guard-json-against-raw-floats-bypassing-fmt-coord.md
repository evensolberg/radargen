---
id: rg-9bv
title: Guard json() against raw floats bypassing fmt_coord
status: open
type: task
priority: 2
tags:
- escape
- determinism
created: 2026-10-10
updated: 2026-10-11
blocks:
- rg-f27
phase: ''
---

# Guard json() against raw floats bypassing fmt_coord

From PR 12 review. escape::json serialises f64 through serde_json/ryu: unrounded, exponents allowed, -0.0 kept, NaN/inf become null — all outside NFR-17. Its doc comment says to pass coordinates as fmt_coord strings, but nothing enforces it. When PR 14 first builds the data island, either type its fields so no f64 reaches json() (e.g. a Coord newtype serialising via fmt_coord) or add a test that the island contains no raw float.
