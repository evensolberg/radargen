---
id: rg-aqb
title: 'PR 05: csv/decode and csv/parse'
status: closed
type: task
priority: 2
tags:
- impl
- csv
created: 2026-10-08
updated: 2026-10-10
closed_reason: 'PR #19: csv/decode and csv/parse'
blocks:
- rg-rnc
blocked_by:
- rg-xr3
phase: v1.0
---

# PR 05: csv/decode and csv/parse

FR-1–FR-4, FR-61: BOM, UTF-8, physical line numbering (NFR-10, D-37), header, field counts.

[2026-10-09] FR-90 (PRD v0.38, D-55): remove characters XML cannot carry (escape::xml_allowed from PR 03) and push one WarningKind::XmlForbidden per affected field (PR 02).

[2026-10-09] Renamed in PR 6: escape::xml_allowed is now escape::displayable, and WarningKind::XmlForbidden is now WarningKind::Undisplayable.

[start] 2026-10-10 16:17:17

[stop]  2026-10-10 16:47:41  30m 24s
