---
id: rg-c9v
title: Optimise the shaping crates in the dev profile to speed up tests
status: open
type: task
priority: 3
tags:
- build
- tests
created: 2026-10-18
updated: 2026-10-18
phase: ''
---

# Optimise the shaping crates in the dev profile to speed up tests

From the fix/cap-and-wrap review (rg-wl7): text shaping dominates debug test time; a 1-char measure costs about 157 us in debug against about 10 us optimised. Adding [profile.dev.package.{harfrust,read-fonts,skrifa}] opt-level = 3 beside the existing resvg/png overrides in Cargo.toml made breaking_a_token_matches_the_linear_scan 17x faster (0.43 s vs about 7.5 s). Check build-time cost and that no golden or timing test changes.
