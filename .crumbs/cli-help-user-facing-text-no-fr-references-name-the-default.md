---
id: rg-v14
title: 'CLI help: user-facing text, no FR references; name the default rings'
status: in_progress
type: bug
priority: 1
tags:
- cli
- docs
created: 2026-10-15
updated: 2026-10-15
phase: v1.0
---

# CLI help: user-facing text, no FR references; name the default rings

From rg-xe7 (maintainer testing, 2026-10-13). clap builds --help from the /// doc comments in src/cli.rs, which cite FRs, sections and OQs (e.g. "(§6.1) … (FR-60)", "§6.6 canonical PNG invocation … (OQ-2)", "Range per FR-64"). Move rationale to // comments; write plain help; a test bans FR-, §, OQ-, D- in --help. The about line promises PNG and SVG, which this version does not write; --theme and --width only affect PNG/SVG (1.1). --rings must name the default rings (DEFAULT_RINGS: Assess, Trial, Adopt, Retire), with a test that the help matches the constant.

[start] 2026-10-15 08:01:43
