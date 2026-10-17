---
id: rg-8xm
title: 'Palette errors: hint at a missing ; when a value spans lines'
status: open
type: idea
priority: 4
tags:
- palette
- diagnostics
created: 2026-10-16
updated: 2026-10-16
phase: ''
---

# Palette errors: hint at a missing ; when a value spans lines

A forgotten ; gives "--surface-1: #FFFFFF… is not a #RRGGBB colour" then "--surface-2 is missing". When a NotHex value holds a line break, add "is a ; missing?" and consider suppressing the knock-on missing role. Also: NBSP before a value quotes as if valid. From the PR 18 local review.
