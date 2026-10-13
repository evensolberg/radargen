---
id: rg-9ay
title: Check Cargo.toml for outdated dependencies (after PR 15)
status: blocked
type: task
priority: 2
tags:
- deps
created: 2026-10-12
updated: 2026-10-12
blocked_by:
- rg-6zu
phase: ''
---

# Check Cargo.toml for outdated dependencies (after PR 15)

Maintainer request, 2026-10-12: once PR 15 (15b-2, 15c) is done, check every dependency in Cargo.toml for a newer release (cargo outdated / cargo upgrade --dry-run), weigh each bump, and update in its own PR.
