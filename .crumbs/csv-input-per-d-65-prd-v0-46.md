---
id: rg-cni
title: CSV input per D-65 (PRD v0.46)
status: closed
type: task
priority: 3
tags:
- csv
created: 2026-10-11
updated: 2026-10-11
blocks:
- rg-f27
phase: ''
---

# CSV input per D-65 (PRD v0.46)

Implements D-65, replacing rg-3ok, rg-ei7 and the message half of rg-q7h.
- FR-61: refuse a NUL byte as not UTF-8 text, naming its line (csv::decode).
- FR-3: the blank-record test counts whitespace FR-90 does not remove plus FR-76's format characters, so a BOM-only line above the header is skipped.
- FR-1: a ';' or tab right after a header field's closing quote, spaces aside, is NotCommaSeparated, not TextAfterQuote; the message states no column count it did not see.
Wider separator detection was deferred in D-65.

[start] 2026-10-11 12:25:53

[stop]  2026-10-11 12:32:38  6m 45s
