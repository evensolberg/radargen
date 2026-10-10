---
id: rg-akv
title: Apply FR-90 to --title, --subtitle and --rings values
status: open
type: task
priority: 2
tags:
- diagnostics
- cli
created: 2026-10-09
updated: 2026-10-09
phase: ''
---

# Apply FR-90 to --title, --subtitle and --rings values

PRD v0.38 FR-90: remove undisplayable characters from flag values using escape::displayable and warn naming the flag (Position::Render-style, no line). For --rings, before FR-63's empty/duplicate checks. Needs a WarningKind shape that names a flag instead of a column.
