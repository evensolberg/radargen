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
updated: 2026-10-17
phase: ''
---

# Run just browser in GitHub CI once Actions works

From RG-015B. just ci runs just browser locally (text-spacing, interaction), but GitHub Actions is blocked by billing. When it works, add a job that installs Playwright chromium-headless-shell and runs just browser, so the behaviour checks gate every PR.

[2026-10-17] Since rg-497, scripts/text-spacing.mjs runs in Chromium, Firefox and WebKit through Playwright (a pinned npm dev dependency). The CI job must run `just browser-setup` (npm ci, then npx playwright install chromium-headless-shell firefox webkit), not only install chromium-headless-shell; on Linux runners also `npx playwright install-deps firefox webkit` for the system libraries.
