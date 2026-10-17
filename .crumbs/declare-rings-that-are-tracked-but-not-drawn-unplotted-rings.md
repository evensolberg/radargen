---
id: rg-4bq
title: Declare rings that are tracked but not drawn (--unplotted-rings)
status: open
type: feature
priority: 3
tags:
- rings
- cli
- diagnostics
- prd
created: 2026-10-17
updated: 2026-10-17
phase: ''
---

# Declare rings that are tracked but not drawn (--unplotted-rings)

A CSV used as an internal register has rows in rings such as In Use or Retired that should stay off the radar. Today each run warns about them (one folded line). A blanket ignore switch would also hide typos (Adpot), so name them instead: e.g. --unplotted-rings "In Use, Retired" (and a --config key, PR 19). Rows in a named unplotted ring stay in the table with no warning; any other unknown ring still warns (FR-17). Needs a PRD requirement and decision: spelling (OQ-2), matching as FR-76, overlap with --rings is an error, NFR-10 source naming. From the README rewrite discussion, 2026-10-17.
