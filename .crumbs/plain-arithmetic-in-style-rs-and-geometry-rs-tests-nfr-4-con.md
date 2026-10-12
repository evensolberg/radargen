---
id: rg-4d0
title: Plain arithmetic in style.rs and geometry.rs tests (NFR-4 convention)
status: open
type: task
priority: 2
tags:
- nfr-4
- test
created: 2026-10-11
updated: 2026-10-11
phase: ''
---

# Plain arithmetic in style.rs and geometry.rs tests (NFR-4 convention)

src/style.rs's two_digit_numbers_fit_inside_the_marker test computes the clear interior with r.mul_add(2.0, -stroke). Tests compared with a tolerance are not NFR-4 output, but the crate convention is plain * and +; align it (and geometry.rs's test mul_adds) when next in the file.
