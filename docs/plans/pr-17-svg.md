---
tags:
  - plan
  - svg
  - output
aliases: []
doc_id: RG-017A
doc_name: pr-17-svg
crumb_id: rg-m2l
document_title: "PR 17 — SVG export with outlined, deduplicated text: Implementation Plan"
created_date: 2026-10-16
synopsis: "-o radar.svg writes the wide arrangement (FR-71) as a standalone SVG: literal colours for --theme (FR-56) over D-87's ground, every text drawn as glyph outlines defined once per face and size in <defs> and placed with <use> (architecture §11), so no font is referenced (SC-15) and PowerPoint, Figma and Illustrator draw it as resvg does. --width sets the intrinsic size, the height following (FR-57). The file carries <!-- radargen 1.2 --> (SC-25), a <title> and <desc>, and names every outlined text for assistive technology. Ships as 1.2.0."
status: draft
type: Plan
revision: "1.0"
review_date:
reviewed_by: []
completed_date:
comments: "1.0 awaits the five plan reviews (Rust, QA, architecture, security, accessibility) and the maintainer's confirmation of D-88."
---
# PR 17 — SVG export with outlined, deduplicated text: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-m2l` · **Release:** 1.2.0 · **After:** 1.1.0 (#90) · **Then:** PR 18, `--palette`, 1.3.0 (`rg-cdo`)

**Goal:** `radargen radar.csv -o radar.svg --theme dark --width 1920` writes a standalone SVG of the wide arrangement. Its pixels match the PNG's to within antialiasing (SC-7), it references no font or other resource (SC-15), and it is byte-identical run to run (NFR-4, SC-8).

**Architecture:**

- **Text is outlined, as architecture §11 and `docs/spikes/svg-font.md` decided.** An embedded `@font-face` draws nothing in resvg and the default face in PowerPoint. So each glyph a text uses is shaped by the same `harfrust` pass that measures it, drawn by `skrifa` into an SVG path, defined **once per face and size** in `<defs>`, and placed with `<use xlink:href="#gN" x= y=>`.
- **The converter is a text mode of the serialiser, not a second serialiser.** The crumb's PR 13 note says to extend `svg::serialise` rather than splicing its string. So:
  - `serialise`'s `Context` gains a `TextMode`: `Text` (HTML, PNG: `<text>` as today) or `Outlined` (SVG). Only the text arm of `node()` reads it.
  - A new `serialise::standalone` writes the export's prelude (`<!-- radargen X.Y -->`, the root's size, `<title>`, `<desc>`, `<defs>`), then the same ground and body `serialise` writes.
  - `Document` is unchanged, and the HTML and PNG bytes are unchanged. The only exceptions are the HTML's generator line, which says 1.2, and the PNG, which carries no version.
- **`src/text.rs` owns reading the fonts.** It gains `FontMetrics::shape` (glyph ids, clusters and font-unit advances), `place` (each glyph's position, built on `shape` exactly as `measure` is), and `outline` (a glyph's contours, via `skrifa`). `measure` is rebuilt on `shape`, so measuring and placing cannot drift apart.
- **`src/emit/svg.rs`** (architecture §3's planned file) is thin:
  - FR-57's intrinsic size;
  - the `<title>`, from the title or D-75's *Technology Radar*;
  - the `<desc>`, the radar's group name;
  - the call to `standalone`.
- **The pipeline:** `.svg` is a format. `render` arranges the wide arrangement only (FR-71) and returns the bytes for `write_atomic` (NFR-14). `Fatal::FormatNotInThisVersion` loses its last trigger and is removed.

**Tech stack:**

- Rust 2024.
- `skrifa` 0.44 moves from dev-dependency to dependency, `default-features = false, features = ["std"]`. usvg already builds it, and it shares `read-fonts` 0.41 with `harfrust`, so `harfrust::FontRef` is the `FontRef` skrifa reads. No new crate enters the tree.

**Spec:**

- `docs/product/radargen-prd.md` v0.72, amended to v0.73 by Task 0. The plan rests on: FR-45, FR-52, FR-54, FR-56, FR-57, FR-64, FR-71, NFR-4, NFR-5, NFR-6, NFR-7, NFR-12, NFR-14, NFR-17, SC-7, SC-8, SC-9, SC-15, SC-18, SC-25, D-75, D-78, D-87, and the new D-88.
- Architecture §3 (`src/emit/svg.rs`), §5 (literal colours, interactive-only nodes dropped), §11 (outlining, `<use>`, `tail_x`), §12 (the waist).
- `docs/spikes/svg-font.md` and `tests/outlines_match_text.rs`, which measured the form built here.

## Global Constraints

- SVG output is the **wide arrangement** only, drawn as one document (FR-71), not D-85's split parts.
- Colours are **literal**, resolved for `--theme` (FR-56), through `Target::Static`, which also drops every interactive-only node (architecture §5).
- **No text element and no font reference** in the exported file: no `<text>`, no `font-family`, no `@font-face`, no `<style>` (SC-15, D-88). Every `xlink:href` is a fragment (`#gN`) into the file's own `<defs>` (NFR-5).
- **Byte-identical** run to run and across platforms (NFR-4, SC-8). Every number is written by `num::fmt_coord`, and all arithmetic is plain `*` and `+`, never `mul_add` (`suboptimal_flops` is expected, with the project's standard reason).
- `--width` sets `width` and `height` on the root, the height following the document's aspect (FR-57). It is held to FR-64's range by clap, as today. The SVG is **not clamped, capped or warned about** (D-88): it allocates nothing and scales without loss.
- The file opens with `<!-- radargen X.Y -->` from `emit::OUTPUT_VERSION` (SC-25), and ends with one LF (NFR-17).
- Every string the CSV or a flag supplies reaches the file through `escape::xml_text` or `escape::xml_attr` (NFR-6, SC-9).
- A failure writes **nothing** and leaves an existing `-o` file byte-unchanged (NFR-8, NFR-14, SC-18).
- Strict clippy: pedantic, nursery and cargo. `as_conversions` and `string_slice` are denied, so every cast needs an `#[expect]` with a reason. No `unwrap`; `missing_docs`.
- A test that writes a golden uses `UPDATE_GOLDEN=1`, which is refused under `CI`.

## Rulings

1. **A glyph is defined once per face and size, already scaled.** The spike measured exactly this form: paths in user units at NFR-17's three decimals, 82 differing subpixels in 3.96 million.
   - **The alternative:** font-unit paths under a `scale(size / 2048)` transform. It is smaller, since there is one definition per face rather than per size, but the scale (0.006103515625 at 12.5) does not survive `fmt_coord`'s three decimals. Writing it would need a second float formatter, which NFR-17 rules out.
   - **What the defined form costs:** the same glyph at two sizes is two `<path>`s. The wide arrangement uses a handful of sizes, so the file stays tens of kilobytes. Task 5 records the measured size. (D-88)
2. **`xlink:href`, not SVG 2's bare `href`.** Every consumer FR-54 names reads SVG 1.1's `xlink:href`, and SVG 2 still requires it to be honoured. Bare `href` is SVG 2 only, and Illustrator's and PowerPoint's support for it is not something this plan can confirm. resvg reads both. (D-88)
3. **Every outlined text outside a control is `role="img"`, named by its text.** Outlined text has no text content, so without this a heading group would be nameless, and the ring, quadrant and key labels would be silent. An image named by its words keeps them readable, and a heading's name is computed from its content, which includes its images' names. So the title stays a level-1 heading named by the title.
   - **Not done to:** a halo, which is already `aria-hidden`; and text inside a marker or legend row, whose control already carries a name and has presentational children (architecture §5).
   - **The root is not `role="img"`.** That would flatten the headings and the markers' names into one image (crumb note, PR 13). The root has a `<title>` and `<desc>` instead.
   - **What is still lost:** selecting and searching text. The spike records that as a decision. (D-88)
4. **The intrinsic size is unrounded and unclamped.** `--width 1000` on a 1025.4 × 824 document writes `width="1000" height="803.588"`. An SVG's size is a length, not a pixel count, so rounding it would only change the aspect. Clamping (D-87) exists to bound a pixmap, and an SVG has none. A default past 10,000 units is written at its own width, and the small-text warning is the PNG's alone, since a vector scales up without loss. (D-88)
5. **`Fatal::FormatNotInThisVersion` is removed, not left idle.** With `.svg` written, no extension FR-52 names is deferred any more. A variant with no trigger is dead code. PR 18 adds no format. `UnrecognisedExtension` now reads "must end in .html, .png or .svg".
6. **An outline failure is `FontError`, so it reads as NFR-15's font failure.** `FontMetrics::outline` fails only if the face lacks a glyph id its own shaper produced, or skrifa cannot draw it. Either means the vendored face is broken, and that is what `FontError` already says ("the embedded Inter Regular font cannot be read: …"). Like `Fatal::Rasterise`, it has no end-to-end trigger.
7. **The release is 1.2.0** (maintainer's rule, 2026-10-16). Every PR is a release, and a feature earns a minor bump. The HTML goldens are re-blessed for `radargen 1.2`. The PNG goldens are untouched, because PNG carries no version.

## Review Focus

1. **`--theme dark`, rasterised by someone else.** Any text left as `<text>`, any `var()`, or any glyph left unpainted would draw wrong, or not at all, outside a browser. Expected: rasterised by resvg with an **empty font database**, the SVG matches our own PNG to within the spike's antialiasing allowance, in both themes and on several fixtures. Test: `the_svg_draws_what_the_png_draws` (Task 4).
2. **Hostile names, titles and quadrants:** `</title>`, `]]>`, `<script>`, `&`, `"` and U+2028 (SC-9). Their only routes into the file are `<title>`, `<desc>` and `aria-label`. Expected: the file parses as XML, holds only the export's vocabulary, and each string reads back exactly as written. Test: `hostile_text_is_only_named_never_markup` (Task 4).
3. **A name in a script Inter lacks** (東京, 😀). Expected: the shaper's `.notdef` glyph is outlined like any other, so the SVG draws the same boxes the PNG does, and the run succeeds. Tests: `a_name_inter_lacks_draws_as_the_png_does` (Task 4), and `an_svg_of_hostile_text_renders` end to end (Task 4).
4. **`--width` on SVG:** 320, 10000, and a default past 10,000 units. Expected: `width` and `height` on the root, the height in proportion and unrounded, no clamp, and no warning. Tests:
   - `the_intrinsic_size_follows_the_width` and `a_wide_default_is_not_clamped` (Task 4);
   - `renders_an_svg_at_the_width_asked` and `an_svg_gives_no_png_warnings` (Task 4).
5. **Every text shape in the wide arrangement:** middle-anchored numbers, multi-line names, letter-spaced headings, halos, and legend tails placed at `tail_x` (D-78). Expected: each glyph sits where resvg draws it. Tests:
   - `place_advances_as_measure_does` (Task 1);
   - the serialiser unit tests in Task 3;
   - the pixel comparison in Task 4, over `long-legend.csv` (tails and wraps) and `text-spacing.csv`.

---

## Tasks

### Task 0: PRD v0.73 (D-88) and architecture

**Files:**
- Modify: `docs/product/radargen-prd.md` (frontmatter `Version`, `date modified`; D-88 after D-87; revision 0.73 row)
- Modify: `docs/design/architecture.md` (§3 `emit/svg.rs` row, §11 consequences)

- [ ] **Step 1: Add D-88 after D-87**

```markdown
| **D-88** | **The exported SVG outlines its text, names it, and takes its size from `--width` unclamped** (FR-54, FR-57, NFR-1, SC-15, SC-25). | *Added by the PR 17 plan, crumb `rg-m2l`.* **Outlines:** every text is drawn as glyph outlines, defined once per face and size in `<defs>` and placed by `<use xlink:href x y>` (architecture §11, `docs/spikes/svg-font.md`): no `<text>`, no `font-family`, no `@font-face`, so PowerPoint, Figma and Illustrator draw it as resvg does. `xlink:href` rather than SVG 2's bare `href`, which not every SVG 1.1 consumer reads. **Size:** `width` and `height` on the root are `--width` and the height in proportion, unrounded, or the document's own size by default; neither is clamped, capped or warned about, because an SVG allocates nothing and scales without loss (D-87's limits are the PNG's). **Names:** the root carries a `<title>` (the title, or D-75's *Technology Radar*) and a `<desc>` (the radar's group name), and is not `role="img"`, which would flatten its headings and markers. Every outlined text outside a control is `role="img"` named by its words, so headings keep their names and labels stay readable; halos stay hidden, and a control's own name covers its text. Text cannot be selected or searched, as the spike decided. **Errors:** with `.svg` written, no extension FR-52 names is deferred, so the "arrives in" message is gone. Ships as 1.2.0. |
```

- [ ] **Step 2: Revision row and frontmatter**

Set `Version: 0.73` and `date modified: 2026-10-16`, then append:

```markdown
| 0.73 | 2026-10-16 | **The exported SVG** (D-88; rg-m2l): outlined text deduplicated through `<use xlink:href>`, an unclamped intrinsic size from `--width`, a `<title>` and `<desc>` on a root that is not an image, every outlined text named for assistive technology. SVG ships as 1.2.0. |
```

- [ ] **Step 3: Architecture §3 and §11**

In §3's module table, the `emit/svg.rs` row reads: "The SVG export (FR-54, D-88): FR-57's intrinsic size, the `<title>` and `<desc>`, and `serialise::standalone`. The outlining itself is `svg/serialise/outline.rs`, a text mode of the serialiser, over `text.rs`'s `shape`, `place` and `outline`."

In §11, replace the first consequence's opening ("`emit/svg.rs` is a converter, not a serialiser. It needs glyph outlines from `skrifa` and a shaping pass it shares with `text.rs`.") with: "**The SVG export is a converter, not a serialiser** — built as a text mode of the one serialiser rather than as a second one, so `Document`, colour binding and the dropping of interactive-only nodes are shared (D-88). It needs glyph outlines from `skrifa` and the shaping pass `text.rs` measures with."

- [ ] **Step 4: Commit**

```bash
git add docs/product/radargen-prd.md docs/design/architecture.md
git commit -m "docs(prd): D-88, the exported SVG (v0.73)"
```

---

### Task 1: `text.rs` shapes, places and outlines

**Files:**
- Modify: `Cargo.toml` (`skrifa` from dev-dependencies to dependencies)
- Modify: `src/text.rs`
- Test: `src/text.rs` (unit tests module, beside the existing ones)

**Interfaces:**
- Produces:
  - `pub struct ShapedGlyph { pub id: u32, pub cluster: u32, pub x_advance: i32, pub x_offset: i32, pub y_offset: i32 }`, in font units.
  - `pub struct Placed { pub glyph: u32, pub x: f64, pub y: f64 }`, in user units from the line's start, y down.
  - `FontMetrics::shape(&self, text: &str) -> Vec<ShapedGlyph>`
  - `FontMetrics::place(&self, text: &str, size_px: f64, letter_spacing_px: f64) -> Vec<Placed>`
  - `FontMetrics::outline(&self, glyph: u32, pen: &mut impl skrifa::outline::OutlinePen) -> Result<(), FontError>`, in font units, y up.
  - `FontMetrics::units_per_em(&self) -> f64`
  - `Weight` derives `PartialOrd, Ord`.
- `measure` keeps its signature and its results, bit for bit.

- [ ] **Step 1: Move skrifa**

In `Cargo.toml`, delete the `skrifa` entry and its comment from `[dev-dependencies]`, and add to `[dependencies]`:

```toml
# SVG export (D-88): glyph outlines. The version usvg reads fonts with,
# sharing read-fonts with harfrust, so no second copy builds (rg-v3a).
skrifa = { version = "0.44", default-features = false, features = ["std"] }
```

Run: `cargo tree -d -e normal | rg -v miniz_oxide`
Expected: no duplicate crates other than `miniz_oxide`, which `clippy.toml` already allows.

- [ ] **Step 2: Write the failing tests**

Add to `src/text.rs`'s tests module:

```rust
/// Placing and measuring share one shaping pass: the last glyph ends
/// exactly where `measure` says the line ends (D-88).
#[test]
fn place_advances_as_measure_does() {
    let fonts = Fonts::embedded().expect("loads");
    for (weight, text, size, spacing) in [
        (Weight::Regular, "Kubernetes", 12.5, 0.0),
        (Weight::SemiBold, "PLATFORMS", 11.0, 0.8),
        (Weight::Regular, "AV Wa. To", 17.0, 0.0),
        (Weight::Regular, "\u{a0}· New", 10.5, 0.2),
    ] {
        let face = fonts.face(weight);
        let placed = face.place(text, size, spacing);
        let shaped = face.shape(text);
        let last = placed.last().expect("glyphs");
        let advance = f64::from(shaped.last().expect("glyphs").x_advance) * size
            / face.units_per_em();
        assert_eq!(
            last.x + advance,
            face.measure(text, size, spacing),
            "{text:?} at {size}"
        );
    }
}

/// Letter-spacing falls between clusters, not after the last (FR-84).
#[test]
fn spacing_falls_between_clusters() {
    let fonts = Fonts::embedded().expect("loads");
    let face = fonts.face(Weight::Regular);
    let tight = face.place("AB", 12.0, 0.0);
    let loose = face.place("AB", 12.0, 2.0);
    assert_eq!(tight.first().map(|g| g.x), Some(0.0), "the first glyph starts the line");
    let gap = |p: &[Placed]| p.get(1).expect("two glyphs").x;
    assert_eq!(gap(&loose) - gap(&tight), 2.0);
}

/// A glyph the shaper produced can be outlined; an empty one draws nothing.
#[test]
fn shaped_glyphs_outline() {
    struct Count(usize);
    impl skrifa::outline::OutlinePen for Count {
        fn move_to(&mut self, _: f32, _: f32) { self.0 += 1; }
        fn line_to(&mut self, _: f32, _: f32) { self.0 += 1; }
        fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) { self.0 += 1; }
        fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) { self.0 += 1; }
        fn close(&mut self) {}
    }
    let fonts = Fonts::embedded().expect("loads");
    let face = fonts.face(Weight::SemiBold);
    let [k, space] = face.shape("K ").try_into().expect("two glyphs");
    let mut ink = Count(0);
    face.outline(k.id, &mut ink).expect("draws");
    assert!(ink.0 > 0, "K has contours");
    let mut blank = Count(0);
    face.outline(space.id, &mut blank).expect("draws");
    assert_eq!(blank.0, 0, "a space has none");
    assert!(face.outline(u32::MAX, &mut blank).is_err(), "no such glyph");
}
```

- [ ] **Step 3: Run them to verify they fail**

Run: `cargo test --lib text::tests 2>&1 | tail -5`
Expected: compile errors: `place`, `shape`, `outline` and `units_per_em` are not found.

- [ ] **Step 4: Implement**

In `src/text.rs`, add `face: &'static str` and `upem: f64` to `FontMetrics`, set in `load`:

```rust
let data = ShaperData::new(&font);
let upem = f64::from(data.shaper(&font).build().units_per_em());
Ok(Self { font, data, face, upem })
```

Derive `PartialOrd, Ord` on `Weight`. Then add the types and methods, and rebuild `measure` on `shape`:

```rust
/// One glyph as the shaper lays it out, in font units (FR-84).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapedGlyph {
    /// The glyph's id in its face.
    pub id: u32,
    /// The character cluster it belongs to; a letter-spacing gap falls
    /// between clusters, never inside one.
    pub cluster: u32,
    /// How far the pen moves past it.
    pub x_advance: i32,
    /// How far it sits right of the pen.
    pub x_offset: i32,
    /// How far it sits above the baseline.
    pub y_offset: i32,
}

/// Where one glyph is drawn: user units from its line's start, y down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// The glyph's id in its face.
    pub glyph: u32,
    /// From the line's start.
    pub x: f64,
    /// From the baseline, down.
    pub y: f64,
}

impl FontMetrics<'_> {
    /// `text` shaped as `usvg` shapes it: left to right (A-4), segment
    /// properties guessed, default options.
    #[must_use]
    pub fn shape(&self, text: &str) -> Vec<ShapedGlyph> {
        if text.is_empty() {
            return Vec::new();
        }
        // A built shaper borrows the font and its data, so it is built per
        // call rather than stored; `usvg` does the same.
        let shaper = self.data.shaper(&self.font).build();
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_direction(Direction::LeftToRight);
        buffer.guess_segment_properties();
        let output = shaper.shape(buffer, ShapeOptions::new());
        output
            .glyph_infos()
            .iter()
            .zip(output.glyph_positions())
            .map(|(info, position)| ShapedGlyph {
                id: info.glyph_id,
                cluster: info.cluster,
                x_advance: position.x_advance,
                x_offset: position.x_offset,
                y_offset: position.y_offset,
            })
            .collect()
    }

    /// The face's units per em.
    #[must_use]
    pub const fn units_per_em(&self) -> f64 {
        self.upem
    }

    /// Where each glyph of `text` is drawn at `size_px`, with
    /// `letter_spacing_px` between clusters as [`Self::measure`] counts it,
    /// so the last glyph ends where `measure` says the line does (D-88).
    #[must_use]
    #[expect(
        clippy::suboptimal_flops,
        reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
    )]
    pub fn place(&self, text: &str, size_px: f64, letter_spacing_px: f64) -> Vec<Placed> {
        let scale = size_px / self.upem;
        let mut pen = 0.0;
        let mut previous = None;
        self.shape(text)
            .into_iter()
            .map(|glyph| {
                if previous.is_some_and(|cluster| cluster != glyph.cluster) {
                    pen += letter_spacing_px;
                }
                previous = Some(glyph.cluster);
                let placed = Placed {
                    glyph: glyph.id,
                    x: pen + f64::from(glyph.x_offset) * scale,
                    y: -f64::from(glyph.y_offset) * scale,
                };
                pen += f64::from(glyph.x_advance) * scale;
                placed
            })
            .collect()
    }

    /// Glyph `glyph`'s contours in font units, y up, unhinted, into `pen`.
    ///
    /// # Errors
    ///
    /// [`FontError`] when the face has no such glyph or cannot draw it:
    /// the shaper produced an id its own face does not hold (NFR-15).
    pub fn outline(
        &self,
        glyph: u32,
        pen: &mut impl skrifa::outline::OutlinePen,
    ) -> Result<(), FontError> {
        use skrifa::MetadataProvider as _;
        use skrifa::instance::{LocationRef, Size};
        use skrifa::outline::DrawSettings;
        let fail = |reason: String| FontError {
            face: self.face,
            reason,
        };
        let outlines = self.font.outline_glyphs();
        let found = outlines
            .get(skrifa::GlyphId::new(glyph))
            .ok_or_else(|| fail(format!("glyph {glyph} is not in the face")))?;
        found
            .draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                pen,
            )
            .map(|_| ())
            .map_err(|e| fail(format!("glyph {glyph} cannot be outlined: {e}")))
    }
}
```

`measure` keeps its doc comment and its `expect`s, and its body becomes:

```rust
let shaped = self.shape(text);
let units: i32 = shaped.iter().map(|g| g.x_advance).sum();
let boundaries = shaped
    .windows(2)
    .filter(|pair| matches!(pair, [a, b] if a.cluster != b.cluster))
    .count();
// …the existing `gaps` conversion and `width` expression, with
// `self.upem` in place of `f64::from(shaper.units_per_em())`.
```

If `harfrust::FontRef` turns out not to be the type skrifa's `MetadataProvider` reads, the read-fonts versions have drifted. Then `load` also builds a `skrifa::FontRef::new(data)`, mapping its error to `FontError` the same way, and `outline` reads that one. Record this as a ruling.

- [ ] **Step 5: Run the tests to verify they pass, and that nothing moved**

Run: `cargo test --lib text::tests && cargo test --test measure --test wrap --test agrees_with_resvg --test tail_oracle`
Expected: all pass. The oracle tests compare `measure` with resvg, so a changed bit would fail them.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/text.rs
git commit -m "feat(text): shape, place and outline glyphs for the SVG export"
```

---

### Task 2: The glyph table

**Files:**
- Create: `src/svg/serialise/outline.rs` (the table half; Task 3 adds the writer)
- Modify: `src/svg/serialise/mod.rs` (`mod outline;`)
- Test: `src/svg/serialise/tests/outline.rs`, registered in `src/svg/serialise/tests/mod.rs`

**Interfaces:**
- Consumes (Task 1): `FontMetrics::shape`, `outline`, `units_per_em`; `Weight: Ord`.
- Produces (crate-private to `serialise`):
  - `pub(super) struct Glyphs`, built by `Glyphs::of(document: &Document, fonts: &Fonts) -> Result<Glyphs, FontError>`.
  - `Glyphs::id(&self, style: TextStyle, glyph: u32) -> Option<usize>`: `None` for a glyph that draws nothing.
  - `Glyphs::defs(&self) -> String`: `<defs>` with one `<path id="gN" d="…"/>` per line, or `""` when no glyph has ink.

- [ ] **Step 1: Write the failing tests**

`src/svg/serialise/tests/outline.rs`:

```rust
//! The glyph table (D-88): each glyph that inks, once per face and size.

use super::super::outline::Glyphs;
use crate::layout::block::fonts;
use crate::style::TextStyle;
use crate::svg::{Anchor, Document, Group, Node, Paint, Point, TextPaint};
use crate::palette::{Neutral, Role};
use crate::text::Weight;

const NAME: TextStyle = TextStyle { weight: Weight::Regular, size: 12.5, letter_spacing: 0.0 };
const HEAD: TextStyle = TextStyle { weight: Weight::SemiBold, size: 12.5, letter_spacing: 0.0 };

pub(super) fn text(style: TextStyle, lines: &[&str]) -> Node {
    Node::Text {
        origin: Point { x: 10.0, y: 20.0 },
        style,
        ink: TextPaint::Fill(Paint::Role(Role::Neutral(Neutral::Ink))),
        anchor: Anchor::Start,
        lines: lines.iter().map(|&l| l.to_owned()).collect(),
        leading: 15.0,
        tail: None,
    }
}

pub(super) fn doc(children: Vec<Node>) -> Document {
    Document { width: 200.0, height: 100.0, root: Node::Group(Group::new(Point::ORIGIN, children)) }
}

fn ids(glyphs: &Glyphs, style: TextStyle, text: &str) -> Vec<Option<usize>> {
    let fonts = fonts();
    fonts.face(style.weight).shape(text).iter().map(|g| glyphs.id(style, g.id)).collect()
}

#[test]
fn a_glyph_is_defined_once() {
    let glyphs = Glyphs::of(&doc(vec![text(NAME, &["aaa", "a"])]), fonts()).expect("draws");
    assert_eq!(glyphs.defs().matches("<path ").count(), 1, "{}", glyphs.defs());
    assert_eq!(ids(&glyphs, NAME, "aa"), [Some(0), Some(0)]);
}

#[test]
fn a_face_or_size_is_a_glyph_of_its_own() {
    let small = TextStyle { size: 9.0, ..NAME };
    let glyphs = Glyphs::of(
        &doc(vec![text(NAME, &["a"]), text(HEAD, &["a"]), text(small, &["a"])]),
        fonts(),
    )
    .expect("draws");
    assert_eq!(glyphs.defs().matches("<path ").count(), 3);
}

#[test]
fn a_blank_glyph_has_no_definition() {
    let glyphs = Glyphs::of(&doc(vec![text(NAME, &["a b"])]), fonts()).expect("draws");
    assert_eq!(ids(&glyphs, NAME, "a b"), [Some(0), None, Some(0)]);
    assert_eq!(glyphs.defs().matches("<path ").count(), 1);
}

/// Ids follow (face, size, glyph), never the order text is met, so the
/// same document writes the same ids (NFR-4).
#[test]
fn ids_follow_the_glyphs_not_the_document() {
    let ab = Glyphs::of(&doc(vec![text(NAME, &["b"]), text(NAME, &["a"])]), fonts()).expect("draws");
    let ba = Glyphs::of(&doc(vec![text(NAME, &["a"]), text(NAME, &["b"])]), fonts()).expect("draws");
    assert_eq!(ab.defs(), ba.defs());
}

/// What a static target drops is not outlined (architecture §5).
#[test]
fn interactive_only_text_is_not_outlined() {
    use crate::svg::{ItemKey, Semantics};
    let indicator = Node::Group(
        Group::new(Point::ORIGIN, vec![text(NAME, &["x"])]).with(Semantics::Indicator(ItemKey(2))),
    );
    let glyphs = Glyphs::of(&doc(vec![indicator]), fonts()).expect("draws");
    assert_eq!(glyphs.defs(), "");
}

/// A path is the glyph at its size, y down, through `fmt_coord`, and holds
/// no paint, so each `<use>` paints it (D-88).
#[test]
fn a_definition_is_a_bare_path() {
    let glyphs = Glyphs::of(&doc(vec![text(NAME, &["l"])]), fonts()).expect("draws");
    let defs = glyphs.defs();
    assert!(defs.starts_with("<defs>\n<path id=\"g0\" d=\"M"), "{defs}");
    assert!(defs.ends_with("Z\"/>\n</defs>\n"), "{defs}");
    assert!(!defs.contains("fill") && !defs.contains("stroke"), "{defs}");
    // An `l` stands on the baseline and rises above it: y runs negative.
    assert!(defs.contains(" -"), "{defs}");
}
```

Register it in `src/svg/serialise/tests/mod.rs` with `mod outline;`. If `Group::new` or `Semantics::Indicator` take other arguments than shown, match what `src/svg/serialise/tests/semantics.rs` already builds.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --lib serialise::tests::outline 2>&1 | tail -5`
Expected: compile error: no `outline` module in `serialise`.

- [ ] **Step 3: Implement the table**

`src/svg/serialise/outline.rs`:

```rust
//! Text as glyph outlines, for the exported SVG (architecture §11, D-88).
//!
//! Each glyph a static document draws is defined once per face and size
//! in `<defs>`, already at its size, and placed by `<use>`: the spike's
//! measured form (`docs/spikes/svg-font.md`). Ids are safe here, though
//! §5 bans them in the page: this file holds one arrangement, not two.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use skrifa::outline::OutlinePen;

use crate::num::fmt_coord;
use crate::style::TextStyle;
use crate::svg::{Document, Node};
use crate::text::{FontError, Fonts, Weight};

/// A glyph in one face at one size: one definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    weight: Weight,
    /// The size's bits: sizes are positive and finite, so their bits
    /// order as they do.
    size: u64,
    glyph: u32,
}

impl Key {
    fn new(style: TextStyle, glyph: u32) -> Self {
        Self { weight: style.weight, size: style.size.to_bits(), glyph }
    }
}

/// Every inked glyph a document draws, numbered in key order.
pub(super) struct Glyphs {
    ids: BTreeMap<Key, usize>,
    paths: Vec<String>,
}

impl Glyphs {
    /// The glyphs `document`'s static rendering draws: every text, line and
    /// tail outside an interactive-only node.
    ///
    /// # Errors
    ///
    /// [`FontError`] when a shaped glyph cannot be outlined (Ruling 6).
    pub(super) fn of(document: &Document, fonts: &Fonts) -> Result<Self, FontError> {
        let mut keys = BTreeMap::new();
        collect(&document.root, fonts, &mut keys);
        let mut ids = BTreeMap::new();
        let mut paths = Vec::new();
        for (key, size) in keys {
            let face = fonts.face(key.weight);
            let mut pen = Pen { d: String::new(), scale: size / face.units_per_em() };
            face.outline(key.glyph, &mut pen)?;
            if !pen.d.is_empty() {
                ids.insert(key, paths.len());
                paths.push(pen.d);
            }
        }
        Ok(Self { ids, paths })
    }

    /// The definition to place for `glyph` in `style`, or `None` when it
    /// draws nothing (a space).
    pub(super) fn id(&self, style: TextStyle, glyph: u32) -> Option<usize> {
        self.ids.get(&Key::new(style, glyph)).copied()
    }

    /// `<defs>` with one path per glyph, or nothing when none inks.
    pub(super) fn defs(&self) -> String {
        if self.paths.is_empty() {
            return String::new();
        }
        let mut out = String::from("<defs>\n");
        for (id, d) in self.paths.iter().enumerate() {
            let _ = writeln!(out, "<path id=\"g{id}\" d=\"{d}\"/>");
        }
        out.push_str("</defs>\n");
        out
    }
}

/// Each glyph `node` draws, keyed, with the size it is drawn at.
fn collect(node: &Node, fonts: &Fonts, keys: &mut BTreeMap<Key, f64>) {
    if node.is_interactive_only() {
        return;
    }
    match node {
        Node::Group(group) => {
            for child in &group.children {
                collect(child, fonts, keys);
            }
        }
        Node::Text { style, lines, tail, .. } => {
            let runs = lines
                .iter()
                .map(|line| (*style, line.as_str()))
                .chain(tail.iter().map(|t| (t.style, t.text.as_str())));
            for (style, text) in runs {
                for glyph in fonts.face(style.weight).shape(text) {
                    keys.insert(Key::new(style, glyph.id), style.size);
                }
            }
        }
        _ => {}
    }
}

/// A glyph's contours as a path's `d`: font units scaled to the size and
/// flipped into SVG's y-down space, every number through `fmt_coord`.
struct Pen {
    d: String,
    scale: f64,
}

impl Pen {
    fn push(&mut self, command: char, points: &[(f32, f32)]) {
        self.d.push(command);
        let coords: Vec<String> = points
            .iter()
            .flat_map(|&(x, y)| {
                [fmt_coord(f64::from(x) * self.scale), fmt_coord(-f64::from(y) * self.scale)]
            })
            .collect();
        self.d.push_str(&coords.join(" "));
    }
}

impl OutlinePen for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.push('M', &[(x, y)]);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.push('L', &[(x, y)]);
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.push('Q', &[(cx, cy), (x, y)]);
    }
    fn curve_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        self.push('C', &[(c1x, c1y), (c2x, c2y), (x, y)]);
    }
    fn close(&mut self) {
        self.d.push('Z');
    }
}
```

`let _ = writeln!(…)` writes to a `String`, which cannot fail. If clippy's `let_underscore_must_use` or a project lint rejects it, use `.expect("a String accepts writes")`, as `tests/outlines_match_text.rs` does.

`fmt_coord` must write `-0` as `0`. Its integer-thousandths form already gives `0` for `-0.0`, since `(-0.0 * 1000.0).round() as i64` is `0`. `a_definition_is_a_bare_path` would show a stray `-0` on Inter's `l`, whose stem starts on the baseline.

Add `mod outline;` to `src/svg/serialise/mod.rs` beside the other modules.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --lib serialise::tests::outline`
Expected: 6 pass. The table is dead code outside the tests until Task 3, so clippy's `dead_code` may warn on it. Task 3 consumes it in the same branch, so do not suppress the warning: commit Tasks 2 and 3 together if it does warn (Step 5).

- [ ] **Step 5: Commit (or carry into Task 3)**

```bash
git add src/svg/serialise/outline.rs src/svg/serialise/mod.rs src/svg/serialise/tests/
git commit -m "feat(svg): a glyph table for outlined text, one path per face and size"
```

---

### Task 3: Outlined text, and the standalone document

**Files:**
- Modify: `src/svg/serialise/mod.rs` (`TextMode` in `Context`; `node()`'s text arm; `serialise` split into opening and body; `pub struct Standalone`, `pub fn standalone`)
- Modify: `src/svg/serialise/outline.rs` (the writer)
- Modify: `src/svg/serialise/text.rs` (`ink` becomes `pub(super)`)
- Modify: `src/svg/mod.rs` (re-export `Standalone`, `standalone`; drop the `qual:api` note on `tail_x`, which now has its production caller)
- Test: `src/svg/serialise/tests/outline.rs`

**Interfaces:**
- Consumes (Task 1): `FontMetrics::place`; `TextStyle::advance`; `Node::tail_x`. (Task 2): `Glyphs`.
- Produces:

```rust
/// The exported SVG's own parts around the shared document (D-88).
pub struct Standalone<'a> {
    pub palette: &'a Palette,
    pub fonts: &'a Fonts,
    /// The root's `width` and `height` (FR-57).
    pub width: f64,
    pub height: f64,
    /// The root's `<title>`.
    pub title: &'a str,
    /// The root's `<desc>`, when there is one.
    pub desc: Option<&'a str>,
    /// What `<!-- … -->` names before the root (SC-25).
    pub generator: &'a str,
}

/// `document` as a standalone SVG file, its text outlined (D-88).
/// # Errors
/// [`FontError`] when a glyph cannot be outlined.
pub fn standalone(document: &Document, file: &Standalone<'_>) -> Result<String, FontError>
```

- `serialise(document, target)` keeps its signature and its bytes.

- [ ] **Step 1: Write the failing tests**

Append to `src/svg/serialise/tests/outline.rs`:

```rust
use crate::palette::{Palette, Theme};
use crate::svg::{Standalone, standalone, Tail};

fn file(document: &Document) -> String {
    let palette = Palette::built_in(Theme::Light);
    standalone(
        document,
        &Standalone {
            palette: &palette,
            fonts: fonts(),
            width: 400.0,
            height: 200.0,
            title: "Radar <&>",
            desc: Some("Technology radar: 1 quadrant"),
            generator: "radargen 9.9",
        },
    )
    .expect("outlines")
}

/// SC-25 and D-88: the comment first, then the root and its names, then the
/// glyphs, then the ground, then the drawing.
#[test]
fn a_standalone_file_opens_with_its_names() {
    let svg = file(&doc(vec![text(NAME, &["a"])]));
    assert!(svg.starts_with(
        "<!-- radargen 9.9 -->\n<svg xmlns=\"http://www.w3.org/2000/svg\" \
         xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"400\" height=\"200\" \
         viewBox=\"0 0 200 100\" xml:lang=\"en\">\n\
         <title>Radar &lt;&amp;&gt;</title>\n\
         <desc>Technology radar: 1 quadrant</desc>\n<defs>\n"
    ), "{svg}");
    let ground = svg.find("<rect").expect("a ground");
    assert!(svg.find("</defs>").expect("defs") < ground, "{svg}");
    assert!(svg.ends_with("</svg>\n"), "one LF at the end");
}

/// SC-15: no text, and nothing that names a font.
#[test]
fn outlined_text_writes_no_text() {
    let svg = file(&doc(vec![text(NAME, &["Kafka", "Streams"])]));
    for banned in ["<text", "<tspan", "font-", "@font-face", "<style"] {
        assert!(!svg.contains(banned), "{banned} in {svg}");
    }
}

/// D-88: a text is an image named by its words; one `<use>` per inked glyph.
#[test]
fn a_text_is_an_image_named_by_its_words() {
    let svg = file(&doc(vec![text(NAME, &["Kafka", "Streams"])]));
    assert!(svg.contains("<g fill=\"#"), "{svg}");
    assert!(svg.contains(" role=\"img\" aria-label=\"Kafka Streams\">"), "{svg}");
    assert_eq!(svg.matches("<use xlink:href=\"#g").count(), "KafkaStreams".len());
}

/// Each line at its own baseline (FR-46); a start-anchored line at the
/// origin.
#[test]
fn lines_sit_at_their_baselines() {
    let svg = file(&doc(vec![text(NAME, &["l", "l"])]));
    assert!(svg.contains("<use xlink:href=\"#g0\" x=\"10\" y=\"20\"/>"), "{svg}");
    assert!(svg.contains("<use xlink:href=\"#g0\" x=\"10\" y=\"35\"/>"), "{svg}");
}

/// A middle-anchored line is centred on its origin by its measured advance.
#[test]
fn a_middle_anchored_line_is_centred() {
    let Node::Text { origin, style, ink, lines, leading, tail, .. } = text(NAME, &["ll"]) else {
        unreachable!("a text")
    };
    let centred = Node::Text { origin, style, ink, anchor: Anchor::Middle, lines, leading, tail };
    let svg = file(&doc(vec![centred]));
    let advance = NAME.advance("ll", fonts());
    let x = crate::num::fmt_coord(10.0 - advance / 2.0);
    assert!(svg.contains(&format!("x=\"{x}\" y=\"20\"/>")), "{x} in {svg}");
}

/// A halo is an outline only, hidden, and not named again (architecture §5).
#[test]
fn a_halo_is_hidden_and_unnamed() {
    let Node::Text { origin, style, anchor, lines, leading, tail, .. } = text(NAME, &["a"]) else {
        unreachable!("a text")
    };
    let halo = Node::Text {
        origin, style, anchor, lines, leading, tail,
        ink: TextPaint::Halo { stroke: Paint::Role(Role::Neutral(Neutral::Surface1)), width: 3.0 },
    };
    let svg = file(&doc(vec![halo]));
    assert!(
        svg.contains("<g fill=\"none\" stroke=\"#") && svg.contains("stroke-width=\"3\" stroke-linejoin=\"round\" aria-hidden=\"true\">"),
        "{svg}"
    );
    assert!(!svg.contains("role=\"img\""), "{svg}");
}

/// D-78: the tail is placed at `tail_x`, in its own paint and size, on the
/// last line, and read as part of the text's name.
#[test]
fn a_tail_sits_at_tail_x() {
    let tail = Tail {
        gap: 4.0,
        style: TextStyle { size: 10.5, ..NAME },
        ink: Paint::Role(Role::Neutral(Neutral::Muted)),
        text: "\u{a0}New".to_owned(),
    };
    let Node::Text { origin, style, ink, anchor, lines, leading, .. } = text(NAME, &["a", "Kafka"]) else {
        unreachable!("a text")
    };
    let node = Node::Text { origin, style, ink, anchor, lines, leading, tail: Some(tail) };
    let x = node.tail_x(fonts()).expect("a tail");
    let svg = file(&doc(vec![node]));
    assert!(svg.contains("aria-label=\"a Kafka\u{a0}New\""), "{svg}");
    let at = svg.find("<g fill=\"#").and_then(|first| svg.get(first + 1..)?.find("<g fill=\"#")).expect("the tail's group");
    let tail_group = svg.get(at..).expect("in range");
    // The tail's first inked glyph, `N`, at `tail_x` past the NBSP's advance.
    let nbsp = fonts().face(NAME.weight).place("\u{a0}N", 10.5, 0.0);
    let n_x = crate::num::fmt_coord(x + nbsp.get(1).expect("N").x);
    assert!(tail_group.contains(&format!("x=\"{n_x}\" y=\"35\"/>")), "{n_x} in {svg}");
}

/// A control's own name covers its text, whose image would read it twice.
#[test]
fn text_in_a_control_is_not_named_again() {
    use crate::svg::{Control, ItemKey, Semantics};
    let control = Control { key: ItemKey(2), label: "1. Kafka".into(), tooltip: None };
    let marker = Node::Group(
        Group::new(Point::ORIGIN, vec![text(NAME, &["1"])]).with(Semantics::Marker(control)),
    );
    let svg = file(&doc(vec![marker]));
    assert_eq!(svg.matches("role=\"img\"").count(), 1, "the control's alone: {svg}");
    assert!(svg.contains("aria-label=\"1. Kafka\""), "{svg}");
}

/// A heading's name is computed from its content: its image's name.
#[test]
fn a_heading_is_named_by_its_text() {
    use crate::svg::Semantics;
    let heading = Node::Group(
        Group::new(Point::ORIGIN, vec![text(HEAD, &["Platforms"])])
            .with(Semantics::Heading(std::num::NonZeroU8::MIN)),
    );
    let svg = file(&doc(vec![heading]));
    assert!(svg.contains("<g role=\"heading\" aria-level=\"1\">\n<g fill="), "{svg}");
    assert!(svg.contains("aria-label=\"Platforms\""), "{svg}");
}

/// The page and the PNG keep `<text>`: the mode is the export's alone.
#[test]
fn serialise_still_writes_text() {
    let palette = Palette::built_in(Theme::Light);
    let svg = crate::svg::serialise(&doc(vec![text(NAME, &["a"])]), crate::svg::Target::Static(&palette));
    assert!(svg.contains("<text") && !svg.contains("<use"), "{svg}");
}
```

Match `Control`'s real fields and constructor to `src/svg/serialise/tests/semantics.rs`. The muted and ink roles must be ones `Neutral` has; use whichever two distinct neutrals the existing tests use.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --lib serialise::tests::outline 2>&1 | tail -5`
Expected: compile error: `Standalone` and `standalone` are not found.

- [ ] **Step 3: The text mode in `Context`**

In `src/svg/serialise/mod.rs`:

```rust
/// How text is written: as `<text>` for a reader with the font (the page,
/// and resvg through our own font database), or as outlines for one
/// without it (the exported SVG, D-88).
#[derive(Clone, Copy)]
enum TextMode<'a> {
    Text,
    Outlined(outline::Outlined<'a>),
}
```

`Context` gains `text: TextMode<'a>`. `Context::top(target)` sets `TextMode::Text`. `node()`'s `Node::Text` arm builds the same `text::Run` as today, then:

```rust
match context.text {
    TextMode::Text => text::text(&run, context),
    TextMode::Outlined(outlined) => {
        outline::text(&run, node.tail_x(outlined.fonts), outlined, context)
    }
}
```

`node` here is the `&Node` parameter. Rename the function's parameter if it shadows the function name awkwardly.

Make `text::ink` `pub(super)`.

- [ ] **Step 4: The writer**

Add to `src/svg/serialise/outline.rs`:

```rust
use super::text::{Run, ink};
use super::{Context, attr, coord, paint};
use crate::svg::{Anchor, Point, TextPaint};

/// What outlined text reads: the faces to place with, and the table.
#[derive(Clone, Copy)]
pub(super) struct Outlined<'a> {
    pub(super) fonts: &'a Fonts,
    pub(super) glyphs: &'a Glyphs,
}

/// A text as a group of placed glyphs: its paint on the group, each line
/// at its baseline, its tail at `tail_x` (D-78). Outside a control and
/// unless a halo, an image named by its words (D-88).
#[expect(
    clippy::suboptimal_flops,
    reason = "NFR-4: plain * and + are bit-identical on every target; mul_add falls back to a libm fma where the CPU lacks one"
)]
pub(super) fn text(
    run: &Run<'_>,
    tail_x: Option<f64>,
    outlined: Outlined<'_>,
    context: Context<'_>,
) -> String {
    if run.lines.is_empty() {
        return String::new();
    }
    let mut out = String::from("<g");
    ink(&mut out, run.ink, context);
    if matches!(run.ink, TextPaint::Fill(_)) && context.control.is_none() {
        attr(&mut out, "role", "img");
        attr(&mut out, "aria-label", &words(run));
    }
    out.push_str(">\n");
    let mut baseline = run.origin.y;
    for (i, line) in (0_u32..).zip(run.lines) {
        baseline = run.origin.y + f64::from(i) * run.leading;
        let x = match run.anchor {
            Anchor::Start => run.origin.x,
            Anchor::Middle => run.origin.x - run.style.advance(line, outlined.fonts) / 2.0,
        };
        out.push_str(&uses(line, run.style, Point { x, y: baseline }, outlined));
    }
    if let (Some(tail), Some(x)) = (run.tail, tail_x) {
        out.push_str("<g");
        attr(&mut out, "fill", &paint(tail.ink, context.target));
        out.push_str(">\n");
        out.push_str(&uses(&tail.text, tail.style, Point { x, y: baseline }, outlined));
        out.push_str("</g>\n");
    }
    out.push_str("</g>\n");
    out
}

/// The text's name: its lines as one, then its tail.
fn words(run: &Run<'_>) -> String {
    let mut name = run.lines.join(" ");
    if let Some(tail) = run.tail {
        name.push_str(&tail.text);
    }
    name
}

/// One `<use>` per inked glyph of `text`, from `at`.
fn uses(text: &str, style: TextStyle, at: Point, outlined: Outlined<'_>) -> String {
    outlined
        .fonts
        .face(style.weight)
        .place(text, style.size, style.letter_spacing)
        .into_iter()
        .filter_map(|placed| Some((outlined.glyphs.id(style, placed.glyph)?, placed)))
        .map(|(id, placed)| {
            let mut out = String::from("<use");
            attr(&mut out, "xlink:href", &format!("#g{id}"));
            coord(&mut out, "x", at.x + placed.x);
            coord(&mut out, "y", at.y + placed.y);
            out.push_str("/>\n");
            out
        })
        .collect()
}
```

`ink` writes `fill`, or the halo's `fill="none" stroke stroke-width stroke-linejoin aria-hidden`. With no transform on the glyphs, the halo's width is in user units, as in `<text>`.

- [ ] **Step 5: The standalone document**

In `src/svg/serialise/mod.rs`, split `serialise` so its ground and body are shared:

```rust
pub fn serialise(document: &Document, target: Target<'_>) -> String {
    let mut out = String::from("<svg");
    attr(&mut out, "xmlns", SVG_NS);
    attr(&mut out, "viewBox", &view_box(document));
    out.push_str(">\n");
    out.push_str(&body(document, Context::top(target)));
    out.push_str("</svg>");
    out
}

/// The ground, for a static target (D-87), then the drawing.
fn body(document: &Document, context: Context<'_>) -> String {
    // …the existing ground rect, when `!context.target.interactive()`,
    // then `node(&document.root, context)`.
}

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

pub fn standalone(document: &Document, file: &Standalone<'_>) -> Result<String, FontError> {
    let glyphs = outline::Glyphs::of(document, file.fonts)?;
    let context = Context {
        target: Target::Static(file.palette),
        control: None,
        text: TextMode::Outlined(outline::Outlined { fonts: file.fonts, glyphs: &glyphs }),
    };
    let mut out = ["<!-- ", file.generator, " -->\n<svg"].concat();
    attr(&mut out, "xmlns", SVG_NS);
    attr(&mut out, "xmlns:xlink", XLINK_NS);
    coord(&mut out, "width", file.width);
    coord(&mut out, "height", file.height);
    attr(&mut out, "viewBox", &view_box(document));
    attr(&mut out, "xml:lang", "en");
    out.push_str(">\n<title>");
    out.push_str(&xml_text(file.title));
    out.push_str("</title>\n");
    if let Some(desc) = file.desc {
        out.push_str("<desc>");
        out.push_str(&xml_text(desc));
        out.push_str("</desc>\n");
    }
    out.push_str(&glyphs.defs());
    out.push_str(&body(document, context));
    out.push_str("</svg>\n");
    Ok(out)
}
```

`xml:lang="en"` matches the page's `lang="en"`: the file's own words (*Technology Radar*, the radar's description) are English. Re-export `Standalone` and `standalone` from `src/svg/mod.rs` beside `Target` and `serialise`.

- [ ] **Step 6: Run the tests to verify they pass, and that the old bytes did not move**

Run: `cargo test --lib serialise && cargo test --test serialise`
Expected: all pass, the serialiser and HTML goldens included, so `serialise` writes what it wrote before.

- [ ] **Step 7: Commit**

```bash
git add src/svg/
git commit -m "feat(svg): outlined text and the standalone document (D-88)"
```

---

### Task 4: `-o radar.svg`

**Files:**
- Create: `src/emit/svg.rs`
- Modify: `src/emit/mod.rs` (`pub mod svg;`)
- Modify: `src/cli.rs` (`OutputFormat::Svg`, `format()`, the about line and the help text)
- Modify: `src/diagnostics.rs` (remove `FormatNotInThisVersion`; the `UnrecognisedExtension` message)
- Modify: `src/pipeline.rs` (the `Svg` arm)
- Create: `tests/svg_matches_png.rs`
- Create: `tests/serialise/export.rs`, registered in `tests/serialise/main.rs`
- Modify: `tests/run.rs`, `tests/cli.rs`, `tests/diagnostics.rs`

**Interfaces:**
- Consumes (Task 3): `svg::{Standalone, standalone}`. (PR 16): `emit::png::{pixel_size, rasterise, usvg_options}`.
- Produces:
  - `emit::svg::intrinsic_size(document: &Document, requested: Option<u32>) -> (f64, f64)`
  - `emit::svg::export(document: &Document, export: &Export<'_>) -> Result<String, Fatal>`, where `pub struct Export<'a> { pub palette: &'a Palette, pub fonts: &'a Fonts, pub width: Option<u32>, pub title: Option<&'a str> }`

- [ ] **Step 1: Write the failing unit tests in `src/emit/svg.rs`**

Create `src/emit/svg.rs` with only its tests module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::arrange::{Arrangement, Blocks, arrange};
    use crate::layout::block::fonts;
    use crate::layout::test_support::{PRD_EXAMPLE, plotted};
    use crate::palette::Theme;

    fn prd() -> Document {
        let (catalogue, order) = plotted(PRD_EXAMPLE, None);
        let blocks = Blocks::new(&catalogue, &order, Some("Radar"), None, fonts());
        arrange(Arrangement::Wide, &blocks)
    }

    /// FR-57, D-88: the height follows the width, unrounded.
    #[test]
    fn the_intrinsic_size_follows_the_width() {
        let doc = prd();
        assert_eq!(intrinsic_size(&doc, None), (doc.width, doc.height));
        let (w, h) = intrinsic_size(&doc, Some(320));
        assert_eq!(w, 320.0);
        assert_eq!(h, 320.0 * doc.height / doc.width);
    }

    /// D-88: no clamp; an SVG allocates nothing.
    #[test]
    fn a_wide_default_is_not_clamped() {
        let doc = Document { width: 12_500.0, ..prd() };
        assert_eq!(intrinsic_size(&doc, None).0, 12_500.0);
    }

    /// D-75 and D-88: the title, or Technology Radar; the radar's name.
    #[test]
    fn the_file_is_named() {
        let palette = Palette::built_in(Theme::Light);
        let export = |title| Export { palette: &palette, fonts: fonts(), width: None, title };
        let titled = super::export(&prd(), &export(Some("Q3 & co"))).expect("exports");
        assert!(titled.contains("<title>Q3 &amp; co</title>"), "{titled}");
        let untitled = super::export(&prd(), &export(None)).expect("exports");
        assert!(untitled.contains("<title>Technology Radar</title>"));
        assert!(untitled.contains("<desc>Technology radar: "), "{untitled}");
        assert!(untitled.starts_with(&format!("<!-- {} -->\n", crate::emit::OUTPUT_VERSION)));
    }
}
```

Use `Blocks::new`'s real signature from `src/emit/png.rs`'s tests, which build the same document.

- [ ] **Step 2: Write the failing integration tests**

`tests/svg_matches_png.rs`:

```rust
//! SC-7 and SC-15 for the exported SVG (D-88): drawn by resvg with no
//! font at all, it draws what our own PNG draws, to within the
//! antialiasing the spike measured (`docs/spikes/svg-font.md`).

use std::sync::Arc;

use radargen::emit::png::{pixel_size, rasterise, usvg_options};
use radargen::emit::svg::{Export, export};
use radargen::layout::arrange::{Arrangement, Blocks, arrange};
use radargen::order::resolve;
use radargen::palette::{Palette, Theme};
use radargen::text::Fonts;
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

#[path = "common/catalogue.rs"]
mod catalogue;

const FIXTURES: [(&str, &str); 4] = [
    ("three-quadrant", include_str!("fixtures/three-quadrant.csv")),
    ("long-legend", include_str!("fixtures/long-legend.csv")),
    ("text-spacing", include_str!("fixtures/text-spacing.csv")),
    ("grown", include_str!("fixtures/grown.csv")),
];

/// Our options with an empty font database: any text the SVG still
/// carried would draw as nothing, and the comparison would fail.
fn fontless() -> usvg::Options<'static> {
    usvg::Options { fontdb: Arc::new(usvg::fontdb::Database::new()), ..usvg_options() }
}

/// Differing subpixels, against the whole.
fn differing(a: &Pixmap, b: &Pixmap) -> (usize, usize) {
    let differ = a.data().iter().zip(b.data()).filter(|(x, y)| x != y).count();
    (differ, a.data().len())
}

fn compare(csv: &str, title: Option<&str>, theme: Theme) -> (usize, usize) {
    let fonts = Fonts::embedded().expect("loads");
    let catalogue = catalogue::catalogue(csv, None);
    let plotted = resolve(&catalogue);
    let blocks = Blocks::new(&catalogue, &plotted, title, None, &fonts);
    let doc = arrange(Arrangement::Wide, &blocks);
    let palette = Palette::built_in(theme);
    let size = pixel_size(&doc, None).expect("fits");
    let png = Pixmap::decode_png(&rasterise(&doc, &palette, size).expect("renders")).expect("a PNG");
    let svg = export(&doc, &Export { palette: &palette, fonts: &fonts, width: None, title })
        .expect("exports");
    let tree = usvg::Tree::from_str(&svg, &fontless()).expect("usvg parses the export");
    let mut drawn = Pixmap::new(size.width(), size.height()).expect("a pixmap");
    resvg::render(
        &tree,
        Transform::from_scale(
            size.width() as f32 / tree.size().width(),
            size.height() as f32 / tree.size().height(),
        ),
        &mut drawn.as_mut(),
    );
    differing(&png, &drawn)
}

/// SC-7 for SVG: within the spike's 100 ppm, every fixture, both themes.
#[test]
fn the_svg_draws_what_the_png_draws() {
    for (name, csv) in FIXTURES {
        for theme in [Theme::Light, Theme::Dark] {
            let (differ, total) = compare(csv, Some("Engineering Radar"), theme);
            eprintln!("{name} {theme:?}: {differ} of {total} subpixels differ");
            assert!(differ * 10_000 < total, "{name} {theme:?}: {differ} of {total}");
        }
    }
}

/// NFR-7: a name Inter lacks draws the shaper's `.notdef`, as resvg does.
#[test]
fn a_name_inter_lacks_draws_as_the_png_does() {
    let csv = "name,quadrant,ring,description\n東京,Tools,Adopt,d\n😀,Tools,Trial,d\nKafka,Tools,Assess,d\n";
    let (differ, total) = compare(csv, None, Theme::Light);
    assert!(differ * 10_000 < total, "{differ} of {total}");
}
```

The `as f32` casts need `#[expect(clippy::cast_precision_loss, clippy::as_conversions, reason = "…")]` on `compare`, or a small `narrow` helper as `src/emit/png.rs` has. If `compare` passes 7 parameters' worth of locals into clippy's `too_many_lines`, split building the document into a helper.

`tests/serialise/export.rs`, with `mod export;` added to `tests/serialise/main.rs`:

```rust
//! The exported SVG's vocabulary and escaping (SC-9, SC-15, NFR-6, D-88).

use radargen::emit::svg::{Export, export};
use radargen::layout::arrange::{Arrangement, arrange};
use radargen::palette::{Palette, Theme};

use super::build;
use super::hostile::{RING, TITLE, csv};

/// The elements the export writes, and nothing else.
const ELEMENTS: [&str; 10] = ["svg", "title", "desc", "defs", "path", "use", "g", "rect", "circle", "line"];

/// SC-9 through an XML parser: the only strings in the file are the
/// title, the description and the names, each read back as written.
#[test]
fn hostile_text_is_only_named_never_markup() {
    let built = build(&csv(), Some(&format!("{RING},Adopt")), Some(TITLE));
    let doc = arrange(Arrangement::Wide, &built.blocks);
    let palette = Palette::built_in(Theme::Dark);
    let fonts = super::fonts::fonts();
    let svg = export(&doc, &Export { palette: &palette, fonts, width: Some(800), title: Some(TITLE) })
        .expect("exports");
    let parsed = roxmltree::Document::parse(&svg).expect("well-formed XML");
    for node in parsed.descendants().filter(roxmltree::Node::is_element) {
        let name = node.tag_name().name();
        assert!(ELEMENTS.contains(&name), "<{name}>");
        for a in node.attributes() {
            assert!(!a.name().starts_with("on"), "<{name} {}>", a.name());
            if a.name() == "href" {
                assert!(a.value().starts_with("#g"), "a local glyph: {}", a.value());
            }
        }
        if !["title", "desc"].contains(&name) {
            let text: String = node.children().filter(roxmltree::Node::is_text).filter_map(|t| t.text()).collect();
            assert!(text.trim().is_empty(), "text in <{name}>: {text:?}");
        }
    }
    let title = parsed.descendants().find(|n| n.has_tag_name("title")).and_then(|n| n.text());
    assert_eq!(title, Some(TITLE));
    // Every drawn name is an image's name, as written.
    let labels: Vec<&str> = parsed.descendants().filter_map(|n| n.attribute("aria-label")).collect();
    for want in super::hostile::Strings::of(&doc.root, false).texts {
        let want = want.replace('\n', " ");
        assert!(labels.iter().any(|l| l.starts_with(&want)), "{want:?} in {labels:?}");
    }
    for banned in ["<text", "font-family", "@font-face", "<style", "<script", "url("] {
        assert!(!svg.contains(banned), "{banned}");
    }
}
```

Fit the `Strings::of` call and `super::fonts::fonts()` to what `tests/serialise/hostile.rs` and `tests/common/fonts.rs` really export. The point is that each drawn text's words are found among the names.

In `tests/run.rs`, rename `bad_extensions_and_svg` to `bad_extensions` and expect "must end in .html, .png or .svg", with no `.svg` case. Then add an SVG section after the PNG one:

```rust
// ---- SVG (FR-54, FR-57, FR-71, D-88)

/// FR-54, FR-57, D-88: a standalone SVG at --width, its height in proportion.
#[test]
fn renders_an_svg_at_the_width_asked() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.SVG", "--width", "1920", "--theme", "dark"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let svg = read(dir.path(), "radar.SVG");
    assert!(svg.starts_with("<!-- radargen 1."), "{}", svg.get(..40).unwrap_or_default());
    assert!(svg.contains(" width=\"1920\" height=\""), "intrinsic size");
    assert!(!svg.contains("<text"), "outlined");
    assert!(svg.ends_with("</svg>\n"));
}

/// D-88: the PNG's clamp and small-text warnings are not the SVG's.
#[test]
fn an_svg_gives_no_png_warnings() {
    let dir = seeded(&[("radar.csv", &too_wide())]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "wide.svg"]);
    assert!(ran.ok(), "{}", ran.stderr);
    assert_eq!(ran.lines(), Vec::<&str>::new(), "no warning at all");
    let svg = read(dir.path(), "wide.svg");
    assert!(!svg.contains("width=\"10000\""), "not clamped");
    let ran = run_in(dir.path(), &["radar.csv", "-o", "narrow.svg", "--width", "320"]);
    assert!(ran.ok() && ran.lines().is_empty(), "{}", ran.stderr);
}

/// SC-18: an SVG into a missing directory fails as any output does.
#[test]
fn an_svg_into_a_missing_directory_fails() {
    let dir = seeded(&[("radar.csv", PRD)]);
    let before = snapshot(dir.path());
    let ran = run_in(dir.path(), &["radar.csv", "-o", "nowhere/radar.svg"]);
    assert!(!ran.ok());
    assert_eq!(ran.lines().len(), 1, "{}", ran.stderr);
    assert_eq!(snapshot(dir.path()), before);
}

/// D-74: an `.svg`-named input is still the input.
#[test]
fn an_svg_output_that_is_the_input_is_refused() {
    let dir = seeded(&[("radar.svg", PRD)]);
    let ran = run_in(dir.path(), &["radar.svg", "-o", "radar.svg"]);
    assert!(!ran.ok());
    assert_eq!(read(dir.path(), "radar.svg"), PRD);
}

/// NFR-7: names in scripts Inter lacks still export.
#[test]
fn an_svg_of_hostile_text_renders() {
    let csv = format!("{HEAD}Кафка,Tools,Adopt,d\n東京,Tools,Trial,d\n\"😀 <b>&amp;\",Tools,Assess,d\n");
    let dir = seeded(&[("radar.csv", &csv)]);
    let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.svg", "--title", "</title>]]>"]);
    assert!(ran.ok(), "{}", ran.stderr);
    let svg = read(dir.path(), "radar.svg");
    assert!(svg.contains("<title>&lt;/title&gt;]]&gt;</title>"), "escaped");
    roxmltree::Document::parse(&svg).expect("well-formed");
}
```

Check `a_png_into_a_missing_directory_fails` for the exact FR-62 line and assert the same line here. If `tests/run.rs` does not yet depend on `roxmltree` (it is a dev-dependency, so it can), import it as the serialise tests do.

In `tests/cli.rs`, rename `png_is_a_format_and_svg_arrives_in_1_2` to `png_and_svg_are_formats` and assert `OutputFormat::Svg` for `e.svg`, `E.SVG` and `e.Svg`. The about-line assertion becomes `about.contains("PNG") && about.contains("SVG")`. `width_help_names_the_range` is unchanged.

In `tests/diagnostics.rs`, delete the `FormatNotInThisVersion` row, and change the `UnrecognisedExtension` row's expected line to `error: -o radar.txt: the output file name must end in .html, .png or .svg` (match the row's own path).

- [ ] **Step 3: Run them to verify they fail**

Run: `cargo test --test svg_matches_png --test run --test cli --test diagnostics 2>&1 | tail -15`
Expected: compile errors: `emit::svg` and `OutputFormat::Svg` do not exist.

- [ ] **Step 4: Implement `src/emit/svg.rs`**

Above the tests module:

```rust
//! The SVG export (FR-54, FR-57, FR-71, D-88): the wide document with
//! literal colours for one theme over its ground, its text as outlines,
//! sized by `--width`.

use crate::diagnostics::Fatal;
use crate::emit::OUTPUT_VERSION;
use crate::emit::html::UNTITLED;
use crate::palette::Palette;
use crate::svg::{Document, Node, Semantics, Standalone, standalone};
use crate::text::Fonts;

/// What one export needs besides the document.
pub struct Export<'a> {
    /// The theme's palette (FR-56).
    pub palette: &'a Palette,
    /// The faces the text is shaped and outlined from.
    pub fonts: &'a Fonts,
    /// `--width` (FR-57).
    pub width: Option<u32>,
    /// FR-65's title, resolved.
    pub title: Option<&'a str>,
}

/// FR-57: `requested` wide and the height in proportion, or the
/// document's own size. Unrounded and unclamped (D-88).
#[must_use]
pub fn intrinsic_size(document: &Document, requested: Option<u32>) -> (f64, f64) {
    requested.map_or((document.width, document.height), |width| {
        let width = f64::from(width);
        (width, width * document.height / document.width)
    })
}

/// FR-54: `document` as a standalone SVG file (D-88).
///
/// # Errors
///
/// [`Fatal::Font`] when a glyph cannot be outlined (Ruling 6).
pub fn export(document: &Document, export: &Export<'_>) -> Result<String, Fatal> {
    let (width, height) = intrinsic_size(document, export.width);
    Ok(standalone(
        document,
        &Standalone {
            palette: export.palette,
            fonts: export.fonts,
            width,
            height,
            title: export.title.unwrap_or(UNTITLED),
            desc: radar_name(&document.root),
            generator: OUTPUT_VERSION,
        },
    )?)
}

/// The radar's group name (D-75), the first named group in the document.
fn radar_name(node: &Node) -> Option<&str> {
    match node {
        Node::Group(group) => match &group.semantics {
            Semantics::Group(name) => Some(name),
            _ => group.children.iter().find_map(radar_name),
        },
        _ => None,
    }
}
```

`Fatal::Font` is whichever variant `From<FontError>` builds today (the pipeline's `Fonts::embedded().map_err(Fatal::from)`). Name it as it is.

Add `pub mod svg;` to `src/emit/mod.rs`.

- [ ] **Step 5: CLI, diagnostics and pipeline**

- **`src/cli.rs`:**
  - `OutputFormat` gains `/// FR-54: the standalone SVG. Svg,`.
  - `format()` maps `"svg" => Ok(OutputFormat::Svg)`. Its doc's `# Errors` names only `UnrecognisedExtension`, and its `§14.1` comment goes.
  - The about line reads: `"Render a technology radar from a CSV file to a self-contained HTML page, a PNG image or an SVG file."`
  - `-o`'s help: `The extension picks the format: .html, .png or .svg, in any case.`
  - `--theme`'s first line: `The theme for PNG and SVG.`
  - `--width`'s first line: `The width in pixels for PNG and SVG.`
  - Check that every `-h` line stays within 120 columns (PR 16 hit this). `tests/cli.rs` checks it.
- **`src/diagnostics.rs`:** delete `FormatNotInThisVersion` and its doc. `UnrecognisedExtension`'s message becomes `"-o {path}: the output file name must end in .html, .png or .svg"`. Remove any `Fatal::render` or ordering arm that names the deleted variant.
- **`src/pipeline.rs`:** in `render`, add the arm below. Its doc's last sentence becomes "…or the wide arrangement as a PNG or an SVG (FR-71)."

```rust
OutputFormat::Svg => Ok(svg::export(
    &arrange(Arrangement::Wide, &blocks),
    &svg::Export {
        palette: &config.palette,
        fonts,
        width: config.width,
        title: config.title.as_deref(),
    },
)?
.into_bytes()),
```

Import `crate::emit::svg`. Also update `Config::palette`'s doc ("FR-56: the static outputs' colours"; unchanged) and `Config::width`'s ("FR-57: `--width`, which HTML ignores"; unchanged).

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test --lib emit::svg && cargo test --test svg_matches_png --test run --test cli --test diagnostics && cargo test --test serialise export`
Expected: all pass. The comparison prints each fixture's differing subpixels.
- If any fixture is over 100 ppm, that is a placement bug, not a threshold to raise. Use superpowers:systematic-debugging: render both, write the difference image, and find the glyphs that moved.
- Ledger each fixture's count, for Task 5's record.

- [ ] **Step 7: Commit**

```bash
git add src/emit/ src/cli.rs src/diagnostics.rs src/pipeline.rs tests/
git commit -m "feat(svg): -o radar.svg writes the standalone export (FR-54)"
```

---

### Task 5: Goldens, determinism and size

**Files:**
- Modify: `tests/serialise/golden.rs` (two SVG goldens)
- Create: `tests/golden/svg/prd-light.svg`, `tests/golden/svg/prd-dark.svg` (blessed)
- Modify: `tests/determinism.rs` (two processes, one SVG)

- [ ] **Step 1: Write the failing tests**

In `tests/serialise/golden.rs`:

```rust
/// The PRD example, wide, exported in `theme` at its own width.
fn prd_svg(theme: Theme) -> String {
    let doc = arrange(Arrangement::Wide, &prd().blocks);
    let palette = Palette::built_in(theme);
    radargen::emit::svg::export(
        &doc,
        &radargen::emit::svg::Export { palette: &palette, fonts: super::fonts::fonts(), width: None, title: None },
    )
    .expect("exports")
}

/// FR-54, NFR-4: the PRD example as a light SVG, byte for byte, on every
/// platform. One test per golden, since a blessing run stops at the first.
#[test]
fn prd_example_matches_its_light_svg_golden() {
    golden("svg", "prd-light.svg", &prd_svg(Theme::Light));
}

/// FR-54, FR-56: the same, dark.
#[test]
fn prd_example_matches_its_dark_svg_golden() {
    golden("svg", "prd-dark.svg", &prd_svg(Theme::Dark));
}
```

In `tests/determinism.rs`:

```rust
/// NFR-4, SC-8 for SVG: two processes write the same bytes, in both
/// themes and at a width, over the hostile case too.
#[test]
fn two_processes_write_the_same_svg() {
    let dir = tempfile::tempdir().expect("tempdir");
    for (csv, args) in [
        (PRD, &["--theme", "dark", "--width", "1200"][..]),
        (HOSTILE, &["--title", "</title><svg onload=x>"][..]),
    ] {
        let first = render(dir.path(), csv, "first.svg", args);
        let second = render(dir.path(), csv, "second.svg", args);
        assert!(first == second, "NFR-4: two processes, one SVG");
    }
}
```

- [ ] **Step 2: Run them to verify they fail, then bless**

Run: `cargo test --test serialise golden::prd_example_matches_its_light_svg_golden`
Expected: FAIL. The golden file is missing.

Run: `UPDATE_GOLDEN=1 cargo test --test serialise golden::prd_example_matches_its_light_svg_golden; UPDATE_GOLDEN=1 cargo test --test serialise golden::prd_example_matches_its_dark_svg_golden`
Expected: each fails with "blessed …".

Run: `cargo test --test serialise golden && cargo test --test determinism`
Expected: all pass.

Open both goldens in a browser, and in resvg (`resvg tests/golden/svg/prd-dark.svg /tmp/dark.png` if the CLI is installed, or through `svg_matches_png`'s output). Check that they look like the PNG goldens. Record each golden's size in the ledger; the PR body quotes it.

- [ ] **Step 3: Commit**

```bash
git add tests/serialise/golden.rs tests/golden/svg tests/determinism.rs
git commit -m "test(svg): goldens and two-process determinism for the export"
```

---

### Task 6: Docs, version, release

**Files:**
- Modify: `README.md` (the intro line, an SVG section after PNG's, Status)
- Modify: `docs/a11y/manual-checks.md` (step 16, a results row, revision 1.4)
- Modify: `docs/design/architecture.md` (§9 step 7 marks SVG done)
- Modify: `Cargo.toml` (`version = "1.2.0"`, the description)
- Modify: `tests/golden/html/*` (re-blessed for `radargen 1.2`)
- Modify: `CHANGELOG.md` (regenerated; it carries the 1.1.0 reconciliation left uncommitted on `main`)
- Move: `docs/plans/pr-17-svg.md` to `docs/plans/done/` (`status: complete`, tasks ticked)

- [ ] **Step 1: README**

- **Intro line:** "…to a self-contained HTML page, a PNG image or an SVG file. One binary, no runtime, no network."
- **SVG section:** "A `.svg` output draws the same wide arrangement as the PNG, as vectors: it stays sharp at any size and opens in PowerPoint, Figma and Illustrator for touch-up. `--theme` picks its colours, and `--width` sets its size, with the height following. The text is drawn as shapes, so the file needs no font and looks the same everywhere, but its words cannot be selected or searched. A screen reader reads its title and each label."
- **Status:** "1.2 writes HTML, PNG and SVG."

- [ ] **Step 2: Manual check 16**

```markdown
16. **The SVG** (D-88, from 1.2.0). Render `three-quadrant.csv` to `radar.svg` with `--theme dark`, and open it in a browser, in PowerPoint (Insert ▸ Pictures), and in Figma or Illustrator if available.
    - *Expect, everywhere:* it looks like the dark PNG, in Inter, with no substituted font and no missing text.
    - *Expect, in PowerPoint and the editors:* it can be ungrouped or edited, and each glyph is a shape.
    - *Expect, in the browser with NVDA or VoiceOver:* the image's name is the title, the headings list holds the title, Key, Legend and each quadrant, and moving through the image reads each label and each marker's name.
```

Add the results row "The SVG: browser + AT, PowerPoint, Figma or Illustrator", and the revision row `| 1.4 | 2026-10-16 | Step 16: the exported SVG (D-88, rg-m2l). |`. Bump the frontmatter revision to match.

- [ ] **Step 3: Version, description and goldens**

`Cargo.toml`: `version = "1.2.0"`. The description reads "…to a self-contained HTML page, a PNG image or an SVG file."

Run: `cargo build && UPDATE_GOLDEN=1 cargo test --test serialise golden::prd_example_matches_its_interactive_golden`, then bless any other HTML golden that fails on the generator line alone. Run `git diff --stat tests/golden` and check that each HTML diff is that one line. The SVG goldens change too, since their comment names the version; re-bless them the same way and check that each diff is the first line alone.

- [ ] **Step 4: Full check**

Run: `just ci 2>&1 | tail -30`
Expected: green, except the known browser check "a long legend scrolls beside the radar". It fails identically on `main` on this machine, from macOS overlay scrollbars, and has its own crumb. Name it in the PR body.

- [ ] **Step 5: Changelog, crumb, plan**

```bash
/Users/evensolberg/.cargo/bin/crumbs close rg-m2l --reason "PR 17: SVG export (1.2.0)"
git mv docs/plans/pr-17-svg.md docs/plans/done/pr-17-svg.md
```

In the moved plan, set `status: complete` and `completed_date`, and tick every task. Regenerate `CHANGELOG.md` with git-cliff as the project's release recipe does, then commit everything:

```bash
git add -A
git commit -m "chore(release): 1.2.0; close rg-m2l"
```

---

## Self-review

- **Spec coverage:**
  - FR-54: Tasks 3 and 4.
  - FR-56 (`--theme` on SVG): Task 4, and the dark golden in Task 5.
  - FR-57 (the intrinsic size): Task 4.
  - FR-71: Task 4's pipeline arm.
  - NFR-4 and SC-8: Task 5.
  - SC-7: Task 4's pixel comparison.
  - SC-9: Task 4's export test, and Task 5's hostile determinism case.
  - SC-15: Tasks 3 and 4.
  - SC-18: Task 4's missing-directory test.
  - SC-25: Tasks 3 and 4.
  - D-75's fallback title: Task 4.
  - D-78's tail: Task 3.
- **Not covered:** PowerPoint, Figma and Illustrator first-hand. Manual check 16 covers them, and the spike already accepted that this is unconfirmed.
- **Placeholders:** none. Where a signature is copied from existing tests (`Blocks::new`, `Control`, `Strings::of`), the step says which file holds the real one.
- **Types:**
  - `Standalone`'s fields match between Tasks 3 and 4.
  - `Export`'s fields match between Tasks 4 and 5.
  - `Glyphs::id(style, glyph)` matches between Tasks 2 and 3.
