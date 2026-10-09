---
id: rg-i9o
title: Revisit D-19 and D-7 — rationale does not support the decision
status: closed
type: task
priority: 3
tags:
- prd
- decisions
created: 2026-08-27
updated: 2026-08-27
phase: ''
---

# Revisit D-19 and D-7 — rationale does not support the decision

Two decisions whose stated reasoning does not get you to the decision made. Raised by a cold read of PRD v0.12; neither is necessarily wrong, but §9 exists to hold reasoning that survives scrutiny.

D-19 / NFR-12 — embedded version string. NFR-12 says a version string would break diffability 'exactly as a timestamp would'. The analogy is false: a timestamp changes on every run, a version string changes only when the version changes — which NFR-12's own first half says is exactly when output is allowed to change. Embedding it costs one diff line per release and delivers NFR-12's stated purpose (a diff explained by a version rather than investigated) more directly than leaving the reader to correlate against a changelog. Either correct the analogy and re-decide, or state the real reason.

D-7 — quadrant order, 'no override flag'. The rationale (first-seen CSV order is unstable because the export step may not be deterministic) fully supports 'derive order from names'. It does not support 'and offer no override'. An explicit --quadrants would be equally stable — it is the exact mechanism D-4 adopts for rings, for the reason that meaningful order cannot be recovered by sorting names. D-7 concedes ThoughtWorks' own sequence becomes inexpressible and R-4 books a one-time reorder against the originating radar, both of which contradict its premise that quadrant order is 'genuinely meaningless'. The decision may still be right; the symmetric option is not among the rejected alternatives §9 is supposed to record.

## Resolved in PRD v0.37

- **D-19 / NFR-12:** the output version is embedded, `major.minor` only (D-52,
  SC-25). The patch is left out because a patch release does not change
  output. HTML: `<meta name="generator">`; SVG: a comment before the root;
  PNG: nothing.
- **D-7:** alphabetical stays; the rationale is corrected and `--quadrants` is
  deferred rather than rejected (OQ-17).
