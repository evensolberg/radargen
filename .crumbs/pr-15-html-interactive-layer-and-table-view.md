---
id: rg-6zu
title: 'PR 15: HTML interactive layer and table view'
status: blocked
type: feature
priority: 1
tags:
- impl
- html
- a11y
created: 2026-10-08
updated: 2026-10-11
blocked_by:
- rg-f27
phase: v1.0
---

# PR 15: HTML interactive layer and table view

FR-38 detail panel, FR-72 cross-highlight, FR-49 keyboard and focus return keyed on the item, FR-41/FR-78 table of every CSV row. Not a straight port of mockup.html: fix FR-50 labels and NFR-6 escaping. Completes v1.0.

[2026-10-11] From PRD v0.46 (D-66): table rows stay native <tr>/<td> with a native <button> in the name cell, named by the visible name plus the visible 'not on the radar' flag (was rg-6ny). Every control carries aria-expanded (true on the one whose panel is open) and aria-controls naming the panel; state stays out of the name (was rg-1uw). The table flag reads 'not on the radar' (was rg-owp; update mockup.html too). Draw focus in a top layer above every marker (was rg-r48). SC-17 now checks 2.5.8 and 4.1.2.
