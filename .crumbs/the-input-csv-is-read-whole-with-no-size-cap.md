---
id: rg-w5k
title: The input CSV is read whole, with no size cap
status: open
type: bug
priority: 3
tags:
- csv
- security
created: 2026-10-16
updated: 2026-10-16
phase: ''
---

# The input CSV is read whole, with no size cap

csv::decode::read uses fs::read, so radargen /dev/zero -o x.html (or a multi-GB file) exhausts memory before any error. PR 18 capped the --palette read at 1 MiB with take() (plan RG-018A, R4); the CSV needs a cap too, sized for a large radar (tens of MB?), refused as a runtime failure naming the path. From the PR 18 plan security review.
