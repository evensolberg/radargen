---
id: rg-6ny
title: 'Table rows: native <tr> with a button in the name cell, not role=button'
status: open
type: task
priority: 2
tags:
- a11y
- prd
- html
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# Table rows: native <tr> with a button in the name cell, not role=button

From the PR 06 plan review (RG-006). FR-50 puts role=button with an aria-label on table rows. On a <tr> that removes the row role (no cell-by-cell navigation or column headers), makes the cells presentational, and the aria-label replaces them, so FR-78's optional cells become unreachable from the view FR-41 calls the non-visual fallback. FR-50's argument against <button> holds only inside the SVG. Amend FR-50/FR-78: keep <tr>/<td>, put a <button> in the name cell named by the visible name (plus the visible unplotted flag). Affects PR 14/15.
