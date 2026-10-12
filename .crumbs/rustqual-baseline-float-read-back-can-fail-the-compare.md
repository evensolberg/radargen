---
id: rg-ksg
title: rustqual baseline float read-back can fail the compare
status: open
type: bug
priority: 2
tags:
- tooling
- rustqual
created: 2026-10-11
updated: 2026-10-11
phase: ''
---

# rustqual baseline float read-back can fail the compare

rustqual 1.8.3 compares summary.quality_score - baseline quality_score < 0.0 (src/adapters/report/baseline.rs:144); its JSON reader can return a stored value one ulp above the computed one (0.9108712900382011 did), so just ci fails on an unchanged score and just quality-baseline cannot re-record (it runs the failing compare first). Workaround used in PR 12b: shorten the stored score by one digit. Fix options: make quality-baseline save without the compare when the score is unchanged, compare with a tolerance in a wrapper, or report upstream.
