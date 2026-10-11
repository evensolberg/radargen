---
id: rg-rnc
title: 'PR 06: model.rs — Item, Catalogue, column validation'
status: open
type: task
priority: 2
tags:
- impl
- csv
- model
created: 2026-10-08
updated: 2026-10-10
blocks:
- rg-efx
- rg-48z
- rg-f27
blocked_by:
- rg-xr3
- rg-aqb
phase: v1.0
---

# PR 06: model.rs — Item, Catalogue, column validation

FR-5–FR-29, FR-27/FR-78 (declared header order, unrecognised cells kept), accessible_name() (FR-50, FR-75). build_catalogue returns Result<_, Vec<Fatal>> (NFR-16 tier two). Normalised sort keys computed once at construction.

[2026-10-10] From the PR 05 plan (RG-005 rev 1.1): build_catalogue takes &Table (header, header_line, records). Rows failing FR-4 are absent from records, but their errors are in parse's Vec<Fatal>, returned beside the table. Decide whether an empty-named header column is accepted, and how FR-78 heads it. Add a test that a ,,, row between data rows does not shift any FR-6 line number.

[2026-10-10] PRD v0.43 (D-61) settles the open questions. FR-93: an empty-named header column is dropped (not in Catalogue header, table or detail panel); warn naming its 1-based position and line when a row holds a value under it, NFR-20 grouping by position; padding-only columns pass silently. FR-76: lib::normalise also trims leading/trailing Unicode Cf (U+FEFF, U+200B…), interior kept; rings.rs FR-15 trim and FR-76 emptiness tests follow (needs a Cf lookup, e.g. a general-category crate). FR-17/NFR-20: the unplotted-ring warning names the ring set source (--rings / the rings key in <file> / the default ring set), so Rings must carry its source and diagnostics.rs messages + tests/diagnostics.rs change.
