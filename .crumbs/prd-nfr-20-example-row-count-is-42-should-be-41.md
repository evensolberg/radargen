---
id: rg-jtg
title: PRD NFR-20 example row count is 42, should be 41
status: open
type: bug
priority: 2
tags:
- prd
- docs
created: 2026-10-09
updated: 2026-10-09
phase: ''
---

# PRD NFR-20 example row count is 42, should be 41

NFR-20's example says '42 rows ... In Use (40 ...), Adpot (1 ...)' but 40+1=41. PR 02's SC-24 test correctly asserts 41.
