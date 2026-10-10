---
id: rg-u6u
title: 'PR 19: --config file and --edition placeholder'
status: open
type: feature
priority: 2
tags:
- impl
- cli
- config
created: 2026-10-09
updated: 2026-10-09
phase: v1.1
---

# PR 19: --config file and --edition placeholder

PRD v0.39 FR-91/FR-92, D-57/D-58, SC-26/SC-27, v1.1. TOML config (serde + toml crates) merged under command-line flags before validation; keys output/title/subtitle/rings/theme/width/palette; unknown/wrong-typed keys fatal; relative paths resolve against the config file; FR-59 across sources. -e/--edition YYYY-MM-DD expands {edition} in output/title/subtitle; missing --edition, unknown placeholder, invalid date are hard errors. Never read the clock.

[2026-10-09] Review clarifications in v0.39: config values re-checked post-merge with clap's wording (FR-56/59/64); rings array entries trimmed, [] = FR-63, comma in entry fatal; diagnostics name key+file, all reported together; empty --title/--subtitle suppress; placeholder regex \{[A-Za-z][A-Za-z0-9_]*\}, unknown fatal only when in use; unused --edition warns; order FR-90 removal -> {edition} -> path resolution -> checks; no ~ expansion.
