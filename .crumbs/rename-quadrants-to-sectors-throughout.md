---
id: rg-k16
title: Rename quadrants to sectors throughout
status: in_progress
type: feature
priority: 2
tags:
- terminology
- csv
- docs
- prd
created: 2026-10-18
updated: 2026-10-22
phase: ''
---

# Rename quadrants to sectors throughout

Maintainer, 2026-10-18: a quadrant is a 90-degree quarter circle, but radargen allows any number of them, so the word is wrong. Decided: (1) CSV column becomes `sector`; `quadrant` stays accepted as an alias so existing and ThoughtWorks BYOR CSVs keep working; both in one file is an error; docs, template and --create-template use `sector`. (2) Rename everything: messages, help, HTML text read aloud, README, docs, PRD, and code identifiers and tests (~2,250 occurrences; mechanical). (3) Do it right after PR 19a merges (19a rewrites many diagnostics), before PR 19b. Needs a PRD decision recording the rename and the alias; minor bump (the alias keeps old input working; text output changes).

[2026-10-18] Rationale for the PRD decision (maintainer, 2026-10-18): ThoughtWorks calls them quadrants because their radar always has exactly four; radargen allows one to eight (FR-88 warns above six), so "sector" is the accurate word. The `quadrant` alias keeps BYOR-shaped CSVs working.

[2026-10-18] Approach (maintainer, 2026-10-18): no review panel. Split into (a) code + output + the quadrant alias, (b) current-tense docs (README, csv-format, live PRD requirements, architecture); historical text (revision rows, decisions, spikes, plans) keeps "quadrant". Verification instead of review: (1) reverse the substitution on re-blessed goldens and pinned strings and diff against main, which must be empty; (2) TDD for the alias (quadrant column works, sector works, both is an error), keeping some fixtures on `quadrant`; (3) full just ci (script names block + browser checks catch class/attribute mismatches).

[2026-10-22] Decided (maintainer, 2026-10-22): the HTML table and detail panel heading for this column is always "Sector", even when the CSV header says quadrant/Quadrant: an exception to FR-27 for this one column, so the page uses one word throughout. Order: rg-k16 before rg-2ss.

[start] 2026-10-22 08:29:33
