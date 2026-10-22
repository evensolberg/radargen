---
id: rg-q2y
title: Drop the repo-wide syn/bitflags duplicate allowance
status: open
type: task
priority: 4
tags:
- deps
- lint
created: 2026-10-22
updated: 2026-10-22
phase: ''
---

# Drop the repo-wide syn/bitflags duplicate allowance

From the PR 19b review (M-4). clippy.toml's allowed-duplicate-crates lists syn and bitflags repo-wide because jiff (-e today, D-58) puts syn 2 in cargo's metadata through jiff-static under a never-true cfg(any()), and bitflags 1 through defmt via a weak defmt?/alloc feature; neither is built. The entry would also hide a real future duplicate of either. Drop both from the list once jiff no longer lists jiff-static under cfg(any()) and the weak-feature lockfile entry is gone (cargo or jiff/jiff-core), checking with cargo clippy --all-targets -- -D warnings.
