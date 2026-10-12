---
id: rg-n88
title: Failure matrix asserts the write error comes last
status: closed
type: task
priority: 4
tags:
- tests
created: 2026-10-12
updated: 2026-10-12
closed_reason: 'PR B: errors by tier (refactor/errors-by-tier)'
phase: ''
---

# Failure matrix asserts the write error comes last

From the PR 14a final review. tests/run.rs a_failed_run_leaves_the_old_file only checks stderr contains "error: ". For the read-only and directory faults, FR-17 precedes the write error: also assert the last line is the error and no warning follows it, pinning third-tier order end to end.
