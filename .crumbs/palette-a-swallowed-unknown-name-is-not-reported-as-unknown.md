---
id: rg-p9a
title: 'Palette: check swallowed declarations as declarations (Unknown, NotHex)'
status: closed
type: bug
priority: 4
tags:
- palette
- diagnostics
created: 2026-10-17
updated: 2026-10-17
closed_reason: 'A forgotten ; is its own error and the next --name: line is checked in the same pass; linear time held by tests; shipped in 1.6.3'
phase: ''
---

# Palette: check swallowed declarations as declarations (Unknown, NotHex)

A name a forgotten ; swallowed (rg-8xm) is checked for repeats (rg-4id) but not by name, so a swallowed --bogus gets no Unknown until the ; is fixed, breaking fix-in-one-pass (FR-102, NFR-16). Probe: --series-1: #000001 then --bogus: #000002; on the next line. Fix: in resolve, push Unknown for a swallowed name property() rejects. From the rg-4id review.

[2026-10-17] From the rg-4id /simplify altitude review: the general fix is in declarations(), not resolve(). End a declaration at each later line of its value that starts --name:, and resume splitting there, so swallowed declarations become real Declarations and get Unknown, NotHex, Duplicate and series checks in one pass; resolve() then needs no swallowed handling (drop Declaration::swallowed, Lines::starting_in, swallowed_name). Cost: the swallowing value can then be valid hex, so add ; needs its own error rather than NotHex.runs_on; rg-8xm tests and docs/palette.md example change. Same gap for a swallowed --series-2: red (no NotHex).

[start] 2026-10-17 16:37:12

[stop]  2026-10-17 16:56:52  19m 40s
