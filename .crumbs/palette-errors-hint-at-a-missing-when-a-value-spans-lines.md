---
id: rg-8xm
title: 'Palette errors: hint at a missing ; when a value spans lines'
status: closed
type: bug
priority: 4
tags:
- palette
- diagnostics
created: 2026-10-16
updated: 2026-10-17
closed_reason: Hint at a forgotten ;, no knock-on missing/gap errors, NBSP shown escaped (1.3.2)
phase: ''
---

# Palette errors: hint at a missing ; when a value spans lines

A forgotten ; gives "--surface-1: #FFFFFF… is not a #RRGGBB colour" then "--surface-2 is missing". When a NotHex value holds a line break, add "is a ; missing?" and consider suppressing the knock-on missing role. Also: NBSP before a value quotes as if valid. From the PR 18 local review.

[start] 2026-10-17 08:58:46

[stop]  2026-10-17 09:28:50  30m 4s
