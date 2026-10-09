---
id: rg-q3w
title: Enable the clippy::pedantic gate and forbid(unsafe_code)
status: open
type: task
priority: 3
tags:
- lint
- ci
created: 2026-10-09
updated: 2026-10-09
phase: v1.0
---

# Enable the clippy::pedantic gate and forbid(unsafe_code)

Architecture §1 says #![forbid(unsafe_code)] at the crate root and clippy::pedantic denied in CI. Neither is in place: lib.rs has no forbid, there is no CI, and `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` fails today with 6 findings in existing code (e.g. must_use_candidate on normalise). Found while planning PRs 01-04. Fix the existing findings, add the attribute, add [lints] to Cargo.toml so the gate is local as well as CI.
