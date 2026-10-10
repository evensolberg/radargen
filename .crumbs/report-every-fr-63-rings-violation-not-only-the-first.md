---
id: rg-tiu
title: Report every FR-63 --rings violation, not only the first
status: closed
type: bug
priority: 2
tags:
- rings
- diagnostics
created: 2026-10-09
updated: 2026-10-10
blocked_by:
- rg-xr3
phase: v1.0
---

# Report every FR-63 --rings violation, not only the first

NFR-16 tier one: "Within a single flag, every violation is reported together, as FR-63 already requires." parse_rings returns on the first violation, so "Adopt,,Trial,adopt" reports only the empty entry. Return every violation and report them together. Found while planning PR 02; depends on PR 02 (rg-xr3) for the Fatal taxonomy it reports through.

[start] 2026-10-10 10:59:45
