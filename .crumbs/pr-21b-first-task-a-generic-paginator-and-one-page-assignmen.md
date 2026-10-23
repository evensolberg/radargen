---
id: rg-mi2
title: 'PR 21b first task: a generic paginator and one page assignment'
status: open
type: task
priority: 2
tags:
- pdf
- layout
- refactor
created: 2026-10-22
updated: 2026-10-22
phase: v1.2
---

# PR 21b first task: a generic paginator and one page assignment

From the PR 21a final review (architecture, Important 1 and 2). flow.rs's greedy loop mixes Entry specifics into its core (Entry::bottom, Under::update, rows tracking), against plan RG-021A Task 2, so 21b cannot lift it for table rows. Extract paginate(units: &[Unit{height/bottom, has_row}], first, rest, opening_height) -> Vec<Range<usize>> on the columns::split pattern, keeping Under/opening/shift as the legend adapter. And paper.rs encodes page membership twice (opens_page closures and page_one/rest.chunks(n)): compute it once (page_of(k)) and derive both. Do this as 21b's first commit, before the table pages use it.
