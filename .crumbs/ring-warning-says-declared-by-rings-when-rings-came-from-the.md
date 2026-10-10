---
id: rg-dy8
title: Ring warning says 'declared by --rings' when rings came from the default or --config
status: open
type: task
priority: 3
tags:
- prd
- diagnostics
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# Ring warning says 'declared by --rings' when rings came from the default or --config

From PR 11 review. FR-17's per-row message and NFR-20's PRD-fixed grouped line both say '--rings'. When the ring set is FR-13's default or a --config rings array (FR-91), naming the flag misleads. Decide wording in the PRD (e.g. 'not a declared ring') and update NFR-20's example with it.
