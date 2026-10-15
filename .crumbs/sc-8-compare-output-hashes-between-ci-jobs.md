---
id: rg-isw
title: 'SC-8: compare output hashes between CI jobs'
status: open
type: task
priority: 2
tags:
- ci
- determinism
created: 2026-10-12
updated: 2026-10-15
phase: v1.0.x
---

# SC-8: compare output hashes between CI jobs

Architecture 10: each OS job publishes its HTML hash; a final job compares them across macOS, Windows, Linux. tests/determinism.rs (PR 14b) covers only the cross-process half on one machine. Release-gating for PR 15.

[2026-10-15] Maintainer ruling 2026-10-15: moved to 1.0.x (PRD D-82). Lands once GitHub Actions runs (rg-3wv).
