---
id: rg-u5q
title: Draw the skip link over the page on every page, not only palette pages
status: open
type: task
priority: 4
tags:
- html
- a11y
created: 2026-10-16
updated: 2026-10-16
phase: ''
---

# Draw the skip link over the page on every page, not only palette pages

PR 18 adds SKIP_OVER_PAGE (.page-controls{position:relative}.skip:focus{position:absolute;z-index:1}) only when there is no toggle, because R11 kept built-in page bytes unchanged. The root cause is that the bar height depends on the toggle. Emit the rule on every page, drop chrome::css toggle parameter, re-bless prd.html, rerun browser scenarios; also drop the dead .theme-toggle rule on palette pages. From the PR 18 simplify (altitude) review.
