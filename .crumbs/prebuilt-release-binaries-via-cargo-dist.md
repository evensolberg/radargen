---
id: rg-g4v
title: Prebuilt release binaries via cargo-dist
status: closed
type: feature
priority: 3
tags:
- release
- distribution
created: 2026-10-24
updated: 2026-10-24
phase: ''
---

# Prebuilt release binaries via cargo-dist

Attach prebuilt binaries to GitHub releases with cargo-dist (dist init), so `cargo binstall radargen` installs without compiling. Blocked until GitHub Actions credits return (after 2026-11-01): check `gh run list` starts jobs first. Targets: macOS ARM/x86, Linux x86_64/ARM, Windows x86_64. Then mention `cargo binstall` in the README install section.
