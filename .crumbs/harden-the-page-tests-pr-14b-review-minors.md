---
id: rg-l0x
title: Harden the page tests (PR 14b review minors)
status: closed
type: task
priority: 3
tags:
- tests
- html
created: 2026-10-12
updated: 2026-10-12
closed_reason: 'PR C1: test/harden-page-tests'
phase: ''
---

# Harden the page tests (PR 14b review minors)

From the PR 14b final review. (1) page() takes focus-rule keys from marker_keys(wide) with only a debug_assert that narrow matches; emit from the union of both, or assert, so a future narrow-only marker never loses its ring while its outline is hidden. (2) html5.rs walks elements but not attributes: assert each attribute name is in the page/serialiser vocabulary and none starts with on. (3) assert the parsed HTML <title> text equals the input title, and no SVG element other than text/tspan/title has non-whitespace text.

[start] 2026-10-12 17:01:22

[stop]  2026-10-12 17:08:27  7m 5s
