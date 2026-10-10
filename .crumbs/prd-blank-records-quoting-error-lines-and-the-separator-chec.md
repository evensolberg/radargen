---
id: rg-lfd
title: 'PRD: blank records, quoting-error lines and the separator check'
status: open
type: task
priority: 2
tags:
- prd
- csv
- diagnostics
created: 2026-10-10
updated: 2026-10-10
phase: ''
---

# PRD: blank records, quoting-error lines and the separator check

From the PR 05 plan (RG-005 rev 1.1) and its review on PR #17. Three plan decisions depart from the PRD's literal wording and should be written into it: (a) FR-3/FR-4/FR-6/FR-78: a blank, whitespace-only or all-empty-fields record (Excel's ,,,) is not a row. It is skipped with its lines still counted, and the header is the first non-blank record. (b) NFR-10/NFR-15: UnterminatedQuote and TextAfterQuote are file-level and name the line of the fault, not the record's first line. (c) NFR-9/NFR-16: list NotCommaSeparated (FR-1, a ;- or tab-separated file refused at its header) as a tier-one hard error; it pre-empts an FR-5 error that would always fire.
