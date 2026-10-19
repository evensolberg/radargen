---
id: rg-37u
title: Hint when a config read through /dev/fd resolves paths there
status: open
type: task
priority: 4
tags:
- config
- diagnostics
created: 2026-10-19
updated: 2026-10-19
phase: ''
---

# Hint when a config read through /dev/fd resolves paths there

From the PR 19a final review (M-8), 2026-10-19: radargen in.csv -c <(printf 'output = "r.html"') prints 'the output key in /dev/fd/63 (r.html): cannot be written: entity not found'. That is correct under FR-91 (relative paths resolve against the file's folder), but the message does not say why. When the config's folder is /dev/fd (or /proc/self/fd), a relative path's error could add a hint: a config read through a process substitution resolves relative paths under /dev/fd, so give absolute paths. settings::file::read's doc invites process substitution.
