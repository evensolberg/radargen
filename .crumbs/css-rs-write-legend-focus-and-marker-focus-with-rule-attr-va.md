---
id: rg-lv5
title: 'css.rs: write legend_focus and marker_focus with rule/attr/var'
status: closed
type: task
priority: 4
tags:
- html
- refactor
created: 2026-10-12
updated: 2026-10-16
closed_reason: legend_focus and marker_focus use rule/attr/var; golden unchanged
phase: ''
---

# css.rs: write legend_focus and marker_focus with rule/attr/var

From the PR 15b-2 simplify seat. legend_focus and marker_focus still build rules from fragment constants (OUTLINE, ROW_SHOWN, VISIBLE, ROW_RING*, SCRIPTED, HIDDEN, BOTH, INSIDE, SHOWN_LAST) while the new state rules use the rule(), attr() and var() helpers. Move them over; the HTML golden confirms the bytes stay the same.
