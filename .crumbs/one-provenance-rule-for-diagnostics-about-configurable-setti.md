---
id: rg-s71
title: One provenance rule for diagnostics about configurable settings
status: open
type: task
priority: 3
tags:
- prd
- diagnostics
- config
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# One provenance rule for diagnostics about configurable settings

From the v0.43 simplify (altitude) review. FR-17 now names the ring set source, FR-90 names the flag or --config key with the file, and NFR-10 names a --config key and file: three local statements of one idea. Consider one rule in NFR-10 or FR-91 (any diagnostic about a configurable setting names its effective source: flag, config key with file, or default), recorded per setting during cli.rs merge rather than as a ring-only field on Config.
