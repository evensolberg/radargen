---
id: rg-p9a
title: 'Palette: a swallowed unknown name is not reported as unknown'
status: open
type: bug
priority: 4
tags:
- palette
- diagnostics
created: 2026-10-17
updated: 2026-10-17
phase: ''
---

# Palette: a swallowed unknown name is not reported as unknown

A name a forgotten ; swallowed (rg-8xm) is checked for repeats (rg-4id) but not by name, so a swallowed --bogus gets no Unknown until the ; is fixed, breaking fix-in-one-pass (FR-102, NFR-16). Probe: --series-1: #000001 then --bogus: #000002; on the next line. Fix: in resolve, push Unknown for a swallowed name property() rejects. From the rg-4id review.
