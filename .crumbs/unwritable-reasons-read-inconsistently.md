---
id: rg-bsb
title: Unwritable reasons read inconsistently
status: open
type: bug
priority: 3
tags:
- diagnostics
- output
created: 2026-10-12
updated: 2026-10-12
phase: ''
---

# Unwritable reasons read inconsistently

From the PR 14a final review. write_atomic flattens the temp-file failure to its io::ErrorKind ("cannot be written: not a directory") but keeps the OS text on persist ("Is a directory (os error 21)"). Pick one shape for both paths (src/emit/write.rs).
