---
id: rg-rq3
title: Make scripts/subset-inter.sh reproducible beyond fonttools
status: open
type: task
priority: 3
tags:
- font
- tooling
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# Make scripts/subset-inter.sh reproducible beyond fonttools

From PR 01 (#10) review. Only fonttools is pinned: brotli (WOFF2 bytes) and the Python uvx picks are not, so a rerun could change WOFF2 bytes for no real reason. The Inter-4.1 source is not verified either; check the zip or both TTFs against a recorded SHA-256 before subsetting. shasum is Perl and missing on some minimal Linux; fall back to sha256sum (same output format).
