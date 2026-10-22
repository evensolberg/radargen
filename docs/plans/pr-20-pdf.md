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
status: draft
type: Plan
revision: "1.0"
review_date:
reviewed_by: []
completed_date:
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
  - `text.rs`: one text node's lines and tail as `BT … TJ … ET`, each glyph where our own shaper places it.
  - `mod.rs`: `real`, the one path a number takes into the PDF (NFR-17's rounding, then `f32`).
- **`src/emit/pdf.rs` (new): the entry point.** `export(&Document, &Export) -> Result<Vec<u8>, Fatal>` assembles catalog, page tree, page, content stream, info dictionary and fonts in a fixed object order.
- **One shared walk.** `Node::static_runs` (in `src/svg/mod.rs`) replaces `outline.rs`'s private `runs`/`keys` walk, so the SVG's glyph table and the PDF's font subsets collect the same glyphs.
- **`OutputFormat::Pdf`** joins `options.rs`; the compiler then names every `match` that needs a PDF arm.

**Tech Stack:** Rust 2024; `pdf-writer` 0.15 (MIT OR Apache-2.0; deps bitflags, itoa, memchr, ryu); `subsetter` 0.2 with `default-features = false` (MIT OR Apache-2.0; dep rustc-hash); `miniz_oxide` 0.8 (already in the lockfile through png) for the font stream's FlateDecode. Dev-only: `lopdf` 0.46 with `default-features = false` (MIT), to read the PDF back in tests.

**Spec:** `docs/product/radargen-prd.md`, v0.106 after Task 1. The plan rests on:

- FR-110, D-103 and SC-30, which Task 1 adds;
- FR-45, FR-52, FR-54, FR-56, FR-57, FR-64, FR-71, FR-100, FR-103, FR-104 and FR-106, which Task 1 amends to name PDF;
- NFR-4, NFR-12, NFR-15 and NFR-17; D-75, D-87, D-88 and D-100;
- the dated notes on crumbs `rg-5uf`, `rg-4b8` and `rg-ffj`.

## Global Constraints

- **Wide arrangement only** (FR-71). The PDF exports `arrange(Arrangement::Wide, &blocks)`, with `--legend-columns` applied as for PNG and SVG (FR-100).
- **One page sized to the document.** MediaBox `[0 0 W×0.75 H×0.75]` in points, where `(W, H)` is `emit::svg::intrinsic_size(document, --width)` in CSS px (96 px = 72 pt). No paper sizes (`rg-4b8`).
- **Real text, never outlines.** Every drawn text is glyphs of an embedded subset of the vendored Inter TTF, with a ToUnicode map that gives the CSV's text back.
- **Untagged** (`rg-ffj`). No structure tree, no marked content. The catalog carries `/Lang (en)` and `/ViewerPreferences << /DisplayDocTitle true >>`; the info dictionary carries `/Title` (`--title`, or `Technology Radar`, D-75) and `/Producer (radargen X.Y)` (major.minor, NFR-12).
- **Reproducible** (NFR-4). No `/CreationDate`, no `/ModDate`, no trailer `/ID`. Objects are numbered in a fixed order. Glyph CIDs are assigned in ascending glyph-id order. Every float goes through `pdf::real`.
- **Compression:** only the font file is Flate-compressed (`miniz_oxide::deflate::compress_to_vec_zlib`, level 6). The content stream and the ToUnicode CMap stay uncompressed, so a golden diffs as text.
- **No new duplicate crate.** `cargo clippy --all-targets -- -D warnings` stays clean with `clippy.toml`'s `allowed-duplicate-crates` unchanged; `cargo deny check` stays clean.
- **Errors** are `Fatal` variants, never panics: a glyph that cannot be measured or subset is `Fatal::Font` (NFR-15). An `expect` is allowed only for a broken internal invariant, with the invariant in its message, as `outline.rs` does.
- **Lints:** `as_conversions`, `indexing_slicing`, `string_slice` and `unwrap_used` are denied. An `#[expect]` names its reason inline; the one `f64 → f32` narrowing lives in `pdf::real`.

## Review Focus

1. **A title with PDF-special characters** (`)`, `(`, `\`, non-ASCII such as `Ærø — Q3`) — the info dictionary's `/Title` still parses and reads back exactly. Pinned in Task 8.
2. **Characters the bundled font lacks** (`中文`) — the PDF draws `.notdef` boxes as the PNG does, FR-104's warning says `the PDF draws them as boxes`, and the file is still valid. Pinned in Tasks 5 and 9.
3. **`--width 320` and `--width 10000`** — the MediaBox follows FR-57 and the content scales with it; no coordinate is written past f32's useful precision because content stays in document units under one `cm`. Pinned in Task 8.
4. **Kerning, letter-spacing and ligatures** (legend headings are letter-spaced; `fi` may ligate) — copying returns the original letters, and glyphs sit where the PNG draws them. Pinned in Tasks 5 and 10.
5. **A radar with no title and the dark theme** — the page still has a `/Title` (`Technology Radar`), the ground is the dark `Surface1`, and the render oracle agrees with the dark PNG. Pinned in Tasks 8 and 10.

---

## File Structure

| File | Change | Responsibility |
| --- | --- | --- |
| `docs/product/radargen-prd.md` | Modify | v0.106: FR-110, D-103, SC-30; PDF named where the static formats are |
| `docs/design/architecture.md` | Modify | §11 gains the PDF writer beside the SVG converter |
| `Cargo.toml`, `Cargo.lock` | Modify | `pdf-writer`, `subsetter`, `miniz_oxide`; dev `lopdf` |
| `src/svg/mod.rs` | Modify | `Node::text_runs`, `Node::static_runs` |
| `src/svg/serialise/outline.rs` | Modify | collect through `Node::static_runs`; drop `runs` |
| `src/text.rs` | Modify | `FontMetrics::{bytes, advance, descriptor}`, `Descriptor`, `FontError::new`, `Weight::postscript` |
| `src/pdf/mod.rs` | Create | module root, `real` |
| `src/pdf/geometry.rs` | Create | Béziers for circles, rounded rects, arcs |
| `src/pdf/font.rs` | Create | subsets, CIDs, widths, ToUnicode, font objects |
| `src/pdf/content.rs` | Create | the content stream: walk, shapes, colours |
| `src/pdf/text.rs` | Create | a text node as `BT … TJ … ET` |
| `src/emit/pdf.rs` | Create | `Export`, `export`: the PDF file |
| `src/emit/mod.rs`, `src/lib.rs` | Modify | `pub mod pdf;` in each |
| `src/options.rs` | Modify | `OutputFormat::Pdf` |
| `src/pipeline.rs` | Modify | the `Pdf` arm |
| `src/diagnostics.rs` | Modify | four-extension message; `Undrawn` for PDF; PNG-and-SVG wording |
| `src/cli.rs`, `src/starter.rs` (via `docs/product/config.sample.toml`) | Modify | help and starter config name PDF |
| `tests/golden/pdf/prd-light.pdf` | Create | the golden |
| `tests/serialise/golden.rs` | Modify | `golden_bytes` |
| `tests/serialise/pdf.rs` | Create | golden, names, hostile title |
| `tests/pdf_matches_png.rs` | Create | render oracle and text oracle |
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
| **FR-110** | **PDF**: the shared document (FR-44) as a **one-page vector PDF**, written directly from the document, with no rasterisation and no intermediate SVG (D-103). **The page is the document's size**: its MediaBox is FR-57's intrinsic size at 72 points per 96 CSS pixels, so `--width` sets the page width as it sets an SVG's, and the height follows; paper sizes are not offered. **Its text is text**: every run is drawn in an embedded subset of the bundled Inter face (FR-85), each glyph where the shaper places it (FR-84), with a ToUnicode map, so a reader can select, copy and search the names the CSV holds. **Untagged**: it carries no structure tree; its catalog sets `/Lang (en)` and asks viewers to show the document title, and its info dictionary holds `/Title` (FR-65's title, or *Technology Radar*, D-75) and `/Producer (radargen X.Y)` (SC-25). **Reproducible**: no creation or modification date and no file identifier, so a render is a function of its inputs (NFR-4); only the embedded font is compressed. |
```

- [ ] **Step 3: Amend FR-52 and the formats it lists.** FR-52: `` `.html`, `.png`, `.svg` `` → `` `.html`, `.png`, `.svg`, `.pdf` ``. FR-45: "SVG export writes it out directly; PNG export rasterises it. **One layout engine, two arrangements (FR-68), three output modes**" → "SVG export writes it out directly; PNG export rasterises it; PDF export draws it as a vector page. **One layout engine, two arrangements (FR-68), four output modes**". FR-71: "**PNG and SVG export the wide arrangement.**" → "**PNG, SVG and PDF export the wide arrangement.**"

- [ ] **Step 4: Name PDF where a requirement lists the static formats.** In each of these, "PNG and SVG" / "PNG or SVG" / "PNG, SVG" becomes "PNG, SVG and PDF" / "PNG, SVG or PDF", with no other change: FR-56 ("Applies to PNG and SVG only" → "Applies to PNG, SVG and PDF only"), FR-57 ("Applies to SVG too" → "Applies to SVG and PDF too"), FR-64, FR-100 (its "and to any later static format that exports the wide arrangement" becomes "and to PDF"), FR-103, FR-104 (title and body: a PDF draws a lacking character as the face's missing-glyph box, as PNG and SVG do), FR-105 ("**PNG and SVG do not warn:**" → "**PNG, SVG and PDF do not warn:**"), FR-106, FR-107's static-format sentence, FR-91's "`legend` and `table` on a PNG or SVG", D-100's bold statement, the §6.6 flag table's *Applies to* cells for `--theme`, `--width` and `--legend-columns` (`PNG, SVG` → `PNG, SVG, PDF`), and `link`'s "Static outputs (PNG, SVG, PDF) carry no links" (already right: leave it). **Do not change** FR-55, D-87, FR-64's pixel cap, or any PNG-only rule (the area cap, `WidthClamped`, `PngTextSmall`): those stay PNG's. Run `rg -n "PNG and SVG|PNG or SVG|PNG, SVG|\.png or \.svg" docs/product/radargen-prd.md` after editing and confirm each remaining hit is either PNG-specific, historical (§9's reasoning, §15's history) or already names PDF.

- [ ] **Step 5: FR-58 and G-7.** FR-58's last sentence: "PNG or SVG export is the answer for print, slides, and PDFs" → "PNG, SVG or PDF export is the answer for print and slides; PDF export is the answer for a PDF (FR-110)". G-7: "Output a **PNG and an SVG**" → "Output a **PNG, an SVG and a PDF**" and drop "PDFs," from its list of uses.

- [ ] **Step 6: Add D-103** at the end of §9's table:

```markdown
| **D-103** | **PDF is written directly from the document, its text real text, untagged for now** (FR-110, FR-45, FR-71; D-88, D-75). | *Added from the PR 20 design with the maintainer, 2026-10-22.* **Directly, not through an SVG-to-PDF crate:** the planned route (crumb `rg-5uf`, 2026-10-12) was svg2pdf on the usvg tree the PNG path builds, but svg2pdf 0.13.0 is on usvg 0.45 and krilla-svg 0.8.1 on 0.47, while radargen is on 0.48; a second usvg is a duplicate crate the lint refuses, and the other tree is not ours to hand over. The document already holds everything a PDF needs — shapes, literal colours once a palette binds them, and text the shaper places — so a writer over it with `pdf-writer` is one more emitter beside the SVG converter, deterministic by construction and with no usvg in its path. **Real text, not outlines:** a PDF is searched and copied from in a way an SVG opened for editing is not, so D-88's outlining would cost more here than it saves; the shaper still places every glyph, so the layout is the PNG's and the SVG's. **Untagged for now:** the light PDF has a title, a language and text in reading order, but no structure tree; tagging both PDFs together is crumb `rg-ffj`, after PR 21's table, which needs structure most. **The page is the document's size**; paper sizes come with the full PDF (crumb `rg-4b8`). Rejected: svg2pdf or krilla-svg on an older resvg (a downgrade the PNG goldens and harfrust's pin would both pay for, `rg-v3a`); outlined text (unsearchable); minimal tagging in this PR (it roughly doubles it, and PR 21 would redo it). |
```

- [ ] **Step 7: Add SC-30** after SC-29:

```markdown
| **SC-30** | `-o radar.pdf` writes one page whose MediaBox is FR-57's size at 0.75 pt per px; text extracted from it holds every sector, ring and legend name of the render, in legend order; drawn at the PNG's pixel size it differs from the PNG by no more than antialiasing does (the SVG's SC-7 budget); and two separate runs write identical bytes, with no creation date and no file identifier. |
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

### Task 2: One walk for the glyphs a static rendering draws

**Files:**
- Modify: `src/svg/mod.rs` (add to `impl Node`, after `is_interactive_only`; tests in its `mod tests`)
- Modify: `src/svg/serialise/outline.rs` (`keys`, the module doc; delete `runs`)

**Interfaces:**
- Produces: `Node::text_runs(&self) -> Vec<(TextStyle, &str)>` and `Node::static_runs(&self) -> Vec<(TextStyle, &str)>`, used by Task 5 (`pdf::font`) and by `outline.rs`.

A refactor: the SVG goldens must not change by a byte.

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
    pub fn text_runs(&self) -> Vec<(TextStyle, &str)> {
        match self {
            Self::Text {
                style, lines, tail, ..
            } => lines
                .iter()
                .map(|line| (*style, line.as_str()))
                .chain(tail.iter().map(|t| (t.style, t.text.as_str())))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Every run a static rendering draws, in document order: the
    /// [`text_runs`](Self::text_runs) of each text outside an
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

Delete the `runs` function. In the module doc, replace "Collecting and writing read text through one function, [`runs`], so the table holds every glyph the writer places." with "Collecting reads text through [`Node::static_runs`], which the PDF's subsets share, so the table holds every glyph the writer places." and in `Glyphs::entry`'s doc replace "collected through [`runs`]" with "collected through [`Node::static_runs`]" (both the doc and the `expect` message: `"every placed glyph was collected through Node::static_runs"`). Remove imports `outline.rs` no longer uses (`Tail` if unused); let the compiler say which.

- [ ] **Step 4: Run the tests**

Run: `~/.cargo/bin/cargo test --lib svg:: && ~/.cargo/bin/cargo test --test serialise`
Expected: PASS, the SVG goldens unchanged.

- [ ] **Step 5: Commit**

```bash
git add src/svg/mod.rs src/svg/serialise/outline.rs
git commit -m "refactor(svg): Node::static_runs, one walk for drawn glyphs

The SVG outliner collected its glyph table through a private walk. The
PDF's font subsets (PR 20) need the same set, so the walk moves onto
Node, where both read it and cannot disagree. No output changes.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Béziers for circles, rounded rectangles and arcs

**Files:**
- Modify: `Cargo.toml` (dependencies; see Step 3)
- Create: `src/pdf/mod.rs`, `src/pdf/geometry.rs`
- Modify: `src/lib.rs` (`pub mod pdf;`, alphabetically after `pub mod palette;`)

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
    /// the radius clamped to half the shorter side, as SVG clamps `rx`.
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
        assert!(round.contains(&Step::Move(Point { x: 10.0, y: 0.0 })), "radius 50 clamps to 10: {round:?}");
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

and `pub mod pdf;` in `src/lib.rs`.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --lib pdf::`
Expected: FAIL to compile: `cannot find function circle`, `cannot find type Step`, `cannot find function real`.

- [ ] **Step 3: Add the dependencies** (the later tasks need them; adding them now keeps the lockfile change in one commit):

```bash
~/.cargo/bin/cargo add pdf-writer@0.15
~/.cargo/bin/cargo add subsetter@0.2 --no-default-features
~/.cargo/bin/cargo add miniz_oxide@0.8
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
# The PDF's font stream (FlateDecode). 0.8 is the version png already
# builds, so no third copy (rg-42m).
miniz_oxide = "0.8"
```

and under `[dev-dependencies]`:

```toml
# Reads the PDF back in tests (SC-30): its text and its content stream.
lopdf = { version = "0.46", default-features = false }
```

Run: `~/.cargo/bin/cargo tree -d --target all | /opt/homebrew/bin/rg '^[a-z]'`
Expected: only the pairs `clippy.toml` already allows (miniz_oxide, syn, bitflags) and same-version feature splits (jiff-core, fastrand). Anything else: stop and report.

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
    let rounded = (v * 1000.0).round() / 1000.0;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        reason = "f32 has no From<f64>; pdf-writer writes f32, and the value is rounded to thousandths first so the narrowing is deterministic"
    )]
    let narrowed = rounded as f32;
    // -0.0 writes as `-0`; the PDF has no use for it.
    if narrowed == 0.0 { 0.0 } else { narrowed }
}
```

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

/// A rectangle from `origin`, `radius` on every corner, clamped to half
/// the shorter side as SVG clamps `rx`; square corners are plain lines.
#[must_use]
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub fn rect(origin: Point, width: f64, height: f64, radius: f64) -> Vec<Step> {
    let (x0, y0, x1, y1) = (origin.x, origin.y, origin.x + width, origin.y + height);
    let p = |x, y| Point { x, y };
    let r = radius.min(width / 2.0).min(height / 2.0).max(0.0);
    if crate::num::fmt_coord(r) == "0" {
        return vec![
            Step::Move(p(x0, y0)),
            Step::Line(p(x1, y0)),
            Step::Line(p(x1, y1)),
            Step::Line(p(x0, y1)),
            Step::Close,
        ];
    }
    let k = QUARTER * r;
    vec![
        Step::Move(p(x0 + r, y0)),
        Step::Line(p(x1 - r, y0)),
        Step::Cubic { c1: p(x1 - r + k, y0), c2: p(x1, y0 + r - k), end: p(x1, y0 + r) },
        Step::Line(p(x1, y1 - r)),
        Step::Cubic { c1: p(x1, y1 - r + k), c2: p(x1 - r + k, y1), end: p(x1 - r, y1) },
        Step::Line(p(x0 + r, y1)),
        Step::Cubic { c1: p(x0 + r - k, y1), c2: p(x0, y1 - r + k), end: p(x0, y1 - r) },
        Step::Line(p(x0, y0 + r)),
        Step::Cubic { c1: p(x0, y0 + r - k), c2: p(x0 + r - k, y0), end: p(x0 + r, y0) },
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
    let pieces = (delta.abs() / FRAC_PI_2).ceil().max(1.0);
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
            assert!(std::ptr::eq(face.bytes(), ttf), "{weight:?}: the vendored TTF");
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
    - `Faces::of(document: &Document, fonts: &Fonts) -> Result<Faces, FontError>`
    - `Faces::cid(&self, weight: Weight, glyph: u32) -> u16`
    - `Faces::width(&self, weight: Weight, cid: u16) -> f32` (thousandths of an em, as the widths array writes it)
    - `Faces::resource(weight: Weight) -> pdf_writer::Name<'static>` (`F0`, `F1`)
    - `Faces::write(&self, pdf: &mut pdf_writer::Pdf, next: &mut pdf_writer::Ref, fonts: &Fonts) -> Result<Vec<(Weight, pdf_writer::Ref)>, FontError>` (the Type0 font's ref per used weight, in `Weight` order)
    - `Faces::unicode(&self, weight: Weight, cid: u16) -> Option<&str>` (for tests and Task 10)

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
        let faces = Faces::of(&doc, fonts()).expect("subsets");
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

    /// The ToUnicode map gives back the text each glyph was shaped from:
    /// shaping "Office" and reading each CID's text joins to "Office",
    /// ligature or not (Review Focus 4).
    #[test]
    fn the_unicode_map_gives_the_text_back() {
        let doc = prd();
        let faces = Faces::of(&doc, fonts()).expect("subsets");
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

    /// A text the face cannot draw shapes to .notdef, which is in every
    /// subset and maps to nothing (Review Focus 2).
    #[test]
    fn notdef_is_drawn_and_maps_to_nothing() {
        let (catalogue, order) = plotted("name,sector,ring,description\n中文,Tools,Adopt,d\n", None);
        let blocks = Blocks::new(&catalogue, &order, None, None, fonts());
        let doc = arrange(Arrangement::Wide, &blocks);
        let faces = Faces::of(&doc, fonts()).expect("subsets");
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
            let faces = Faces::of(&doc, fonts()).expect("subsets");
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
use pdf_writer::{Filter, Name, Pdf, Rect, Ref, Str};
use subsetter::GlyphRemapper;

use super::real;
use crate::svg::Document;
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
    pub fn of(document: &Document, fonts: &Fonts) -> Result<Self, FontError> {
        let runs = document.root.static_runs();
        let mut glyphs: BTreeMap<Weight, BTreeSet<u16>> = BTreeMap::new();
        for &(style, text) in &runs {
            let face = fonts.face(style.weight);
            for glyph in face.shape(text) {
                glyphs.entry(style.weight).or_default().insert(gid(face, glyph.id)?);
            }
        }
        let mut subsets = BTreeMap::new();
        for (weight, set) in glyphs {
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
        // Document order, first mapping wins: deterministic, and the
        // first reading of a glyph is the one a reader most likely copies.
        for &(style, text) in &runs {
            let face = fonts.face(style.weight);
            let subset = subsets
                .get_mut(&style.weight)
                .expect("every run's weight was collected above");
            for (glyph, chars) in clusters(face, text) {
                let cid = subset
                    .remapper
                    .get(gid(face, glyph)?)
                    .expect("every shaped glyph was remapped above");
                if cid != 0 && !chars.is_empty() {
                    subset.unicode.entry(cid).or_insert(chars);
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
                .flags(FontFlags::NON_SYMBOLIC)
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
            let start = usize::try_from(g.cluster).unwrap_or(usize::MAX);
            let chars = if seen.insert(start) {
                let end = starts.range(start + 1..).next().copied().unwrap_or(text.len());
                text.get(start..end).unwrap_or_default().to_owned()
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
- `clusters`'s `unwrap_or(usize::MAX)` / `unwrap_or_default()` cover only a cluster that is not a byte offset of `text`, which harfrust guarantees cannot happen; `FontMetrics::missing` treats the same guarantee with an `expect`. If clippy or review prefers that, use `expect("a cluster is the byte offset of a character in text")` instead, as `missing` does.
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
- Consumes: `pdf::geometry::{Step, circle, rect, path}`, `pdf::real` (Task 3); `pdf::font::Faces` (Task 5).
- Produces:
  - `pub struct Page<'a> { pub document: &'a Document, pub palette: &'a Palette, pub fonts: &'a Fonts, pub faces: &'a Faces, pub scale: f64 }`
  - `pub fn content(page: &Page<'_>) -> Vec<u8>` — the whole page's content stream: the flip-and-scale `cm`, miter limit 4, the ground, then the document.
  - `pub(super) fn colour(paint: Paint, palette: &Palette) -> Option<[f32; 3]>` (Task 7 uses it)

Text nodes call `super::text::text(...)`, which Task 7 writes; in this task, write it as a stub that writes nothing so the walk compiles, and Task 7 replaces the stub test-first. (This is the one forward reference: the stub's body is `{}` and its signature is the one Task 7 gives.)

- [ ] **Step 1: Write the failing tests** in `src/pdf/content.rs`'s `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::block::fonts;
    use crate::palette::{Neutral, Role, Theme};
    use crate::svg::{Group, ItemKey, Semantics};

    fn stream(root: Node, palette: &Palette) -> String {
        let document = Document { width: 200.0, height: 100.0, root };
        let faces = Faces::of(&document, fonts()).expect("subsets");
        let page = Page { document: &document, palette, fonts: fonts(), faces: &faces, scale: 0.75 };
        String::from_utf8(content(&page)).expect("ASCII")
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
        assert!(s.ends_with("Q\n"), "{s}");
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
        let ink = Paint::Role(Role::Neutral(Neutral::Text));
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
        // so each painted one opens with `0 0 m`; the unpainted one writes no
        // path at all.
        assert_eq!(s.matches("\n0 0 m\n").count(), 4, "the ground and the three painted rects: {s}");
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
Expected: FAIL to compile: no `Page`, `content`, `colour`.

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
use crate::palette::{Hex, Neutral, Palette, Role};
use crate::svg::{Document, Node, Paint, Point};
use crate::text::Fonts;

/// SVG's default `stroke-miterlimit`; PDF's default is 10.
const MITER_LIMIT: f32 = 4.0;

/// What drawing a page reads.
pub struct Page<'a> {
    /// The arranged document.
    pub document: &'a Document,
    /// The theme's palette, bound to literals (FR-56).
    pub palette: &'a Palette,
    /// The faces text is placed with.
    pub fonts: &'a Fonts,
    /// The embedded subsets.
    pub faces: &'a Faces,
    /// Points per document unit: 0.75 × FR-57's width over the
    /// document's.
    pub scale: f64,
}

/// The whole page: the transform, the ground (D-87), the document.
#[must_use]
pub fn content(page: &Page<'_>) -> Vec<u8> {
    let mut out = Content::new();
    let s = real(page.scale);
    out.save_state()
        .transform([s, 0.0, 0.0, -s, 0.0, real(page.document.height * page.scale)])
        .set_miter_limit(MITER_LIMIT);
    let ground = Node::Rect {
        origin: Point::ORIGIN,
        width: page.document.width,
        height: page.document.height,
        radius: 0.0,
        fill: Paint::Role(Role::Neutral(Neutral::Surface1)),
        stroke: Paint::None,
        stroke_width: 0.0,
    };
    node(&mut out, &ground, page);
    node(&mut out, &page.document.root, page);
    out.restore_state();
    out.finish().to_vec()
}

/// One node, or nothing for an interactive-only one.
fn node(out: &mut Content, node: &Node, page: &Page<'_>) {
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
                self::node(out, child, page);
            }
            out.restore_state();
        }
        Node::Text { .. } => super::text::text(out, node, page),
        Node::Circle { centre, radius, fill, stroke, stroke_width } => {
            shape(out, &geometry::circle(*centre, *radius), Paints::new(*fill, *stroke, *stroke_width), page);
        }
        Node::Rect { origin, width, height, radius, fill, stroke, stroke_width } => {
            shape(out, &geometry::rect(*origin, *width, *height, *radius), Paints::new(*fill, *stroke, *stroke_width), page);
        }
        Node::Line { from, to, stroke, stroke_width } => {
            shape(out, &[Step::Move(*from), Step::Line(*to)], Paints::new(Paint::None, *stroke, *stroke_width), page);
        }
        Node::Path { commands, fill, stroke, stroke_width } => {
            shape(out, &geometry::path(commands), Paints::new(*fill, *stroke, *stroke_width), page);
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
fn shape(out: &mut Content, steps: &[Step], paints: Paints, page: &Page<'_>) {
    let fill = colour(paints.fill, page.palette);
    let stroke = colour(paints.stroke, page.palette);
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

Add the stub `src/pdf/text.rs` (Task 7 replaces it):

```rust
//! A text node as PDF text (FR-110). Written in Task 7.

use pdf_writer::Content;

use super::content::Page;
use crate::svg::Node;

/// `node`'s lines and tail as PDF text.
pub(super) fn text(_out: &mut Content, _node: &Node, _page: &Page<'_>) {}
```

`src/pdf/mod.rs`: `pub mod content;` and `mod text;`.

`content()` tests check exact operator text, which depends on pdf-writer's formatting (`0.75 0 0 -0.75 0 75 cm`, `4 M`, `1.5 w`, `h`, `f`, `S`, `B`). If its output differs in spacing (it writes one operator per line, operands space-separated), adjust the expected strings to what it writes, not the code.

- [ ] **Step 4: Run the tests and lints**

Run: `~/.cargo/bin/cargo test --lib pdf:: && ~/.cargo/bin/cargo clippy --all-targets -- -D warnings`
Expected: PASS; no warnings. If `node`'s match trips the crate's function-length check (the `qual:allow` comment in `serialise/mod.rs` marks the same dispatch), add the same `// qual:allow(complexity, max_function_lines=90) reason: "one dispatch arm per node kind, each only unpacking its fields for its writer"` line above it.

- [ ] **Step 5: Commit**

```bash
git add src/pdf/content.rs src/pdf/text.rs src/pdf/mod.rs
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
- Modify: `src/pdf/text.rs` (replace the stub)

**Interfaces:**
- Consumes: `Page` and `colour` (Task 6); `Faces::{cid, width, resource}` (Task 5); `FontMetrics::place`, `TextStyle::advance`, `Node::tail_x` (existing).
- Produces: `pub(super) fn text(out: &mut Content, node: &Node, page: &Page<'_>)`.

For each line: `BT`, font and size, render mode (fill, or stroke for a halo with its width and round join), a text matrix `[1 0 0 -1 x y]` that flips glyphs back upright under the page's flip, then one `TJ` whose adjustments put each glyph at `place()`'s x. Glyphs with a vertical offset start a new text matrix. A tail is drawn after the last line at `Node::tail_x`, in its own style and fill (D-78).

The `TJ` adjustment after glyph *i* is `(x_i + w_i·size/1000 − x_{i+1}) · 1000/size`, where `w_i` is `Faces::width` (the widths array's own number), so the viewer's pen lands on `x_{i+1}` exactly. One that rounds to zero is not written.

- [ ] **Step 1: Write the failing tests** in `src/pdf/text.rs`'s `mod tests`:

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
        let faces = Faces::of(&document, fonts()).expect("subsets");
        let palette = Palette::built_in(Theme::Light);
        let page = crate::pdf::content::Page { document: &document, palette: &palette, fonts: fonts(), faces: &faces, scale: 1.0 };
        let mut out = Content::new();
        text(&mut out, node, &page);
        (String::from_utf8(out.finish().to_vec()).expect("ASCII operators; strings are hex"), faces)
    }

    fn fill() -> TextPaint {
        TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::Text)))
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
    }

    /// TJ adjustments put each glyph where place() puts it: replaying the
    /// widths array's advances and the adjustments reaches every x.
    #[test]
    fn glyphs_land_where_the_shaper_places_them() {
        let node = label(&["AVATAR Office"], fill(), Anchor::Start, 1.5, None);
        let (s, faces) = written(&node);
        let face = fonts().face(Weight::Regular);
        let placed = face.place("AVATAR Office", 10.0, 1.5);
        let tj = s.lines().find(|l| l.ends_with(" TJ")).expect("a TJ");
        let mut pen = 0.0_f64;
        let mut seen = Vec::new();
        for item in parse_tj(tj) {
            match item {
                Item::Glyphs(cids) => {
                    for cid in cids {
                        seen.push((cid, pen));
                        pen += f64::from(faces.width(Weight::Regular, cid)) * 10.0 / 1000.0;
                    }
                }
                Item::Adjust(a) => pen -= a * 10.0 / 1000.0,
            }
        }
        assert_eq!(seen.len(), placed.len(), "{tj}");
        for ((cid, x), want) in seen.iter().zip(&placed) {
            assert_eq!(*cid, faces.cid(Weight::Regular, want.glyph));
            assert!((x - want.x).abs() < 0.002, "glyph at {x}, placed at {}: {tj}", want.x);
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

    enum Item {
        Glyphs(Vec<u16>),
        Adjust(f64),
    }

    /// `[<0003000A> -12.5 <0004>] TJ` as items. Strings are hex, two
    /// bytes per CID; pdf-writer writes a byte string with non-printable
    /// bytes as hex.
    fn parse_tj(line: &str) -> Vec<Item> {
        let inner = line.trim_start_matches('[').split("] TJ").next().expect("a TJ array");
        let mut items = Vec::new();
        let mut rest = inner;
        while !rest.is_empty() {
            if let Some(after) = rest.strip_prefix('<') {
                let (hex, tail) = after.split_once('>').expect("a closed hex string");
                let bytes: Vec<u8> = (0..hex.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(hex.get(i..i + 2).expect("pairs"), 16).expect("hex"))
                    .collect();
                items.push(Item::Glyphs(
                    bytes.chunks_exact(2).map(|p| u16::from_be_bytes(p.try_into().expect("two bytes"))).collect(),
                ));
                rest = tail;
            } else {
                let end = rest.find(['<', ' ']).unwrap_or(rest.len());
                let (number, tail) = rest.split_at(end);
                if !number.is_empty() {
                    items.push(Item::Adjust(number.parse().expect("a number")));
                }
                rest = tail.trim_start_matches(' ');
            }
        }
        items
    }
}
```

`trim_start_matches` is not one of the `str` methods `clippy.toml` disallows (only `trim`, `trim_start`, `trim_end`).

pdf-writer writes a `Str` as a literal `( … )` string with escapes when it judges that shorter, and as hex `< … >` otherwise. If it writes literal strings, either make `parse_tj` decode literal strings too, or (simpler) write the glyph bytes as hex explicitly: pdf-writer 0.15 has no public hex switch, so prefer extending `parse_tj` with a literal-string branch that handles `\\`, `\(`, `\)` and octal `\ddd` escapes. Do not change the writer to suit the test.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --lib pdf::text::`
Expected: FAIL: the stub writes nothing (`BT` count 0).

- [ ] **Step 3: Implement** `src/pdf/text.rs`:

```rust
//! A text node as PDF text (FR-110): each line in its face's subset, each
//! glyph where the shaper places it (FR-84), so the PDF's layout is the
//! PNG's and the SVG's; a halo as the stroked outline only; the tail after
//! the last line at `Node::tail_x` (D-78).

use pdf_writer::Content;
use pdf_writer::types::{LineJoinStyle, TextRenderingMode};
use pdf_writer::Str;

use super::content::{Page, colour};
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
pub(super) fn text(out: &mut Content, node: &Node, page: &Page<'_>) {
    let Node::Text { origin, style, ink, anchor, lines, leading, tail, .. } = node else {
        return;
    };
    let Some(mode) = paint(out, *ink, page) else {
        return;
    };
    let mut baseline = origin.y;
    for (i, line) in (0_u32..).zip(lines) {
        baseline = origin.y + f64::from(i) * leading;
        let x = match anchor {
            Anchor::Start => origin.x,
            Anchor::Middle => origin.x - style.advance(line, page.fonts) * CENTRED,
        };
        run(out, line, *style, Point { x, y: baseline }, mode, page);
    }
    if let (Some(tail), Some(x)) = (tail, node.tail_x(page.fonts)) {
        debug_assert!(matches!(ink, TextPaint::Fill(_)), "only a filled text carries a tail (D-78)");
        if paint(out, TextPaint::Fill(tail.ink), page).is_some() {
            run(out, &tail.text, tail.style, Point { x, y: baseline }, TextRenderingMode::Fill, page);
        }
    }
}

/// Sets `ink`'s colour (and a halo's width and join), returning the
/// render mode, or `None` when nothing is painted.
fn paint(out: &mut Content, ink: TextPaint, page: &Page<'_>) -> Option<TextRenderingMode> {
    match ink {
        TextPaint::Fill(fill) => {
            let [r, g, b] = colour(fill, page.palette)?;
            out.set_fill_rgb(r, g, b);
            Some(TextRenderingMode::Fill)
        }
        TextPaint::Halo { stroke, width } => {
            let [r, g, b] = colour(stroke, page.palette)?;
            out.set_stroke_rgb(r, g, b)
                .set_line_width(real(width))
                .set_line_join(LineJoinStyle::RoundJoin);
            Some(TextRenderingMode::Stroke)
        }
    }
}

/// One line of `text` from `at`, its glyphs split wherever the vertical
/// offset changes, each piece one text matrix and one TJ.
fn run(out: &mut Content, text: &str, style: TextStyle, at: Point, mode: TextRenderingMode, page: &Page<'_>) {
    let placed = page
        .fonts
        .face(style.weight)
        .place(text, style.size, style.letter_spacing);
    if placed.is_empty() {
        return;
    }
    out.begin_text();
    out.set_font(Faces::resource(style.weight), real(style.size));
    out.set_text_rendering_mode(mode);
    for piece in placed.chunk_by(|a, b| a.y == b.y) {
        let Some(first) = piece.first() else { continue };
        out.set_text_matrix([1.0, 0.0, 0.0, -1.0, real(at.x + first.x), real(at.y + first.y)]);
        show(out, piece, style, page);
    }
    out.end_text();
}

/// One TJ for `piece`: runs of CIDs, with an adjustment wherever the
/// widths array's advance does not reach the next glyph's x.
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
fn show(out: &mut Content, piece: &[Placed], style: TextStyle, page: &Page<'_>) {
    let mut shown = out.show_positioned();
    let mut items = shown.items();
    let mut bytes: Vec<u8> = Vec::new();
    for (i, glyph) in piece.iter().enumerate() {
        let cid = page.faces.cid(style.weight, glyph.glyph);
        bytes.extend(cid.to_be_bytes());
        let Some(next) = piece.get(i + 1) else { break };
        let advance = f64::from(page.faces.width(style.weight, cid)) * style.size / EM;
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
git add src/pdf/text.rs
git commit -m "feat(pdf): text placed glyph by glyph from the shaper

Each line is one BT/ET in its face's subset, upright under the page's
flip, with TJ adjustments computed from the widths array so every glyph
lands where FontMetrics::place puts it: kerning and letter-spacing as
the PNG draws them (FR-84, FR-110). A halo is stroke mode with a round
join; a tail follows the last line at Node::tail_x (D-78).

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: The PDF file

**Files:**
- Create: `src/emit/pdf.rs`
- Modify: `src/emit/mod.rs` (`pub mod pdf;`, after `pub mod png;`)
- Modify: `tests/serialise/golden.rs` (`golden_bytes`), `tests/serialise/main.rs` (`mod pdf;`)
- Create: `tests/serialise/pdf.rs`, `tests/golden/pdf/prd-light.pdf` (blessed)

**Interfaces:**
- Consumes: `Faces` (Task 5), `content::{Page, content}` (Task 6), `emit::svg::intrinsic_size` (existing).
- Produces:
  - `pub struct Export<'a> { pub palette: &'a Palette, pub fonts: &'a Fonts, pub width: Option<u32>, pub title: Option<&'a str>, pub producer: &'a str }`
  - `pub fn export(document: &Document, export: &Export<'_>) -> Result<Vec<u8>, Fatal>`
  - `pub const POINTS_PER_PX: f64 = 0.75`

Object numbers are fixed: 1 catalog, 2 page tree, 3 page, 4 content stream, 5 info, then the fonts from 6 (Task 5's order).

- [ ] **Step 1: Write the failing tests.** In `src/emit/pdf.rs`'s `mod tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::block::fonts;
    use crate::layout::test_support::{PRD_EXAMPLE, plotted};
    use crate::palette::Theme;

    fn prd(title: Option<&str>) -> Document {
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        let blocks = Blocks::new(&catalogue, &order, title, None, fonts());
        arrange(Arrangement::Wide, &blocks)
    }

    fn pdf(doc: &Document, title: Option<&str>, width: Option<u32>, theme: Theme) -> Vec<u8> {
        let palette = Palette::built_in(theme);
        export(doc, &Export { palette: &palette, fonts: fonts(), width, title, producer: "radargen 9.9" })
            .expect("exports")
    }

    fn media_box(bytes: &[u8]) -> [f32; 4] {
        let doc = lopdf::Document::load_mem(bytes).expect("parses");
        let page = *doc.get_pages().values().next().expect("one page");
        let dict = doc.get_dictionary(page).expect("a page dictionary");
        let mb = dict.get(b"MediaBox").and_then(lopdf::Object::as_array).expect("a MediaBox");
        let n = |i: usize| mb.get(i).and_then(|o| o.as_float().ok()).expect("a number");
        [n(0), n(1), n(2), n(3)]
    }

    /// FR-110, FR-57: one page, the document's size at 0.75 pt per px,
    /// or `--width` px wide with the height following (Review Focus 3).
    #[test]
    fn the_page_is_the_documents_size() {
        let doc = prd(None);
        let natural = media_box(&pdf(&doc, None, None, Theme::Light));
        assert_eq!(natural, [0.0, 0.0, real(doc.width * 0.75), real(doc.height * 0.75)]);
        for width in [320, 10_000] {
            let got = media_box(&pdf(&doc, None, Some(width), Theme::Light));
            let (w, h) = crate::emit::svg::intrinsic_size(&doc, Some(width));
            assert_eq!(got, [0.0, 0.0, real(w * 0.75), real(h * 0.75)], "--width {width}");
        }
    }

    /// FR-110, NFR-4: the same bytes twice; no date, no file identifier;
    /// the title, language and producer are set (D-75, SC-25).
    #[test]
    fn the_file_is_reproducible_and_named() {
        let doc = prd(None);
        let first = pdf(&doc, None, None, Theme::Dark);
        assert!(first == pdf(&doc, None, None, Theme::Dark), "NFR-4");
        let text = String::from_utf8_lossy(&first);
        for absent in ["/CreationDate", "/ModDate", "/ID"] {
            assert!(!text.contains(absent), "{absent}");
        }
        for present in ["/Title (Technology Radar)", "/Producer (radargen 9.9)", "/Lang (en)", "/DisplayDocTitle true"] {
            assert!(text.contains(present), "{present}");
        }
        assert!(first.starts_with(b"%PDF-1.7\n"));
    }

    /// Review Focus 1: a title with PDF string delimiters and non-ASCII
    /// reads back exactly.
    #[test]
    fn a_hostile_title_reads_back() {
        let title = r"Q3 (draft) \ Ærø — 中";
        let doc = prd(Some(title));
        let bytes = pdf(&doc, Some(title), None, Theme::Light);
        let parsed = lopdf::Document::load_mem(&bytes).expect("parses");
        let info = parsed.trailer.get(b"Info").and_then(lopdf::Object::as_reference).expect("an Info");
        let got = parsed.get_dictionary(info).and_then(|d| d.get(b"Title")).expect("a Title");
        assert_eq!(lopdf::decode_text_string(got).expect("a text string"), title);
    }
}
```

If `lopdf::decode_text_string` is not the 0.46 name, use the function lopdf provides for decoding a PDF text string (PDFDocEncoding or UTF-16BE with BOM); check with `cargo doc -p lopdf`.

In `tests/serialise/golden.rs`, add beside `golden`:

```rust
/// Byte-compares `got` with `tests/golden/{dir}/{name}`, for a binary
/// golden such as the PDF (its font stream is compressed). The producer
/// the test writes is already [`PLACEHOLDER`](super::version::PLACEHOLDER),
/// so nothing is replaced. `UPDATE_GOLDEN=1` blesses, as for [`golden`].
pub fn golden_bytes(dir: &str, name: &str, got: &[u8]) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(dir)
        .join(name);
    let want = bless_or_read(&path, got);
    if want != got {
        let at = want.iter().zip(got).take_while(|(a, b)| a == b).count();
        panic!(
            "{name} differs at byte {at} (want {} bytes, got {}); diff the uncompressed content with `git diff --text`",
            want.len(),
            got.len()
        );
    }
}
```

`tests/serialise/pdf.rs`:

```rust
//! The exported PDF's golden (FR-110, NFR-4).

use radargen::emit::pdf::{Export, export};
use radargen::layout::arrange::{Arrangement, arrange};
use radargen::palette::{Palette, Theme};

use super::golden::golden_bytes;
use super::version::PLACEHOLDER;
use super::prd;

/// The PRD example as a light PDF, its producer the placeholder, so a
/// minor bump changes no byte.
#[test]
fn prd_pdf_matches_its_golden() {
    let built = prd();
    let doc = arrange(Arrangement::Wide, &built.blocks);
    let palette = Palette::built_in(Theme::Light);
    let bytes = export(
        &doc,
        &Export { palette: &palette, fonts: &built.fonts, width: None, title: Some("Technology Radar"), producer: PLACEHOLDER },
    )
    .expect("exports");
    golden_bytes("pdf", "prd-light.pdf", &bytes);
}
```

Use the field names `Built` actually has (read `tests/serialise/main.rs`'s `build`/`Built`); if it does not keep the fonts, call `radargen::text::Fonts::embedded()` here.

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --lib emit::pdf:: ; ~/.cargo/bin/cargo test --test serialise pdf`
Expected: FAIL to compile: no `emit::pdf`.

- [ ] **Step 3: Implement** `src/emit/pdf.rs`:

```rust
//! The PDF export (FR-110, D-103): the wide document as one vector page
//! the document's size, its text real text in embedded Inter subsets,
//! untagged, with no date or identifier (NFR-4).

use pdf_writer::{Name, Pdf, Rect, Ref, TextStr};

use crate::diagnostics::Fatal;
use crate::emit::UNTITLED;
use crate::emit::svg::intrinsic_size;
use crate::palette::Palette;
use crate::pdf::content::{Page, content};
use crate::pdf::font::Faces;
use crate::pdf::real;
use crate::svg::Document;
use crate::text::Fonts;

/// PDF points per CSS pixel: 72 per inch over 96.
pub const POINTS_PER_PX: f64 = 0.75;

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
    let (width, height) = intrinsic_size(document, export.width);
    let faces = Faces::of(document, export.fonts)?;
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
    p.parent(tree)
        .media_box(Rect::new(0.0, 0.0, real(width * POINTS_PER_PX), real(height * POINTS_PER_PX)))
        .contents(stream);
    let mut resources = p.resources();
    let mut dict = resources.fonts();
    for (weight, font) in &fonts {
        dict.pair(Faces::resource(*weight), *font);
    }
    dict.finish();
    resources.finish();
    p.finish();

    let drawn = content(&Page {
        document,
        palette: export.palette,
        fonts: export.fonts,
        faces: &faces,
        scale: POINTS_PER_PX * width / document.width,
    });
    pdf.stream(stream, &drawn);

    pdf.document_info(info)
        .title(TextStr(export.title.unwrap_or(UNTITLED)))
        .producer(TextStr(export.producer));
    Ok(pdf.finish())
}
```

`Faces::write` numbers the font objects from 6 because `next` has handed out 1–5; the fonts are written before the page so the page can name their refs, and object numbers, not file order, are what the fixed-order rule fixes. `Name` may turn out unused; drop it if so. `Fatal::Font` already has `#[from] FontError`, so `?` converts.

`src/emit/mod.rs`: `pub mod pdf;`. `tests/serialise/main.rs`: `mod pdf;`.

- [ ] **Step 4: Bless and run**

Run: `UPDATE_GOLDEN=1 ~/.cargo/bin/cargo test --test serialise prd_pdf_matches_its_golden` (expected: FAIL with `blessed …prd-light.pdf`), then:

Run: `~/.cargo/bin/cargo test --lib emit::pdf:: && ~/.cargo/bin/cargo test --test serialise`
Expected: PASS.

Open `tests/golden/pdf/prd-light.pdf` in Preview and in a browser: the radar, key and legend as the PNG shows them; drag-select a legend name and paste it; search for `Adopt`. Note the result for the PR body.

- [ ] **Step 5: Commit**

```bash
git add src/emit/pdf.rs src/emit/mod.rs tests/serialise/golden.rs tests/serialise/main.rs tests/serialise/pdf.rs tests/golden/pdf/prd-light.pdf
git commit -m "feat(emit): the PDF file, one page the document's size

Catalog with /Lang and DisplayDocTitle, one page whose MediaBox is
FR-57's size at 0.75 pt per px, the content stream, the fonts, and an
info dictionary with /Title (D-75) and /Producer. No dates and no ID, in
a fixed object order, so two exports are the same bytes (FR-110,
NFR-4). Adds the PRD golden.

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
                producer: OUTPUT_VERSION,
            },
        )?),
```

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

### Task 10: The PDF draws what the PNG draws, and reads back as text (SC-30)

**Files:**
- Create: `tests/pdf_matches_png.rs`
- Modify: `tests/determinism.rs`

**Interfaces:**
- Consumes: `emit::pdf`, `emit::png::{pixel_size, rasterise, usvg_options}`, `lopdf`, `skrifa` and `resvg` (normal dependencies of the crate, so integration tests can use them), `tests/common/catalogue.rs`.

**Why not a PDF rasteriser:** `hayro`, the pure-Rust PDF renderer, brings second copies of `skrifa`, `read-fonts` and `fearless_simd`, which the duplicate-crate lint refuses. Instead the oracle **reads the PDF back** with lopdf and **redraws it as an SVG** — paths from the content stream's operators, glyphs outlined from the embedded subset font itself — which resvg rasterises and compares with the PNG, as `svg_matches_png.rs` compares the exported SVG. That checks the content stream, the transforms, the colours, the TJ positioning, the widths array and the embedded font file all at once, none of them through our own writer's code.

- [ ] **Step 1: Write the oracle and its tests** in `tests/pdf_matches_png.rs`:

```rust
//! SC-30 (FR-110): the PDF, read back from its bytes and redrawn, draws
//! what our PNG draws to within antialiasing (the SVG's SC-7 budget), and
//! its text reads back as the render's names, in legend order.
//!
//! The redraw handles exactly the operators the writer emits — q Q cm M w
//! j RG rg m l c h f S B BT ET Tf Tr Tm TJ — and fails on any other, so a
//! writer change that adds one cannot pass unread.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use lopdf::content::Content;
use lopdf::{Document as Pdf, Object};
use radargen::emit::pdf::{Export, export};
use radargen::emit::png::{pixel_size, rasterise, usvg_options};
use radargen::layout::arrange::{Arrangement, Blocks, arrange};
use radargen::order::resolve;
use radargen::palette::{Palette, Theme};
use radargen::text::Fonts;
use resvg::tiny_skia::Pixmap;
use resvg::usvg;
use skrifa::MetadataProvider as _;

#[path = "common/catalogue.rs"]
mod catalogue;

/// svg_matches_png.rs's budget and ceiling: the same antialiasing residue.
const BUDGET_PPM: usize = 123;
const MAX_DELTA: u8 = 64;

const CASES: [(&str, &str, Option<&str>); 3] = [
    ("three-sector", include_str!("fixtures/three-sector.csv"), Some("Technology Radar")),
    ("text-spacing", include_str!("fixtures/text-spacing.csv"), Some("Text spacing")),
    ("long-legend", include_str!("fixtures/long-legend.csv"), None),
];

/// A 2×3 affine matrix, PDF's [a b c d e f].
#[derive(Clone, Copy)]
struct M([f64; 6]);

impl M {
    const ID: Self = Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    /// `self` then `outer`: PDF's `cm` premultiplies.
    fn then(self, outer: Self) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let [oa, ob, oc, od, oe, of] = outer.0;
        Self([
            a * oa + b * oc,
            a * ob + b * od,
            c * oa + d * oc,
            c * ob + d * od,
            e * oa + f * oc + oe,
            e * ob + f * od + of,
        ])
    }
    fn apply(self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.0;
        (a * x + c * y + e, b * x + d * y + f)
    }
}

fn num(o: &Object) -> f64 {
    o.as_float().map(f64::from).expect("a number operand")
}

/// One embedded font: its subset face's outlines and widths, by CID.
struct Font {
    data: Vec<u8>,
    widths: BTreeMap<u16, f64>,
}

/// Every font the page names, by resource name, with its embedded file.
fn fonts(pdf: &Pdf, page: lopdf::ObjectId) -> BTreeMap<Vec<u8>, Font> {
    let mut out = BTreeMap::new();
    for (name, dict) in pdf.get_page_fonts(page).expect("fonts") {
        let descendant = dict
            .get(b"DescendantFonts").and_then(Object::as_array).expect("Type0")
            .first().and_then(|o| o.as_reference().ok()).expect("a CIDFont ref");
        let cid = pdf.get_dictionary(descendant).expect("CIDFont");
        let descriptor = cid.get(b"FontDescriptor").and_then(Object::as_reference).expect("a descriptor");
        let file = pdf.get_dictionary(descriptor).and_then(|d| d.get(b"FontFile2")).and_then(Object::as_reference).expect("FontFile2");
        let data = pdf.get_object(file).and_then(Object::as_stream).expect("a stream")
            .decompressed_content().expect("inflates");
        // W is [0 [w0 w1 …]] as the writer writes it.
        let w = cid.get(b"W").and_then(Object::as_array).expect("W");
        let first = w.first().and_then(|o| o.as_i64().ok()).expect("a start CID");
        let first = u16::try_from(first).expect("16-bit");
        let list = w.get(1).and_then(|o| o.as_array().ok()).expect("a width list");
        let widths = list.iter().enumerate()
            .map(|(i, o)| (first + u16::try_from(i).expect("16-bit"), num(o)))
            .collect();
        out.insert(name, Font { data, widths });
    }
    out
}

/// The page redrawn as SVG paths in device space (y down, px).
fn redraw(bytes: &[u8], px_per_pt: f64) -> String {
    let pdf = Pdf::load_mem(bytes).expect("parses");
    let page = *pdf.get_pages().values().next().expect("a page");
    let mb = pdf.get_dictionary(page).and_then(|d| d.get(b"MediaBox")).and_then(Object::as_array).expect("MediaBox");
    let (w_pt, h_pt) = (num(mb.get(2).expect("w")), num(mb.get(3).expect("h")));
    let fonts = fonts(&pdf, page);
    let ops = Content::decode(&pdf.get_page_content(page).expect("content")).expect("decodes").operations;
    // Device: PDF points, y up, to px, y down.
    let device = M([px_per_pt, 0.0, 0.0, -px_per_pt, 0.0, h_pt * px_per_pt]);
    let mut svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}">"#, w_pt * px_per_pt, h_pt * px_per_pt);
    let (mut ctm, mut stack) = (M::ID, Vec::new());
    let (mut fill, mut stroke, mut width, mut round) = ("#000000".to_owned(), "#000000".to_owned(), 1.0, false);
    let mut d = String::new();
    let (mut font, mut size, mut mode, mut tm) = (Vec::new(), 0.0, 0_i64, M::ID);
    // The writer's channels are c/255 rounded to thousandths, so ×255
    // rounds back to the byte exactly.
    let hex = |o: &[Object]| {
        let c = |i: usize| {
            let v = (num(o.get(i).expect("rgb")) * 255.0).round();
            (0_u8..=255).find(|&b| f64::from(b) == v).expect("a channel")
        };
        format!("#{:02X}{:02X}{:02X}", c(0), c(1), c(2))
    };
    for op in ops {
        let o = &op.operands;
        let at = |i: usize| num(o.get(i).expect("operand"));
        let full = |m: M| m.then(ctm).then(device);
        match op.operator.as_str() {
            "q" => stack.push((ctm, fill.clone(), stroke.clone(), width, round)),
            "Q" => (ctm, fill, stroke, width, round) = stack.pop().expect("balanced q/Q"),
            "cm" => ctm = M([at(0), at(1), at(2), at(3), at(4), at(5)]).then(ctm),
            "M" | "BT" | "ET" => {}
            "w" => width = at(0),
            "j" => round = at(0) == 1.0,
            "rg" => fill = hex(o),
            "RG" => stroke = hex(o),
            "m" | "l" => {
                let (x, y) = full(M::ID).apply(at(0), at(1));
                let _ = write!(d, "{}{x} {y} ", if op.operator == "m" { "M" } else { "L" });
            }
            "c" => {
                let m = full(M::ID);
                let (ax, ay) = m.apply(at(0), at(1));
                let (bx, by) = m.apply(at(2), at(3));
                let (x, y) = m.apply(at(4), at(5));
                let _ = write!(d, "C{ax} {ay} {bx} {by} {x} {y} ");
            }
            "h" => d.push_str("Z "),
            "f" | "S" | "B" => {
                let scale = (full(M::ID).0[0].powi(2) + full(M::ID).0[1].powi(2)).sqrt();
                let f = if op.operator == "S" { "none".to_owned() } else { fill.clone() };
                let s = if op.operator == "f" { "none".to_owned() } else { stroke.clone() };
                let _ = write!(svg, r#"<path d="{d}" fill="{f}" stroke="{s}" stroke-width="{}"/>"#, width * scale);
                d.clear();
            }
            "Tf" => {
                font = o.first().and_then(|n| n.as_name().ok()).expect("a font name").to_vec();
                size = at(1);
            }
            "Tr" => mode = o.first().and_then(|n| n.as_i64().ok()).expect("a mode"),
            "Tm" => tm = M([at(0), at(1), at(2), at(3), at(4), at(5)]),
            "TJ" => {
                let f = fonts.get(&font).expect("a declared font");
                let face = skrifa::FontRef::new(&f.data).expect("the embedded subset parses");
                let outlines = face.outline_glyphs();
                let upem = f64::from(face.head().expect("head").units_per_em());
                let mut pen = 0.0;
                for item in o.first().and_then(|a| a.as_array().ok()).expect("a TJ array") {
                    match item {
                        Object::String(bytes, _) => {
                            for pair in bytes.chunks_exact(2) {
                                let cid = u16::from_be_bytes(pair.try_into().expect("two bytes"));
                                // Text space: em-scaled, at the pen, through Tm and the CTM.
                                let glyph = M([size / upem, 0.0, 0.0, size / upem, pen, 0.0]).then(tm);
                                let m = full(glyph);
                                let mut pen_path = Pen(String::new(), m);
                                if let Some(g) = outlines.get(skrifa::GlyphId::new(u32::from(cid))) {
                                    g.draw(skrifa::outline::DrawSettings::unhinted(skrifa::instance::Size::unscaled(), skrifa::instance::LocationRef::default()), &mut pen_path).expect("draws");
                                }
                                if !pen_path.0.is_empty() {
                                    let scale = (full(tm).0[0].powi(2) + full(tm).0[1].powi(2)).sqrt();
                                    let paint = if mode == 1 {
                                        format!(r#"fill="none" stroke="{stroke}" stroke-width="{}" stroke-linejoin="{}""#, width * scale, if round { "round" } else { "miter" })
                                    } else {
                                        format!(r#"fill="{fill}""#)
                                    };
                                    let _ = write!(svg, r#"<path d="{}" {paint}/>"#, pen_path.0);
                                }
                                pen += f.widths.get(&cid).copied().expect("a width") * size / 1000.0;
                            }
                        }
                        other => pen -= num(other) * size / 1000.0,
                    }
                }
            }
            other => panic!("the writer emitted an operator the oracle does not read: {other}"),
        }
    }
    svg.push_str("</svg>");
    svg
}

/// Collects a glyph's outline in device space.
struct Pen(String, M);

impl skrifa::outline::OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.1.apply(f64::from(x), f64::from(y));
        let _ = write!(self.0, "M{x} {y} ");
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.1.apply(f64::from(x), f64::from(y));
        let _ = write!(self.0, "L{x} {y} ");
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (cx, cy) = self.1.apply(f64::from(cx), f64::from(cy));
        let (x, y) = self.1.apply(f64::from(x), f64::from(y));
        let _ = write!(self.0, "Q{cx} {cy} {x} {y} ");
    }
    fn curve_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        let (a, b) = self.1.apply(f64::from(c1x), f64::from(c1y));
        let (c, d) = self.1.apply(f64::from(c2x), f64::from(c2y));
        let (x, y) = self.1.apply(f64::from(x), f64::from(y));
        let _ = write!(self.0, "C{a} {b} {c} {d} {x} {y} ");
    }
    fn close(&mut self) {
        self.0.push_str("Z ");
    }
}

/// SC-30: redrawn at the PNG's size, the PDF differs from the PNG by no
/// more than antialiasing, in both themes (Review Focus 5).
#[test]
fn the_pdf_draws_what_the_png_draws() {
    let fonts = Fonts::embedded().expect("fonts");
    for (name, csv, title) in CASES {
        for theme in [Theme::Light, Theme::Dark] {
            let catalogue = catalogue::catalogue(csv, None);
            let plotted = resolve(&catalogue);
            let blocks = Blocks::new(&catalogue, &plotted, title, None, &fonts);
            let doc = arrange(Arrangement::Wide, &blocks);
            let palette = Palette::built_in(theme);
            let size = pixel_size(&doc, None).expect("a size");
            let png = Pixmap::decode_png(&rasterise(&doc, &palette, size).expect("rasterises")).expect("decodes");
            let pdf = export(&doc, &Export { palette: &palette, fonts: &fonts, width: None, title, producer: "t" }).expect("exports");
            let svg = redraw(&pdf, f64::from(size.width()) / (doc.width * 0.75));
            let tree = usvg::Tree::from_str(&svg, &usvg_options()).expect("the redraw parses");
            let mut drawn = Pixmap::new(png.width(), png.height()).expect("a pixmap");
            resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut drawn.as_mut());
            let (mut differing, mut worst) = (0_usize, 0_u8);
            for (a, b) in png.data().iter().zip(drawn.data()) {
                let delta = a.abs_diff(*b);
                if delta > 0 { differing += 1; }
                worst = worst.max(delta);
            }
            let ppm = differing * 1_000_000 / png.data().len();
            assert!(worst <= MAX_DELTA, "{name} {theme:?}: a subpixel differs by {worst}");
            assert!(ppm <= BUDGET_PPM, "{name} {theme:?}: {ppm} ppm differ");
        }
    }
}

/// SC-30: the text reads back with every name the render draws, in
/// legend order (Review Focus 4: letter-spaced headings, kerning).
#[test]
fn the_pdf_reads_back_as_its_names() {
    let fonts = Fonts::embedded().expect("fonts");
    for (name, csv, title) in CASES {
        let catalogue = catalogue::catalogue(csv, None);
        let plotted = resolve(&catalogue);
        let blocks = Blocks::new(&catalogue, &plotted, title, None, &fonts);
        let doc = arrange(Arrangement::Wide, &blocks);
        let palette = Palette::built_in(Theme::Light);
        let pdf = export(&doc, &Export { palette: &palette, fonts: &fonts, width: None, title, producer: "t" }).expect("exports");
        let parsed = Pdf::load_mem(&pdf).expect("parses");
        let text = parsed.extract_text(&[1]).expect("extracts");
        let squashed: String = text.split_whitespace().collect();
        let mut from = 0;
        for (_, run) in doc.root.static_runs() {
            let want: String = run.split_whitespace().collect();
            if want.is_empty() { continue; }
            let at = squashed.get(from..).and_then(|rest| rest.find(&want)).unwrap_or_else(|| {
                panic!("{name}: '{run}' not found after offset {from} in: {text}")
            });
            from += at + want.len();
        }
    }
}
```

The redraw is written without `as` casts or slice indexing, since `--all-targets` lints tests too; if clippy still flags something in it (`suboptimal_flops` on the matrix arithmetic, say), fix it the way the rest of the suite does, with an `#[expect]` and its reason, never `#[allow]`. `static_runs` is document (paint) order, which for the wide arrangement is title, key, radar labels, then the legend in reading order; if the radar's labels are painted in an order lopdf extracts differently (lopdf follows the content stream, which follows paint order), the in-order search still holds because both read the same order. Names are compared with whitespace removed because lopdf inserts spaces between TJ items heuristically.

In `tests/determinism.rs`, add:

```rust
/// NFR-4 for PDF (SC-30): two processes write the same bytes, with no
/// date and no file identifier.
#[test]
fn two_processes_write_the_same_pdf() {
    let dir = tempfile::tempdir().expect("tempdir");
    for theme in ["light", "dark"] {
        let first = render(dir.path(), PRD, "first.pdf", &["--theme", theme]);
        let second = render(dir.path(), PRD, "second.pdf", &["--theme", theme]);
        assert!(first == second, "NFR-4: two processes, one PDF ({theme})");
        let text = String::from_utf8_lossy(&first);
        assert!(!text.contains("/CreationDate") && !text.contains("/ID"), "no date, no ID");
    }
}
```

- [ ] **Step 2: Run them**

Run: `~/.cargo/bin/cargo test --test pdf_matches_png --test determinism`
Expected: PASS. These are oracle tests over code that already exists, so a failure here is a real defect to diagnose with `superpowers:systematic-debugging`, not a test to loosen. If the render oracle fails on text only, compare the redraw's glyph positions with `FontMetrics::place` first; if it fails on shapes only, compare the arc conversion with the SVG's arcs.

Plant a defect to confirm the oracle bites: temporarily make `pdf::text::show` skip the last glyph of each piece, run `the_pdf_draws_what_the_png_draws`, confirm it fails, and revert. Note the planted defect and its result in the commit message.

- [ ] **Step 3: Commit**

```bash
git add tests/pdf_matches_png.rs tests/determinism.rs
git commit -m "test(pdf): the PDF draws what the PNG draws, and reads as text

SC-30's oracle reads the PDF back with lopdf and redraws it as SVG —
paths from the content stream, glyphs outlined from the embedded subset
itself — for resvg to compare with the PNG within the SVG's SC-7
budget, in both themes. Text extracted from it holds every drawn name in
order. Two processes write the same PDF. hayro was not used: it would
add second copies of skrifa, read-fonts and fearless_simd. A planted
dropped glyph fails the render oracle.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 11: Documentation

**Files:**
- Modify: `README.md`, `docs/csv-format.md`, `docs/a11y/manual-checks.md`, `docs/design/architecture.md`

No code. One commit.

- [ ] **Step 1: README.**
  - The section `## PNG and SVG` becomes `## PNG, SVG and PDF`, with a paragraph after its existing SVG text:

    ```markdown
    A `.pdf` output is one vector page the size of the drawing, with the same radar, key and legend. Its text is real text, so you can select, copy and search the names in any PDF viewer. `--width` sets the page width in pixels at 96 per inch (so `--width 1056` is an 11-inch-wide page), and the height follows. It is not tagged for screen readers yet.
    ```
  - The flags table (lines ~147–151): `--theme` "The colours for PNG, SVG and PDF"; `--legend` "PNG, SVG and PDF always draw it"; `--table` (if it says PNG and SVG) likewise; `--width` "The width in pixels for PNG, SVG and PDF"; `--legend-columns` "Flows a PNG, SVG or PDF legend".
  - Line ~203: "ignored without a warning for PNG, SVG and PDF".
  - Line ~270: "your CSV, HTML, PNG, SVG and PDF".
  - Wherever the README lists `-o`'s extensions, add `.pdf`.

- [ ] **Step 2: `docs/csv-format.md`** line ~188: "(PNG and SVG only)" → "(PNG, SVG and PDF only)". Line 159 already names PDF.

- [ ] **Step 3: `docs/a11y/manual-checks.md`.** Add a step after the SVG step (step 17), numbered next in the file's sequence, following the file's existing step format, and bump its frontmatter `revision` and Revisions table:

    ```markdown
    **PDF (FR-110).** Render the three-sector fixture to `radar.pdf` and open it in Preview, a browser and Acrobat Reader. In each: the page shows the radar, key and legend as the PNG does; the window title is the radar's title; drag-selecting a legend name and pasting it gives the name as written; searching for a ring name finds it. Record the viewer and its version in the results table. Screen-reader reading is not checked: the PDF is untagged until rg-ffj.
    ```

- [ ] **Step 4: `docs/design/architecture.md`.** In §11 (the SVG converter), add a short subsection:

    ```markdown
    **PDF (FR-110, D-103).** A second converter over the same `Document`, in `src/pdf/`: `content.rs` walks the tree as the static serialiser does and writes PDF operators under one flip-and-scale `cm`; `geometry.rs` turns circles, rounded rects and arcs into cubics; `text.rs` places each glyph from `FontMetrics::place` with `TJ` adjustments against the widths array; `font.rs` subsets each used weight (`subsetter`) and writes Type0/CIDFontType2 with a ToUnicode map from shaper clusters. `Node::static_runs` is the one walk both this and the SVG outliner collect glyphs through. `emit/pdf.rs` assembles the file in a fixed object order with no dates or ID.
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
