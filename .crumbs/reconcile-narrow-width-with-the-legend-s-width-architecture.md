---
id: rg-3pp
title: Reconcile NARROW_WIDTH with the legend's width (architecture §4)
status: closed
type: idea
priority: 2
tags:
- layout
- architecture
created: 2026-10-09
updated: 2026-10-09
phase: ''
---

# Reconcile NARROW_WIDTH with the legend's width (architecture §4)

NARROW_WIDTH 420 cannot hold a LEGEND_WIDTH_MAX 440 legend plus margins; architecture §4 has one Blocks for both arrangements. Either wrap legend per arrangement (amend §4) or derive narrow width from legend. Decide before PRs 08-12.
