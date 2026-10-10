---
id: rg-q7h
title: Separator detection beyond a one-column header
status: open
type: task
priority: 3
tags:
- prd
- csv
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# Separator detection beyond a one-column header

From the PR 05 reviews (#19). FR-1/D-59 refuse a non-comma separator only when the header is one column. Gaps: (1) a ; export whose header cell holds a comma, which Excel leaves unquoted there: name;Beschreibung, lang gives a two-column header and a field-count error on every row; (2) "name";,ring is told 'the header is one column holding semicolons' though it has three; (3) "name" ;ring (space before ;) gets TextAfterQuote. Decide in the PRD whether detection widens (e.g. first header cell holds ; and the separator count is consistent), then update FR-1, D-59 and csv::parse::check_separator/header.
