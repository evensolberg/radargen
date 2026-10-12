---
id: rg-nyb
title: FR-95 message interpolates its viewport and target constants
status: closed
type: task
priority: 4
tags:
- diagnostics
created: 2026-10-12
updated: 2026-10-12
closed_reason: 'PR A: FR-95 interpolates NARROW_MIN_VIEWPORT_PX and MIN_TARGET_PX'
phase: ''
---

# FR-95 message interpolates its viewport and target constants

From the PR 14a final review. The FR-95 message interpolates NARROW_MAX_WIDTH but hard-codes "320-pixel" and "24 pixels"; style.rs has NARROW_MIN_VIEWPORT_PX and MIN_TARGET_PX. Interpolate them so the text cannot drift (src/diagnostics.rs NarrowWidened).
