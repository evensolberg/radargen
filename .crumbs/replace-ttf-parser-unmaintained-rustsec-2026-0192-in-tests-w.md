---
id: rg-8im
title: Replace ttf-parser (unmaintained, RUSTSEC-2026-0192) in tests with skrifa
status: closed
type: task
priority: 3
tags:
- deps
- tests
created: 2026-10-13
updated: 2026-10-15
closed_reason: Tests read fonts with skrifa 0.44 (usvg's); advisories added to just ci and the CI deny job.
phase: ''
---

# Replace ttf-parser (unmaintained, RUSTSEC-2026-0192) in tests with skrifa

Found by cargo deny check advisories during the 2026-10-13 dependency check. ttf-parser 0.25.1 is unmaintained (RUSTSEC-2026-0192; no safe upgrade). It is a dev-dependency only, used in src/text.rs tests, tests/font_assets.rs, tests/measure.rs and tests/outlines_match_text.rs; nothing shipped depends on it. The advisory suggests skrifa (fontations), the same family harfrust builds on (read-fonts). Note: just ci runs cargo deny check licenses only, not advisories; consider adding advisories to just ci once this is resolved.

[start] 2026-10-15 16:49:19

[stop]  2026-10-15 16:54:58  5m 39s
