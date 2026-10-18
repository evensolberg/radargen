---
id: rg-4bq
title: Declare rings that are tracked but not drawn (--unplotted-rings)
status: closed
type: feature
priority: 3
tags:
- rings
- cli
- diagnostics
- prd
created: 2026-10-17
updated: 2026-10-18
closed_reason: 'FR-107/D-96: --unplotted-rings ships; overlap with the ring set is an error'
phase: ''
---

# Declare rings that are tracked but not drawn (--unplotted-rings)

A CSV used as an internal register has rows in rings such as In Use or Retired that should stay off the radar. Today each run warns about them (one folded line). A blanket ignore switch would also hide typos (Adpot), so name them instead: e.g. --unplotted-rings "In Use, Retired" (and a --config key, PR 19). Rows in a named unplotted ring stay in the table with no warning; any other unknown ring still warns (FR-17). Needs a PRD requirement and decision: spelling (OQ-2), matching as FR-76, overlap with --rings is an error, NFR-10 source naming. From the README rewrite discussion, 2026-10-17.

[2026-10-17] Design shown to user 2026-10-17: --unplotted-rings list, FR-76 matching, no warning for those rows, overlap with --rings is an error, unplotted_rings config key.

[start] 2026-10-17 23:47:00

[stop]  2026-10-18 00:04:12  17m 12s
