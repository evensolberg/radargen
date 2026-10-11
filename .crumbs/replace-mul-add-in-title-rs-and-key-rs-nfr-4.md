---
id: rg-c9x
title: Replace mul_add in title.rs and key.rs (NFR-4)
status: open
type: bug
priority: 2
tags:
- nfr-4
- layout
created: 2026-10-11
updated: 2026-10-11
phase: ''
---

# Replace mul_add in title.rs and key.rs (NFR-4)

NFR-4 (geometry.rs policy): production arithmetic is plain * and + under #[expect(clippy::suboptimal_flops, reason = "NFR-4: …")], never mul_add, which falls back to a libm fma where the CPU lacks one and rounds differently. Found by the PR 10b post-simplify review: src/layout/title.rs:72 (gaps.mul_add(self.leading, self.first_baseline)) and src/layout/key.rs:79 ((index as f64).mul_add(KEY_ROW, KEY_FIRST_ROW)) still use mul_add from PR 09. style.rs:309 is in a test closure — check whether it should match too.
