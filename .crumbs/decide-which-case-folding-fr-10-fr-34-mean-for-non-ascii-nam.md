---
id: rg-wld
title: Pin down non-ASCII ordering before the Rust port (FR-10/FR-34)
status: open
type: task
priority: 2
tags:
- prd
- spec
- ordering
created: 2026-08-26
updated: 2026-10-08
blocks:
- rg-efx
phase: ''
---

# Pin down non-ASCII ordering before the Rust port (FR-10/FR-34)

FR-10 and FR-34 require ordering "case-insensitively, locale-independently"
but do not say which case folding. The mockup and a Rust port agree exactly
on ASCII, and every current quadrant and item name is ASCII, so nothing is
broken today.

They can diverge on non-ASCII input: JavaScript `String.prototype.toLowerCase`
and Rust `str::to_lowercase` differ on some edge cases, and neither is the
same as Unicode full case folding (`str::to_lowercase` does not fold e.g.
Turkish dotted/dotless I the way a locale-independent caseless match ideally
would). Two implementations reading the same CSV could then order quadrants
or names differently, which is exactly what NFR-4 promises cannot happen.

Decide and record in the PRD before the layout engine lands:
- name the folding explicitly (simple Unicode lowercase vs full case folding),
  and say so in FR-9, FR-10, FR-16, FR-34 and FR-76; or
- state that ordering is defined over ASCII names only, and say what the tool
  does with a non-ASCII one.

Raised during the v0.11 product review (commit 203b00d). Not urgent — gate it
on the first non-ASCII entry, or on implementation planning, whichever first.

## Second divergence: code-unit vs code-point order

Found by code review of the same commit, and verified by running both sides.

The prototype's `ordinal()` uses JavaScript `<`/`>`, which compares **UTF-16
code units**. Rust's `str::cmp` compares **UTF-8 bytes**, which is code point
order. These agree across the BMP and diverge above it, because a surrogate
lead unit (0xD800-0xDBFF) sorts below BMP characters in U+E000-U+FFFF while
the corresponding code point sorts above them.

Demonstrated with U+1F600 vs U+FB00:
  JS  `ordinal`      -> -1   (emoji first)
  Rust UTF-8 `cmp`   -> +1   (emoji last)

Same consequence as the case-folding question: two implementations reading one
CSV would number items differently, which NFR-4 says cannot happen. Same fix
shape too, so decide both together:
- say which order FR-10/FR-34 mean (code point is the natural choice for Rust,
  and `Array.from(s)` makes the JS prototype match), or
- scope the ordering guarantee to ASCII names and say what happens otherwise.

`mockup.html`'s comparator comment now states the caveat rather than claiming
the two are equivalent, which it did until this was caught.
