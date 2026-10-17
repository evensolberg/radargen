---
id: rg-cdo
title: 'PR 18: --palette flag, conflict rule and validation'
status: closed
type: feature
priority: 2
tags:
- impl
- palette
- cli
created: 2026-10-08
updated: 2026-10-16
closed_reason: 'PR 18: --palette (1.3.0)'
blocks:
- rg-0fm
phase: v1.1
---

# PR 18: --palette flag, conflict rule and validation

FR-59 conflicts_with, FR-86/FR-87 file validation; tier-one failure while building Config. SC-21. Completes v1.1.

[2026-10-09] D-54: Palette::number_ink computes WCAG contrast from #RRGGBB, so --palette values must be #RRGGBB (reject other colour syntax as a hard error, or normalise it).

[2026-10-09] FR-86 (PRD v0.38) now requires every --palette value to be #RRGGBB; anything else is a hard error naming the role.

[2026-10-09] From PR 6 review: when parsing --palette, store colours as a parsed Rgb([u8;3]) newtype (Display -> #RRGGBB) built by a fallible constructor, so PR 04's luminance()/number_ink become total instead of panicking on a non-#RRGGBB string.

[2026-10-10] From PR 12 (PR 03) review: escape::css_value is now an allow-list ('#' and ASCII alphanumerics), so it accepts the empty string; --palette validation must reject an empty value along with anything not #RRGGBB (FR-86), and wrap CssValueError in a Fatal variant.

[2026-10-10] From PR 13 (PR 04) review: palette::luminance slices hex[1..7] and parses with u8::from_str_radix, which panics on a non-ASCII boundary and accepts a leading '+' ('#+F+F+F'). The --palette validator must check len == 7, a leading '#', and that every remaining byte is_ascii_hexdigit — not merely that parsing succeeds.

[2026-10-12] From PR 13 (security): every Palette constructor validates FR-86 #RRGGBB before storing a value (consider a HexColour newtype), so a --palette value can never reach an SVG attribute unchecked; a url(...) in fill would be an external paint server. The serialiser's static_paint_is_a_hex_literal test pins the built-in case.

[2026-10-12] From PR 14b: the HTML stylesheet (emit/html/css.rs palettes()) writes Palette values verbatim as custom-property declarations, relying on construction-time validation. A --palette constructor must check FR-86 #RRGGBB before storing, or a value with ; } < reaches the <style> block (NFR-6 context 5).

[2026-10-15] From the rg-nuy review: script.js assumes the theme toggle exists. When --palette omits it, guard or skip the toggle block, or toggle.addEventListener throws before the D-66 loop, the js class and the skip link run, and the page loses its interactive layer.

[2026-10-16] PRD v0.78: FR-102 / D-92 fix the file grammar — exactly one :root rule, comments allowed, leading BOM ignored; any other selector/at-rule/second :root, an unknown property (not an FR-82 role or --series-N without leading zeros, case-sensitive), or a duplicate is a hard error naming it and its line; all of a file's problems (grammar, FR-86, FR-87) reported together. SC-21 extended. --create-palette is split out to rg-0fm (PR 18b).

[2026-10-16] PRD v0.78 review: PR 18 also rewrites docs/product/palette.sample.css to FR-101's single-:root form (light values; each neutral's dark value in a /* */ comment after its ;) so 1.3.0 never ships a sample FR-102 rejects; replace tests/palette.rs's two-block drift check with: the sample parses through the new parser to Palette::built_in(Light), and its comments carry every DARK value. FR-102 details: comments stripped first (legal anywhere), selector exactly :root ASCII case-insensitive, NUL refused first, duplicate names both lines, values stored uppercased, FR-86/87 checked over the accepted rule only. NFR-15 now covers only unreadable / non-UTF-8 palette files.

[2026-10-16] PRD v0.78 re-review: PR 18 also writes docs/palette.md (the guide the sample header links to: each role, where it is drawn with an annotated illustration, built-in light/dark values, contrast each needs; link from README). The sample header must not name --create-palette. Before rewriting the sample, copy the current two-block file to tests/fixtures/ as the SC-21 second-block fixture. Line numbers in palette diagnostics are those of the file as written; a removed comment counts as whitespace and keeps its line breaks.
