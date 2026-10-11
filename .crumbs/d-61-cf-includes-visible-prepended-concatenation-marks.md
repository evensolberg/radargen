---
id: rg-q2b
title: 'D-61: Cf includes visible prepended concatenation marks'
status: closed
type: idea
priority: 4
tags:
- prd
- text
created: 2026-10-10
updated: 2026-10-11
phase: ''
---

# D-61: Cf includes visible prepended concatenation marks

From the PR 06 plan review. General category Cf also holds visible signs written before what they mark: U+0600–U+0605, U+06DD, U+070F, U+0890–U+0891, U+08E2, U+110BD, U+110CD. FR-76's trim strips a leading one from comparisons and from FR-9/FR-15 labels. Decide whether to exempt Grapheme_Cluster_Break=Prepend as the tag characters are exempted.

[2026-10-11] Folded into rg-kzl (right-to-left, CJK and Indic support), out of scope by A-4 and D-68.
