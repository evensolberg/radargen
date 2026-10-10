---
id: rg-m33
title: Pin GitHub Actions to commit SHAs and add Dependabot
status: open
type: task
priority: 3
tags:
- ci
- security
created: 2026-10-09
updated: 2026-10-09
phase: ''
---

# Pin GitHub Actions to commit SHAs and add Dependabot

ci.yml pins actions by tag (actions/checkout@v4, Swatinem/rust-cache@v2, EmbarkStudios/cargo-deny-action@v2). Pin to full SHAs with a '# vX.Y.Z' comment and add .github/dependabot.yml for the github-actions ecosystem (and cargo, weekly) to keep them current. permissions: contents: read already limits the blast radius. From the PR 8 review.
