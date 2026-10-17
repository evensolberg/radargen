---
id: rg-4id
title: 'Palette: a swallowed name repeated later is not reported as a duplicate'
status: in_progress
type: bug
priority: 4
tags:
- palette
- diagnostics
created: 2026-10-17
updated: 2026-10-17
phase: ''
---

# Palette: a swallowed name repeated later is not reported as a duplicate

A declaration swallowed by a forgotten ; (rg-8xm) never enters seen, so a later declaration of the same name is not reported as Duplicate until the ; is fixed, breaking fix-in-one-pass (FR-102, NFR-16). Probe: --series-2: #000002 then --series-1: #000003; after a valid --series-1. Fix: have swallowed_names return each name line offset and insert into seen. Pre-dates rg-8xm. From the rg-8xm second-seat review.

[start] 2026-10-17 10:28:19
