---
id: rg-f4o
title: Configure rustqual's SDP check for the pipeline's layering
status: open
type: task
priority: 4
tags:
- tooling
- quality
created: 2026-10-11
updated: 2026-10-11
phase: ''
---

# Configure rustqual's SDP check for the pipeline's layering

From PR 08's final review. rustqual reports SDP violations model→csv and order→model: layout depending on model and order lowers their instability below their own dependencies'. The direction is the designed pipeline (csv → model → order → layout → svg → emit, architecture §2), so these are artefacts, and each later stage (svg, emit) will add more. Configure or suppress SDP for these edges with a reason rather than letting the baseline absorb them, so a real violation stays visible.
