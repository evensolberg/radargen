---
id: rg-nuy
title: Theme button says what a press does
status: closed
type: task
priority: 1
tags:
- html
- a11y
created: 2026-10-15
updated: 2026-10-15
closed_reason: Toggle labelled by its action; no aria-pressed
phase: v1.0
---

# Theme button says what a press does

From rg-xe7. The toggle always reads "Dark theme"; its state is aria-pressed and a small dot, which a sighted reader cannot read. Maintainer ruling 2026-10-15: the label says what a press does, "Switch to dark theme" or "Switch to light theme" while dark shows; no aria-pressed. PRD FR-56 (0.62 specified the pressed toggle) changes; the script names block, chrome.rs markup and the dot rules, interaction.mjs scenarios, the a11y audit (toggle dot check) and the manual checklist (step 8) follow.

[start] 2026-10-15 08:10:21

[stop]  2026-10-15 08:23:41  13m 20s
