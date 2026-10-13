---
id: rg-8im
title: Replace ttf-parser (unmaintained, RUSTSEC-2026-0192) in tests with skrifa
status: open
type: task
priority: 3
tags:
- deps
- tests
created: 2026-10-13
updated: 2026-10-13
phase: ''
---

# Replace ttf-parser (unmaintained, RUSTSEC-2026-0192) in tests with skrifa

Found by cargo deny check advisories during the 2026-10-13 dependency check. ttf-parser 0.25.1 is unmaintained (RUSTSEC-2026-0192; no safe upgrade). It is a dev-dependency only, used in src/text.rs tests, tests/font_assets.rs, tests/measure.rs and tests/outlines_match_text.rs; nothing shipped depends on it. The advisory suggests skrifa (fontations), the same family harfrust builds on (read-fonts). Note: just ci runs cargo deny check licenses only, not advisories; consider adding advisories to just ci once this is resolved.
