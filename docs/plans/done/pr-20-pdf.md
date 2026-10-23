---
tags:
  - plan
  - pdf
  - output
aliases: []
doc_id: RG-020A
doc_name: pr-20-pdf
crumb_id: rg-5uf
document_title: "PR 20 — PDF output, light (radar, key and legend): Implementation Plan"
created_date: 2026-10-22
synopsis: "-o radar.pdf writes the wide arrangement as a one-page vector PDF sized to the document, from the same Document PNG and SVG export. The PDF is written directly with pdf-writer (no SVG-to-PDF crate: none supports usvg 0.48), its text real text in an embedded Inter subset (subsetter) with a ToUnicode map, so it selects, copies and searches. Untagged (rg-ffj); no dates or ID, so it is byte-reproducible (NFR-4). Paper sizes are PR 21's."
status: complete
type: Plan
revision: "1.2"
review_date:
reviewed_by: []
completed_date: "2026-10-22"
comments: "Designed with the maintainer, 2026-10-22: option C (our own writer over Document), real text (A), untagged with a crumb for tagging (rg-ffj), page sized to the document with paper sizes in PR 21 (rg-4b8). Task 1 amends the PRD to 0.106. The release is a minor bump (1.19.0), done by the orchestrator, not by this plan."
---
# PR 20 — PDF output, light (radar, key and legend): Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumbs:** `rg-5uf` · **After:** 1.18.0 (#141) · **Then:** PR 21, the full PDF (`rg-4b8`), and tagging both (`rg-ffj`)

**Goal:** `radargen entries.csv -o radar.pdf` writes a one-page vector PDF of the wide arrangement — the same radar, key and legend the PNG and SVG draw — whose text selects, copies and searches as the words the CSV holds, and which is byte-identical from run to run.

**Architecture:**

- **No SVG-to-PDF crate (D-103).** svg2pdf 0.13.0 is on usvg 0.45 and krilla-svg 0.8.1 on usvg 0.47; radargen is on 0.48. A second usvg fails `clippy::multiple_crate_versions`, and their trees are not ours anyway. The PDF is written straight from our `svg::Document` with `pdf-writer`, which has no usvg dependency.
- **`src/pdf/` (new): the writer, one job per file.**
  - `geometry.rs`: circles, rounded rectangles and SVG circular arcs as cubic Béziers (PDF has no arc operator). Pure functions.
  - `font.rs`: per weight, the glyphs the document draws, subset with `subsetter` and written as a Type0 / CIDFontType2 font with a widths array and a ToUnicode CMap built from shaper clusters.
  - `content.rs`: walks the `Node` tree (dropping interactive-only nodes, as the static SVG does) and writes the page's content stream: groups as `q`/`cm`/`Q`, shapes as paths, literal palette colours.
  - `show.rs`: one text node's lines and tail as `BT … TJ … ET` inside its own `q … Q`, each glyph where our own shaper places it.
  - `mod.rs`: `real`, the one path a coordinate takes into the PDF (NFR-17's rounding, then `f32`), and `scale`, the page scale narrowed unrounded.
- **`src/emit/pdf.rs` (new): the entry point.** `export(&Document, &Export) -> Result<Vec<u8>, Fatal>` assembles catalog, page tree, page, content stream, info dictionary and fonts in a fixed object order.
- **What static outputs share moves to where they all read it** (Task 2, its own refactor commit). `Node::static_runs` replaces `outline.rs`'s private walk, so the SVG's glyph table and the PDF's subsets collect the same glyphs; `Document::ground` is the one D-87 ground; `emit::intrinsic_size` leaves `emit::svg`, so no emitter imports another.
- **Checked by an independent rasteriser.** The render oracle draws the PDF with hayro and compares it with the PNG (Task 8, written before `export`), with committed damage tests; a text oracle reads it back with lopdf against the catalogue (Task 10).
- **`OutputFormat::Pdf`** joins `options.rs`; the compiler then names every `match` that needs a PDF arm.

**Tech Stack:** Rust 2024; `pdf-writer` 0.15 (MIT OR Apache-2.0; deps bitflags, itoa, memchr, ryu); `subsetter` 0.2 with `default-features = false` (MIT OR Apache-2.0; dep rustc-hash); `miniz_oxide` 0.9 (already in the lockfile through flate2; 0.9 rather than png's direct 0.8, so the existing duplicate goes away when png moves, `rg-42m`) for the font stream's FlateDecode. Dev-only: `hayro` 0.8 (MIT OR Apache-2.0), an independent pure-Rust PDF rasteriser, as the render oracle; `lopdf` 0.46 with `default-features = false` (MIT), to read the PDF's text and structure back in tests.

**Spec:** `docs/product/radargen-prd.md`, v0.106 after Task 1. The plan rests on:

- FR-110, D-103 and SC-30, which Task 1 adds;
- FR-45, FR-52, FR-54, FR-56, FR-57, FR-64, FR-71, FR-100, FR-103, FR-104 and FR-106, which Task 1 amends to name PDF;
- NFR-4, NFR-12, NFR-15 and NFR-17; D-75, D-87, D-88 and D-100;
- the dated notes on crumbs `rg-5uf`, `rg-4b8` and `rg-ffj`.

## Global Constraints

- **Wide arrangement only** (FR-71). The PDF exports `arrange(Arrangement::Wide, &blocks)`, with `--legend-columns` applied as for PNG and SVG (FR-100).
- **One page sized to the document.** MediaBox `[0 0 W×0.75 H×0.75]` in points, where `(W, H)` is `emit::intrinsic_size(document, --width)` in CSS px (96 px = 72 pt). If either side passes 14,400 pt, the page carries `/UserUnit u` with `u` the smallest whole number that brings both under, and the MediaBox and the content scale are divided by `u`. No paper sizes (`rg-4b8`). The fixed object numbers (1–5) are a property of the light PDF; PR 21 renumbers and re-blesses.
- **Real text, never outlines.** Every drawn text is glyphs of an embedded subset of the vendored Inter TTF, with a ToUnicode map that gives the CSV's text back.
- **Untagged** (`rg-ffj`). No structure tree, no marked content. The catalog carries `/Lang (en)` and `/ViewerPreferences << /DisplayDocTitle true >>`; the info dictionary carries `/Title` (`--title`, or `Technology Radar`, D-75), `/Subject` (the radar's name, the SVG's `desc`) and `/Producer (radargen X.Y)` (major.minor, NFR-12).
- **Reproducible** (NFR-4). No `/CreationDate`, no `/ModDate`, no trailer `/ID`. Objects are numbered in a fixed order. Glyph CIDs are assigned in ascending glyph-id order. Every coordinate goes through `pdf::real`; the page scale, a quotient with no trigonometry in it, is narrowed to `f32` unrounded so the drawing meets the MediaBox (`pdf::scale`).
- **Compression:** only the font file is Flate-compressed (`miniz_oxide::deflate::compress_to_vec_zlib`, level 6). The content stream and the ToUnicode CMap stay uncompressed, so a golden diffs as text.
- **No new duplicate crate in what ships.** `cargo tree -d -e normal` lists only the duplicates `clippy.toml`'s `allowed-duplicate-crates` already names; `cargo clippy --all-targets -- -D warnings` and `cargo deny check` stay clean with that list unchanged. hayro's second skrifa, read-fonts, fearless_simd, fastrand and syn are dev-only: `clippy::multiple_crate_versions` counts the normal graph only (checked 2026-10-22 by a trial add), and `deny.toml`'s `multiple-versions` is `warn`.
- **Errors** are `Fatal` variants, never panics: a glyph that cannot be measured or subset is `Fatal::Font` (NFR-15). An `expect` is allowed only for a broken internal invariant, with the invariant in its message, as `outline.rs` does.
- **Lints:** `as_conversions`, `indexing_slicing`, `string_slice` and `unwrap_used` are denied. An `#[expect]` names its reason inline; the `f64 → f32` narrowing lives in `pdf::narrow`, which `real` and `scale` call.
- **Lint pitfalls the review found in this plan's code** (fix as they come, the crate's way): `doc_markdown` wants backticks on `ToUnicode`, `CIDFont`, `CIDFontType2`, `FlateDecode`, `MediaBox` and file names in `///` docs; `float_cmp` forbids `==` on floats (compare `to_bits`, or a tolerance); `suboptimal_flops` fires on `a + b * c` (the crate's `#[expect]` with the NFR-4 reason, never `mul_add`), and an `#[expect]` that no longer fires is itself a warning; `many_single_char_names` and `missing_const_for_fn` reach test helpers too; `imprecise_flops` wants `hypot`; `unwrap_in_result` forbids `expect` in a function returning `Result`; `.finish()` on pdf-writer writers needs `use pdf_writer::Finish;`; the text role is `Neutral::TextPrimary`.
- **Visibility:** `mod pdf;` is private in `lib.rs`; items inside it stay `pub` (clippy's `redundant_pub_crate` objects to `pub(crate)` in a private module). The public surface is `emit::pdf::{Export, export}` and `emit::intrinsic_size`. Until Task 8 uses them, the `pdf` items are dead code to the compiler: each of Tasks 3–7 adds `#[cfg_attr(not(test), expect(dead_code, reason = "used by emit::pdf from Task 8"))]` on `mod pdf;` only if the build warns, and Task 8 removes it.

## Review Focus

1. **A title with PDF-special characters** (`)`, `(`, `\`, non-ASCII such as `Ærø — Q3`) — the info dictionary's `/Title` still parses and reads back exactly. Pinned in Task 8.
2. **Characters the bundled font lacks** (`中文`) — the PDF draws `.notdef` boxes as the PNG does, FR-104's warning says `the PDF draws them as boxes`, and the file is still valid. Pinned in Tasks 5 and 9.
3. **`--width 320`, `--width 10000` and a tall long legend** — the MediaBox follows FR-57, the drawing meets its edges exactly (the scale is written unrounded, so the drawing and its ground meet the MediaBox's edges), and a side past 14,400 pt gets a `/UserUnit` instead of a page viewers refuse. Pinned in Task 8 (sizes) and Task 8's oracle (drawing at 320).
4. **Kerning, letter-spacing and ligatures** (legend headings are letter-spaced; `fi` may ligate) — copying returns the original letters, each name intact, and glyphs sit where the PNG draws them. Pinned in Tasks 5 and 10; how viewers break letter-spaced words is the manual check's (Task 11), since lopdf's extraction ignores TJ gaps.
5. **A radar with no title and the dark theme** — the page still has a `/Title` (`Technology Radar`), the ground is the dark `Surface1` to the page's edges, and the render oracle agrees with the dark PNG. Pinned in Task 8.
6. **A halo before a stroked shape** — the halo's round join does not leak into the legend's square strokes. Pinned in Task 7.

---

## File Structure

| File | Change | Responsibility |
| --- | --- | --- |
| `docs/product/radargen-prd.md` | Modify | v0.106: FR-110, D-103, SC-30; PDF named where the static formats are |
| `docs/design/architecture.md` | Modify | §11 gains the PDF writer beside the SVG converter |
| `Cargo.toml`, `Cargo.lock` | Modify | `pdf-writer`, `subsetter`, `miniz_oxide`; dev `hayro`, `lopdf` |
| `src/svg/mod.rs` | Modify | `Node::static_runs` (and private `text_runs`), `Document::ground` |
| `src/svg/serialise/outline.rs` | Modify | collect through `Node::static_runs`; drop `runs` |
| `src/text.rs` | Modify | `FontMetrics::{bytes, advance, descriptor}`, `Descriptor`, `FontError::new`, `Weight::postscript` |
| `src/pdf/mod.rs` | Create | module root, `real` |
| `src/pdf/geometry.rs` | Create | Béziers for circles, rounded rects, arcs |
| `src/pdf/font.rs` | Create | subsets, CIDs, widths, ToUnicode, font objects |
| `src/pdf/content.rs` | Create | `Canvas`; the content stream: walk, shapes, colours |
| `src/pdf/show.rs` | Create | a text node as `BT … TJ … ET` |
| `src/emit/pdf.rs` | Create | `Export`, `export`: the PDF file |
| `src/emit/mod.rs`, `src/lib.rs` | Modify | `pub mod pdf;` in `emit`, private `mod pdf;` in `lib.rs`; `intrinsic_size` moves to `emit` |
| `src/options.rs` | Modify | `OutputFormat::Pdf` |
| `src/pipeline.rs` | Modify | the `Pdf` arm |
| `src/diagnostics.rs` | Modify | four-extension message; `Undrawn` for PDF; PNG-and-SVG wording |
| `src/cli.rs`, `src/starter.rs` (via `docs/product/config.sample.toml`) | Modify | help and starter config name PDF |
| `tests/golden/pdf/prd-light.pdf.txt` | Create | the golden, as text |
| `tests/serialise/pdf.rs` | Create | golden, names, hostile title |
| `tests/pdf_matches_png.rs` | Create | the render oracle (hayro) and its damage tests (Task 8) |
| `tests/pdf_text.rs` | Create | the text oracle (Task 10) |
| `tests/determinism.rs` | Modify | the PDF across processes |
| `tests/run.rs`, `tests/diagnostics.rs`, `tests/config.rs`, `tests/edition.rs`, `tests/cli.rs` | Modify | `.pdf` is recognised; new wording |
| `README.md`, `docs/csv-format.md`, `docs/a11y/manual-checks.md` | Modify | PDF documented; a manual selection and search step |

---

### Task 1: The PRD names PDF (v0.106)

**Files:**
- Modify: `docs/product/radargen-prd.md`

No code, no test: a spec amendment the later tasks argue from. Every edit below goes in this one commit.

- [ ] **Step 1: Frontmatter.** `Version: 0.105` → `Version: 0.106`. The `Synopsis` and §1's first paragraph say "HTML/PNG/SVG" and "plus PNG and SVG exports": make them "HTML/PNG/SVG/PDF" and "plus PNG, SVG and PDF exports".

- [ ] **Step 2: Add FR-110** in §6.5, directly after FR-55's row:

```markdown
| **FR-110** | **PDF**: the shared document (FR-44) as a **one-page vector PDF**, written directly from the document, with no rasterisation and no intermediate SVG (D-103). **The page is the document's size**: its MediaBox is FR-57's intrinsic size at 72 points per 96 CSS pixels, so `--width` sets the page width as it sets an SVG's, and the height follows; paper sizes are not offered. A side past the 14,400 points (200 inches) viewers accept is written with a `/UserUnit` that brings it under, so the page keeps its size without being refused or cropped. PNG's pixel caps and small-text warning do not apply (D-87): a vector page has no pixels to run out of and zooms. **Its text is text**: every run is drawn in an embedded subset of the bundled Inter face (FR-85), each glyph where the shaper places it (FR-84), with a ToUnicode map, so a reader can select, copy and search the names the CSV holds. **Untagged**: it carries no structure tree; its catalog sets `/Lang (en)` and asks viewers to show the document title, and its info dictionary holds `/Title` (FR-65's title, or *Technology Radar*, D-75), `/Subject` (the radar's name, as the SVG's `<desc>` carries it) and `/Producer (radargen X.Y)` (SC-25). **Reproducible**: no creation or modification date and no file identifier, so a render is a function of its inputs (NFR-4); only the embedded font is compressed. |
```

- [ ] **Step 3: Amend FR-52 and the formats it lists.** FR-52: `` `.html`, `.png`, `.svg` `` → `` `.html`, `.png`, `.svg`, `.pdf` ``. FR-45: "SVG export writes it out directly; PNG export rasterises it. **One layout engine, two arrangements (FR-68), three output modes**" → "SVG export writes it out directly; PNG export rasterises it; PDF export draws it as a vector page. **One layout engine, two arrangements (FR-68), four output modes**". FR-71: "**PNG and SVG export the wide arrangement.**" → "**PNG, SVG and PDF export the wide arrangement.**"

- [ ] **Step 4: Name PDF where a requirement lists the static formats.** In each of these, "PNG and SVG" / "PNG or SVG" / "PNG, SVG" becomes "PNG, SVG and PDF" / "PNG, SVG or PDF", with no other change: FR-56 ("Applies to PNG and SVG only" → "Applies to PNG, SVG and PDF only"), FR-57 ("Applies to SVG too" → "Applies to SVG and PDF too"), FR-64, FR-100 (its "and to any later static format that exports the wide arrangement" becomes "and to PDF"), FR-103, FR-104 (title and body: a PDF draws a lacking character as the face's missing-glyph box, as PNG and SVG do), FR-105 ("**PNG and SVG do not warn:**" → "**PNG, SVG and PDF do not warn:**"), FR-106, FR-107's static-format sentence, FR-91's "`legend` and `table` on a PNG or SVG", D-100's bold statement, the §6.6 flag table's *Applies to* cells for `--theme`, `--width` and `--legend-columns` (`PNG, SVG` → `PNG, SVG, PDF`), and `link`'s "Static outputs (PNG, SVG, PDF) carry no links" (already right: leave it). **Do not change** FR-55, D-87, FR-64's pixel cap, or any PNG-only rule (the area cap, `WidthClamped`, `PngTextSmall`): those stay PNG's. Run `rg -n "PNG and SVG|PNG or SVG|PNG, SVG|\.png or \.svg" docs/product/radargen-prd.md` after editing and confirm each remaining hit is either PNG-specific, historical (§9's reasoning, §15's history) or already names PDF.

- [ ] **Step 5: FR-58 and G-7.** FR-58's last sentence: "PNG or SVG export is the answer for print, slides, and PDFs" → "PNG, SVG or PDF export is the answer for print and slides; PDF export is the answer for a document to share or print (FR-110)". G-7: "Output a **PNG and an SVG**" → "Output a **PNG, an SVG and a PDF**" and drop "PDFs," from its list of uses.

- [ ] **Step 6: Add D-103** at the end of §9's table:

```markdown
| **D-103** | **PDF is written directly from the document, its text real text, untagged for now** (FR-110, FR-45, FR-71; D-88, D-75). | *Added from the PR 20 design with the maintainer, 2026-10-22.* **Directly, not through an SVG-to-PDF crate:** the planned route (crumb `rg-5uf`, 2026-10-12) was svg2pdf on the usvg tree the PNG path builds, but svg2pdf 0.13.0 is on usvg 0.45 and krilla-svg 0.8.1 on 0.47, while radargen is on 0.48; a second usvg is a duplicate crate the lint refuses, and the other tree is not ours to hand over. The document already holds everything a PDF needs — shapes, literal colours once a palette binds them, and text the shaper places — so a writer over it with `pdf-writer` is one more emitter beside the SVG converter, deterministic by construction and with no usvg in its path. **Real text, not outlines:** a PDF is searched and copied from in a way an SVG opened for editing is not, so D-88's outlining would cost more here than it saves; the shaper still places every glyph, so the layout is the PNG's and the SVG's. **Untagged for now:** the light PDF has a title, a language and text in reading order, but no structure tree; tagging both PDFs together is crumb `rg-ffj`, after PR 21's table, which needs structure most; until then a screen-reader user is better served by the HTML page. `/Lang (en)` is fixed, as the HTML page's `lang` is; a CSV in another language is a known limit for `rg-ffj`. **The page is the document's size**; paper sizes come with the full PDF (crumb `rg-4b8`). Rejected: svg2pdf or krilla-svg on an older resvg (a downgrade the PNG goldens and harfrust's pin would both pay for, `rg-v3a`); outlined text (unsearchable); minimal tagging in this PR (it roughly doubles it, and PR 21 would redo it). |
```

- [ ] **Step 7: Add SC-30** after SC-29:

```markdown
| **SC-30** | `-o radar.pdf` writes one page whose MediaBox is FR-57's size at 0.75 pt per px; text extracted from it holds every sector, ring and item name the bundled font can draw, each intact and in legend order; rendered at the PNG's pixel size by an independent PDF rasteriser (hayro) it differs from the PNG by no more than antialiasing does (a budget measured as SC-7's was); and two separate runs write identical bytes, with no creation date and no file identifier. |
```

- [ ] **Step 8: §14.1.** After the paragraph beginning "From 1.2 every format FR-52 names ships", add: "PDF export (FR-110) ships as **1.19.0**, a v1.2 slice of its own with the full PDF (crumb `rg-4b8`) and tagging (crumb `rg-ffj`) to follow." Change that paragraph's "From 1.2 every format FR-52 names ships" to "From 1.2 every format FR-52 named then shipped".

- [ ] **Step 9: Revision history.** Append to §15's table:

```markdown
| 0.106 | 2026-10-22 | **PDF export, light** (FR-110, D-103, SC-30 new; FR-45, FR-52, FR-56, FR-57, FR-58, FR-64, FR-71, FR-91, FR-100, FR-103, FR-104, FR-105, FR-106, FR-107, D-100, G-7, §1, §6.6's table and §14.1 amended; crumb `rg-5uf`). `-o radar.pdf` writes the wide arrangement as a one-page vector PDF the size of the document, its text real text in an embedded Inter subset so it selects and searches, untagged (`rg-ffj`), with no dates or identifier. Written directly from the document with pdf-writer, since no SVG-to-PDF crate is on usvg 0.48. Paper sizes are PR 21's (`rg-4b8`). |
```

- [ ] **Step 10: Check and commit.**

Run: `/opt/homebrew/bin/rg -n "FR-110|D-103|SC-30" docs/product/radargen-prd.md`
Expected: each defined once (its row) and referenced from the amended rows.

```bash
git add docs/product/radargen-prd.md
git commit -m "docs(prd): PDF export, light (FR-110, D-103, SC-30; v0.106)

PR 20 (rg-5uf): -o radar.pdf writes the wide arrangement as a one-page
vector PDF with real, searchable text, untagged (rg-ffj), no dates or
ID. Written directly with pdf-writer: no SVG-to-PDF crate is on usvg
0.48. Every rule that lists the static formats now names PDF.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: What every static rendering shares: its glyphs, its ground, its size

**Files:**
- Modify: `src/svg/mod.rs` (add to `impl Node`, after `is_interactive_only`, and to `impl Document`; tests in its `mod tests`)
- Modify: `src/svg/serialise/outline.rs` (`keys`, the module doc; delete `runs`)
- Modify: `src/svg/serialise/mod.rs` (`body` paints `document.ground()`)
- Modify: `src/emit/mod.rs`, `src/emit/svg.rs` (`intrinsic_size` and its test move to `emit/mod.rs`)

**Interfaces:**
- Produces: `Node::static_runs(&self) -> Vec<(TextStyle, &str)>`, used by Task 5 (`pdf::font`) and by `outline.rs`; `Document::ground(&self) -> Node`, used by Task 6; `emit::intrinsic_size(&Document, Option<u32>) -> (f64, f64)`, used by Task 8.

A refactor, its own commit: the SVG goldens must not change by a byte. The PDF is a third static rendering, so the three things the SVG computed privately — the glyphs drawn, the D-87 ground and FR-57's size — move to where every static emitter reads them, and no emitter imports another.

- [ ] **Step 1: Write the failing test** in `src/svg/mod.rs`'s `mod tests` (create the module at the end of the file if it does not exist, with `use super::*;`):

```rust
    fn style(weight: Weight) -> TextStyle {
        TextStyle {
            weight,
            size: 12.0,
            letter_spacing: 0.0,
        }
    }

    fn text(lines: &[&str], tail: Option<Tail>) -> Node {
        Node::Text {
            origin: Point::ORIGIN,
            style: style(Weight::Regular),
            ink: TextPaint::Fill(Paint::None),
            anchor: Anchor::Start,
            lines: lines.iter().map(|&l| l.to_owned()).collect(),
            words: lines.join(" "),
            leading: 14.0,
            tail,
        }
    }

    /// The runs a static rendering draws: lines in the text's style, then
    /// its tail in its own; nothing under an interactive-only node; document
    /// order (D-78, D-88).
    #[test]
    fn static_runs_are_what_a_static_rendering_draws() {
        let tail = Tail {
            gap: 4.0,
            style: style(Weight::SemiBold),
            ink: Paint::None,
            text: "\u{a0}meta".to_owned(),
        };
        let indicator = Node::Group(
            Group::new(Point::ORIGIN, vec![text(&["hidden"], None)])
                .with(Semantics::Indicator(ItemKey(3))),
        );
        let root = Node::Group(Group::new(
            Point::ORIGIN,
            vec![
                text(&["one", "two"], Some(tail)),
                indicator,
                Node::HitArea {
                    centre: Point::ORIGIN,
                    radius: 4.0,
                    key: ItemKey(3),
                },
                text(&["three"], None),
            ],
        ));
        let got: Vec<(Weight, &str)> = root
            .static_runs()
            .into_iter()
            .map(|(s, t)| (s.weight, t))
            .collect();
        assert_eq!(
            got,
            [
                (Weight::Regular, "one"),
                (Weight::Regular, "two"),
                (Weight::SemiBold, "\u{a0}meta"),
                (Weight::Regular, "three"),
            ]
        );
        assert!(Node::HitArea { centre: Point::ORIGIN, radius: 1.0, key: ItemKey(1) }
            .text_runs()
            .is_empty(), "only a text has runs");
    }
```

Add `use crate::text::Weight;` to the test module's imports if `Weight` is not already in scope.

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --lib svg::tests::static_runs_are_what_a_static_rendering_draws`
Expected: FAIL to compile, `no method named static_runs`.

- [ ] **Step 3: Implement** in `impl Node` in `src/svg/mod.rs`, after `is_interactive_only`:

```rust
    /// The runs this text draws, in paint order: each line in its style,
    /// then its tail in its own (D-78). Empty for anything but a text.
    #[must_use]
    fn text_runs(&self) -> Vec<(TextStyle, &str)> {
        match self {
            Self::Text {
                style, lines, tail, ..
            } => lines
                .iter()
                .map(|line| (*style, line.as_str()))
                .chain(tail.iter().map(|t| (t.style, t.text.as_str())))
                .collect(),
            Self::Group(_)
            | Self::Circle { .. }
            | Self::Rect { .. }
            | Self::Line { .. }
            | Self::Path { .. }
            | Self::HitArea { .. } => Vec::new(),
        }
    }

    /// Every run a static rendering draws, in document order: the
    /// runs of each text (its lines, then its tail) outside an
    /// interactive-only node. The exported SVG's glyph table (D-88) and
    /// the PDF's font subsets (FR-110) both collect through this, so
    /// neither can miss a glyph the writer then places.
    #[must_use]
    pub fn static_runs(&self) -> Vec<(TextStyle, &str)> {
        match self {
            _ if self.is_interactive_only() => Vec::new(),
            Self::Group(group) => group.children.iter().flat_map(Self::static_runs).collect(),
            _ => self.text_runs(),
        }
    }
```

In `src/svg/serialise/outline.rs`, replace `keys` with:

```rust
/// Each glyph `node` draws, keyed by face, size and id.
fn keys(node: &Node, fonts: &Fonts) -> BTreeSet<Key> {
    node.static_runs()
        .into_iter()
        .flat_map(|(style, text)| {
            fonts
                .face(style.weight)
                .shape(text)
                .into_iter()
                .map(move |glyph| Key::new(style, glyph.id))
        })
        .collect()
}
```

The `text_runs` match lists every variant, with no `_` arm, so a future variant that carries text fails to compile here rather than being skipped by both writers. These are all seven of today's variants.

Add to `impl Document`, with a test beside the one above asserting `doc.ground()` is a `Node::Rect` at the origin of `doc.width × doc.height`, radius 0, filled with `Paint::Role(Role::Neutral(Neutral::Surface1))`, unstroked:

```rust
    /// The ground a static rendering paints under the drawing (D-87): the
    /// theme's `Surface1` over the whole document. The exported SVG and
    /// the PDF both paint this one rect.
    #[must_use]
    pub const fn ground(&self) -> Node {
        Node::Rect {
            origin: Point::ORIGIN,
            width: self.width,
            height: self.height,
            radius: 0.0,
            fill: Paint::Role(Role::Neutral(Neutral::Surface1)),
            stroke: Paint::None,
            stroke_width: 0.0,
        }
    }
```

(If `const fn` does not compile for the struct literal, drop `const`; clippy's `missing_const_for_fn` decides.) In `src/svg/serialise/mod.rs`'s `body`, replace the inline `let ground = Node::Rect { … };` with `let ground = document.ground();` and drop imports it no longer uses.

Move `intrinsic_size` (with its doc and its `the_intrinsic_size_follows_the_width` test) from `src/emit/svg.rs` to `src/emit/mod.rs`, beside `OUTPUT_VERSION` and `UNTITLED`, and have `emit/svg.rs` call `super::intrinsic_size`. Update any other caller the compiler names.

Delete the `runs` function. In the module doc, replace "Collecting and writing read text through one function, [`runs`], so the table holds every glyph the writer places." with "Collecting reads text through [`Node::static_runs`], which the PDF's subsets share, so the table holds every glyph the writer places." and in `Glyphs::entry`'s doc replace "collected through [`runs`]" with "collected through [`Node::static_runs`]" (both the doc and the `expect` message: `"every placed glyph was collected through Node::static_runs"`). Remove imports `outline.rs` no longer uses (`Tail` if unused); let the compiler say which.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --lib svg:: emit:: && ~/.cargo/bin/cargo test --test serialise --test svg_matches_png`
Expected: PASS, the SVG goldens unchanged.

- [ ] **Step 5: Commit**

```bash
git add src/svg/mod.rs src/svg/serialise/outline.rs src/svg/serialise/mod.rs src/emit/mod.rs src/emit/svg.rs
git commit -m "refactor(svg): the glyphs, ground and size every static output shares

The SVG outliner collected its glyph table through a private walk, the
serialiser built the D-87 ground inline and emit::svg owned FR-57's
intrinsic size. The PDF (PR 20) needs all three, so they move to
Node::static_runs, Document::ground and emit::intrinsic_size, where
every static emitter reads them and none imports another. No output
changes.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Béziers for circles, rounded rectangles and arcs

**Files:**
- Modify: `Cargo.toml` (dependencies; see Step 3)
- Create: `src/pdf/mod.rs`, `src/pdf/geometry.rs`
- Modify: `src/lib.rs` (`mod pdf;`, private, after `pub mod palette;`)

**Interfaces:**
- Produces:
  - `pdf::real(v: f64) -> f32`
  - `pdf::geometry::Step { Move(Point), Line(Point), Cubic { c1: Point, c2: Point, end: Point }, Close }`
  - `pdf::geometry::circle(centre: Point, radius: f64) -> Vec<Step>`
  - `pdf::geometry::rect(origin: Point, width: f64, height: f64, radius: f64) -> Vec<Step>`
  - `pdf::geometry::path(commands: &[PathCommand]) -> Vec<Step>`

- [ ] **Step 1: Write the failing tests** at the end of the new `src/pdf/geometry.rs` (start the file with just the module doc and `#[cfg(test)] mod tests` so it compiles to a failure on the missing functions):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// The point a cubic from `p0` reaches at `t`.
    #[expect(
        clippy::suboptimal_flops,
        clippy::many_single_char_names,
        reason = "the Bernstein form as written, for a test"
    )]
    fn at(p0: Point, c1: Point, c2: Point, p3: Point, t: f64) -> Point {
        let u = 1.0 - t;
        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        Point {
            x: a * p0.x + b * c1.x + c * c2.x + d * p3.x,
            y: a * p0.y + b * c1.y + c * c2.y + d * p3.y,
        }
    }

    /// The largest distance from `centre` minus `radius` along `steps`,
    /// sampled 32 times per cubic.
    fn worst(steps: &[Step], centre: Point, radius: f64) -> f64 {
        let mut pen = Point::ORIGIN;
        let mut worst: f64 = 0.0;
        for step in steps {
            match *step {
                Step::Move(p) | Step::Line(p) => pen = p,
                Step::Cubic { c1, c2, end } => {
                    for i in 0..=32 {
                        let p = at(pen, c1, c2, end, f64::from(i) / 32.0);
                        let d = ((p.x - centre.x).hypot(p.y - centre.y) - radius).abs();
                        worst = worst.max(d);
                    }
                    pen = end;
                }
                Step::Close => {}
            }
        }
        worst
    }

    /// A circle is four quarter cubics, closed, within the quarter-cubic
    /// approximation's error of 2.8e-4 of the radius.
    #[test]
    fn a_circle_is_four_quarters() {
        let centre = Point { x: 10.0, y: 20.0 };
        let steps = circle(centre, 100.0);
        assert_eq!(steps.len(), 6, "move, four cubics, close: {steps:?}");
        assert_eq!(steps.first(), Some(&Step::Move(Point { x: 110.0, y: 20.0 })));
        assert_eq!(steps.last(), Some(&Step::Close));
        assert!(worst(&steps, centre, 100.0) < 100.0 * 2.8e-4);
    }

    /// Square corners are four lines; round ones add a quarter cubic each,
    /// `rx` clamped to half the width and `ry` (which is `rx`) to half the
    /// height, as SVG clamps each (the SVG writes `rx` alone).
    #[test]
    fn rects_square_and_round() {
        let square = rect(Point::ORIGIN, 40.0, 20.0, 0.0);
        assert_eq!(
            square,
            [
                Step::Move(Point::ORIGIN),
                Step::Line(Point { x: 40.0, y: 0.0 }),
                Step::Line(Point { x: 40.0, y: 20.0 }),
                Step::Line(Point { x: 0.0, y: 20.0 }),
                Step::Close,
            ]
        );
        let round = rect(Point::ORIGIN, 40.0, 20.0, 50.0);
        let cubics = round.iter().filter(|s| matches!(s, Step::Cubic { .. })).count();
        assert_eq!(cubics, 4, "{round:?}");
        assert!(round.contains(&Step::Move(Point { x: 20.0, y: 0.0 })), "rx 50 clamps to 20: {round:?}");
        assert!(round.contains(&Step::Line(Point { x: 40.0, y: 10.0 })), "ry 50 clamps to 10: {round:?}");
    }

    /// SVG's arc flags: from (100, 0) around the origin to (0, 100). In
    /// SVG's y-down space, sweep 1 is clockwise on screen, through
    /// (70.7, 70.7); sweep 0 with the large arc goes the long way round,
    /// through (-100, 0). Both stay on the circle and end exactly at `end`.
    #[test]
    fn arcs_follow_svg_flags() {
        let start = Point { x: 100.0, y: 0.0 };
        let end = Point { x: 0.0, y: 100.0 };
        let short = path(&[
            PathCommand::MoveTo(start),
            PathCommand::Arc { radius: 100.0, large_arc: false, sweep: true, end },
        ]);
        let long = path(&[
            PathCommand::MoveTo(start),
            PathCommand::Arc { radius: 100.0, large_arc: true, sweep: false, end },
        ]);
        let cubics = |s: &[Step]| s.iter().filter(|s| matches!(s, Step::Cubic { .. })).count();
        assert_eq!(cubics(&short), 1, "90 degrees is one cubic: {short:?}");
        assert_eq!(cubics(&long), 3, "270 degrees is three: {long:?}");
        for steps in [&short, &long] {
            assert!(worst(steps, Point::ORIGIN, 100.0) < 100.0 * 2.8e-4);
            assert!(matches!(steps.last(), Some(Step::Cubic { end: e, .. }) if *e == end));
        }
        // The short arc's midpoint is in the lower right quadrant (y down).
        let Some(Step::Cubic { c1, c2, end: e }) = short.get(1).copied() else {
            panic!("a cubic: {short:?}");
        };
        let mid = at(start, c1, c2, e, 0.5);
        assert!(mid.x > 0.0 && mid.y > 0.0, "{mid:?}");
        // The long arc passes the far side, x near -100.
        let far = long.iter().any(|s| matches!(s, Step::Cubic { end: e, .. } if (e.x + 100.0).abs() < 1e-9));
        assert!(far, "{long:?}");
    }

    /// SVG F.6.6: a radius too small for the chord is scaled up until the
    /// arc is a half circle; a zero radius is a straight line; an arc to
    /// its own start draws nothing.
    #[test]
    fn degenerate_arcs() {
        let start = Point { x: 0.0, y: 0.0 };
        let end = Point { x: 10.0, y: 0.0 };
        let small = path(&[
            PathCommand::MoveTo(start),
            PathCommand::Arc { radius: 1.0, large_arc: false, sweep: true, end },
        ]);
        assert!(worst(&small, Point { x: 5.0, y: 0.0 }, 5.0) < 5.0 * 2.8e-4, "{small:?}");
        let zero = path(&[
            PathCommand::MoveTo(start),
            PathCommand::Arc { radius: 0.0, large_arc: false, sweep: true, end },
        ]);
        assert_eq!(zero, [Step::Move(start), Step::Line(end)]);
        let none = path(&[
            PathCommand::MoveTo(start),
            PathCommand::Arc { radius: 5.0, large_arc: false, sweep: true, end: start },
        ]);
        assert_eq!(none, [Step::Move(start)]);
    }

    /// NFR-17 for PDF: thousandths, half away from zero, then f32.
    #[test]
    fn real_rounds_as_fmt_coord_does() {
        assert_eq!(crate::pdf::real(1.0005), 1.001);
        assert_eq!(crate::pdf::real(-7.654_32), -7.654);
        assert_eq!(crate::pdf::real(1e-13), 0.0);
        assert_eq!(crate::pdf::real(260.0), 260.0);
    }
}
```

`src/pdf/mod.rs` for now:

```rust
//! The PDF export's writer (FR-110, D-103): the shared document drawn as
//! one vector page, its text real text in an embedded Inter subset.

pub mod geometry;
```

and `mod pdf;` in `src/lib.rs` (private: the writer's internals are not public API; `emit::pdf` is the entry point).

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --lib pdf::`
Expected: FAIL to compile: `cannot find function circle`, `cannot find type Step`, `cannot find function real`.

- [ ] **Step 3: Add the dependencies** (the later tasks need them; adding them now keeps the lockfile change in one commit):

```bash
~/.cargo/bin/cargo add pdf-writer@0.15
~/.cargo/bin/cargo add subsetter@0.2 --no-default-features
~/.cargo/bin/cargo add miniz_oxide@0.9
~/.cargo/bin/cargo add --dev hayro@0.8
~/.cargo/bin/cargo add --dev lopdf@0.46 --no-default-features
```

Then give each a comment in `Cargo.toml`, matching the file's style:

```toml
# PDF export (FR-110, D-103): written directly from the document, so no
# usvg in its path; no SVG-to-PDF crate is on usvg 0.48.
pdf-writer = "0.15"
# The PDF's embedded Inter subsets. No variable-font support: the vendored
# faces are static, and the feature would pull a second skrifa.
subsetter = { version = "0.2", default-features = false }
# The PDF's font stream (FlateDecode). 0.9 is the version flate2 already
# builds, so no third copy; png's direct 0.8 is the duplicate rg-42m
# tracks, and it goes when png moves to 0.9.
miniz_oxide = "0.9"
```

and under `[dev-dependencies]`:

```toml
# SC-30's render oracle: an independent pure-Rust PDF rasteriser, so the
# PDF is checked by a reading of the format that is not ours. Its second
# skrifa, read-fonts and fearless_simd are dev-only, which
# clippy::multiple_crate_versions does not count.
hayro = "0.8"
# Reads the PDF's text and structure back in tests (SC-30).
lopdf = { version = "0.46", default-features = false }
```

Run: `~/.cargo/bin/cargo tree -d -e normal --target all | /opt/homebrew/bin/rg '^[a-z]'`
Expected: only the pairs `clippy.toml` already allows (miniz_oxide, syn, bitflags) and same-version feature splits (jiff-core). Anything else in the **normal** graph: stop and report. The dev graph (`cargo tree -d`) adds hayro's skrifa, read-fonts, fearless_simd, fastrand and syn; that is expected.

Confirm lopdf without default features still inflates FlateDecode (the ToUnicode maps are uncompressed, but Task 8's golden inflates the FontFile2 to summarise it): `~/.cargo/bin/cargo tree -e normal,dev -i flate2 2>/dev/null | /opt/homebrew/bin/rg lopdf` or read `lopdf`'s `Cargo.toml` for which feature carries `flate2`; if it is behind a default feature, add that one feature (`features = ["…"]`) and say so in the comment.

- [ ] **Step 4: Implement.** Add `real` to `src/pdf/mod.rs`:

```rust
/// The one path a number takes into the PDF (NFR-17): rounded to
/// thousandths, half away from zero, as [`fmt_coord`] rounds it, then
/// narrowed to the `f32` pdf-writer writes. The rounding absorbs a trig
/// residue that differs between platforms (NFR-4); the narrowing is
/// exact for the thousandths of any coordinate below 8192 and within a
/// thousandth below 16384, and deterministic beyond.
///
/// # Panics
///
/// On NaN, infinity or a magnitude of 10¹² or more, as [`fmt_coord`]
/// does: each can only come from a layout bug.
///
/// [`fmt_coord`]: crate::num::fmt_coord
#[must_use]
pub fn real(v: f64) -> f32 {
    assert!(
        v.is_finite() && v.abs() < 1e12,
        "pdf::real: {v} is not a drawable coordinate"
    );
    // pdf-writer writes an integral float, -0.0 included, as an integer,
    // so `-0` never reaches the file.
    narrow((v * 1000.0).round() / 1000.0)
}

/// The page's px-to-pt scale (FR-110), narrowed unrounded. It is a
/// quotient of two layout numbers with no trigonometry in it, so it is
/// already the same on every platform; rounding it to thousandths, as
/// [`real`] rounds a coordinate, would mis-size the drawing against the
/// MediaBox by up to 0.3% at `--width 320` and leave a sliver of paper at
/// the page's edge.
///
/// # Panics
///
/// On a scale that is not finite and positive: a layout bug.
#[must_use]
pub fn scale(v: f64) -> f32 {
    assert!(v.is_finite() && v > 0.0, "pdf::scale: {v} is not a page scale");
    narrow(v)
}
```

and the one narrowing both share:

```rust
/// `f64` to the `f32` pdf-writer writes: the crate's one narrowing.
#[expect(
    clippy::cast_possible_truncation,
    clippy::as_conversions,
    reason = "f32 has no From<f64>; pdf-writer writes f32, and every caller has made the value deterministic first"
)]
const fn narrow(v: f64) -> f32 {
    v as f32
}
```

Add a test beside `real`'s: `scale(0.18987)` is `0.18987_f32` (not `0.19`), and `scale(0.0)` panics (`#[should_panic(expected = "pdf::scale")]`).

`src/pdf/geometry.rs`, above the tests:

```rust
//! Circles, rounded rectangles and SVG circular arcs as cubic Béziers:
//! PDF has lines and cubics but no arcs (FR-110). Every arc is cut into
//! pieces of at most a quarter turn, whose cubic stays within 2.8e-4 of
//! the radius.

use std::f64::consts::{FRAC_PI_2, TAU};

use crate::svg::{PathCommand, Point};

/// One step of a PDF path, in document units (y down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Step {
    /// Start a sub-path.
    Move(Point),
    /// A straight edge to here.
    Line(Point),
    /// A cubic curve: two control points, then where it ends.
    Cubic {
        /// The first control point.
        c1: Point,
        /// The second control point.
        c2: Point,
        /// Where the curve ends.
        end: Point,
    },
    /// Close the sub-path.
    Close,
}

/// The control-arm length for a quarter turn of a unit circle:
/// 4/3 · tan(π/8).
const QUARTER: f64 = 0.552_284_749_830_793_4;

/// A point at `angle` (radians, y down) on the circle at `centre`.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn on(centre: Point, radius: f64, angle: f64) -> Point {
    Point {
        x: centre.x + radius * angle.cos(),
        y: centre.y + radius * angle.sin(),
    }
}

/// One cubic for the arc from angle `a` to `b` (at most a quarter turn)
/// ending exactly at `end`.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn piece(centre: Point, radius: f64, a: f64, b: f64, end: Point) -> Step {
    let k = 4.0 / 3.0 * ((b - a) / 4.0).tan() * radius;
    let start = on(centre, radius, a);
    Step::Cubic {
        c1: Point {
            x: start.x - k * a.sin(),
            y: start.y + k * a.cos(),
        },
        c2: Point {
            x: end.x + k * b.sin(),
            y: end.y - k * b.cos(),
        },
        end,
    }
}

/// The circle at `centre`: a move to its rightmost point, four quarter
/// cubics clockwise on screen, and a close.
#[must_use]
pub fn circle(centre: Point, radius: f64) -> Vec<Step> {
    let mut steps = vec![Step::Move(on(centre, radius, 0.0))];
    for quarter in 0_u8..4 {
        let a = f64::from(quarter) * FRAC_PI_2;
        let b = a + FRAC_PI_2;
        steps.push(piece(centre, radius, a, b, on(centre, radius, b)));
    }
    steps.push(Step::Close);
    steps
}

/// A rectangle from `origin`, `radius` on every corner as the SVG
/// serialiser writes it: `rx` only, so `ry` is `rx`, and each is clamped
/// to half its own side (SVG 1.1 §9.2), which makes a radius past half
/// the height an elliptical corner. Square corners are plain lines.
#[must_use]
pub fn rect(origin: Point, width: f64, height: f64, radius: f64) -> Vec<Step> {
    let (x0, y0, x1, y1) = (origin.x, origin.y, origin.x + width, origin.y + height);
    let p = |x, y| Point { x, y };
    let rx = radius.min(width / 2.0).max(0.0);
    let ry = radius.min(height / 2.0).max(0.0);
    if crate::num::fmt_coord(rx) == "0" || crate::num::fmt_coord(ry) == "0" {
        return vec![
            Step::Move(p(x0, y0)),
            Step::Line(p(x1, y0)),
            Step::Line(p(x1, y1)),
            Step::Line(p(x0, y1)),
            Step::Close,
        ];
    }
    let (kx, ky) = (QUARTER * rx, QUARTER * ry);
    vec![
        Step::Move(p(x0 + rx, y0)),
        Step::Line(p(x1 - rx, y0)),
        Step::Cubic { c1: p(x1 - rx + kx, y0), c2: p(x1, y0 + ry - ky), end: p(x1, y0 + ry) },
        Step::Line(p(x1, y1 - ry)),
        Step::Cubic { c1: p(x1, y1 - ry + ky), c2: p(x1 - rx + kx, y1), end: p(x1 - rx, y1) },
        Step::Line(p(x0 + rx, y1)),
        Step::Cubic { c1: p(x0 + rx - kx, y1), c2: p(x0, y1 - ry + ky), end: p(x0, y1 - ry) },
        Step::Line(p(x0, y0 + ry)),
        Step::Cubic { c1: p(x0, y0 + ry - ky), c2: p(x0 + rx - kx, y0), end: p(x0 + rx, y0) },
        Step::Close,
    ]
}

/// A path's commands as steps: each `Arc` from the current point, by SVG's
/// endpoint-to-centre conversion for a circle (SVG 1.1 F.6.5, F.6.6).
#[must_use]
pub fn path(commands: &[PathCommand]) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut pen = Point::ORIGIN;
    for command in commands {
        match *command {
            PathCommand::MoveTo(p) => {
                steps.push(Step::Move(p));
                pen = p;
            }
            PathCommand::Arc { radius, large_arc, sweep, end } => {
                steps.extend(arc(pen, radius, large_arc, sweep, end));
                pen = end;
            }
        }
    }
    steps
}

/// SVG F.6.5 for `rx = ry = radius`, no rotation, in y-down space.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn arc(from: Point, radius: f64, large_arc: bool, sweep: bool, end: Point) -> Vec<Step> {
    if from == end {
        return Vec::new();
    }
    let mut r = radius.abs();
    if r == 0.0 {
        return vec![Step::Line(end)];
    }
    // Half the chord, in a frame centred on its midpoint.
    let (hx, hy) = ((from.x - end.x) / 2.0, (from.y - end.y) / 2.0);
    let d2 = hx * hx + hy * hy;
    // F.6.6: a radius too small for the chord grows to reach it.
    r = r.max(d2.sqrt());
    let sign = if large_arc == sweep { -1.0 } else { 1.0 };
    let coef = sign * ((r * r - d2) / d2).max(0.0).sqrt();
    let centre = Point {
        x: coef * hy + (from.x + end.x) / 2.0,
        y: -coef * hx + (from.y + end.y) / 2.0,
    };
    let start = (from.y - centre.y).atan2(from.x - centre.x);
    let mut delta = (end.y - centre.y).atan2(end.x - centre.x) - start;
    if sweep && delta < 0.0 {
        delta += TAU;
    } else if !sweep && delta > 0.0 {
        delta -= TAU;
    }
    // The 1e-9 keeps an exact quarter or half turn (four- and two-sector
    // radars) from tipping into one more piece on a platform whose atan2
    // rounds its last bit up (NFR-4).
    let pieces = (delta.abs() / FRAC_PI_2 - 1e-9).ceil().max(1.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::as_conversions,
        reason = "at most 4, from the ceiling of a quarter-turn count of at most a full turn"
    )]
    let count = pieces as u8;
    let step = delta / pieces;
    (1..=count)
        .map(|i| {
            let a = start + f64::from(i - 1) * step;
            let b = start + f64::from(i) * step;
            let to = if i == count { end } else { on(centre, r, b) };
            piece(centre, r, a, b, to)
        })
        .collect()
}
```

If clippy flags the `pieces` cast more strictly than the `#[expect]` lists, add the lint it names to that same `#[expect]`; do not add `#[allow]`.

- [ ] **Step 5: Run the tests and lints**

Run: `~/.cargo/bin/cargo test --lib pdf:: && ~/.cargo/bin/cargo clippy --all-targets -- -D warnings`
Expected: PASS; no warnings. (Unused `pdf-writer`, `subsetter` and `miniz_oxide` are not lint errors for a library crate's dependencies; if `unused_crate_dependencies` is enabled and fires, move Step 3 to the start of Task 5 instead.)

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/pdf/mod.rs src/pdf/geometry.rs
git commit -m "feat(pdf): Béziers for circles, rounded rects and SVG arcs

PDF has no arc operator, so the document's circles, rounded rects and
circular arcs become quarter-turn cubics (FR-110). pdf::real is NFR-17's
rounding for the f32 numbers pdf-writer writes. Adds pdf-writer,
subsetter (no variable fonts) and miniz_oxide 0.8, and lopdf for tests.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: What a PDF font needs from a face

**Files:**
- Modify: `src/text.rs` (`FontMetrics`, `FontError`, `Weight`; tests in its `mod tests`)

**Interfaces:**
- Produces:
  - `FontError::new(face: &'static str, reason: String) -> FontError`
  - `Weight::postscript(self) -> &'static str` (`"Inter-Regular"`, `"Inter-SemiBold"`)
  - `FontMetrics::bytes(&self) -> &'a [u8]` (the TTF)
  - `FontMetrics::name(&self) -> &'static str` (`"Inter Regular"`, for errors)
  - `FontMetrics::advance(&self, glyph: u32) -> Result<f64, FontError>` (font units)
  - `FontMetrics::descriptor(&self) -> Result<Descriptor, FontError>`
  - `pub struct Descriptor { pub bbox: [f64; 4], pub ascent: f64, pub descent: f64, pub cap_height: f64, pub italic_angle: f64 }` (font units; descent negative)

skrifa stays private to `text.rs`, as its module doc says; the PDF code reads these instead.

- [ ] **Step 1: Write the failing tests** in `src/text.rs`'s `mod tests`:

```rust
    /// What a PDF font object reads from a face (FR-110): its bytes, its
    /// PostScript name as the face's name table gives it, a glyph's advance
    /// as the shaper uses it, and the descriptor's metrics.
    #[test]
    fn a_face_describes_itself_for_pdf() {
        use skrifa::MetadataProvider as _;
        let fonts = Fonts::embedded().expect("embedded");
        for (weight, ttf) in [(Weight::Regular, REGULAR_TTF), (Weight::SemiBold, SEMIBOLD_TTF)] {
            let face = fonts.face(weight);
            assert_eq!(face.bytes(), ttf, "{weight:?}: the vendored TTF");
            let named = skrifa::FontRef::new(ttf)
                .expect("parses")
                .localized_strings(skrifa::string::StringId::POSTSCRIPT_NAME)
                .english_or_first()
                .expect("a PostScript name")
                .to_string();
            assert_eq!(weight.postscript(), named);
            // `A`'s advance is what the shaper advances by for "A" alone.
            let shaped = face.shape("A");
            let [glyph] = shaped.as_slice() else { panic!("one glyph: {shaped:?}") };
            assert_eq!(face.advance(glyph.id).expect("advance"), f64::from(glyph.x_advance));
            let d = face.descriptor().expect("descriptor");
            assert_eq!(d.ascent / face.units_per_em(), TEXT_ASCENT);
            assert_eq!(-d.descent / face.units_per_em(), TEXT_DESCENT);
            assert!(d.cap_height > 0.0 && d.bbox[2] > d.bbox[0] && d.bbox[3] > d.bbox[1], "{d:?}");
            assert_eq!(d.italic_angle, 0.0);
        }
        let missing = fonts.face(Weight::Regular).advance(u32::from(u16::MAX));
        assert!(missing.is_err(), "a glyph the face lacks is a FontError");
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --lib text::tests::a_face_describes_itself_for_pdf`
Expected: FAIL to compile: no `bytes`, `postscript`, `advance`, `descriptor`.

- [ ] **Step 3: Implement.** In `Weight`:

```rust
    /// The face's PostScript name, as its `name` table gives it: the PDF's
    /// `BaseFont` after the subset tag (FR-110).
    #[must_use]
    pub const fn postscript(self) -> &'static str {
        match self {
            Self::Regular => "Inter-Regular",
            Self::SemiBold => "Inter-SemiBold",
        }
    }
```

`FontError`:

```rust
impl FontError {
    /// A failure in `face` (`Inter Regular`), for a writer outside this
    /// module that reads the face's bytes, such as the PDF's subsetter.
    #[must_use]
    pub const fn new(face: &'static str, reason: String) -> Self {
        Self { face, reason }
    }
}
```

`FontMetrics` gains a `bytes: &'a [u8]` field, set in `load` (`bytes: data` — rename `load`'s parameter use accordingly; it is the slice passed in). Then, in `impl<'a> FontMetrics<'a>`:

```rust
    /// The face's TTF, for a writer that embeds it (FR-110).
    #[must_use]
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// The face as an error names it: `Inter Regular`.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.face
    }

    /// Glyph `glyph`'s advance in font units, from `hmtx`: what a PDF's
    /// widths array holds for it (FR-110).
    ///
    /// # Errors
    ///
    /// [`FontError`] when the face has no such glyph.
    pub fn advance(&self, glyph: u32) -> Result<f64, FontError> {
        use skrifa::MetadataProvider as _;
        use skrifa::instance::{LocationRef, Size};
        self.font
            .glyph_metrics(Size::unscaled(), LocationRef::default())
            .advance_width(skrifa::GlyphId::new(glyph))
            .map(f64::from)
            .ok_or_else(|| FontError::new(self.face, format!("glyph {glyph} is not in the face")))
    }

    /// What a PDF font descriptor states about the face, in font units.
    ///
    /// # Errors
    ///
    /// [`FontError`] when the face has no bounding box (`head`).
    pub fn descriptor(&self) -> Result<Descriptor, FontError> {
        use skrifa::MetadataProvider as _;
        use skrifa::instance::{LocationRef, Size};
        let m = self.font.metrics(Size::unscaled(), LocationRef::default());
        let b = m
            .bounds
            .ok_or_else(|| FontError::new(self.face, "the face has no bounding box".to_owned()))?;
        Ok(Descriptor {
            bbox: [b.x_min, b.y_min, b.x_max, b.y_max].map(f64::from),
            ascent: f64::from(m.ascent),
            descent: f64::from(m.descent),
            cap_height: m.cap_height.map_or(f64::from(m.ascent), f64::from),
            italic_angle: f64::from(m.italic_angle),
        })
    }
```

and, beside `ShapedGlyph`:

```rust
/// A face's metrics as a PDF font descriptor states them, in font units,
/// y up: the descent is negative (FR-110).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Descriptor {
    /// The face's bounding box: left, bottom, right, top.
    pub bbox: [f64; 4],
    /// Above the baseline.
    pub ascent: f64,
    /// Below the baseline, negative.
    pub descent: f64,
    /// The height of a capital letter.
    pub cap_height: f64,
    /// Zero for an upright face.
    pub italic_angle: f64,
}
```

If the test's `ascent`/`descent` equalities fail because `metrics()` reads `OS/2` typo metrics rather than `hhea` (the constants are `1984/2048` and `494/2048`), keep the test's intent by comparing with what `TEXT_ASCENT` documents its source as, and make the descriptor read that same table; do not loosen the assertion to a tolerance.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --lib text::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/text.rs
git commit -m "feat(text): the face's bytes, advances and descriptor for PDF

A PDF font object needs the TTF to subset, each glyph's hmtx advance for
its widths array, and the descriptor metrics (FR-110). skrifa stays
private to text.rs; the PDF writer reads these.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Font subsets, CIDs and the ToUnicode map

**Files:**
- Create: `src/pdf/font.rs`
- Modify: `src/pdf/mod.rs` (`pub mod font;`)

**Interfaces:**
- Consumes: `Node::static_runs` (Task 2); `FontMetrics::{bytes, advance, descriptor, name}`, `Weight::postscript`, `FontError::new` (Task 4); `pdf::real` (Task 3).
- Produces:
  - `pub struct Faces` with
    - `Faces::of<'a>(roots: impl IntoIterator<Item = &'a Node>, fonts: &Fonts) -> Result<Faces, FontError>` (the drawings one PDF's fonts serve: `[&document.root]` here; PR 21's report passes every page's, so one subset per weight spans them)
    - `Faces::cid(&self, weight: Weight, glyph: u32) -> u16`
    - `Faces::width(&self, weight: Weight, cid: u16) -> f32` (thousandths of an em, as the widths array writes it)
    - `Faces::resource(weight: Weight) -> pdf_writer::Name<'static>` (`F0`, `F1`)
    - `Faces::write(&self, pdf: &mut pdf_writer::Pdf, next: &mut pdf_writer::Ref, fonts: &Fonts) -> Result<Vec<(Weight, pdf_writer::Ref)>, FontError>` (the Type0 font's ref per used weight, in `Weight` order)
    - `Faces::unicode(&self, weight: Weight, cid: u16) -> Option<&str>` (for tests)

- [ ] **Step 1: Write the failing tests** in `src/pdf/font.rs`'s `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::block::fonts;
    use crate::layout::test_support::{PRD_EXAMPLE, plotted};

    fn prd() -> Document {
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        let blocks = Blocks::new(&catalogue, &order, Some("Technology Radar"), None, fonts());
        arrange(Arrangement::Wide, &blocks)
    }

    /// Every glyph the document draws has a CID; CIDs follow glyph ids, so
    /// they do not depend on where in the document a glyph first appears;
    /// .notdef is CID 0 (NFR-4).
    #[test]
    fn every_drawn_glyph_has_a_cid_in_glyph_order() {
        let doc = prd();
        let faces = Faces::of([&doc.root], fonts()).expect("subsets");
        for (style, text) in doc.root.static_runs() {
            for glyph in fonts().face(style.weight).shape(text) {
                let _ = faces.cid(style.weight, glyph.id);
            }
        }
        for weight in [Weight::Regular, Weight::SemiBold] {
            let subset = faces.subsets.get(&weight).expect("the PRD uses both weights");
            assert_eq!(subset.remapper.get(0), Some(0), ".notdef first");
            let gids: Vec<u16> = subset.remapper.remapped_gids().collect();
            let mut sorted = gids.clone();
            sorted.sort_unstable();
            assert_eq!(gids, sorted, "{weight:?}: CIDs ascend with glyph ids");
        }
    }

    /// The ToUnicode map gives back the text each glyph was shaped from,
    /// for every run of the PRD document (Review Focus 4).
    #[test]
    fn the_unicode_map_gives_the_text_back() {
        let doc = prd();
        let faces = Faces::of([&doc.root], fonts()).expect("subsets");
        for (style, text) in doc.root.static_runs() {
            let joined: String = fonts()
                .face(style.weight)
                .shape(text)
                .into_iter()
                .filter_map(|g| faces.unicode(style.weight, faces.cid(style.weight, g.id)))
                .collect();
            assert_eq!(joined, text, "{style:?}");
        }
    }

    /// A cluster of several characters drawn as one glyph maps back to all
    /// of them, and a character drawn as several glyphs maps once.
    ///
    /// Before writing the asserts, probe Inter with a scratch test: which
    /// of `ffi`, `fi`, `fl` and `Office` shape to fewer glyphs than they
    /// have characters (Inter may not ligate by default), and which of
    /// `e` + U+0301 and a precomposed letter shape to a count different
    /// from their characters'. Pin each class that occurs, with an
    /// `assert!` that the shaping really is many-to-one or one-to-many
    /// before the round-trip assert, so the test cannot pass vacuously.
    /// A class Inter does not produce is named in this doc as absent.
    #[test]
    fn clusters_round_trip_whatever_their_shape() {
        let face = fonts().face(Weight::Regular);
        for text in ["Office", "e\u{301}"] {
            let shaped = face.shape(text);
            let run = Node::Text {
                origin: crate::svg::Point::ORIGIN,
                style: crate::svg::TextStyle { weight: Weight::Regular, size: 12.0, letter_spacing: 0.0 },
                ink: crate::svg::TextPaint::Fill(crate::svg::Paint::None),
                anchor: crate::svg::Anchor::Start,
                lines: vec![text.to_owned()],
                words: text.to_owned(),
                leading: 14.0,
                tail: None,
            };
            let faces = Faces::of([&run], fonts()).expect("subsets");
            let joined: String = shaped
                .iter()
                .filter_map(|g| faces.unicode(Weight::Regular, faces.cid(Weight::Regular, g.id)))
                .collect();
            assert_eq!(joined, text);
        }
    }

    /// D-103: a glyph two texts reach maps to the first text's spelling.
    /// `Café` (U+00E9) then `Cafe` + U+0301: the shaper composes both to
    /// one glyph, which copies as U+00E9.
    #[test]
    fn a_shared_glyph_maps_to_its_first_text() {
        let face = fonts().face(Weight::Regular);
        let (nfc, nfd) = ("Caf\u{e9}", "Cafe\u{301}");
        assert_eq!(
            face.shape(nfc).iter().map(|g| g.id).collect::<Vec<_>>(),
            face.shape(nfd).iter().map(|g| g.id).collect::<Vec<_>>(),
            "precondition: the shaper draws both spellings with the same glyphs"
        );
        let csv = format!("name,sector,ring,description\n{nfc},Tools,Adopt,d\n{nfd} x,Tools,Adopt,d\n");
        let (catalogue, order) = plotted(&csv, None);
        let blocks = Blocks::new(&catalogue, &order, None, None, fonts());
        let doc = arrange(Arrangement::Wide, &blocks);
        let faces = Faces::of([&doc.root], fonts()).expect("subsets");
        let e = face.shape(nfc).last().expect("a glyph").id;
        assert_eq!(faces.unicode(Weight::Regular, faces.cid(Weight::Regular, e)), Some("\u{e9}"));
    }

    /// A drawing in one weight writes one font, F0 only.
    #[test]
    fn one_weight_writes_one_font() {
        let run = Node::Text {
            origin: crate::svg::Point::ORIGIN,
            style: crate::svg::TextStyle { weight: Weight::Regular, size: 12.0, letter_spacing: 0.0 },
            ink: crate::svg::TextPaint::Fill(crate::svg::Paint::None),
            anchor: crate::svg::Anchor::Start,
            lines: vec!["Adopt".to_owned()],
            words: "Adopt".to_owned(),
            leading: 14.0,
            tail: None,
        };
        let faces = Faces::of([&run], fonts()).expect("subsets");
        let mut pdf = pdf_writer::Pdf::new();
        let mut next = pdf_writer::Ref::new(1);
        let refs = faces.write(&mut pdf, &mut next, fonts()).expect("writes");
        assert_eq!(refs.iter().map(|(w, _)| *w).collect::<Vec<_>>(), [Weight::Regular]);
    }

    /// A text the face cannot draw shapes to .notdef, which is in every
    /// subset and maps to nothing (Review Focus 2).
    #[test]
    fn notdef_is_drawn_and_maps_to_nothing() {
        let (catalogue, order) = plotted("name,sector,ring,description\n中文,Tools,Adopt,d\n", None);
        let blocks = Blocks::new(&catalogue, &order, None, None, fonts());
        let doc = arrange(Arrangement::Wide, &blocks);
        assert!(
            fonts().face(Weight::Regular).shape("中文").iter().all(|g| g.id == 0),
            "precondition: Inter draws 中文 as .notdef"
        );
        let faces = Faces::of([&doc.root], fonts()).expect("subsets");
        assert_eq!(faces.cid(Weight::Regular, 0), 0);
        assert_eq!(faces.unicode(Weight::Regular, 0), None);
    }

    /// The subset tag is six capitals, fixed by the face and its glyphs.
    #[test]
    fn the_tag_is_six_capitals_from_the_glyphs() {
        let a = tag("Inter-Regular", &[0, 3, 9]);
        assert_eq!(a.len(), 6);
        assert!(a.bytes().all(|b| b.is_ascii_uppercase()), "{a}");
        assert_eq!(a, tag("Inter-Regular", &[0, 3, 9]));
        assert_ne!(a, tag("Inter-Regular", &[0, 3, 10]));
        assert_ne!(a, tag("Inter-SemiBold", &[0, 3, 9]));
    }

    /// Writing produces one Type0 font per used weight, in Weight order,
    /// and the same bytes twice.
    #[test]
    fn fonts_write_deterministically() {
        let doc = prd();
        let write = || {
            let faces = Faces::of([&doc.root], fonts()).expect("subsets");
            let mut pdf = pdf_writer::Pdf::new();
            let mut next = pdf_writer::Ref::new(1);
            let refs = faces.write(&mut pdf, &mut next, fonts()).expect("writes");
            (refs, pdf.finish())
        };
        let (refs, first) = write();
        let (_, second) = write();
        assert_eq!(refs.iter().map(|(w, _)| *w).collect::<Vec<_>>(), [Weight::Regular, Weight::SemiBold]);
        assert!(first == second, "NFR-4");
        let text = String::from_utf8_lossy(&first);
        assert!(text.contains("/Identity-H"), "Identity-H encoding");
        assert!(text.contains("+Inter-Regular"), "a tagged BaseFont");
        assert!(text.contains("/FontFile2"), "embedded TrueType");
        assert!(text.contains("beginbfchar"), "a ToUnicode map");
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --lib pdf::font::`
Expected: FAIL to compile: no `Faces`, `tag`.

- [ ] **Step 3: Implement** `src/pdf/font.rs`:

```rust
//! The PDF's fonts (FR-110): per weight, an embedded subset of the
//! vendored Inter TTF holding exactly the glyphs the document draws, as a
//! Type0 font over a CIDFontType2 with Identity-H encoding, a widths
//! array and a ToUnicode map, so the text selects and searches.
//!
//! CIDs are the subset's glyph ids, assigned in ascending original glyph
//! id, so they depend only on which glyphs are drawn (NFR-4).

use std::collections::{BTreeMap, BTreeSet};

use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Filter, Finish, Name, Pdf, Rect, Ref, Str};
use subsetter::GlyphRemapper;

use super::real;
use crate::svg::Node;
use crate::text::{FontError, FontMetrics, Fonts, Weight};

/// The width of a PDF glyph space em.
const EM: f64 = 1000.0;
/// The font stream's deflate level: the default, fixed so the bytes are.
const LEVEL: u8 = 6;

/// One weight's subset.
struct Subset {
    remapper: GlyphRemapper,
    /// Each CID's text, from the shaper's clusters.
    unicode: BTreeMap<u16, String>,
    /// Each CID's advance in thousandths of an em, by CID.
    widths: Vec<f32>,
}

/// The weights a document draws, each with its subset.
pub struct Faces {
    subsets: BTreeMap<Weight, Subset>,
}

impl Faces {
    /// The subsets `document`'s static rendering needs.
    ///
    /// # Errors
    ///
    /// [`FontError`] when a shaped glyph id does not fit 16 bits or has
    /// no advance (NFR-15).
    pub fn of<'a>(
        roots: impl IntoIterator<Item = &'a Node>,
        fonts: &Fonts,
    ) -> Result<Self, FontError> {
        // Each run shaped once, in document order: its weight, and each
        // glyph with the text of the cluster it starts (empty for the rest
        // of a cluster).
        let mut shaped: Vec<(Weight, Vec<(u16, String)>)> = Vec::new();
        for (style, text) in roots.into_iter().flat_map(Node::static_runs) {
            let face = fonts.face(style.weight);
            let glyphs = clusters(face, text)
                .into_iter()
                .map(|(glyph, chars)| Ok((gid(face, glyph)?, chars)))
                .collect::<Result<Vec<_>, FontError>>()?;
            shaped.push((style.weight, glyphs));
        }
        let mut used: BTreeMap<Weight, BTreeSet<u16>> = BTreeMap::new();
        for (weight, run) in &shaped {
            used.entry(*weight).or_default().extend(run.iter().map(|&(g, _)| g));
        }
        let mut subsets = BTreeMap::new();
        for (weight, set) in used {
            let face = fonts.face(weight);
            let sorted: Vec<u16> = set.into_iter().collect();
            let remapper = GlyphRemapper::new_from_glyphs_sorted(&sorted);
            let scale = EM / face.units_per_em();
            let widths = remapper
                .remapped_gids()
                .map(|g| face.advance(u32::from(g)).map(|a| real(a * scale)))
                .collect::<Result<_, _>>()?;
            subsets.insert(weight, Subset { remapper, unicode: BTreeMap::new(), widths });
        }
        // The ToUnicode map, first mapping wins, in document order (D-103).
        // A glyph that is its cluster alone (a letter, a ligature) maps
        // first; the first glyph of a cluster of several (a base drawn
        // with a separate mark) takes the cluster's text only if nothing
        // mapped it alone, so a plain `e` never copies as `e` + U+0301.
        for alone in [true, false] {
            for (weight, run) in &shaped {
                let Some(subset) = subsets.get_mut(weight) else { continue };
                for (i, (glyph, chars)) in run.iter().enumerate() {
                    let single = run.get(i + 1).is_none_or(|(_, next)| !next.is_empty());
                    if single != alone || chars.is_empty() {
                        continue;
                    }
                    if let Some(cid) = subset.remapper.get(*glyph)
                        && cid != 0
                    {
                        subset.unicode.entry(cid).or_insert_with(|| chars.clone());
                    }
                }
            }
        }
        Ok(Self { subsets })
    }

    /// The CID `glyph` is drawn with in `weight`'s subset.
    ///
    /// # Panics
    ///
    /// If the glyph was not collected: every placed glyph is drawn from a
    /// [`Node::static_runs`](crate::svg::Node::static_runs) run, which
    /// [`Self::of`] collected; a miss is a broken invariant.
    #[must_use]
    pub fn cid(&self, weight: Weight, glyph: u32) -> u16 {
        self.subsets
            .get(&weight)
            .and_then(|s| u16::try_from(glyph).ok().and_then(|g| s.remapper.get(g)))
            .expect("every placed glyph was collected through Node::static_runs")
    }

    /// CID `cid`'s advance in thousandths of an em, as the widths array
    /// writes it, so a viewer's pen and ours agree.
    ///
    /// # Panics
    ///
    /// If `cid` is not in `weight`'s subset (a broken invariant).
    #[must_use]
    pub fn width(&self, weight: Weight, cid: u16) -> f32 {
        *self
            .subsets
            .get(&weight)
            .and_then(|s| s.widths.get(usize::from(cid)))
            .expect("a CID comes from Self::cid")
    }

    /// CID `cid`'s text, as the ToUnicode map gives it.
    #[must_use]
    pub fn unicode(&self, weight: Weight, cid: u16) -> Option<&str> {
        self.subsets.get(&weight)?.unicode.get(&cid).map(String::as_str)
    }

    /// The page's resource name for `weight`'s font.
    #[must_use]
    pub const fn resource(weight: Weight) -> Name<'static> {
        match weight {
            Weight::Regular => Name(b"F0"),
            Weight::SemiBold => Name(b"F1"),
        }
    }

    /// Each used weight's Type0 font and the objects under it, numbered
    /// from `next` in a fixed order: Type0, CIDFont, descriptor, font
    /// file, ToUnicode.
    ///
    /// # Errors
    ///
    /// [`FontError`] when the subsetter or the face's metrics fail.
    pub fn write(
        &self,
        pdf: &mut Pdf,
        next: &mut Ref,
        fonts: &Fonts,
    ) -> Result<Vec<(Weight, Ref)>, FontError> {
        let mut out = Vec::new();
        for (&weight, subset) in &self.subsets {
            let face = fonts.face(weight);
            let [type0, cid_font, descriptor, file, cmap] =
                [(); 5].map(|()| next.bump());
            let gids: Vec<u16> = subset.remapper.remapped_gids().collect();
            let base = format!("{}+{}", tag(weight.postscript(), &gids), weight.postscript());
            let base = Name(base.as_bytes());
            let info = SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            };

            pdf.type0_font(type0)
                .base_font(base)
                .encoding_predefined(Name(b"Identity-H"))
                .descendant_font(cid_font)
                .to_unicode(cmap);

            let mut cid = pdf.cid_font(cid_font);
            cid.subtype(CidFontType::Type2)
                .base_font(base)
                .system_info(info)
                .font_descriptor(descriptor)
                .cid_to_gid_map_predefined(Name(b"Identity"));
            cid.widths().consecutive(0, subset.widths.iter().copied());
            cid.finish();

            let d = face.descriptor()?;
            let scale = EM / face.units_per_em();
            let [left, bottom, right, top] = d.bbox.map(|v| real(v * scale));
            pdf.font_descriptor(descriptor)
                .name(base)
                // SYMBOLIC: an Identity-H CIDFont has no standard Latin
                // encoding, and preflight tools flag NON_SYMBOLIC here.
                .flags(FontFlags::SYMBOLIC)
                .bbox(Rect::new(left, bottom, right, top))
                .italic_angle(real(d.italic_angle))
                .ascent(real(d.ascent * scale))
                .descent(real(d.descent * scale))
                .cap_height(real(d.cap_height * scale))
                .stem_v(real(10.0 + 0.244 * (f64::from(weight.css()) - 50.0)))
                .font_file2(file);

            let bytes = subsetter::subset(face.bytes(), 0, &subset.remapper)
                .map_err(|e| FontError::new(face.name(), format!("cannot be subset: {e}")))?;
            let packed = miniz_oxide::deflate::compress_to_vec_zlib(&bytes, LEVEL);
            pdf.stream(file, &packed)
                .filter(Filter::FlateDecode)
                .pair(Name(b"Length1"), i32::try_from(bytes.len()).map_err(|_| {
                    FontError::new(face.name(), "the subset is over 2 GiB".to_owned())
                })?);

            let mut map = UnicodeCmap::new(Name(b"Custom"), info);
            for (&cid, text) in &subset.unicode {
                map.pair_with_multiple(cid, text.chars());
            }
            pdf.cmap(cmap, &map.finish());
            out.push((weight, type0));
        }
        Ok(out)
    }
}

/// `glyph` as a 16-bit id, which TrueType and the subsetter use.
fn gid(face: &FontMetrics<'_>, glyph: u32) -> Result<u16, FontError> {
    u16::try_from(glyph)
        .map_err(|_| FontError::new(face.name(), format!("glyph {glyph} is past 16 bits")))
}

/// Each glyph `text` shapes to, with the text its cluster stands for: the
/// first glyph of a cluster carries the cluster's characters, the rest of
/// it none, so a ligature maps to its letters and a decomposition to its
/// character once.
fn clusters(face: &FontMetrics<'_>, text: &str) -> Vec<(u32, String)> {
    let shaped = face.shape(text);
    let starts: BTreeSet<usize> = shaped
        .iter()
        .filter_map(|g| usize::try_from(g.cluster).ok())
        .collect();
    let mut seen = BTreeSet::new();
    shaped
        .iter()
        .map(|g| {
            let start = usize::try_from(g.cluster)
                .expect("a cluster is the byte offset of a character in text");
            let chars = if seen.insert(start) {
                let end = starts.range(start + 1..).next().copied().unwrap_or(text.len());
                text.get(start..end)
                    .expect("a cluster is the byte offset of a character in text")
                    .to_owned()
            } else {
                String::new()
            };
            (g.id, chars)
        })
        .collect()
}

/// PDF 9.6.4's subset tag: six capitals, here from FNV-1a over the face's
/// name and its glyph ids, so a subset's tag depends only on what it
/// holds (NFR-4) and two different subsets rarely share one.
fn tag(postscript: &str, gids: &[u16]) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    let bytes = postscript
        .bytes()
        .chain(gids.iter().flat_map(|g| g.to_le_bytes()));
    let mut hash = bytes.fold(OFFSET, |h, b| (h ^ u64::from(b)).wrapping_mul(PRIME));
    (0..6)
        .map(|_| {
            let letter = b'A' + u8::try_from(hash % 26).expect("below 26");
            hash /= 26;
            char::from(letter)
        })
        .collect()
}
```

Notes for the implementer:
- `pdf_writer::types` is where `CidFontType`, `FontFlags`, `SystemInfo` and `UnicodeCmap` live in 0.15; if a path differs, follow the compiler (`cargo doc -p pdf-writer --open`).
- `UnicodeCmap::new` is generic over the glyph type; `u16` is its default. `pdf.cmap(ref, bytes)` writes a CMap stream; if 0.15 names it differently, use `pdf.stream(cmap, &bytes)`.
- `clusters`'s `expect`s state harfrust's guarantee that a cluster is a byte offset of `text`, as `FontMetrics::missing` does; `clusters` returns no `Result`, so `unwrap_in_result` does not apply. Give it a `# Panics` doc line.
- `Faces::of` has no `expect`: it returns `Result`, and `unwrap_in_result` is denied (precedent `src/csv/decode.rs:98`). Every weight in `shaped` is in `subsets` by construction; the `let … else { continue }` is the non-panicking statement of that.
- `stem_v`'s `10.0 + 0.244 * (…)` trips `suboptimal_flops`: give `write` the crate's usual `#[expect(clippy::suboptimal_flops, reason = "NFR-4: …")]`.
- `.finish()` on `CidFont` comes from the `pdf_writer::Finish` trait: `use pdf_writer::Finish;`.
- The test `every_drawn_glyph_has_a_cid_in_glyph_order` reads `faces.subsets` and `subset.remapper`, so they stay private fields readable from the same module's tests.

`src/pdf/mod.rs`: add `pub mod font;`.

- [ ] **Step 4: Run the tests and lints**

Run: `~/.cargo/bin/cargo test --lib pdf::font:: && ~/.cargo/bin/cargo clippy --all-targets -- -D warnings`
Expected: PASS; no warnings.

- [ ] **Step 5: Commit**

```bash
git add src/pdf/font.rs src/pdf/mod.rs
git commit -m "feat(pdf): Inter subsets with a ToUnicode map

Per weight, the glyphs the document draws become an embedded subset
(subsetter), written as Type0 over CIDFontType2 with Identity-H, a
widths array from hmtx and a ToUnicode map from the shaper's clusters,
so text in the PDF selects, copies and searches (FR-110). CIDs follow
glyph ids and the subset tag hashes the glyph set, so the bytes depend
only on what is drawn (NFR-4). Only the font file is compressed.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: The content stream: shapes, groups and colours

**Files:**
- Create: `src/pdf/content.rs`
- Modify: `src/pdf/mod.rs` (`pub mod content;`)

**Interfaces:**
- Consumes: `pdf::geometry::{Step, circle, rect, path}`, `pdf::{real, scale}` (Task 3); `pdf::font::Faces` (Task 5); `Document::ground` (Task 2).
- Produces:
  - `pub struct Canvas<'a> { pub document: &'a Document, pub palette: &'a Palette, pub fonts: &'a Fonts, pub faces: &'a Faces, pub scale: f64 }`
  - `pub fn content(canvas: &Canvas<'_>) -> Vec<u8>` — the whole page's content stream: the flip-and-scale `cm`, miter limit 4, the ground, then the document.
  - `pub(super) fn colour(paint: Paint, palette: &Palette) -> Option<[f32; 3]>` (Task 7 uses it)

Text nodes call `super::show::text(...)`, which Task 7 writes; in this task, write it as a stub that writes nothing so the walk compiles, and Task 7 replaces the stub test-first. (This is the one forward reference: the stub's body is `{}` and its signature is the one Task 7 gives.)

- [ ] **Step 1: Write the failing tests** in `src/pdf/content.rs`'s `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::fonts;
    use crate::palette::{Neutral, Role, Theme};
    use crate::svg::{Group, ItemKey, Point, Semantics};

    fn stream(root: Node, palette: &Palette) -> String {
        let document = Document { width: 200.0, height: 100.0, root };
        let faces = Faces::of([&document.root], fonts()).expect("subsets");
        let canvas = Canvas { document: &document, palette, fonts: fonts(), faces: &faces, scale: 0.75 };
        String::from_utf8(content(&canvas)).expect("ASCII")
    }

    fn rgb(palette: &Palette, role: Role) -> String {
        let [r, g, b] = colour(Paint::Role(role), palette).expect("a role is painted");
        format!("{r} {g} {b}")
    }

    /// The page opens with one flip into PDF's y-up space at the export
    /// scale, SVG's miter limit, and the theme's ground over the page
    /// (D-87), and closes the state it saved.
    #[test]
    fn the_page_is_flipped_scaled_and_grounded() {
        let palette = Palette::built_in(Theme::Dark);
        let s = stream(Node::Group(Group::new(Point::ORIGIN, Vec::new())), &palette);
        assert!(s.starts_with("q\n0.75 0 0 -0.75 0 75 cm\n4 M\n"), "{s}");
        assert!(s.contains(&format!("{} rg\n0 0 m\n200 0 l\n200 100 l\n0 100 l\nh\nf\n", rgb(&palette, Role::Neutral(Neutral::Surface1)))), "{s}");
        // Content::finish drops the last newline.
        assert!(s.ends_with("Q"), "{s}");
    }

    /// At a scale thousandths cannot hold (`--width 320` on a 1264-unit
    /// document), the scale is written unrounded, so the drawing meets the
    /// MediaBox's edge (Review Focus 3).
    #[test]
    fn an_awkward_scale_is_written_unrounded() {
        let palette = Palette::built_in(Theme::Dark);
        let document = Document { width: 1264.0, height: 1004.0, root: Node::Group(Group::new(Point::ORIGIN, Vec::new())) };
        let faces = Faces::of([&document.root], fonts()).expect("subsets");
        let scale = 0.75 * 320.0 / 1264.0;
        let canvas = Canvas { document: &document, palette: &palette, fonts: fonts(), faces: &faces, scale };
        let s = String::from_utf8(content(&canvas)).expect("ASCII");
        let first = s.lines().nth(1).expect("the cm line");
        let written: f64 = first.split(' ').next().expect("a").parse().expect("a number");
        assert!((written * 1004.0 - scale * 1004.0).abs() < 0.01, "{first}");
        assert!(!first.starts_with("0.19 "), "not rounded to thousandths: {first}");
    }

    /// A group translates its children inside its own saved state; an
    /// interactive-only node draws nothing (FR-50, FR-94).
    #[test]
    fn groups_translate_and_interactive_nodes_vanish() {
        let palette = Palette::built_in(Theme::Light);
        let dot = Node::Circle {
            centre: Point::ORIGIN,
            radius: 2.0,
            fill: Paint::Role(Role::Series(0)),
            stroke: Paint::None,
            stroke_width: 0.0,
        };
        let hidden = Node::Group(
            Group::new(Point::ORIGIN, vec![dot.clone()]).with(Semantics::Indicator(ItemKey(2))),
        );
        let hit = Node::HitArea { centre: Point::ORIGIN, radius: 9.0, key: ItemKey(2) };
        let root = Node::Group(Group::new(Point { x: 10.0, y: 5.0 }, vec![dot, hidden, hit]));
        let s = stream(root, &palette);
        assert!(s.contains("q\n1 0 0 1 10 5 cm\n"), "{s}");
        assert_eq!(s.matches(" c\n").count(), 4, "one circle's four cubics: {s}");
    }

    /// Fill only is `f`, stroke only `S` with its width, both `B`, neither
    /// nothing; a line is stroked.
    #[test]
    fn paint_picks_the_operator() {
        let palette = Palette::built_in(Theme::Light);
        let rect = |fill, stroke| Node::Rect {
            origin: Point::ORIGIN,
            width: 4.0,
            height: 4.0,
            radius: 0.0,
            fill,
            stroke,
            stroke_width: 1.5,
        };
        let ink = Paint::Role(Role::Neutral(Neutral::TextPrimary));
        let root = Node::Group(Group::new(
            Point::ORIGIN,
            vec![
                rect(ink, Paint::None),
                rect(Paint::None, ink),
                rect(ink, ink),
                rect(Paint::None, Paint::None),
                Node::Line { from: Point::ORIGIN, to: Point { x: 3.0, y: 0.0 }, stroke: ink, stroke_width: 0.5 },
            ],
        ));
        let s = stream(root, &palette);
        // The ground's `f`, then the first rect's.
        assert_eq!(s.matches("\nf\n").count(), 2, "{s}");
        assert_eq!(s.matches("\nS\n").count(), 2, "the stroked rect and the line: {s}");
        assert_eq!(s.matches("\nB\n").count(), 1, "{s}");
        assert!(s.contains("1.5 w\n") && s.contains("0.5 w\n"), "{s}");
        // Every rect here starts at the origin and a square rect is m l l l h,
        // so each painted one opens with `0 0 m`, as does the line; the
        // unpainted rect writes no path at all.
        assert_eq!(s.matches("\n0 0 m\n").count(), 5, "the ground, the three painted rects and the line: {s}");
    }

    /// Colours are the palette's literals, a number's ink included.
    #[test]
    fn colours_are_the_palettes_literals() {
        let palette = Palette::built_in(Theme::Light);
        let [r, g, b] = Hex::parse(palette.value(Role::Series(1))).expect("hex").rgb();
        let want = [r, g, b].map(|c| real(f64::from(c) / 255.0));
        assert_eq!(colour(Paint::Role(Role::Series(1)), &palette), Some(want));
        assert!(colour(Paint::NumberOn(0), &palette).is_some());
        assert_eq!(colour(Paint::None, &palette), None);
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --lib pdf::content::`
Expected: FAIL to compile: no `Canvas`, `content`, `colour`.

- [ ] **Step 3: Implement** `src/pdf/content.rs`:

```rust
//! The page's content stream (FR-110): the document's static rendering as
//! PDF operators, in document order, in document units under one
//! flip-and-scale transform. Interactive-only nodes are dropped, as the
//! static SVG drops them (FR-50).

use pdf_writer::Content;

use super::font::Faces;
use super::geometry::{self, Step};
use super::real;
use crate::palette::{Hex, Palette};
use crate::svg::{Document, Node, Paint};
use crate::text::Fonts;

/// SVG's default `stroke-miterlimit`; PDF's default is 10.
const MITER_LIMIT: f32 = 4.0;

/// What drawing a page reads.
pub struct Canvas<'a> {
    /// The arranged document.
    pub document: &'a Document,
    /// The theme's palette, bound to literals (FR-56).
    pub palette: &'a Palette,
    /// The faces text is placed with.
    pub fonts: &'a Fonts,
    /// The embedded subsets.
    pub faces: &'a Faces,
    /// The content-to-page transform's scale: points per document unit,
    /// 0.75 × FR-57's width over the document's, divided by the page's
    /// `/UserUnit`. A paper size (PR 21) adds an offset beside it, not a
    /// second transform.
    pub scale: f64,
}

/// The whole page: the transform, the ground (D-87), the document.
#[must_use]
pub fn content(canvas: &Canvas<'_>) -> Vec<u8> {
    let mut out = Content::new();
    // Unrounded (pdf::scale), so the drawing ends where the MediaBox
    // does; the translate is the MediaBox's own height, the same
    // `real(height × scale)` emit::pdf writes there.
    let s = super::scale(canvas.scale);
    out.save_state()
        .transform([s, 0.0, 0.0, -s, 0.0, real(canvas.document.height * canvas.scale)])
        .set_miter_limit(MITER_LIMIT);
    node(&mut out, &canvas.document.ground(), canvas);
    node(&mut out, &canvas.document.root, canvas);
    out.restore_state();
    out.finish().into_vec()
}

/// One node, or nothing for an interactive-only one.
fn node(out: &mut Content, node: &Node, canvas: &Canvas<'_>) {
    if node.is_interactive_only() {
        return;
    }
    match node {
        Node::Group(group) => {
            out.save_state();
            // Decided on the rounded value, as the SVG's `shows` decides, so a
            // 1e-13 trig residue cannot write a transform on one platform and
            // none on another (NFR-4).
            let (x, y) = (real(group.offset.x), real(group.offset.y));
            if x != 0.0 || y != 0.0 {
                out.transform([1.0, 0.0, 0.0, 1.0, x, y]);
            }
            for child in &group.children {
                self::node(out, child, canvas);
            }
            out.restore_state();
        }
        Node::Text { .. } => super::show::text(out, node, canvas),
        Node::Circle { centre, radius, fill, stroke, stroke_width } => {
            shape(out, &geometry::circle(*centre, *radius), Paints::new(*fill, *stroke, *stroke_width), canvas);
        }
        Node::Rect { origin, width, height, radius, fill, stroke, stroke_width } => {
            shape(out, &geometry::rect(*origin, *width, *height, *radius), Paints::new(*fill, *stroke, *stroke_width), canvas);
        }
        Node::Line { from, to, stroke, stroke_width } => {
            shape(out, &[Step::Move(*from), Step::Line(*to)], Paints::new(Paint::None, *stroke, *stroke_width), canvas);
        }
        Node::Path { commands, fill, stroke, stroke_width } => {
            shape(out, &geometry::path(commands), Paints::new(*fill, *stroke, *stroke_width), canvas);
        }
        // Interactive-only, returned above.
        Node::HitArea { .. } => {}
    }
}

/// A shape's fill, stroke and stroke width.
#[derive(Clone, Copy)]
struct Paints {
    fill: Paint,
    stroke: Paint,
    width: f64,
}

impl Paints {
    const fn new(fill: Paint, stroke: Paint, width: f64) -> Self {
        Self { fill, stroke, width }
    }
}

/// `steps` painted: `f`, `S` or `B` by which paints are set; nothing at
/// all when neither is, so an unpainted shape writes no path.
fn shape(out: &mut Content, steps: &[Step], paints: Paints, canvas: &Canvas<'_>) {
    let fill = colour(paints.fill, canvas.palette);
    let stroke = colour(paints.stroke, canvas.palette);
    if fill.is_none() && stroke.is_none() {
        return;
    }
    if let Some([r, g, b]) = fill {
        out.set_fill_rgb(r, g, b);
    }
    if let Some([r, g, b]) = stroke {
        out.set_stroke_rgb(r, g, b).set_line_width(real(paints.width));
    }
    trace(out, steps);
    match (fill, stroke) {
        (Some(_), Some(_)) => out.fill_nonzero_and_stroke(),
        (Some(_), None) => out.fill_nonzero(),
        _ => out.stroke(),
    };
}

/// The path operators for `steps`.
fn trace(out: &mut Content, steps: &[Step]) {
    for step in steps {
        match *step {
            Step::Move(p) => out.move_to(real(p.x), real(p.y)),
            Step::Line(p) => out.line_to(real(p.x), real(p.y)),
            Step::Cubic { c1, c2, end } => out.cubic_to(
                real(c1.x),
                real(c1.y),
                real(c2.x),
                real(c2.y),
                real(end.x),
                real(end.y),
            ),
            Step::Close => out.close_path(),
        };
    }
}

/// `paint` as a literal RGB triple from `palette`, or `None` for
/// [`Paint::None`]: the PDF binds roles as the static SVG does
/// (architecture §5).
pub(super) fn colour(paint: Paint, palette: &Palette) -> Option<[f32; 3]> {
    let value = match paint {
        Paint::Role(role) => palette.value(role),
        Paint::NumberOn(index) => palette.number_ink(index),
        Paint::None => return None,
    };
    let hex = Hex::parse(value).expect("a palette holds only #RRGGBB values (FR-86)");
    Some(hex.rgb().map(|c| real(f64::from(c) / 255.0)))
}
```

Add the stub `src/pdf/show.rs` (Task 7 replaces it):

```rust
//! A text node as PDF text (FR-110). Written in Task 7.

use pdf_writer::Content;

use super::content::Canvas;
use crate::svg::Node;

/// `node`'s lines and tail as PDF text.
pub(super) fn text(_out: &mut Content, _node: &Node, _canvas: &Canvas<'_>) {}
```

`src/pdf/mod.rs`: `pub mod content;` and `mod show;`.

`content()` tests check exact operator text, which depends on pdf-writer's formatting (`0.75 0 0 -0.75 0 75 cm`, `4 M`, `1.5 w`, `h`, `f`, `S`, `B`). If its output differs in spacing (it writes one operator per line, operands space-separated), adjust the expected strings to what it writes, not the code.

- [ ] **Step 4: Run the tests and lints**

Run: `~/.cargo/bin/cargo test --lib pdf:: && ~/.cargo/bin/cargo clippy --all-targets -- -D warnings`
Expected: PASS; no warnings. If `node`'s match trips the crate's function-length check (the `qual:allow` comment in `serialise/mod.rs` marks the same dispatch), add the same `// qual:allow(complexity, max_function_lines=90) reason: "one dispatch arm per node kind, each only unpacking its fields for its writer"` line above it.

- [ ] **Step 5: Commit**

```bash
git add src/pdf/content.rs src/pdf/show.rs src/pdf/mod.rs
git commit -m "feat(pdf): the content stream's shapes, groups and colours

The document's static rendering as PDF operators under one flip-and-
scale transform: the ground (D-87), groups as q/cm/Q, shapes as Bézier
paths painted f, S or B with the palette's literals, interactive-only
nodes dropped as the static SVG drops them (FR-110). Text is a stub
until the next commit.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: Text, glyph by glyph where the shaper places it

**Files:**
- Modify: `src/pdf/show.rs` (replace the stub)

**Interfaces:**
- Consumes: `Canvas` and `colour` (Task 6); `Faces::{cid, width, resource}` (Task 5); `FontMetrics::place`, `TextStyle::advance`, `Node::tail_x` (existing).
- Produces: `pub(super) fn text(out: &mut Content, node: &Node, canvas: &Canvas<'_>)`.

The whole text node is wrapped in `q … Q`, so a halo's stroke width and round join end with it. For each line: `BT`, font and size, render mode (fill, or stroke for a halo with its width and round join), a text matrix `[1 0 0 -1 x y]` that flips glyphs back upright under the page's flip, then one `TJ` whose adjustments put each glyph at `place()`'s x. Glyphs with a vertical offset start a new text matrix. A tail is drawn after the last line at `Node::tail_x`, in its own style and fill (D-78).

The `TJ` adjustment after glyph *i* is `(x_i + w_i·size/1000 − x_{i+1}) · 1000/size`, where `w_i` is `Faces::width` (the widths array's own number), so the viewer's pen lands on `x_{i+1}` exactly. One that rounds to zero is not written.

- [ ] **Step 1: Write the failing tests** in `src/pdf/show.rs`'s `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::fonts;
    use crate::palette::{Neutral, Palette, Role, Theme};
    use crate::pdf::font::Faces;
    use crate::style::TextStyle;
    use crate::svg::{Anchor, Document, Paint, Point, Tail, TextPaint};
    use crate::text::Weight;

    fn label(lines: &[&str], ink: TextPaint, anchor: Anchor, letter_spacing: f64, tail: Option<Tail>) -> Node {
        Node::Text {
            origin: Point { x: 50.0, y: 20.0 },
            style: TextStyle { weight: Weight::Regular, size: 10.0, letter_spacing },
            ink,
            anchor,
            lines: lines.iter().map(|&l| l.to_owned()).collect(),
            words: lines.join(" "),
            leading: 12.0,
            tail,
        }
    }

    fn written(node: &Node) -> (String, Faces) {
        let document = Document { width: 200.0, height: 100.0, root: node.clone() };
        let faces = Faces::of([&document.root], fonts()).expect("subsets");
        let palette = Palette::built_in(Theme::Light);
        let canvas = crate::pdf::content::Canvas { document: &document, palette: &palette, fonts: fonts(), faces: &faces, scale: 1.0 };
        let mut out = Content::new();
        text(&mut out, node, &canvas);
        (String::from_utf8(out.finish().into_vec()).expect("ASCII operators; strings are hex"), faces)
    }

    fn fill() -> TextPaint {
        TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::TextPrimary)))
    }

    /// Each line is one BT … ET at its baseline, upright under the page's
    /// flip, in the face's resource at the style's size, filled.
    #[test]
    fn lines_sit_on_their_baselines() {
        let (s, _) = written(&label(&["Adopt", "Trial"], fill(), Anchor::Start, 0.0, None));
        assert_eq!(s.matches("BT\n").count(), 2, "{s}");
        assert!(s.contains("/F0 10 Tf\n"), "{s}");
        assert!(s.contains("0 Tr\n"), "fill mode: {s}");
        assert!(s.contains("1 0 0 -1 50 20 Tm\n"), "first baseline: {s}");
        assert!(s.contains("1 0 0 -1 50 32 Tm\n"), "second, one leading below: {s}");
        assert!(s.contains(" TJ\n"), "{s}");
    }

    /// A middle-anchored line starts half its advance left of the origin.
    #[test]
    fn a_middle_line_is_centred() {
        let node = label(&["Adopt"], fill(), Anchor::Middle, 0.0, None);
        let (s, _) = written(&node);
        let style = TextStyle { weight: Weight::Regular, size: 10.0, letter_spacing: 0.0 };
        let x = real(50.0 - style.advance("Adopt", fonts()) * 0.5);
        assert!(s.contains(&format!("1 0 0 -1 {x} 20 Tm\n")), "{s}");
    }

    /// A halo is the outline only: stroke mode, its width, a round join.
    #[test]
    fn a_halo_strokes() {
        let halo = TextPaint::Halo { stroke: Paint::Role(Role::Neutral(Neutral::Surface1)), width: 3.0 };
        let (s, _) = written(&label(&["Adopt"], halo, Anchor::Start, 0.0, None));
        assert!(s.contains("1 Tr\n") && s.contains("3 w\n") && s.contains("1 j\n"), "{s}");
        // Review Focus 6: the join is the halo's alone, so a stroke drawn
        // after it keeps SVG's miter join.
        assert!(s.starts_with("q\n") && s.ends_with("Q"), "a text is its own graphics state: {s}");
    }

    /// TJ adjustments put each glyph where place() puts it: replaying the
    /// widths array's advances and the adjustments reaches every x.
    #[test]
    fn glyphs_land_where_the_shaper_places_them() {
        let node = label(&["AVATAR Office"], fill(), Anchor::Start, 1.5, None);
        let (s, faces) = written(&node);
        let face = fonts().face(Weight::Regular);
        let placed = face.place("AVATAR Office", 10.0, 1.5);
        // Decoded by lopdf, not by hand: pdf-writer writes a CID string as
        // a literal `( … )` with octal escapes whenever every byte is below
        // 0x80, and as hex otherwise.
        let ops = lopdf::content::Content::decode(s.as_bytes()).expect("decodes").operations;
        let tj = ops.iter().find(|op| op.operator == "TJ").expect("a TJ");
        let items = tj.operands.first().and_then(|a| a.as_array().ok()).expect("a TJ array");
        let mut pen = 0.0_f64;
        let mut seen = Vec::new();
        for item in items {
            match item {
                lopdf::Object::String(bytes, _) => {
                    for pair in bytes.chunks_exact(2) {
                        let cid = u16::from_be_bytes(pair.try_into().expect("two bytes"));
                        seen.push((cid, pen));
                        pen += f64::from(faces.width(Weight::Regular, cid)) * 10.0 / 1000.0;
                    }
                }
                number => pen -= f64::from(number.as_float().expect("an adjustment")) * 10.0 / 1000.0,
            }
        }
        assert_eq!(seen.len(), placed.len(), "{s}");
        for ((cid, x), want) in seen.iter().zip(&placed) {
            assert_eq!(*cid, faces.cid(Weight::Regular, want.glyph));
            assert!((x - want.x).abs() < 0.002, "glyph at {x}, placed at {}: {s}", want.x);
        }
    }

    /// A tail is drawn after the last line, at tail_x, in its own style
    /// and ink (D-78).
    #[test]
    fn the_tail_follows_the_last_line() {
        let tail = Tail {
            gap: 4.0,
            style: TextStyle { weight: Weight::SemiBold, size: 8.0, letter_spacing: 0.0 },
            ink: Paint::Role(Role::Neutral(Neutral::TextMuted)),
            text: "\u{a0}new".to_owned(),
        };
        let node = label(&["Adopt", "Trial"], fill(), Anchor::Start, 0.0, Some(tail));
        let (s, _) = written(&node);
        let x = real(node.tail_x(fonts()).expect("a tail"));
        assert!(s.contains("/F1 8 Tf\n"), "{s}");
        assert!(s.contains(&format!("1 0 0 -1 {x} 32 Tm\n")), "on the last baseline: {s}");
    }
}
```

lopdf is a dev-dependency (Task 3), so the crate's unit tests can use it.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --lib pdf::show::`
Expected: FAIL: the stub writes nothing (`BT` count 0).

- [ ] **Step 3: Implement** `src/pdf/show.rs`:

```rust
//! A text node as PDF text (FR-110): each line in its face's subset, each
//! glyph where the shaper places it (FR-84), so the PDF's layout is the
//! PNG's and the SVG's; a halo as the stroked outline only; the tail after
//! the last line at `Node::tail_x` (D-78).

use pdf_writer::Content;
use pdf_writer::types::{LineJoinStyle, TextRenderingMode};
use pdf_writer::Str;

use super::content::{Canvas, colour};
use super::font::Faces;
use super::real;
use crate::style::TextStyle;
use crate::svg::{Anchor, Node, Paint, Point, TextPaint};
use crate::text::Placed;

/// How much of its advance a middle-anchored line starts before its
/// origin, as the SVG outliner and resvg place it.
const CENTRED: f64 = 0.5;
/// Glyph space units per em in a TJ adjustment.
const EM: f64 = 1000.0;

/// `node`'s lines, then its tail. Anything but a text writes nothing.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub(super) fn text(out: &mut Content, node: &Node, canvas: &Canvas<'_>) {
    let Node::Text { origin, style, ink, anchor, lines, leading, tail, .. } = node else {
        return;
    };
    // As the SVG outliner: a text with no lines draws nothing, its tail
    // included.
    if lines.is_empty() {
        return;
    }
    // Its own graphics state: a halo's width and round join must not
    // reach the strokes drawn after it (the legend's square markers).
    out.save_state();
    let Some(mode) = paint(out, *ink, canvas) else {
        out.restore_state();
        return;
    };
    let mut baseline = origin.y;
    for (i, line) in (0_u32..).zip(lines) {
        baseline = origin.y + f64::from(i) * leading;
        let x = match anchor {
            Anchor::Start => origin.x,
            Anchor::Middle => origin.x - style.advance(line, canvas.fonts) * CENTRED,
        };
        run(out, line, *style, Point { x, y: baseline }, mode, canvas);
    }
    if let (Some(tail), Some(x)) = (tail, node.tail_x(canvas.fonts)) {
        debug_assert!(matches!(ink, TextPaint::Fill(_)), "only a filled text carries a tail (D-78)");
        if paint(out, TextPaint::Fill(tail.ink), canvas).is_some() {
            run(out, &tail.text, tail.style, Point { x, y: baseline }, TextRenderingMode::Fill, canvas);
        }
    }
    out.restore_state();
}

/// Sets `ink`'s colour (and a halo's width and join), returning the
/// render mode, or `None` when nothing is painted.
fn paint(out: &mut Content, ink: TextPaint, canvas: &Canvas<'_>) -> Option<TextRenderingMode> {
    match ink {
        TextPaint::Fill(fill) => {
            let [r, g, b] = colour(fill, canvas.palette)?;
            out.set_fill_rgb(r, g, b);
            Some(TextRenderingMode::Fill)
        }
        TextPaint::Halo { stroke, width } => {
            let [r, g, b] = colour(stroke, canvas.palette)?;
            out.set_stroke_rgb(r, g, b)
                .set_line_width(real(width))
                .set_line_join(LineJoinStyle::RoundJoin);
            Some(TextRenderingMode::Stroke)
        }
    }
}

/// One line of `text` from `at`, its glyphs split wherever the vertical
/// offset changes, each piece one text matrix and one TJ.
fn run(out: &mut Content, text: &str, style: TextStyle, at: Point, mode: TextRenderingMode, canvas: &Canvas<'_>) {
    let placed = canvas
        .fonts
        .face(style.weight)
        .place(text, style.size, style.letter_spacing);
    if placed.is_empty() {
        return;
    }
    out.begin_text();
    out.set_font(Faces::resource(style.weight), real(style.size));
    out.set_text_rendering_mode(mode);
    // Exact equality is meant: one shaper offset, not two near ones
    // (`to_bits`, so clippy's float_cmp has nothing to flag).
    for piece in placed.chunk_by(|a, b| a.y.to_bits() == b.y.to_bits()) {
        let Some(first) = piece.first() else { continue };
        out.set_text_matrix([1.0, 0.0, 0.0, -1.0, real(at.x + first.x), real(at.y + first.y)]);
        show(out, piece, style, canvas);
    }
    out.end_text();
}

/// One TJ for `piece`: runs of CIDs, with an adjustment wherever the
/// widths array's advance does not reach the next glyph's x.
fn show(out: &mut Content, piece: &[Placed], style: TextStyle, canvas: &Canvas<'_>) {
    let mut shown = out.show_positioned();
    let mut items = shown.items();
    let mut bytes: Vec<u8> = Vec::new();
    for (i, glyph) in piece.iter().enumerate() {
        let cid = canvas.faces.cid(style.weight, glyph.glyph);
        bytes.extend(cid.to_be_bytes());
        let Some(next) = piece.get(i + 1) else { break };
        let advance = f64::from(canvas.faces.width(style.weight, cid)) * style.size / EM;
        let adjust = real((glyph.x + advance - next.x) * EM / style.size);
        if adjust != 0.0 {
            items.show(Str(&bytes));
            bytes.clear();
            items.adjust(adjust);
        }
    }
    if !bytes.is_empty() {
        items.show(Str(&bytes));
    }
}
```

`Paint` is imported only for the `TextPaint::Fill(tail.ink)` pattern's type; drop it if the compiler says it is unused. If `<[T]>::chunk_by` is unavailable on the pinned toolchain, group by hand with a loop over `placed` that starts a new piece when `y` changes. If pdf-writer's `ShowPositioned`/`PositionedItems` borrow rules forbid holding `items` across the loop as written, collect `(Vec<u8>, Option<f32>)` pairs first and write them in a second loop.

- [ ] **Step 4: Run the tests and lints**

Run: `~/.cargo/bin/cargo test --lib pdf:: && ~/.cargo/bin/cargo clippy --all-targets -- -D warnings`
Expected: PASS; no warnings.

- [ ] **Step 5: Commit**

```bash
git add src/pdf/show.rs
git commit -m "feat(pdf): text placed glyph by glyph from the shaper

Each line is one BT/ET in its face's subset, upright under the page's
flip, with TJ adjustments computed from the widths array so every glyph
lands where FontMetrics::place puts it: kerning and letter-spacing as
the PNG draws them (FR-84, FR-110). A halo is stroke mode with a round
join; a tail follows the last line at Node::tail_x (D-78).

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: The PDF file, and an independent rasteriser's view of it (SC-30)

**Files:**
- Create: `src/emit/pdf.rs`
- Modify: `src/emit/mod.rs` (`pub mod pdf;`, after `pub mod png;`), `src/lib.rs` (remove the `dead_code` expectation on `mod pdf;` if Tasks 3–7 added one)
- Create: `tests/pdf_matches_png.rs` (the render oracle)
- Modify: `tests/serialise/main.rs` (`mod pdf;`)
- Create: `tests/serialise/pdf.rs`, `tests/golden/pdf/prd-light.pdf.txt` (blessed)

**Interfaces:**
- Consumes: `Faces` (Task 5), `content::{Canvas, content}` (Task 6), `pdf::real` (Task 3), `emit::intrinsic_size` (Task 2); dev: `hayro`, `lopdf` (Task 3).
- Produces:
  - `pub struct Export<'a> { pub palette: &'a Palette, pub fonts: &'a Fonts, pub width: Option<u32>, pub title: Option<&'a str>, pub desc: &'a str, pub producer: &'a str }`
  - `pub fn export(document: &Document, export: &Export<'_>) -> Result<Vec<u8>, Fatal>`
  - `pub const POINTS_PER_PX: f64 = 0.75`

Object numbers are fixed for the light PDF: 1 catalog, 2 page tree, 3 page, 4 content stream, 5 info, then the fonts from 6 (Task 5's order). PR 21 renumbers and re-blesses.

The render oracle is written here, before `export` exists, so the writer is built against it (TDD), and it keeps committed tests that damage a good PDF in each way a writer bug would and must fail: the oracle is shown to bite on every run, not once by hand.

- [ ] **Step 1: Write the failing tests.** In `src/emit/pdf.rs`'s `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::block::fonts;
    use crate::layout::test_support::{PRD_EXAMPLE, plotted};
    use crate::palette::Theme;
    use crate::svg::{Group, Node, Point};

    fn prd(title: Option<&str>) -> Document {
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        let blocks = Blocks::new(&catalogue, &order, title, None, fonts());
        arrange(Arrangement::Wide, &blocks)
    }

    fn pdf(doc: &Document, title: Option<&str>, width: Option<u32>, theme: Theme) -> Vec<u8> {
        let palette = Palette::built_in(theme);
        export(
            doc,
            &Export { palette: &palette, fonts: fonts(), width, title, desc: "Technology radar: 3 sectors", producer: "radargen 9.9" },
        )
        .expect("exports")
    }

    fn page(bytes: &[u8]) -> lopdf::Dictionary {
        let doc = lopdf::Document::load_mem(bytes).expect("parses");
        let page = *doc.get_pages().values().next().expect("one page");
        doc.get_dictionary(page).expect("a page dictionary").clone()
    }

    fn media_box(bytes: &[u8]) -> [f32; 4] {
        let dict = page(bytes);
        let mb = dict.get(b"MediaBox").and_then(lopdf::Object::as_array).expect("a MediaBox");
        let n = |i: usize| mb.get(i).and_then(|o| o.as_float().ok()).expect("a number");
        [n(0), n(1), n(2), n(3)]
    }

    fn near(got: f32, want: f64) -> bool {
        (f64::from(got) - want).abs() <= 0.001
    }

    /// FR-110, FR-57: one page, the document's size at 0.75 pt per px,
    /// or `--width` px wide with the height following (Review Focus 3).
    #[test]
    fn the_page_is_the_documents_size() {
        let doc = prd(None);
        for width in [None, Some(320), Some(10_000)] {
            let [x0, y0, w, h] = media_box(&pdf(&doc, None, width, Theme::Light));
            let (want_w, want_h) = crate::emit::intrinsic_size(&doc, width);
            assert!(x0 == 0.0 && y0 == 0.0, "--width {width:?}");
            assert!(near(w, want_w * 0.75) && near(h, want_h * 0.75), "--width {width:?}: {w} × {h}");
            assert!(page(&pdf(&doc, None, width, Theme::Light)).get(b"UserUnit").is_err(), "under the limit, no UserUnit");
        }
    }

    /// Review Focus 3: a side past the 14,400 pt viewers accept is written
    /// with a /UserUnit that brings it under, keeping the page's size.
    #[test]
    fn a_page_past_the_viewers_limit_gets_a_user_unit() {
        // 30,000 px tall is 22,500 pt: a long single-column legend's shape.
        let doc = Document { width: 100.0, height: 30_000.0, root: Node::Group(Group::new(Point::ORIGIN, Vec::new())) };
        let bytes = pdf(&doc, None, None, Theme::Light);
        let unit = page(&bytes).get(b"UserUnit").and_then(lopdf::Object::as_float).expect("a UserUnit");
        assert!((unit - 2.0).abs() < f32::EPSILON, "the smallest whole unit that fits: {unit}");
        let [_, _, w, h] = media_box(&bytes);
        assert!(h <= 14_400.0, "{h}");
        assert!(near(w * unit, 75.0) && near(h * unit, 22_500.0), "{w} × {h} at {unit}");
    }

    /// FR-110, NFR-4: the same bytes twice; no date, no file identifier;
    /// the title, subject, language and producer are set (D-75, SC-25).
    #[test]
    fn the_file_is_reproducible_and_named() {
        let doc = prd(None);
        let first = pdf(&doc, None, None, Theme::Dark);
        assert!(first == pdf(&doc, None, None, Theme::Dark), "NFR-4");
        let parsed = lopdf::Document::load_mem(&first).expect("parses");
        assert!(parsed.trailer.get(b"ID").is_err(), "no file identifier");
        let info = parsed.trailer.get(b"Info").and_then(lopdf::Object::as_reference).expect("an Info");
        let info = parsed.get_dictionary(info).expect("an info dictionary");
        for absent in [&b"CreationDate"[..], b"ModDate"] {
            assert!(info.get(absent).is_err(), "{}", String::from_utf8_lossy(absent));
        }
        let text = |key: &[u8]| lopdf::decode_text_string(info.get(key).expect("set")).expect("a text string");
        assert_eq!(text(b"Title"), "Technology Radar");
        assert_eq!(text(b"Subject"), "Technology radar: 3 sectors");
        assert_eq!(text(b"Producer"), "radargen 9.9");
        let raw = String::from_utf8_lossy(&first);
        assert!(raw.contains("/Lang (en)") && raw.contains("/DisplayDocTitle true"), "the catalog");
        assert!(first.starts_with(b"%PDF-1.7\n"));
    }

    /// Review Focus 1: a title with PDF string delimiters, an unbalanced
    /// parenthesis, line breaks and non-ASCII reads back exactly.
    #[test]
    fn a_hostile_title_reads_back() {
        let title = "Q3 (draft) \\ Ærø — 中 ) \r\n end";
        let doc = prd(Some(title));
        let bytes = pdf(&doc, Some(title), None, Theme::Light);
        let parsed = lopdf::Document::load_mem(&bytes).expect("parses");
        let info = parsed.trailer.get(b"Info").and_then(lopdf::Object::as_reference).expect("an Info");
        let got = parsed.get_dictionary(info).and_then(|d| d.get(b"Title")).expect("a Title");
        assert_eq!(lopdf::decode_text_string(got).expect("a text string"), title);
    }
}
```

If `lopdf::decode_text_string` or `Object::as_float` differ in 0.46, use what lopdf provides (`cargo doc -p lopdf`); the rust review of this plan confirmed `decode_text_string` exists. If `Blocks::new` refuses the hostile title's `\r\n` (FR-65 may strip or reject control characters), keep them in the `Export` title only and build the document with the title minus them: the test is about the info dictionary.

`tests/serialise/pdf.rs`, the golden as text, so a change reads as a diff:

```rust
//! The exported PDF's golden (FR-110, NFR-4), as text: the font files,
//! the only binary in the file, are replaced by their inflated length and
//! a hash, so a diff names the object that changed, and a subsetter
//! patch release shows as one line.

use std::fmt::Write as _;

use radargen::emit::pdf::{Export, export};
use radargen::layout::arrange::{Arrangement, arrange};
use radargen::palette::{Palette, Theme};
use radargen::text::Fonts;

use super::golden::golden;
use super::prd;
use super::version::PLACEHOLDER;

/// FNV-1a, 64-bit.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

/// `bytes` with each `FontFile2` stream (the only streams with a
/// `/Length1`) summarised, then every byte outside printable ASCII and
/// newline (the header's binary marker) written as `\xNN`.
fn readable(bytes: &[u8]) -> String {
    let parsed = lopdf::Document::load_mem(bytes).expect("parses");
    let mut out = bytes.to_vec();
    for object in parsed.objects.values() {
        let Ok(stream) = object.as_stream() else { continue };
        if stream.dict.get(b"Length1").is_err() {
            continue;
        }
        let inflated = stream.decompressed_content().expect("inflates");
        let summary = format!("<FontFile2: {} bytes inflated, fnv1a64 {:016x}>", inflated.len(), fnv(&inflated));
        let raw = &stream.content;
        let at = out.windows(raw.len()).position(|w| w == raw.as_slice()).expect("the raw stream is in the file");
        out.splice(at..at + raw.len(), summary.into_bytes());
    }
    let mut text = String::new();
    for &b in &out {
        if b == b'\n' || (b' '..=b'~').contains(&b) {
            text.push(char::from(b));
        } else {
            let _ = write!(text, "\\x{b:02x}");
        }
    }
    text
}

/// The PRD example as a light PDF, its producer the placeholder, so a
/// minor bump changes no byte.
#[test]
fn prd_pdf_matches_its_golden() {
    let built = prd();
    let fonts = Fonts::embedded().expect("fonts");
    let doc = arrange(Arrangement::Wide, &built.blocks);
    let palette = Palette::built_in(Theme::Light);
    let bytes = export(
        &doc,
        &Export {
            palette: &palette,
            fonts: &fonts,
            width: None,
            title: Some("Technology Radar"),
            desc: &built.blocks.radar.name(),
            producer: PLACEHOLDER,
        },
    )
    .expect("exports");
    golden("pdf", "prd-light.pdf.txt", &readable(&bytes));
}
```

lopdf keeps a loaded stream's raw (still compressed) bytes in `content` unless it is asked to decompress; if 0.46 inflates on load, find the raw bytes by the stream's `/Length` after its object's `stream\n` keyword instead. `Built` has no fonts field (checked in review), hence `Fonts::embedded()`. If `prd()` or `blocks.radar.name()` are spelled differently, follow `tests/serialise/main.rs`.

`tests/pdf_matches_png.rs`, the render oracle:

```rust
//! SC-30 (FR-110): the PDF, drawn by hayro — an independent PDF
//! rasteriser, so the check is not our own reading of the format —
//! draws what our PNG draws, to within antialiasing; and a PDF damaged
//! the ways a writer bug would damage it does not.
//!
//! hayro (vello_cpu) and tiny-skia antialias differently, so the budget
//! and the ceiling are measured, as `svg_matches_png.rs`'s were; the
//! damage tests are what show the measured budget still separates a
//! faithful PDF from a broken one.

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_interpret::util::TransformExt as _;
use hayro::hayro_syntax::Pdf;
use hayro::kurbo::Affine;
use hayro::vello_cpu::color::palette::css::TRANSPARENT;
use hayro::vello_cpu::{Pixmap as Drawn, RasterizerSettings, RenderContext, Resources, TargetInit};
use hayro::{RenderCache, RenderSettings, render_into};
use lopdf::Object;
use lopdf::content::{Content, Operation};
use radargen::emit::pdf::{Export, export};
use radargen::emit::png::{PixelSize, pixel_size, rasterise};
use radargen::layout::arrange::{Arrangement, Blocks, arrange};
use radargen::order::resolve;
use radargen::palette::{Palette, Theme};
use radargen::svg::Document;
use radargen::text::Fonts;
use resvg::tiny_skia::Pixmap;

#[path = "common/catalogue.rs"]
mod catalogue;

/// Measured in Step 4 (1.5× the worst case's residue, with the date and
/// platform); 123 is a placeholder from the SVG suite until then.
const BUDGET_PPM: usize = 123;
/// Measured in Step 4 (the worst residue plus 16, at most 255).
const MAX_DELTA: u8 = 64;

const LONG_TITLE: &str = "Engineering Technology Radar for the Platform and Delivery Groups";

/// `svg_matches_png.rs`'s five cases, a right-to-left name, and names
/// Inter lacks (Review Focus 2), each with its title.
const CASES: [(&str, &str, Option<&str>); 7] = [
    ("three-sector", include_str!("fixtures/three-sector.csv"), Some("Technology Radar")),
    ("long-legend", include_str!("fixtures/long-legend.csv"), None),
    ("text-spacing", include_str!("fixtures/text-spacing.csv"), Some("Text spacing")),
    ("grown", include_str!("fixtures/grown.csv"), None),
    ("wrapped title", include_str!("fixtures/three-sector.csv"), Some(LONG_TITLE)),
    ("text-spacing-rtl", include_str!("fixtures/text-spacing-rtl.csv"), None),
    ("not in Inter", "name,sector,ring,description\n東京,Tools,Adopt,d\n😀,Tools,Trial,d\nKafka,Tools,Assess,d\n", None),
];

/// A case's wide document and the radar's name.
fn wide(csv: &str, title: Option<&str>, fonts: &Fonts) -> (Document, String) {
    let catalogue = catalogue::catalogue(csv, None);
    let plotted = resolve(&catalogue);
    let blocks = Blocks::new(&catalogue, &plotted, title, None, fonts);
    (arrange(Arrangement::Wide, &blocks), blocks.radar.name())
}

fn pdf_of(doc: &Document, desc: &str, palette: &Palette, fonts: &Fonts, title: Option<&str>, width: Option<u32>) -> Vec<u8> {
    export(doc, &Export { palette, fonts, width, title, desc, producer: "t" }).expect("exports")
}

/// `pdf` drawn by hayro at `size`, premultiplied RGBA like tiny-skia's.
/// x and y are scaled separately, as the PNG's transform is, since the
/// PNG's height is rounded to whole pixels.
fn drawn(pdf: &[u8], size: PixelSize) -> Vec<u8> {
    let pdf = Pdf::new(pdf.to_vec()).expect("hayro parses the PDF");
    let page = pdf.pages().first().expect("one page");
    let (w_pt, h_pt) = page.render_dimensions();
    let w = u16::try_from(size.width()).expect("a test page fits u16");
    let h = u16::try_from(size.height()).expect("a test page fits u16");
    let fit = Affine::scale_non_uniform(f64::from(w) / f64::from(w_pt), f64::from(h) / f64::from(h_pt));
    let cache = RenderCache::new();
    let mut ctx = RenderContext::new(w, h);
    render_into(
        page,
        &cache,
        &InterpreterSettings::default(),
        &RenderSettings::default(),
        &mut ctx,
        fit * page.initial_transform(true).to_kurbo(),
    );
    ctx.flush();
    let mut out = Drawn::new(w, h);
    ctx.render_with(
        &mut out,
        &mut Resources::default(),
        RasterizerSettings { target_init: TargetInit::Clear(TRANSPARENT), ..Default::default() },
    );
    out.data_as_u8_slice().to_vec()
}

/// Differing subpixels, the worst channel delta, and the total.
struct Difference {
    differ: usize,
    worst: u8,
    total: usize,
}

impl Difference {
    const fn within(&self) -> bool {
        self.differ * 1_000_000 <= BUDGET_PPM * self.total && self.worst <= MAX_DELTA
    }
}

/// `pdf` drawn by hayro against `doc`'s PNG in `png_palette` at `width`.
fn against_png(doc: &Document, pdf: &[u8], png_palette: &Palette, width: Option<u32>) -> Difference {
    let size = pixel_size(doc, width).expect("fits");
    let png = Pixmap::decode_png(&rasterise(doc, png_palette, size).expect("renders")).expect("a PNG");
    let drawn = drawn(pdf, size);
    let pairs = || png.data().iter().zip(&drawn);
    Difference {
        differ: pairs().filter(|(a, b)| a != b).count(),
        worst: pairs().map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0),
        total: png.data().len(),
    }
}

/// SC-30: every case, both themes (Review Focus 2, 4 and 5).
#[test]
fn the_pdf_draws_what_the_png_draws() {
    let fonts = Fonts::embedded().expect("loads");
    for (name, csv, title) in CASES {
        let (doc, desc) = wide(csv, title, &fonts);
        for theme in [Theme::Light, Theme::Dark] {
            let palette = Palette::built_in(theme);
            let d = against_png(&doc, &pdf_of(&doc, &desc, &palette, &fonts, title, None), &palette, None);
            eprintln!("{name} {theme:?}: {} of {} subpixels ({} ppm), worst {}", d.differ, d.total, d.differ * 1_000_000 / d.total, d.worst);
            assert!(d.within(), "{name} {theme:?}");
        }
    }
}

/// Review Focus 3: at a set width the drawing scales with the page and
/// meets its edges, in the dark theme, where a sliver would show.
#[test]
fn the_pdf_at_a_set_width_draws_what_the_png_draws() {
    let fonts = Fonts::embedded().expect("loads");
    let (_, csv, title) = CASES[0];
    let (doc, desc) = wide(csv, title, &fonts);
    let palette = Palette::built_in(Theme::Dark);
    for width in [320, 2400] {
        let d = against_png(&doc, &pdf_of(&doc, &desc, &palette, &fonts, title, Some(width)), &palette, Some(width));
        eprintln!("--width {width}: {} ppm, worst {}", d.differ * 1_000_000 / d.total, d.worst);
        assert!(d.within(), "--width {width}");
    }
}

/// `pdf` with its page's content operators edited, re-saved by lopdf.
fn damaged(pdf: &[u8], edit: impl FnOnce(&mut Vec<Operation>)) -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(pdf).expect("parses");
    let page = *doc.get_pages().values().next().expect("a page");
    let mut content = Content::decode(&doc.get_page_content(page)).expect("decodes");
    edit(&mut content.operations);
    doc.change_page_content(page, content.encode().expect("encodes")).expect("replaces");
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("saves");
    out
}

/// The case the damage tests use: three sectors, light.
fn good() -> (Document, Vec<u8>, Palette) {
    let fonts = Fonts::embedded().expect("loads");
    let (_, csv, title) = CASES[0];
    let (doc, desc) = wide(csv, title, &fonts);
    let palette = Palette::built_in(Theme::Light);
    let pdf = pdf_of(&doc, &desc, &palette, &fonts, title, None);
    (doc, pdf, palette)
}

/// The control: lopdf's re-encoding alone changes nothing hayro draws,
/// so each failure below is the damage's.
#[test]
fn a_round_trip_through_lopdf_still_passes() {
    let (doc, pdf, palette) = good();
    assert!(against_png(&doc, &damaged(&pdf, |_| {}), &palette, None).within());
}

/// One glyph dropped from the first TJ fails.
#[test]
fn a_dropped_glyph_fails() {
    let (doc, pdf, palette) = good();
    let broken = damaged(&pdf, |ops| {
        let tj = ops.iter_mut().find(|op| op.operator == "TJ").expect("a TJ");
        let Some(Object::Array(items)) = tj.operands.first_mut() else { panic!("a TJ array") };
        let Some(Object::String(bytes, _)) = items.iter_mut().find(|o| matches!(o, Object::String(..))) else {
            panic!("a glyph string")
        };
        bytes.drain(..2);
    });
    assert!(!against_png(&doc, &broken, &palette, None).within(), "a dropped glyph must fail");
}

/// Every TJ adjustment's sign flipped (kerning and letter-spacing the
/// wrong way) fails.
#[test]
fn flipped_adjustments_fail() {
    let (doc, pdf, palette) = good();
    let broken = damaged(&pdf, |ops| {
        for op in ops.iter_mut().filter(|op| op.operator == "TJ") {
            if let Some(Object::Array(items)) = op.operands.first_mut() {
                for item in items {
                    match item {
                        Object::Real(v) => *v = -*v,
                        Object::Integer(v) => *v = -*v,
                        _ => {}
                    }
                }
            }
        }
    });
    assert!(!against_png(&doc, &broken, &palette, None).within(), "reversed kerning must fail");
}

/// One fill colour changed (the first after the ground's) fails.
#[test]
fn a_wrong_colour_fails() {
    let (doc, pdf, palette) = good();
    let broken = damaged(&pdf, |ops| {
        let rg = ops.iter_mut().filter(|op| op.operator == "rg").nth(1).expect("a second fill");
        let red = rg.operands.first().and_then(|o| o.as_float().ok()).expect("a channel");
        if let Some(first) = rg.operands.first_mut() {
            *first = Object::Real(1.0 - red);
        }
    });
    assert!(!against_png(&doc, &broken, &palette, None).within(), "a wrong colour must fail");
}

/// The light PDF against the dark PNG fails: the comparison sees the
/// ground.
#[test]
fn the_wrong_theme_fails() {
    let (doc, pdf, _) = good();
    assert!(!against_png(&doc, &pdf, &Palette::built_in(Theme::Dark), None).within());
}
```

Notes for the implementer:
- hayro's API is as read from its 0.8.0 source and `examples/render.rs`: `Pdf::new(impl Into<PdfData>)`, `pdf.pages()` derefs to a slice of `Page`, `page.render_dimensions() -> (f32, f32)`, `page.initial_transform(true)`, `render_into(page, &cache, &InterpreterSettings, &RenderSettings, &mut RenderContext, Affine)`, then `ctx.flush()` and `ctx.render_with(&mut Pixmap, &mut Resources, RasterizerSettings)`. `hayro::render` (the simple entry point) truncates the page size to whole pixels and would come out a pixel short of the PNG, which is why the oracle sizes its own `RenderContext`. If a name differs, follow the compiler and `hayro/examples/render.rs`.
- No font resolver is needed: every font is embedded, so `InterpreterSettings::default()` serves.
- `--all-targets` lints these tests: an `#[expect]` names its reason; `CASES[0]` is a constant in-bounds index, which `indexing_slicing` allows, as `svg_matches_png.rs` does.
- If a test needs a larger page than u16 allows, it is the wrong test: the oracle compares at the PNG's size, which FR-64 caps at 10,000 px wide.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --lib emit::pdf:: ; ~/.cargo/bin/cargo test --test serialise pdf ; ~/.cargo/bin/cargo test --test pdf_matches_png`
Expected: FAIL to compile: no `emit::pdf`.

- [ ] **Step 3: Implement** `src/emit/pdf.rs`:

```rust
//! The PDF export (FR-110, D-103): the wide document as one vector page
//! the document's size, its text real text in embedded Inter subsets,
//! untagged, with no date or identifier (NFR-4).

use pdf_writer::{Finish, Pdf, Rect, Ref, TextStr};

use super::{UNTITLED, intrinsic_size};
use crate::diagnostics::Fatal;
use crate::palette::Palette;
use crate::pdf::content::{Canvas, content};
use crate::pdf::font::Faces;
use crate::pdf::real;
use crate::svg::Document;
use crate::text::Fonts;

/// PDF points per CSS pixel: 72 per inch over 96.
pub const POINTS_PER_PX: f64 = 0.75;
/// The longest page side viewers accept, in points: 200 inches, Acrobat's
/// limit (PDF 1.7, Annex C).
const MAX_SIDE: f64 = 14_400.0;

/// What one export needs besides the document.
pub struct Export<'a> {
    /// The theme's palette (FR-56).
    pub palette: &'a Palette,
    /// The faces the text is placed and subset from.
    pub fonts: &'a Fonts,
    /// `--width` (FR-57), in px.
    pub width: Option<u32>,
    /// FR-65's title, resolved.
    pub title: Option<&'a str>,
    /// The radar's name (D-75), `/Subject`: what the SVG's `<desc>` holds.
    pub desc: &'a str,
    /// `/Producer`: [`OUTPUT_VERSION`](crate::emit::OUTPUT_VERSION) from
    /// the pipeline (SC-25); a test passes its placeholder.
    pub producer: &'a str,
}

/// `document` as a PDF file (FR-110).
///
/// # Errors
///
/// [`Fatal::Font`] when a glyph cannot be measured or subset (NFR-15).
pub fn export(document: &Document, export: &Export<'_>) -> Result<Vec<u8>, Fatal> {
    let (width, _) = intrinsic_size(document, export.width);
    let natural = POINTS_PER_PX * width / document.width;
    let unit = user_unit(document.width * natural, document.height * natural);
    // The content-to-page scale, in the page's own units.
    let scale = natural / f64::from(unit);
    let faces = Faces::of([&document.root], export.fonts)?;
    let mut next = Ref::new(1);
    let [catalog, tree, page, stream, info] = [(); 5].map(|()| next.bump());
    let mut pdf = Pdf::new();

    let mut cat = pdf.catalog(catalog);
    cat.pages(tree).lang(TextStr("en"));
    cat.viewer_preferences().display_doc_title(true);
    cat.finish();

    pdf.pages(tree).kids([page]).count(1);

    let fonts = faces.write(&mut pdf, &mut next, export.fonts)?;
    let mut p = pdf.page(page);
    // The same real(side × scale) content() translates by, so the drawing
    // meets the MediaBox's edges (Review Focus 3).
    p.parent(tree)
        .media_box(Rect::new(0.0, 0.0, real(document.width * scale), real(document.height * scale)))
        .contents(stream);
    if unit > 1 {
        p.user_unit(f32::from(unit));
    }
    let mut resources = p.resources();
    let mut dict = resources.fonts();
    for (weight, font) in &fonts {
        dict.pair(Faces::resource(*weight), *font);
    }
    dict.finish();
    resources.finish();
    p.finish();

    let drawn = content(&Canvas {
        document,
        palette: export.palette,
        fonts: export.fonts,
        faces: &faces,
        scale,
    });
    pdf.stream(stream, &drawn);

    pdf.document_info(info)
        .title(TextStr(export.title.unwrap_or(UNTITLED)))
        .subject(TextStr(export.desc))
        .producer(TextStr(export.producer));
    Ok(pdf.finish())
}

/// The page's `/UserUnit` (PDF 1.6): 1, or the smallest whole number that
/// brings both sides of a `width` × `height` pt page under [`MAX_SIDE`],
/// so a tall legend's page keeps its size rather than being refused.
///
/// # Panics
///
/// Past `u16::MAX`, a page over 900 km long: a layout bug.
fn user_unit(width: f64, height: f64) -> u16 {
    let longest = width.max(height);
    (1_u16..=u16::MAX)
        .find(|&u| longest / f64::from(u) <= MAX_SIDE)
        .expect("no layout is 65,535 × 200 inches long")
}
```

`Faces::write` numbers the font objects from 6 because `next` has handed out 1–5; the fonts are written before the page so the page can name their refs, and object numbers, not file order, are what the fixed-order rule fixes. `Fatal::Font` already has `#[from] FontError`, so `?` converts. `.finish()` on the catalog, dictionaries and page comes from `pdf_writer::Finish`.

`src/emit/mod.rs`: `pub mod pdf;`. `tests/serialise/main.rs`: `mod pdf;`.

- [ ] **Step 4: Measure, bless and run**

Run: `~/.cargo/bin/cargo test --test pdf_matches_png -- --nocapture 2>&1 | /opt/homebrew/bin/rg 'ppm|worst|test '`

Record each case's ppm and worst delta. Set `BUDGET_PPM` to 1.5× the worst ppm (rounded up) and `MAX_DELTA` to the worst delta plus 16, at most 255, and rewrite both constants' docs with the measurements, the date and the platform, in `svg_matches_png.rs`'s style. Then run the whole file: every damage test must still fail `within()` under the measured constants. **If any damage test passes, stop and report**: the two rasterisers' antialiasing is too far apart for a pixel budget to separate a faithful PDF from a broken one, and the comparison needs another design (for example, comparing at 2× and downsampling, or masking edge pixels) — that is a decision for the orchestrator, not a constant to loosen. Likewise, if the worst ppm exceeds about 5,000 (0.5%), stop and report before setting it.

Run: `UPDATE_GOLDEN=1 ~/.cargo/bin/cargo test --test serialise prd_pdf_matches_its_golden` (expected: FAIL with `blessed …prd-light.pdf.txt`), then read the golden: the catalog, page, info and font dictionaries, the content stream and the ToUnicode CMap in text, two `<FontFile2: …>` lines, nothing else binary.

Run: `~/.cargo/bin/cargo test --lib emit::pdf:: && ~/.cargo/bin/cargo test --test serialise --test pdf_matches_png && ~/.cargo/bin/cargo clippy --all-targets -- -D warnings`
Expected: PASS; no warnings.

Write one PDF to the scratchpad (a throwaway test or `cargo run` after Task 9) and open it in Preview and a browser: the radar, key and legend as the PNG shows them; drag-select a legend name and paste it; search for `Adopt`. Note the result for the PR body.

- [ ] **Step 5: Commit**

```bash
git add src/emit/pdf.rs src/emit/mod.rs src/lib.rs tests/pdf_matches_png.rs tests/serialise/main.rs tests/serialise/pdf.rs tests/golden/pdf/prd-light.pdf.txt
git commit -m "feat(emit): the PDF file, checked by an independent rasteriser

Catalog with /Lang and DisplayDocTitle, one page whose MediaBox is
FR-57's size at 0.75 pt per px (with a /UserUnit past viewers' 200-inch
limit), the content stream, the fonts, and an info dictionary with
/Title (D-75), /Subject and /Producer. No dates and no ID, in a fixed
object order, so two exports are the same bytes (FR-110, NFR-4).

SC-30's render oracle draws the PDF with hayro and compares it with
the PNG in seven cases and both themes, within a measured budget;
committed damage tests (a dropped glyph, reversed kerning, a wrong
colour, the wrong theme) fail it. The golden is text, its font files
summarised by length and hash.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 9: `-o radar.pdf`

**Files:**
- Modify: `src/options.rs`, `src/pipeline.rs` (the `match config.format`), `src/diagnostics.rs` (`UnrecognisedExtension`'s message; `Unplotted::of`; the `--legend`/`--table` warnings), `src/cli.rs` (help text), `docs/product/config.sample.toml`
- Modify tests: `tests/run.rs`, `tests/diagnostics.rs`, `tests/config.rs`, `tests/edition.rs`, `tests/cli.rs`

**Interfaces:**
- Consumes: `emit::pdf::{Export, export}` (Task 8), `OUTPUT_VERSION`.
- Produces: `OutputFormat::Pdf`, displayed `PDF`; `is_snapshot()` true for it.

- [ ] **Step 1: Write the failing tests.** In `src/options.rs` (add a `mod tests` if absent):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// FR-52: `.pdf` in any case is PDF, a static snapshot.
    #[test]
    fn pdf_is_a_snapshot_format() {
        for name in ["radar.pdf", "RADAR.PDF", "a.b.Pdf"] {
            assert_eq!(OutputFormat::from_path(Path::new(name)), Some(OutputFormat::Pdf), "{name}");
        }
        assert!(OutputFormat::Pdf.is_snapshot());
        assert_eq!(OutputFormat::Pdf.to_string(), "PDF");
        assert_eq!(OutputFormat::from_path(Path::new("radar.txt")), None);
    }
}
```

In `tests/run.rs`, add:

```rust
/// FR-110: `-o radar.pdf` writes a PDF of the wide arrangement, exit 0,
/// the PRD example's one warning as for PNG, its producer this version.
#[test]
fn a_pdf_is_written() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.pdf", "--title", "Q3"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let bytes = fs::read(dir.path().join("radar.pdf")).expect("written");
    assert!(bytes.starts_with(b"%PDF-1.7\n"));
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains(&format!("/Producer ({})", radargen::emit::OUTPUT_VERSION)), "SC-25");
    assert!(text.contains("/Title (Q3)"));
}
```

and extend the static-format loops to PDF: in `a_static_output_that_is_the_input_is_refused` the array becomes `["data.png", "data.svg", "data.pdf"]`; in `a_static_output_into_a_missing_directory_fails` it becomes `["nowhere/radar.png", "nowhere/radar.svg", "nowhere/radar.pdf"]`; wherever a test in `tests/run.rs` loops over `["png", "svg"]` or `[".png", ".svg"]` to check a static-format rule (the `--legend hidden` and `--table all` warnings, FR-107, FR-100), add `pdf`. Find them with `rg -n '"svg"|\.svg"' tests/run.rs`.

Change every test that uses `.pdf` as the *unrecognised* example to `.txt`, with `.html, .png, .svg or .pdf` in the expected message:
- `tests/diagnostics.rs` `fatal_messages`: `radar.pdf` → `radar.txt`, and both expected strings end `must end in .html, .png, .svg or .pdf`.
- `tests/diagnostics.rs:1297`, `tests/config.rs:123, 150, 549`: `radar.pdf`/`a.pdf` → `radar.txt`/`a.txt` in the fixture and the expectation, same ending.
- `tests/edition.rs:375`, `tests/run.rs:490`: the ending only.

The two warnings' expected text becomes:
- `"--legend hidden applies to HTML pages; PNG, SVG and PDF always draw the legend"`
- `"--table all applies to HTML pages; PNG, SVG and PDF have no table"`

(`tests/diagnostics.rs:570, 1366`; `tests/run.rs:36, 558`.) `tests/cli.rs:286`'s help line becomes `"…Accepted in any letter case. PNG, SVG and PDF have no table."`.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --lib options:: ; ~/.cargo/bin/cargo test --test run a_pdf_is_written`
Expected: FAIL: no variant `Pdf`; the binary refuses `radar.pdf`.

- [ ] **Step 3: Implement.**

`src/options.rs`:

```rust
    /// `.pdf`: the wide arrangement as a one-page vector PDF, its text
    /// real text (FR-110, D-103).
    Pdf,
```

`from_path`: `"pdf" => Some(Self::Pdf),`. `is_snapshot`: `matches!(self, Self::Png | Self::Svg | Self::Pdf)`, its doc "a PNG, an SVG or a PDF". `Display`: `Self::Pdf => "PDF",`. The `Legend` and `Table` docs: "PNG and SVG" → "PNG, SVG and PDF".

`src/pipeline.rs`, after the `Svg` arm:

```rust
        OutputFormat::Pdf => Ok(pdf::export(
            &arrange(Arrangement::Wide, &blocks),
            &pdf::Export {
                palette: &config.colours.snapshot(),
                fonts,
                width: config.width.as_ref().map(|w| w.value),
                title: config.title(),
                desc: &blocks.radar.name(),
                producer: OUTPUT_VERSION,
            },
        )?),
```

No PNG-style width clamp or small-text warning here: FR-110 says a vector page has neither (D-87); `/UserUnit` (Task 8) handles a page past viewers' limit.

with `pdf` imported beside `svg` and `png` from `crate::emit`, and `OUTPUT_VERSION` from `crate::emit` if not already imported.

`src/diagnostics.rs`:
- `#[error("{output}: the output file name must end in .html, .png or .svg")]` → `#[error("{output}: the output file name must end in .html, .png, .svg or .pdf")]`.
- `Unplotted::of`: `(OutputFormat::Png | OutputFormat::Svg | OutputFormat::Pdf, _) => Self::Undrawn,`.
- The two warning texts at lines ~549 and ~553 as in Step 1; the doc comment at ~201 "on a PNG, SVG or PDF".
- FR-104's ending (`boxes`) names the format through `Display`, so it reads "the PDF draws them as boxes" with no change; confirm with the test below.

Add to `tests/run.rs` (Review Focus 2):

```rust
/// FR-104 for PDF: a name the font lacks warns, naming the PDF.
#[test]
fn a_pdf_warns_about_missing_glyphs() {
    let dir = seeded(&[("radar.csv", &format!("{HEAD}中文,Tools,Adopt,d\n"))]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.pdf"]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(ran.stderr.contains("the PDF draws them as boxes"), "{}", ran.stderr);
}
```

`src/cli.rs`: the `-o` help "The extension picks the format: .html, .png or .svg, in any case." → ".html, .png, .svg or .pdf, in any case."; each "PNG and SVG"/"PNG or SVG" in help text and comments (lines ~103–149) names PDF as well.

`docs/product/config.sample.toml`: the `output` comment lists `.pdf`; the `theme`, `width` and `legend_columns` comments say "PNG, SVG and PDF" / "a PNG, SVG or PDF legend". FR-109 requires the sample to be exactly the bytes `--create-config` writes, so change the generator (`src/starter.rs` or wherever the text lives; find it with `rg -n "The extension picks the format" src`) and the sample together, then run the test that ties them (`rg -n "config.sample.toml" tests src` names it).

- [ ] **Step 4: Run everything**

Run: `/opt/homebrew/bin/just ci`
Expected: PASS. (If `just ci` runs browser checks that need Playwright and they are unrelated, run `~/.cargo/bin/cargo test --all-targets && ~/.cargo/bin/cargo clippy --all-targets -- -D warnings` and report which part of `just ci` was not run.)

- [ ] **Step 5: Commit**

```bash
git add -A src tests docs/product/config.sample.toml
git commit -m "feat(cli): -o radar.pdf writes the PDF (FR-110)

.pdf joins FR-52's extensions, any case. PDF is a static snapshot:
--theme, --palette, --width and --legend-columns apply, --legend and
--table warn as on PNG and SVG, and unplotted rows are undrawn (D-100).
Messages that listed the static formats now name PDF, and tests that
used radar.pdf as the unrecognised example use radar.txt.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 10: The PDF reads back as its names, and two processes write it alike (SC-30)

**Files:**
- Create: `tests/pdf_text.rs`
- Modify: `tests/determinism.rs`

**Interfaces:**
- Consumes: `emit::pdf::{Export, export}` (Task 8); `radargen::model::{Catalogue, Item, Sector}`, `radargen::order::{Plotted, resolve}` (existing: `Plotted::items` is FR-34's legend order as indices into `Catalogue::items`; `Item::name`, `Sector::label`, `Catalogue::rings`); `lopdf` (dev); `tests/common/catalogue.rs`.

The render oracle is Task 8's. This task checks what a reader gets by copying and searching, from the **catalogue**, not from the document walk the fonts were collected through, so a run that walk missed is caught rather than missing from both sides (QA B1).

- [ ] **Step 1: Write the failing tests** in `tests/pdf_text.rs`:

```rust
//! SC-30 (FR-110): text extracted from the PDF holds every name the
//! bundled font can draw, each intact and in legend order; without its
//! ToUnicode map it does not, so the check reads the map.
//!
//! lopdf's extraction concatenates a TJ's strings and ignores its
//! adjustments, so it cannot show how a viewer breaks a letter-spaced
//! word; that is the manual check's (`docs/a11y/manual-checks.md`).

use lopdf::Object;
use radargen::emit::pdf::{Export, export};
use radargen::layout::arrange::{Arrangement, Blocks, arrange};
use radargen::model::Catalogue;
use radargen::order::resolve;
use radargen::palette::{Palette, Theme};
use radargen::text::Fonts;

#[path = "common/catalogue.rs"]
mod catalogue;

const CASES: [(&str, &str, Option<&str>); 4] = [
    ("three-sector", include_str!("fixtures/three-sector.csv"), Some("Technology Radar")),
    ("long-legend", include_str!("fixtures/long-legend.csv"), None),
    ("text-spacing", include_str!("fixtures/text-spacing.csv"), Some("Text spacing")),
    ("grown", include_str!("fixtures/grown.csv"), None),
];

/// Whitespace runs as one space, the ends dropped: lopdf may break a
/// line with a newline or join two TJs with a space, and a name must
/// still read as itself.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn pdf_of(catalogue: &Catalogue, title: Option<&str>, fonts: &Fonts) -> Vec<u8> {
    let plotted = resolve(catalogue);
    let blocks = Blocks::new(catalogue, &plotted, title, None, fonts);
    let doc = arrange(Arrangement::Wide, &blocks);
    let palette = Palette::built_in(Theme::Light);
    export(&doc, &Export { palette: &palette, fonts, width: None, title, desc: &blocks.radar.name(), producer: "t" })
        .expect("exports")
}

fn extracted(pdf: &[u8]) -> String {
    let parsed = lopdf::Document::load_mem(pdf).expect("parses");
    collapsed(&parsed.extract_text(&[1]).expect("extracts"))
}

/// The names a reader should find, from the catalogue: every item name
/// in legend order (FR-34), then each sector label and declared ring
/// once each, wherever they fall.
fn names(catalogue: &Catalogue) -> (Vec<String>, Vec<String>) {
    let plotted = resolve(catalogue);
    let items = plotted
        .items
        .iter()
        .map(|&i| collapsed(&catalogue.items.get(i).expect("a plotted index").name))
        .collect();
    let labels = catalogue
        .sectors
        .iter()
        .map(|s| collapsed(&s.label))
        .chain(catalogue.rings.iter().map(|r| collapsed(r)))
        .collect();
    (items, labels)
}

/// SC-30: every item name intact, in legend order; every sector and
/// ring present.
#[test]
fn the_pdf_reads_back_as_its_names() {
    let fonts = Fonts::embedded().expect("fonts");
    for (name, csv, title) in CASES {
        let catalogue = catalogue::catalogue(csv, None);
        let text = extracted(&pdf_of(&catalogue, title, &fonts));
        let (items, labels) = names(&catalogue);
        let mut from = 0;
        for item in &items {
            let at = text.get(from..).and_then(|rest| rest.find(item.as_str())).unwrap_or_else(|| {
                panic!("{name}: item '{item}' not found intact after offset {from} in: {text}")
            });
            from += at + item.len();
        }
        for label in &labels {
            assert!(text.contains(label.as_str()), "{name}: '{label}' is not in: {text}");
        }
        if let Some(title) = title {
            assert!(text.contains(&collapsed(title)), "{name}: the title");
        }
    }
}

/// A multi-word name and a letter-spaced heading read back as words
/// (Review Focus 4), and a name Inter lacks adds nothing but does not
/// break the names around it (Review Focus 2).
#[test]
fn awkward_names_read_back() {
    let fonts = Fonts::embedded().expect("fonts");
    let csv = "name,sector,ring,description\nApache Kafka Streams,Tools,Adopt,d\nOffice fittings,Tools,Adopt,d\n東京,Tools,Trial,d\nAVATAR,Tools,Trial,d\n";
    let catalogue = catalogue::catalogue(csv, None);
    let text = extracted(&pdf_of(&catalogue, None, &fonts));
    for want in ["Apache Kafka Streams", "Office fittings", "AVATAR", "Tools", "Adopt", "Trial"] {
        assert!(text.contains(want), "'{want}' in: {text}");
    }
    assert!(!text.contains('東'), "a .notdef box maps to nothing: {text}");
}

/// The negative control: with its ToUnicode maps removed, the same PDF
/// no longer reads back, so the test above is reading the maps (and
/// lopdf really decodes Identity-H through them).
#[test]
fn without_to_unicode_the_names_are_gone() {
    let fonts = Fonts::embedded().expect("fonts");
    let (_, csv, title) = CASES[0];
    let catalogue = catalogue::catalogue(csv, None);
    let mut parsed = lopdf::Document::load_mem(&pdf_of(&catalogue, title, &fonts)).expect("parses");
    for object in parsed.objects.values_mut() {
        if let Object::Dictionary(dict) = object {
            dict.remove(b"ToUnicode");
        }
    }
    let mut stripped = Vec::new();
    parsed.save_to(&mut stripped).expect("saves");
    let text = extracted(&stripped);
    let (items, _) = names(&catalogue);
    let first = items.first().expect("a plotted item");
    assert!(!text.contains(first.as_str()), "'{first}' read back without a map: {text}");
}
```

If `Item::name` or `Sector::label` is not public, or `Catalogue::rings` is not `Vec<String>`, use the public accessor that `src/emit/html` or `tests/serialise` reads them through; the review of this plan read them as public fields. If lopdf without a map returns an error rather than garbage, assert on the error instead: either way the names must not come back.

In `tests/determinism.rs`, add:

```rust
/// NFR-4 for PDF (SC-30): two processes write the same bytes, with no
/// date and no file identifier, across the cases most likely to differ:
/// both themes, a hostile title, a scale that is not a round number,
/// letter-spaced text and a name Inter lacks.
#[test]
fn two_processes_write_the_same_pdf() {
    let spacing = include_str!("fixtures/text-spacing.csv");
    let cjk = "name,sector,ring,description\n中文,Tools,Adopt,d\nKafka,Tools,Trial,d\n";
    let cases: [(&str, &[&str]); 6] = [
        (PRD, &["--theme", "light"]),
        (PRD, &["--theme", "dark"]),
        (HOSTILE, &["--title", "Q3 (draft) \\ Ærø — 中 )"]),
        (PRD, &["--width", "1200"]),
        (spacing, &[]),
        (cjk, &[]),
    ];
    for (csv, args) in cases {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = render(dir.path(), csv, "first.pdf", args);
        let second = render(dir.path(), csv, "second.pdf", args);
        assert!(first == second, "NFR-4: two processes, one PDF ({args:?})");
        let parsed = lopdf::Document::load_mem(&first).expect("parses");
        assert!(parsed.trailer.get(b"ID").is_err(), "no file identifier ({args:?})");
        let info = parsed.trailer.get(b"Info").and_then(lopdf::Object::as_reference).expect("an Info");
        let info = parsed.get_dictionary(info).expect("an info dictionary");
        assert!(info.get(b"CreationDate").is_err() && info.get(b"ModDate").is_err(), "no date ({args:?})");
    }
}
```

The trailer and info dictionary are read through lopdf rather than searched as raw bytes, which include the compressed font data.

- [ ] **Step 2: Run them**

Run: `~/.cargo/bin/cargo test --test pdf_text --test determinism`
Expected: PASS. The writer already exists, so a failure is a real defect: diagnose it with `superpowers:systematic-debugging` rather than loosening the test. A name missing from the extraction usually means a ToUnicode entry missing for its glyphs (Task 5's cluster rule); a name present but out of order means the content stream's paint order differs from the legend's, which is worth reporting before changing either.

- [ ] **Step 3: Commit**

```bash
git add tests/pdf_text.rs tests/determinism.rs
git commit -m "test(pdf): the PDF reads back as its names; two processes agree

SC-30's text check takes its names from the catalogue, not from the
walk the fonts were collected through: every item name intact and in
legend order, every sector and ring, the title; multi-word, kerned and
letter-spaced names read as words, and a name Inter lacks adds nothing.
Stripping the ToUnicode maps makes it fail, so it reads them. Two
processes write the same PDF across both themes, a hostile title, an
awkward width, letter-spacing and a CJK name, with no date or ID.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 11: Documentation

**Files:**
- Modify: `README.md`, `docs/csv-format.md`, `docs/a11y/manual-checks.md`, `docs/design/architecture.md`

No code. One commit.

- [ ] **Step 1: README.**
  - The section `## PNG and SVG` becomes `## PNG, SVG and PDF`. Its opening example block gains `radargen radar.csv -o radar.pdf --theme light`; "Both draw the wide layout" becomes "All three draw the wide layout".
  - Its **Scripts** bullet: after "draw as boxes", add "(in a PDF they also cannot be copied or searched)".
  - Its **Accessibility** bullet gains a last sentence: "A PDF is not tagged for screen readers yet: to share a radar with someone who uses one, send the HTML page."
  - Add a `### PDF` subsection after `### SVG`:

    ```markdown
    ### PDF

    A PDF is one vector page the size of the drawing, with the same radar, key and legend. Its text is real text, so you can select, copy and search the names in any PDF viewer. Links are not clickable, as in the PNG and SVG.

    - **Page size.** `--width` sets the page width in pixels at 96 per inch, so `--width 1056` is an 11-inch-wide page; the height follows. Without `--width` the page is the layout's natural width, about 13 inches for a typical radar. The page is not a paper size: to print, use your viewer's *Fit to page*, and `--legend-columns 2` for a shorter page. A4 and Letter pages are planned.
    - **Printing.** `--theme light` saves ink; the dark theme prints its full background.
    - **Accessibility.** The PDF has a title and its text reads in order, but it is not tagged for screen readers yet. Send the HTML page to someone who uses one.
    ```
  - The Printing bullet in the HTML section (line ~89): "Use a PNG or an SVG for print." → "Use a PDF, PNG or SVG for print."
  - The flags table (lines ~147–151): `--theme` "The colours for PNG, SVG and PDF"; `--legend` "PNG, SVG and PDF always draw it"; `--table` (if it says PNG and SVG) likewise; `--width` "The width in pixels for PNG, SVG and PDF" (pixels, not points, for PDF too); `--legend-columns` "Flows a PNG, SVG or PDF legend".
  - Line ~203: "ignored without a warning for PNG, SVG and PDF".
  - Line ~270: "your CSV, HTML, PNG, SVG and PDF".
  - Wherever the README lists `-o`'s extensions, add `.pdf`.

- [ ] **Step 2: `docs/csv-format.md`** line ~188: "(PNG and SVG only)" → "(PNG, SVG and PDF only; in a PDF they also cannot be copied or searched)". Line 159 already names PDF.

- [ ] **Step 3: `docs/a11y/manual-checks.md`.** Add step 19, after step 18 (the `--palette` page), in the file's existing step format; add its row to the Results table; bump the frontmatter `revision` and add a Revisions row:

    ```markdown
    19. **The PDF** (FR-110, from 1.19.0). Render `three-sector.csv` with `--title "Technology Radar"` to `light.pdf`, again with `--theme dark` to `dark.pdf`, again with `--palette dark.css` (step 18) to `palette.pdf`, again without `--title` to `untitled.pdf`, and a copy of `three-sector.csv` with one item renamed `中文` to `cjk.pdf`. Open each in Preview, Acrobat Reader, Chrome (PDFium) and Firefox (pdf.js).
        - *Expect:* each page looks like its PNG (step 16); at 400% zoom a ring name's halo is smooth; the dark page shows no light seam at its edges; the window title is "Technology Radar", for `untitled.pdf` too.
        - *Expect, selecting:* drag-selecting a legend name and pasting it gives the name as written; a letter-spaced legend heading and a kerned name ("AVATAR" if the fixture has none, add one) paste as words, not spaced letters; a selection across a wrapped name pastes with a space or line break, not run together.
        - *Expect, searching:* searching for an item name and for a ring name finds each.
        - *Expect, Select All, copy, paste into a text editor:* the title, key, ring labels and legend in reading order. This is what a screen reader reads from an untagged PDF; record any disorder.
        - *Expect, `cjk.pdf`:* the name draws as boxes, the file opens without a warning from the viewer, and copying it gives nothing for those characters.
        - *Expect, printing `light.pdf` to A4 or Letter with Fit to page:* the legend text is readable.
        - If installed, `qpdf --check light.pdf` reports no errors and `pdffonts light.pdf` lists two embedded subset fonts with ToUnicode.
        - Small text on Windows may hint slightly differently from the PNG; that is expected. Screen-reader navigation is not checked: the PDF is untagged until rg-ffj.
    ```

    Results row: `| The PDF: Preview, Acrobat Reader, Chrome, Firefox; light, dark, palette | | | | |`.

- [ ] **Step 4: `docs/design/architecture.md`.** In §11 (the SVG converter), add a short subsection:

    ```markdown
    **PDF (FR-110, D-103).** A second converter over the same `Document`, in the crate-private `src/pdf/`: `content.rs` walks the tree as the static serialiser does and writes PDF operators onto a `Canvas` under one flip-and-scale `cm`; `geometry.rs` turns circles, rounded rects and arcs into cubics; `show.rs` places each glyph from `FontMetrics::place` with `TJ` adjustments against the widths array, each text in its own graphics state; `font.rs` subsets each used weight (`subsetter`) and writes Type0/CIDFontType2 with a ToUnicode map from shaper clusters. What every static output shares lives where they all read it: `Node::static_runs` (the glyphs drawn), `Document::ground` (D-87) and `emit::intrinsic_size` (FR-57). `emit/pdf.rs` assembles the file in a fixed object order with no dates or ID, and a `/UserUnit` past viewers' 200-inch limit. The rule that only `text.rs` reads skrifa holds for `src/`; tests may read fonts directly. `tests/pdf_matches_png.rs` checks the PDF with hayro, an independent rasteriser, against the PNG.
    ```

- [ ] **Step 5: Check links and commit**

Run: `~/.cargo/bin/lychee --offline README.md docs/csv-format.md docs/a11y/manual-checks.md docs/design/architecture.md`
Expected: no broken links.

```bash
git add README.md docs/csv-format.md docs/a11y/manual-checks.md docs/design/architecture.md
git commit -m "docs: PDF output in the README, CSV guide, checks and architecture

Documents -o radar.pdf (FR-110): one page the drawing's size, real
searchable text, --width in px at 96 per inch, untagged for now. Adds a
manual check for selection, copying and search in three viewers.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

## After the tasks (orchestrator, not this plan)

Per the repo's workflow: code review until clean; the review seat Copilot would fill is a local multi-agent review (Copilot has no credits until November 2026, so say so in the PR body); simplify; a final review; `crumbs close rg-5uf`; bump **minor** to 1.19.0 and regenerate the changelog; `just ci` green before merge (no Actions credits); merge; tag `v1.19.0` after checking it does not exist; move this plan to `docs/plans/done/` with `status: complete`.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-22 | Initial version, from the PR 20 design with the maintainer. |
| 1.1 | 2026-10-22 | After the five-agent plan review (rust-engineer, qa-expert, ui-designer, architect-reviewer, ux-researcher) and the maintainer's rulings (hayro as the render oracle; one PR). **Oracle:** hayro, an independent rasteriser, replaces the hand-written lopdf-to-SVG redraw (a trial add showed its duplicates are dev-only, which the lint does not count); it moves into Task 8, before `export`, with committed damage tests and a measured budget; the text oracle (Task 10) takes its names from the catalogue, collapses rather than deletes whitespace, and has a ToUnicode-stripped negative control. **Page:** the scale is written unrounded so the drawing meets the MediaBox; a side past 14,400 pt gets a `/UserUnit`; `/Subject` carries the radar's name. **Text:** each text in its own `q … Q`, so a halo's round join cannot reach later strokes; a cluster's text maps single-glyph clusters first; canonically equivalent spellings share a glyph's mapping. **Structure:** `Document::ground` and `emit::intrinsic_size` join `Node::static_runs` in Task 2; `content::Page` is `Canvas`; `pdf/text.rs` is `pdf/show.rs`; `mod pdf` is crate-private; `Faces::of` takes any set of roots (PR 21). **Fixes:** compile and lint faults (`Finish`, `Neutral::TextPrimary`, `unwrap_in_result`, unfulfilled and missing `#[expect]`s), SVG's per-axis corner clamp, the arc piece count at exact quarter turns, `miniz_oxide` 0.9, the text golden in place of a binary one. **Docs:** the README's PDF subsection (page size, print, accessibility), the manual check as step 19 with four viewers, SC-30 scoped to names the font can draw. |
| 1.2 | 2026-10-22 | Shipped as 1.19.0. Beyond the tasks: rustqual's baseline gate (Task 12: IOSP splits, named constants); the five-agent panel's fixes (halos drawn as outlines so each label is text once; the page box rounded up to whole points with the ground over it; ToUnicode entries capped for poppler; the dark golden, interior-colour, halo, raised-mark and CLI tests); a simplify pass (one rounding helper, one line-start rule shared with the SVG outliner, shared test fixtures). The render oracle gates on the worst channel difference (93), measured, since hayro and tiny-skia antialias differently. |
