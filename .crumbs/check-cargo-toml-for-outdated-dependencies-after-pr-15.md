---
id: rg-9ay
title: Check Cargo.toml for outdated dependencies (after PR 15)
status: closed
type: task
priority: 2
tags:
- deps
created: 2026-10-12
updated: 2026-10-13
closed_reason: 'Dependency check 2026-10-13: serde_json 1.0.154, sha2 0.11.1 and clap_lex 1.1.1 applied; harfrust 0.14 held for usvg (D-67) and html5ever 0.40 held for markup5ever_rcdom, tracked in a new crumb.'
blocked_by:
- rg-6zu
phase: ''
---

# Check Cargo.toml for outdated dependencies (after PR 15)

Maintainer request, 2026-10-12: once PR 15 (15b-2, 15c) is done, check every dependency in Cargo.toml for a newer release (cargo outdated / cargo upgrade --dry-run), weigh each bump, and update in its own PR.

[start] 2026-10-13 00:22:48

[stop]  2026-10-13 00:27:21  4m 33s
