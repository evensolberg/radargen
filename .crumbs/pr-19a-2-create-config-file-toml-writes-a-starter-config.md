---
id: rg-2ss
title: 'PR 19a-2: --create-config <file.toml> writes a starter config'
status: open
type: feature
priority: 2
tags:
- config
- cli
- starter
created: 2026-10-18
updated: 2026-10-22
phase: ''
---

# PR 19a-2: --create-config <file.toml> writes a starter config

Maintainer, 2026-10-18: alongside --create-template (FR-99) and --create-palette (FR-101), add --create-config <file.toml>. It writes a starter radar.toml with all 11 FR-91 keys commented out, each with its default and a one-line explanation, so the file changes nothing until a line is uncommented. It refuses to overwrite an existing file, does not combine with a render, and follows FR-99/FR-101 (src/starter.rs). Needs a PRD FR and decision; a minor bump. Do it after PR 19a merges, before the quadrant-to-sector rename (rg-k16).

Default subtitle: "Created {edition}" to show off the edition functionality.

[2026-10-22] Update (2026-10-22): PR 19b adds an `input` key too, so the starter lists 13 keys, commented out, including `# input = "radar.csv"`. With input, output, edition = "today" and a {edition} subtitle uncommented, `radargen -c radar.toml` runs a scheduled render with no other arguments.
