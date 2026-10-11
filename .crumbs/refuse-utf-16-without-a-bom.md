---
id: rg-3ok
title: Refuse UTF-16 without a BOM
status: closed
type: task
priority: 3
tags:
- prd
- csv
created: 2026-10-10
updated: 2026-10-11
phase: ''
---

# Refuse UTF-16 without a BOM

From the PR 05 QA review (#19). UTF-16LE without a BOM is valid UTF-8 full of NULs, so it passes FR-61's check and yields a field-count error on every other line plus a huge FR-90 warning, instead of one 'not UTF-8' error. Excel always writes the BOM, so this is rare. A NUL byte in the input is a cheap tell; decide in the PRD whether FR-1/FR-61 refuse it, then add it to csv::decode.

[2026-10-11] Decided in PRD v0.46 (D-65); implementation tracked in rg-cni. Wider separator detection deferred in D-65.
