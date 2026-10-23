---
id: rg-6wj
title: 'Tagged PDF: row header verbosity fallback'
status: open
type: task
priority: 2
tags:
- pdf
- a11y
- table
created: 2026-10-23
updated: 2026-10-23
phase: v1.23.x
---

# Tagged PDF: row header verbosity fallback

From the 21c-2 review (UX 8, a11y residual): the full PDF table row header is the name cell, which also holds the folded line (domain, qualifier, status, dates) and the flag, so NVDA + Acrobat announce about 10-12 words on every vertical move. manual-checks step 20 measures it. If too verbose: make the number cell the TH /Scope /Row (it reads only "1"), or drop the row scope. Decide from the step-20 run.
