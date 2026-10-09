---
id: rg-f27
title: 'PR 14: HTML output — both arrangements, theming, inlined font'
status: blocked
type: feature
priority: 1
tags:
- impl
- html
- output
created: 2026-10-08
updated: 2026-10-08
blocks:
- rg-6zu
blocked_by:
- rg-rnc
- rg-6ku
phase: v1.0
---

# PR 14: HTML output — both arrangements, theming, inlined font

Wire the pipeline end to end in main.rs. FR-69 media-query switch, FR-56 dual palette + prefers-color-scheme + [data-theme] override, inlined WOFF2 (NFR-5). First golden fixtures and the cross-process determinism test. First user-visible change.
