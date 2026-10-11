# SVG font embedding spike — findings

> **Since PRD v0.46 (D-67):** measurement uses `harfrust`, the shaper `resvg` 0.48 uses; this spike's `rustybuzz` findings carry over unchanged, because the oracle test still agrees to the pixel.

**Closes the question design §11 raised:** how does the exported SVG carry its
font? SC-15 forbids an external font reference in that file as firmly as NFR-5
does in the HTML, and FR-54 sells it on opening in PowerPoint, Figma and
Illustrator for touch-up. The PRD asserts both without noticing they meet.

**Verdict: outline the text, deduplicated through `<use>`.** The embedding
candidate is not a trade-off that went the other way — it does not work in the
consumers FR-54 names, and it does not work in our own rasteriser either.

Tests live in `tests/outlines_match_text.rs`, 4 tests, clippy-pedantic clean.

## The finding that decided it

`@font-face` inside an SVG is honoured by browsers and by almost nothing else.

| Consumer | Honours a base64 `@font-face`? | Evidence |
| --- | --- | --- |
| Browsers | Yes | Well established; not the consumer FR-54 is about |
| **PowerPoint** | **No** | Microsoft's own support answer: *"SVG supports a wide variety of features. Some of them are not yet supported in PowerPoint. PowerPoint does not use CSS as a styling mechanism."* The reporter's case is ours exactly — base64 WOFF in `<defs>`, correct in Chrome, default font in PowerPoint |
| Illustrator, Figma | Reported no | Secondary sources only, consistently. Figma's own SVG **export** offers an "Outline Text" option, which is the same conclusion from the other direction |
| **`resvg`** | **No — renders nothing at all** | Measured here: **0 inked pixels.** Not a fallback face, not tofu. Nothing |

The `resvg` result is the one that settles it rather than the vendor reports,
because it is first-hand, it is reproducible, and `resvg` is *our own* PNG path.
A mechanism that produces an empty rectangle in the renderer we ship is not a
mechanism we can hand to three applications we do not control.

Only PowerPoint is confirmed first-party. Illustrator and Figma rest on
secondary sources — but the decision does not need them, since one confirmed
failure among the three named consumers is already disqualifying, and `resvg`
makes it two.

## What outlining costs

A realistic edition: 44 strings — title block, quadrant and ring labels, thirty
legend rows, the key — **749 glyph instances over 56 distinct glyphs**.

| Variant | Bytes | gzipped |
| --- | --- | --- |
| `<text>`, external font *(forbidden by SC-15)* | 4,150 | — |
| Outlined, one path per glyph instance | 645,673 | 210,015 |
| **Outlined, deduplicated through `<use>`** | **73,508** | **19,002** |
| Embedded `@font-face`, WOFF2 base64 | 216,658 | 162,090 |

**The naive form costs nearly nine times the deduplicated one**, and quoting it
would make outlining look untenable — 749 instances over 56 glyphs is a ratio
the per-instance form pays for every time. One `<path>` per distinct glyph in
`<defs>`, placed by `<use x= y=>`, is the form to build.

**Size is not what decided it, and the honest reading is that it is roughly a
wash.** The embed column uses the *unsubset* Libertinus stand-in; a Latin-subset
Inter WOFF2 would be far smaller, and comparable to 73 kB uncompressed. The
gzip column is where they part — outlines compress about 3.9×, base64 of an
already-compressed WOFF2 essentially not at all — which matters only if the file
is zipped for email.

Ids in the exported SVG are fine, incidentally: design §5 bans them because the
HTML holds two arrangements in one DOM, and the exported SVG is one document
holding one.

## Fidelity: visually identical, not bit-identical

Rasterising the outlined document and the `<text>` document through `resvg` and
comparing pixel buffers:

| Coordinate precision | Differing subpixels of 3,960,000 | Worst channel delta |
| --- | --- | --- |
| 3 dp *(NFR-17)* | 82 (20 ppm) | 64 |
| 4 dp | 44 | 64 |
| 6 dp | 30 | 64 |
| 9 dp | 30 | 64 |

Two things worth reading off this. **It does not fall to zero** — about 30
subpixels are precision-independent, so roughly 52 of the 82 are NFR-17's
rounding and the remainder is something in `resvg`'s own text pipeline that a
path does not reproduce. **It is not clustered**: the residue spreads over 67
scanlines of 1,100, one or two subpixels each, never a glyph rendering wrongly.

**This costs nothing that is specified.** NFR-4 is byte-identity of *our own*
output, which outlining does not touch — the same input still produces the same
SVG. SC-7's parity is about items, positions, marker treatments and wrap points,
all of which are upstream of outlining and unchanged. What the residue would
affect is somebody rasterising our exported SVG themselves and diffing it
against our PNG, which nothing requires and nobody has asked for. Recorded
because "outlining is lossless" is the kind of claim that gets repeated after
nobody can remember measuring it.

## Checked and closed on the way past

**The OFL does not require a licence notice in the output.** Worth stating
because it looks like it should: the HTML inlines the font (NFR-5), and OFL 1.1
clause 2 requires each redistributed copy of the Font Software to carry the
notice and licence. The FAQ settles it — **1.10**: *"The only situation in which
an OFL font can be distributed without the text of the OFL … is when a font is
embedded in a document or bundled within a program."* **1.12** covers subsets
explicitly. Both of our cases are named: the font `include_bytes!`'d into the
binary is *bundled within a program*, and the font base64'd into an emitted HTML
file is *embedded in a document*. FR-85's "licence text ships beside the font"
is sufficient and nothing needs to change. Note the distinction the FAQ draws:
a font **served** as a web font is distribution and does need the package —
a self-contained HTML file with a data URI is not that.

**R-7's remaining half is answered: Inter declares no Reserved Font Name**, so a
Latin subset may keep the name. Its `LICENSE.txt` at v4.0 reads only
`Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter)`
with no reserved name after the copyright statement, and OFL 1.1 defines a
Reserved Font Name as one *"specified as such after the copyright statement(s)"*.
(The Libertinus stand-in, by contrast, reserves "Linux Libertine", "Biolinum"
and "STIX Fonts" — none of which is "Libertinus", so even it was never at risk.)
"Inter" is a trademark of Rasmus Andersson, which is separate from the OFL and
does not bite on vendoring a subset inside a tool.

## What this does *not* settle

- **Illustrator and Figma first-hand.** Neither was tested; both rest on
  secondary reports. Cheap to confirm when v1.1 is built, and the decision does
  not wait on it.
- **Whether the HTML output changes.** It does not. The HTML keeps `<text>` plus
  an inlined WOFF2 — browsers honour `@font-face`, FR-46's explicit `<tspan>`s
  mean the browser never re-wraps, and the text stays selectable and searchable.
  Outlining is the **SVG emitter's** business alone.
- **Accessibility and selectability of the exported SVG.** Outlined text cannot
  be selected, searched or read by a screen reader. Nothing requires it to be —
  NFR-1 is scoped to the HTML output, and FR-54's job for this file is touch-up
  in a design tool, where the alternative was text in the wrong font anyway.
  Stated so the loss is a decision rather than a discovery.

## Consequences

- **`emit/svg.rs` is not thin**, which design §11 assumed it would be. It needs
  glyph outlines from `ttf-parser`, a shaping pass it can share with `text.rs`,
  and a distinct-glyph `<defs>` table. The PNG emitter is unaffected — it
  rasterises the `<text>` document through our own `fontdb`.
- **No new dependency.** `ttf-parser` and `rustybuzz` are already direct
  dependencies; `base64` is a new dev-dependency for the `@font-face` test only
  (MIT OR Apache-2.0, so §1's permissive-tree claim holds).
- **The estimate moves.** §14.2 costs "PNG + SVG export" at 1–1.5 days with SVG
  "effectively free once rasterisation works". That is no longer true — call the
  SVG half a day to a day on its own. Still v1.1, still small.
