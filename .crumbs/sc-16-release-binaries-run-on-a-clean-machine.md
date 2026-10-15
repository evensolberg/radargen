---
id: rg-xpn
title: 'SC-16: release binaries, run on a clean machine'
status: open
type: task
priority: 2
tags:
- ci
- release
created: 2026-10-15
updated: 2026-10-15
phase: v1.0.x
---

# SC-16: release binaries, run on a clean machine

Maintainer ruling 2026-10-15: moved to 1.0.x (PRD D-84). Build release binaries for macOS (arm64), Linux (x86-64) and Windows in CI, attach them to the GitHub release, and run one on a clean machine (architecture 10). Needs GitHub Actions (rg-3wv). Then update the README's Install section to lead with the binaries.
