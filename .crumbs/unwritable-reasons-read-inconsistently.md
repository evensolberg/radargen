---
id: rg-bsb
title: Unwritable reasons read inconsistently
status: closed
type: bug
priority: 3
tags:
- diagnostics
- output
created: 2026-10-12
updated: 2026-10-12
closed_reason: 'PR A: every read/write reason renders the OS text without (os error N), lowercase'
phase: ''
---

# Unwritable reasons read inconsistently

From the PR 14a final review. write_atomic flattens the temp-file failure to its io::ErrorKind ("cannot be written: not a directory") but keeps the OS text on persist ("Is a directory (os error 21)"). Pick one shape for both paths (src/emit/write.rs).
