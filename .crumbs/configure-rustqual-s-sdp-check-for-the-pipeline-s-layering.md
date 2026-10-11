---
id: rg-f4o
title: Configure rustqual's SDP check for the pipeline's layering
status: closed
type: task
priority: 4
tags:
- tooling
- quality
created: 2026-10-11
updated: 2026-10-11
blocks:
- rg-6ku
phase: ''
---

# Configure rustqual's SDP check for the pipeline's layering

From PR 08's final review. rustqual reports SDP violations model→csv and order→model: layout depending on model and order lowers their instability below their own dependencies'. The direction is the designed pipeline (csv → model → order → layout → svg → emit, architecture §2), so these are artefacts, and each later stage (svg, emit) will add more. Configure or suppress SDP for these edges with a reason rather than letting the baseline absorb them, so a real violation stays visible.

[start] 2026-10-11 11:27:12

[stop]  2026-10-11 11:36:28  9m 16s
