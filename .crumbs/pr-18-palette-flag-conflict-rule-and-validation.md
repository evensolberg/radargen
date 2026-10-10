---
id: rg-cdo
title: 'PR 18: --palette flag, conflict rule and validation'
status: blocked
type: feature
priority: 2
tags:
- impl
- palette
- cli
created: 2026-10-08
updated: 2026-10-09
blocked_by:
- rg-42m
phase: v1.1
---

# PR 18: --palette flag, conflict rule and validation

FR-59 conflicts_with, FR-86/FR-87 file validation; tier-one failure while building Config. SC-21. Completes v1.1.

[2026-10-09] D-54: Palette::number_ink computes WCAG contrast from #RRGGBB, so --palette values must be #RRGGBB (reject other colour syntax as a hard error, or normalise it).
