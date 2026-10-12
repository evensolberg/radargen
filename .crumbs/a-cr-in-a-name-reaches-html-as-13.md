---
id: rg-c4l
title: A CR in a name reaches HTML as &#13;
status: closed
type: bug
priority: 3
tags:
- html
- escaping
created: 2026-10-12
updated: 2026-10-12
closed_reason: 'PR D: FR-97 — line breaks are a space in labels, a plain newline in prose (D-79)'
phase: ''
---

# A CR in a name reaches HTML as &#13;

From the PR 14b QA review. FR-90 keeps CR, so a quoted CSV field with CRLF inside reaches an aria-label as &#13; (xml_attr), which an HTML5 parser reports as a parse error (control-character-reference). Decide: normalise CR/CRLF to LF on input (FR-76?), or write it differently in the HTML context. Add "a\rb" to the html5ever round trip.
