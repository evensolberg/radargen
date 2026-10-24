---
id: rg-1bw
title: Config key problems name their line
status: closed
type: feature
priority: 3
tags:
- config
- diagnostics
created: 2026-10-18
updated: 2026-10-23
closed_reason: 'feat: config key problems name their line (FR-91), 1.27.0'
phase: ''
---

# Config key problems name their line

From the PR 19a UX review (item 12), 2026-10-18: a --config key problem names the key and the file but not its line. Reading the file through toml::Spanned (or toml's de::ValueDeserializer spans) would let each KeyProblem carry a span, rendered through decode::line_at as 'the width key in radar.toml, line 3: ...', as a TOML syntax error already is. Location-first wording (UX item 13) stays the maintainer's call.

[2026-10-23] Maintainer decision 2026-10-23: append the line to the existing wording ("the width key in radar.toml, line 3: ..."), not location-first. Settles UX item 13. Plan: parse a second time with toml:🇩🇪:DeTable for key spans only; values keep toml::Table.

[start] 2026-10-23 22:19:59

[stop]  2026-10-23 22:39:42  19m 43s
