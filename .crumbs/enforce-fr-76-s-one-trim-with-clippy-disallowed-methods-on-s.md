---
id: rg-lag
title: Enforce FR-76's one trim with clippy disallowed-methods on str::trim
status: open
type: task
priority: 4
tags:
- lint
- text
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# Enforce FR-76's one trim with clippy disallowed-methods on str::trim

From the PR 06 plan review (RG-006 rev 1.3, architecture). After PR 06, src/ has no str::trim left. clippy.toml: disallowed-methods = [{ path = "str::trim", reason = "FR-76 (D-61): use radargen::trim" }] flags both .trim() and .map(str::trim) (verified on toolchain 1.99). tests/palette.rs and tests/font_assets.rs trim fixture text and need #![expect(clippy::disallowed_methods, reason = "trims fixture text, not CSV or --rings values")].
