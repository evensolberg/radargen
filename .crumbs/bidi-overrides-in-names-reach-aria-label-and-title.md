---
id: rg-f67
title: Remove bidi embedding, override and isolate characters (FR-90)
status: in_progress
type: task
priority: 2
tags:
- security
- a11y
- text
created: 2026-10-12
updated: 2026-10-12
phase: ''
---

# Remove bidi embedding, override and isolate characters (FR-90)

From the PR 13 security review: U+202A-202E and U+2066-2069 are Cf, not control characters, so FR-90 does not remove them and FR-76 trims only at the ends; inside a name they reach aria-label and <title> and can visually spoof a name in a tooltip. Not an injection. Decide whether FR-90 should remove them, or accept.

[start] 2026-10-12 11:15:55
