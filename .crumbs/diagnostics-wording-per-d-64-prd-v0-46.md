---
id: rg-fnu
title: Diagnostics wording per D-64 (PRD v0.46)
status: closed
type: task
priority: 2
tags:
- diagnostics
created: 2026-10-11
updated: 2026-10-11
blocks:
- rg-f27
phase: ''
---

# Diagnostics wording per D-64 (PRD v0.46)

Implements D-64, replacing rg-85c, rg-3z2, rg-bvl, rg-fju and rg-fp5. Before PR 14's golden tests pin the strings:
- 'value' and 'column', never 'field' (FieldCount, grouped Undisplayable).
- Column by position as 'column 6 (F)': FR-93's warning, UnterminatedQuote, TextAfterQuote, DuplicateHeader.
- FR-28: name the earlier line and its spelling, the matching rule, both drawn; DuplicateName needs a first_value field; grouped form lists the first spelling.
- FR-17 default source: 'not one of the default rings (Assess, Trial, Adopt, Retire)', list the declared rings, say how to plot the row.
- Escape interior Cf chars in quoted values (escape_controls), except U+200C, U+200D and tag chars.

[start] 2026-10-11 11:55:55

[stop]  2026-10-11 12:08:12  12m 17s
