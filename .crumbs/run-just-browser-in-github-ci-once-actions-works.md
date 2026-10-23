---
id: rg-3wv
title: Run just browser in GitHub CI once Actions works
status: open
type: task
priority: 2
tags:
- ci
- a11y
created: 2026-10-12
updated: 2026-10-22
phase: ''
---

# Run just browser in GitHub CI once Actions works

From RG-015B. just ci runs just browser locally (text-spacing, interaction), but GitHub Actions is blocked by billing. When it works, add a job that installs Playwright chromium-headless-shell and runs just browser, so the behaviour checks gate every PR.

[2026-10-17] Since rg-497, scripts/text-spacing.mjs runs in Chromium, Firefox and WebKit through Playwright (a pinned npm dev dependency). The CI job must run `just browser-setup` (npm ci, then npx playwright install chromium-headless-shell firefox webkit), not only install chromium-headless-shell; on Linux runners also `npx playwright install-deps firefox webkit` for the system libraries.

[2026-10-22] From rg-ho8 (2026-10-22): tests/pdf_matches_png.rs now gates at 2x averaged down with MAX_DELTA 44 against a faithful peak of 36, measured on macOS arm64 only. vello_cpu picks its SIMD level at runtime (NEON here; SSE/AVX2 on x86). When CI runs on Linux x86, check the faithful worst there and widen the margin if needed.

[2026-10-22] From PR 21a (2026-10-22): paper legend pages have their own oracle budget, LEGEND_PAGE_MAX_PPM 1,600 (faithful 848); A4 light page 1 reaches worst 39 against MAX_DELTA 44. Both measured on macOS arm64 only; check on Linux x86 when CI runs.
