---
id: rg-85c
title: Say 'value' and 'column', not 'field', in existing messages
status: open
type: task
priority: 3
tags:
- diagnostics
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# Say 'value' and 'column', not 'field', in existing messages

From the PR 06 plan review. FieldCount ('the row's field count is 3 but the header's is 4') and the grouped Undisplayable line ('N fields had characters…') still say 'field', against the house rule. Proposed: 'line 4: the row has 3 values but the header has 4 columns; …' and 'N values had characters…'. Golden tests do not pin them yet (SC-10 lands in PR 13).
