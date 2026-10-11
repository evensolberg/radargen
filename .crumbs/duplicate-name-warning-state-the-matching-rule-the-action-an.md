---
id: rg-3z2
title: 'Duplicate-name warning: state the matching rule, the action, and the first spelling'
status: closed
type: task
priority: 3
tags:
- diagnostics
created: 2026-10-10
updated: 2026-10-11
blocks:
- rg-f27
phase: ''
---

# Duplicate-name warning: state the matching rule, the action, and the first spelling

From the PR 06 plan review (RG-006 rev 1.3, UX). Per row: "line {line}, name: '{value}' repeats line {first_line}'s name (names match ignoring case and surrounding spaces); both are drawn, so rename or delete one if they are the same item". Grouped: "{n} rows repeat an earlier name — Kafka (line 2; repeated on 2 rows, first line 3); every row is drawn, so rename or delete the repeats that are the same item". Today the grouped line pairs a later row's spelling ('kafka') with the first row's line, so DuplicateName needs a first_value field. Settle before PR 14's golden tests pin the strings.

[2026-10-11] Decided in PRD v0.46 (D-64); implementation tracked in rg-fnu.
