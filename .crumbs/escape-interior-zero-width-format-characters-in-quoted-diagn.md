---
id: rg-fp5
title: Escape interior zero-width format characters in quoted diagnostic values
status: open
type: task
priority: 3
tags:
- diagnostics
- text
created: 2026-10-10
updated: 2026-10-10
blocks:
- rg-f27
phase: ''
---

# Escape interior zero-width format characters in quoted diagnostic values

From the PR 06 plan review (RG-006 rev 1.3, UX). After D-61, an invisible character INSIDE a value is the only way a visibly correct value fails to match: a cell 'Ad<U+200B>opt' prints "ring: 'Adopt' is not declared by --rings", and a header 'na<U+200B>me' gets "no 'name' column" on a header that visibly has it. escape_controls (src/diagnostics.rs) escapes bidi controls but not U+200B, U+2060, U+FEFF or U+00AD. Proposed: escape those inside quoted values (leave U+200C/U+200D, which are emoji content), e.g. "ring: 'Ad\u{200b}opt' is not declared …". Related: rg-q2b.
