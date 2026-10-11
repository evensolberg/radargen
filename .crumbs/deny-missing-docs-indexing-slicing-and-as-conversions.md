---
id: rg-hz0
title: Deny missing_docs, indexing_slicing and as_conversions
status: open
type: task
priority: 2
tags:
- lint
- tooling
created: 2026-10-11
updated: 2026-10-11
blocks:
- rg-y1t
phase: ''
---

# Deny missing_docs, indexing_slicing and as_conversions

The maintainer wants lints as strict as possible (2026-10-11), and chose to do these before PR 09. Counts on main after PR A: missing_docs 119 library items (mostly struct fields and enum variants), clippy::indexing_slicing 28 library sites (97 with tests), clippy::as_conversions 5. Add each to Cargo.toml's [lints] as deny and fix every finding: document every public item; replace indexing with get()/iterators or a reasoned #[expect] where an invariant guarantees the index; replace as with From/TryFrom (the usize→f64 casts in layout::geometry::real and text::measure need a reasoned #[expect] or a lossless path). Behaviour must not change.
