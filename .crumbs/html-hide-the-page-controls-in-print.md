---
id: rg-kw7
title: 'HTML: hide the page controls in print'
status: closed
type: bug
priority: 4
tags:
- html
- print
created: 2026-10-17
updated: 2026-10-17
closed_reason: The page controls are not printed (@media print); shipped in 1.6.4
phase: ''
---

# HTML: hide the page controls in print

From the RG-102 whole-branch review. Print shows the legend, but the controls bar prints too, with the crossed-list icon above a printed legend when the page was hidden, and the theme toggle. @media print{.page-controls{display:none}}.

[start] 2026-10-17 17:01:30

[stop]  2026-10-17 17:06:08  4m 38s
