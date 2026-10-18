---
id: rg-qag
title: 'Config: how a file''s legend = hidden warns for PNG and SVG'
status: open
type: task
priority: 3
tags:
- config
- cli
created: 2026-10-17
updated: 2026-10-18
phase: ''
---

# Config: how a file's legend = hidden warns for PNG and SVG

From the RG-102 UX review, for FR-91 (rg-ahj). A legend key in a TOML shared between HTML and PNG runs would warn on every PNG run, naming --legend though the user never typed it. Decide: silent when the value comes from the file, or worded by its source.

[2026-10-18] Decided (maintainer, 2026-10-18): silent when the value comes from the config file; the command-line flag still warns. Applies to every format-specific key (legend, table, legend_columns), since a shared file naturally holds keys for every format (D-96 habituation argument).
