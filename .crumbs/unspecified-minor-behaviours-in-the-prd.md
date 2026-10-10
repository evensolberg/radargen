---
id: rg-dsa
title: Unspecified minor behaviours in the PRD
status: open
type: task
priority: 3
tags:
- prd
- spec
created: 2026-08-27
updated: 2026-10-09
phase: ''
---

# Unspecified minor behaviours in the PRD

Small gaps found by the v0.13 audits, none worth an open question on its own.

- FR-65: what does --title "" do, versus omitting the flag? And --title "   "? FR-76 scopes itself to CSV and --rings values, so title/subtitle fall outside the trim rule entirely.
- FR-24: if an unparsable date renders verbatim and a parsable one also renders verbatim, the parse exists only to decide whether to warn. Say so — an implementer reading FR-24 alone may infer that parsable dates get reformatted, and 'to what format' is then a locale question NFR-4 forbids. Also: after FR-76 trims for parsing, does ' 2026-06-01' display with the leading space?
- FR-56/FR-57 are described as accepted-and-ignored on HTML in v1.0, but §14.1's v1.0 Contains cell names --theme's HTML behaviour and not --width's.
- Repo size: UC-1 commits every edition, NFR-5 inlines the font into each HTML file, and FR-69 embeds both arrangements. Each dated edition therefore carries a fresh font blob plus roughly double the graphic markup. Not a blocker; nobody has said it out loud.
- The decision log's '*Added on this review*' markers now span seven reviews and no longer identify which one.

[2026-10-09] PRD v0.39 FR-65 settles the first point: an empty or whitespace-only --title/--subtitle counts as omitted (and on the command line suppresses a --config value).
