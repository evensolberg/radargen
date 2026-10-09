---
id: rg-rnc
title: 'PR 6: model.rs — Item, Catalogue, column validation'
status: blocked
type: task
priority: 2
tags:
- impl
- csv
- model
created: 2026-10-08
updated: 2026-10-08
blocks:
- rg-efx
- rg-48z
- rg-f27
blocked_by:
- rg-xr3
- rg-aqb
phase: v1.0
---

# PR 6: model.rs — Item, Catalogue, column validation

FR-5–FR-29, FR-27/FR-78 (declared header order, unrecognised cells kept), accessible_name() (FR-50, FR-75). build_catalogue returns Result<_, Vec<Fatal>> (NFR-16 tier two). Normalised sort keys computed once at construction.
