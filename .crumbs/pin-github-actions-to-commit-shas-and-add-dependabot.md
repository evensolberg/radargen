---
id: rg-m33
title: Pin GitHub Actions to commit SHAs and add Dependabot
status: closed
type: task
priority: 3
tags:
- ci
- security
created: 2026-10-09
updated: 2026-10-11
phase: ''
---

# Pin GitHub Actions to commit SHAs and add Dependabot

ci.yml pins actions by tag (actions/checkout@v4, Swatinem/rust-cache@v2, EmbarkStudios/cargo-deny-action@v2). Pin to full SHAs with a '# vX.Y.Z' comment and add .github/dependabot.yml for the github-actions ecosystem (and cargo, weekly) to keep them current. permissions: contents: read already limits the blast radius. From the PR 8 review.

[start] 2026-10-11 09:07:24

[2026-10-11] Done: ci.yml pins actions/checkout v4.4.0, Swatinem/rust-cache v2.9.2 and EmbarkStudios/cargo-deny-action v2.1.1 by SHA, on their current majors so behaviour is unchanged; .github/dependabot.yml updates github-actions and cargo weekly, minor/patch grouped, majors separate. Dependabot's first PR should propose checkout v7.

[stop]  2026-10-11 09:07:24  0s
