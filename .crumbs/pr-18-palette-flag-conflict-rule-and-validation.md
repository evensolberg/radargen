---
id: rg-cdo
title: 'PR 18: --palette flag, conflict rule and validation'
status: blocked
type: feature
priority: 2
tags:
- impl
- palette
- cli
created: 2026-10-08
updated: 2026-10-10
blocked_by:
- rg-42m
phase: v1.1
---

# PR 18: --palette flag, conflict rule and validation

FR-59 conflicts_with, FR-86/FR-87 file validation; tier-one failure while building Config. SC-21. Completes v1.1.

[2026-10-09] D-54: Palette::number_ink computes WCAG contrast from #RRGGBB, so --palette values must be #RRGGBB (reject other colour syntax as a hard error, or normalise it).

[2026-10-09] FR-86 (PRD v0.38) now requires every --palette value to be #RRGGBB; anything else is a hard error naming the role.

[2026-10-09] From PR 6 review: when parsing --palette, store colours as a parsed Rgb([u8;3]) newtype (Display -> #RRGGBB) built by a fallible constructor, so PR 04's luminance()/number_ink become total instead of panicking on a non-#RRGGBB string.

[2026-10-10] From PR 12 (PR 03) review: escape::css_value is now an allow-list ('#' and ASCII alphanumerics), so it accepts the empty string; --palette validation must reject an empty value along with anything not #RRGGBB (FR-86), and wrap CssValueError in a Fatal variant.

[2026-10-10] From PR 13 (PR 04) review: palette::luminance slices hex[1..7] and parses with u8::from_str_radix, which panics on a non-ASCII boundary and accepts a leading '+' ('#+F+F+F'). The --palette validator must check len == 7, a leading '#', and that every remaining byte is_ascii_hexdigit — not merely that parsing succeeds.
