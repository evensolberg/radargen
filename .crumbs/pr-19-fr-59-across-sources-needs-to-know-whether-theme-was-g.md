---
id: rg-ahj
title: 'PR 19: FR-59 across sources needs to know whether --theme was given'
status: closed
type: task
priority: 2
tags:
- config
- cli
- palette
created: 2026-10-16
updated: 2026-10-18
closed_reason: 'Done in PR 19a: FR-59 across sources'
phase: ''
---

# PR 19: FR-59 across sources needs to know whether --theme was given

Cli::theme has default_value_t = Light, so after a --config merge a given --theme cannot be told from the default. FR-59 applies across sources (a palette key with --theme, or theme key with --palette). Use ArgMatches::value_source or make theme Option<Theme> before building on Colours::BuiltIn(cli.theme). From the PR 18 plan architecture review (RG-018A, R19).

[2026-10-18] Done in PR 19a (RG-019A): Option fields on every overridable flag, MergeProblem::Conflict for FR-59 across sources.
