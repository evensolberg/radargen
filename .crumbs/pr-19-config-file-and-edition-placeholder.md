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
updated: 2026-10-18
phase: v1.1
---

# PR 19: --config file and --edition placeholder

PRD v0.39 FR-91/FR-92, D-57/D-58, SC-26/SC-27, v1.1. TOML config (serde + toml crates) merged under command-line flags before validation; keys output/title/subtitle/rings/theme/width/palette; unknown/wrong-typed keys fatal; relative paths resolve against the config file; FR-59 across sources. -e/--edition YYYY-MM-DD expands {edition} in output/title/subtitle; missing --edition, unknown placeholder, invalid date are hard errors. Never read the clock.

[2026-10-09] Review clarifications in v0.39: config values re-checked post-merge with clap's wording (FR-56/59/64); rings array entries trimmed, [] = FR-63, comma in entry fatal; diagnostics name key+file, all reported together; empty --title/--subtitle suppress; placeholder regex \{[A-Za-z][A-Za-z0-9_]*\}, unknown fatal only when in use; unused --edition warns; order FR-90 removal -> {edition} -> path resolution -> checks; no ~ expansion.

[2026-10-10] From PR 06 (#25): RingSource needs a variant for --config's rings key, naming the file as given (FR-17, FR-91). Hold the path as Arc<str>. RingSource and diagnostics::Class then become Clone rather than Copy, and RowKind::class stops being const fn. Build the config's RingSet through parse_rings' validation (FR-63: non-empty, no duplicates under FR-76), not a struct literal; if a second constructor appears, make RingSet's fields private behind accessors.

[2026-10-11] From PRD v0.46 NFR-10 (D-64, was rg-s71): any diagnostic about a configurable setting names its effective source — flag, --config key with file, or default. Record the source per setting during the cli.rs merge rather than as a ring-only field.

[2026-10-18] rg-4bq (FR-107, D-96) adds an unplotted_rings key to FR-91, spec only. PR 19 must (1) check FR-107's overlap after the merge, across sources, naming both sides by effective source: the entry as --unplotted-rings or the unplotted_rings key with its file, the ring set as --rings, the rings key with its file, or the default set (FR-107's examples); (2) keep the accepted gap: a file's unplotted_rings cannot be cleared from the command line, since --unplotted-rings "" is FR-107's 'no rings named' error.
