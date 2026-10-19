---
id: rg-1bw
title: Config key problems name their line
status: open
type: feature
priority: 3
tags:
- config
- diagnostics
created: 2026-10-18
updated: 2026-10-18
phase: ''
---

# Config key problems name their line

From the PR 19a UX review (item 12), 2026-10-18: a --config key problem names the key and the file but not its line. Reading the file through toml::Spanned (or toml's de::ValueDeserializer spans) would let each KeyProblem carry a span, rendered through decode::line_at as 'the width key in radar.toml, line 3: ...', as a TOML syntax error already is. Location-first wording (UX item 13) stays the maintainer's call.
