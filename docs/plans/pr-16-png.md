---
tags:
  - plan
  - png
  - output
aliases: []
doc_id: RG-016B
doc_name: pr-16-png
crumb_id: rg-42m
document_title: "PR 16 — PNG export; --theme and --width take effect: Implementation Plan"
created_date: 2026-10-16
synopsis: "-o radar.png writes the wide arrangement (FR-71) as a PNG: the document serialised with literal colours for --theme (FR-56), rasterised by resvg through the vendored Inter faces alone (NFR-7), over the theme's own ground, at --width pixels or the document's own width (FR-57), the height following its aspect. The default width is clamped to FR-64's 10,000 with a warning, and an image past 100 megapixels is refused before it is allocated. HTML still ignores both flags. .svg stays refused until PR 17."
status: draft
type: Plan
revision: "1.0"
review_date:
reviewed_by: []
completed_date:
comments: "First version. The release number (1.1.0) waits on the maintainer: see Rulings."
---
# PR 16 — PNG export; --theme and --width take effect: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-42m` · **Phase:** v1.1 · **After:** 1.0.1 (#89) · **Then:** PR 17, SVG export (`rg-m2l`), then PR 18, `--palette` (`rg-cdo`)

**Goal:** `radargen radar.csv -o radar.png --theme dark --width 1920` writes a PNG of the wide arrangement that matches the HTML's wide arrangement item for item (SC-7) and stands alone on a slide (SC-11).

**Architecture:**

- **`src/emit/png.rs`** (architecture §3's planned file) is thin. It:
  - serialises the wide `Document` with `svg::serialise(doc, Target::Static(&palette))`, which binds every colour to a literal (architecture §5: resvg renders an unresolved `var()` black);
  - parses that with `usvg`, using a font database holding only `text::REGULAR_TTF` and `text::SEMIBOLD_TTF` (NFR-7);
  - fills a `tiny_skia::Pixmap` with the theme's `Surface1`;
  - renders the tree scaled to the pixel width;
  - encodes PNG bytes.
- **The pipeline** decides the format from `-o` (FR-52), as today. For `.png` it arranges the wide arrangement only (FR-71) and hands the bytes to `write_atomic` (NFR-14). Nothing is written unless rasterising succeeds, so SC-18 holds by construction.
- **`--theme` and `--width`** stay where clap parses them (`cli.rs`). `Config` carries them. HTML ignores them, as today (FR-56, FR-57).

**Tech stack:**

- Rust 2024.
- `resvg` 0.48 moves from dev-dependency to dependency, with default features off and `text` on, so no system font is ever loaded. It re-exports `usvg` (and through it `fontdb`) and `tiny_skia`, whose default `png-format` feature gives `Pixmap::encode_png` and `decode_png`.

**Spec:**

- `docs/product/radargen-prd.md` v0.71, amended to v0.72 by Task 0: FR-52, FR-55, FR-56, FR-57, FR-64, FR-71, NFR-4, NFR-7, NFR-12, NFR-14, SC-7, SC-11, SC-18, SC-25, D-40, D-86, and the new D-87.
- Architecture §3 (`src/emit/png.rs`), §5 (literal colours for the static target) and §10 (PNG compared by decoded pixels).

## Global Constraints

- `--width` accepts **320–10000** inclusive, enforced by clap (FR-64, already in `cli.rs`).
- Height is **always** computed from the document's aspect ratio, never a flag (FR-57).
- PNG output is the **wide arrangement** only (FR-71).
- Colours are **literal**, resolved for `--theme` at generation time. `--theme` is **ignored, not an error, on HTML** (FR-56), as is `--width` (FR-57).
- Fonts are **pinned**: only the vendored faces, and no system font lookup (NFR-7).
- PNG is **pixel-identical** run to run, not byte-identical (NFR-4, D-40). It carries **no output version** (SC-25).
- A failure writes **nothing** and leaves an existing `-o` file byte-unchanged (NFR-8, NFR-14, SC-18).
- Strict clippy: pedantic, nursery and cargo; `as_conversions` is denied, so every cast needs an `#[expect]` with a reason; no `unwrap`; `missing_docs`.
- NFR-4: plain `*` and `+`, never `mul_add`. `suboptimal_flops` is expected with the project's standard reason.
- A test that writes a golden uses `UPDATE_GOLDEN=1`, refused under `CI`.

## Rulings (decided here; the maintainer may overturn)

1. **The PNG has a ground.** The documents draw no background; HTML paints the page's. A transparent PNG puts dark-theme ink (pale) on whatever slide it lands on, which fails SC-11. The pixmap is filled with the theme's `Surface1` before rendering. (D-87)
2. **The default width is clamped to FR-64's ceiling.** FR-57's default is the document's own width, and since 1.0.1 (rg-tzz) a document can be wider than 10,000 units. A default past 10,000 is drawn at 10,000, with a warning naming the natural width and `--width`. Refusing would turn a layout limit into a fatal error on valid input, which D-71 rejects. (D-87)
3. **An image past 100 megapixels is refused before it is allocated.** FR-64 bounds the width, but a long legend makes the document tall, and 10,000 × 50,000 is a 2 GB allocation that would kill the process rather than fail it. 100 megapixels is 10,000 × 10,000, the square FR-64's ceiling implies (400 MB of pixels). Past it the run fails with a structured error saying so and suggesting a smaller `--width`. (D-87)
4. **The release number** — *for the maintainer.* The standing rule is that every PR is a release and a feature earns a minor bump, so this PR ships as **1.1.0**, SVG (PR 17) as 1.2.0, and `--palette` (PR 18) as 1.3.0; the PRD's "v1.1" then names the release line, not one version. The alternative is to hold PNG unreleased until SVG lands and ship both as 1.1.0, which breaks the standing rule. This plan assumes **1.1.0**, and the `.svg` refusal then says "arrives in radargen 1.2".

## Review Focus

1. **A long legend at `--width 10000`**, an image past 100 megapixels. Expected: a structured error, exit 1, nothing written, no allocation attempted. Test: `pixel_size_refuses_past_the_area_cap` (Task 1) and `an_image_too_large_fails_and_writes_nothing` (Task 3).
2. **`--theme dark`**, where any colour left as `var()` would render black on near-black. Expected: no `var(` reaches resvg, and the ground is the dark `Surface1`. Test: `the_static_svg_has_no_custom_properties` and `the_ground_is_the_themes_surface` (Task 1).
3. **`--width 320`**, the floor, on a wide document. Expected: a valid PNG 320 wide, its height in proportion, never 0. Test: `the_floor_width_renders` (Task 1).
4. **`-o RADAR.PNG`**, and `-o` naming the input under a `.png` name. Expected: the first works (FR-52); the second is still D-74's error. Test: `an_upper_case_png_extension_renders` (Task 3); D-74's existing tests run for every format.
5. **A rasterisation failure over an existing file.** Expected: exit 1, the old file byte-identical (SC-18). Test: `an_image_too_large_fails_and_writes_nothing` (Task 3) hashes the old file before and after.

---

## File map

| File | Change |
| --- | --- |
| `Cargo.toml` | `resvg` to `[dependencies]` (`default-features = false, features = ["text"]`); drop `tiny-skia`, `usvg` and `fontdb` from dev-dependencies (tests use `resvg::{usvg, tiny_skia}` and `usvg::fontdb`) |
| `src/emit/png.rs` | **new**: `PixelSize`, `pixel_size`, `rasterise`, `MAX_PNG_WIDTH`, `MAX_PNG_PIXELS` |
| `src/emit/mod.rs` | `pub mod png;` |
| `src/diagnostics.rs` | `Fatal::Rasterise`, `Fatal::ImageTooLarge`, `RenderKind::WidthClamped` |
| `src/cli.rs` | `format()` accepts `.png`; `.svg` arrives in 1.2; help text |
| `src/pipeline.rs` | `Config` gains `format`, `theme`, `width`; `render` returns bytes and may fail |
| `tests/common/fonts.rs` | `usvg_options` uses `radargen::emit::png::usvg_options` |
| `tests/run.rs` | PNG end to end |
| `tests/serialise/golden.rs` | `golden_png`, comparing decoded pixels; two PNG goldens |
| `tests/golden/png/` | **new**: `prd-light.png`, `prd-dark.png` |
| `tests/determinism.rs` | PNG pixels identical across processes |
| `README.md`, `docs/product/radargen-prd.md`, `docs/design/architecture.md` | Task 0 and Task 5 |

---

### Task 0: Spec — D-87 and v0.72

**Files:** `docs/product/radargen-prd.md`.

- [ ] **Step 1:** Add **D-87** after D-86, in the table's form:

  > **The PNG has a ground, a clamped default width and an area cap** (FR-55, FR-57, FR-64, SC-11, SC-18). *Added by the PR 16 plan.* The documents draw no background, so the PNG is filled with the theme's `Surface1` before rendering: a transparent dark PNG would put pale ink on whatever slide it lands on. FR-57's default width is the document's own, which since 1.0.1 can pass 10,000 units; such a default is drawn at FR-64's 10,000 with a warning naming the natural width, since refusing valid input for a layout limit is what D-71 rejects. FR-64 bounds the width, not the height, and a long legend makes the document tall, so an image past 100 megapixels (FR-64's 10,000 squared) is refused before it is allocated, with an error suggesting a smaller `--width`; an allocation of several gigabytes would otherwise kill the process instead of failing it. **Rejected:** a transparent PNG; refusing a default width past 10,000; letting the allocator decide.

- [ ] **Step 2:** Add a revision row: `| 0.72 | <date> | **The PNG export's ground, default width and size cap** (D-87; rg-42m). |`, and set `Version: 0.72`.
- [ ] **Step 3:** Commit: `docs(prd): D-87, the PNG's ground, default width and size cap`.

---

### Task 1: `emit/png.rs` — size and rasterise

**Files:**
- Create: `src/emit/png.rs`
- Modify: `src/emit/mod.rs`, `src/diagnostics.rs`, `Cargo.toml`, `tests/common/fonts.rs`

**Interfaces:**
- Produces:
  - `pub const MAX_PNG_WIDTH: u32 = 10_000;`
  - `pub const MAX_PNG_PIXELS: u64 = 100_000_000;`
  - `pub struct PixelSize { pub width: u32, pub height: u32, pub clamped: Option<u32> }`, where `clamped` is the natural width when the default was clamped.
  - `pub fn pixel_size(document: &Document, requested: Option<u32>) -> Result<PixelSize, Fatal>`
  - `pub fn rasterise(document: &Document, palette: &Palette, size: PixelSize) -> Result<Vec<u8>, Fatal>`
  - `pub fn usvg_options() -> usvg::Options<'static>`
  - `Fatal::Rasterise { reason: String }`, `Fatal::ImageTooLarge { width: u32, height: u32 }`, `RenderKind::WidthClamped { natural: u32 }`

- [ ] **Step 1: Dependencies.** In `Cargo.toml` `[dependencies]` add:

  ```toml
  # PNG export (FR-55). No system fonts (NFR-7): only `text`, so usvg
  # never looks past the font database the emitter builds.
  resvg = { version = "0.48", default-features = false, features = ["text"] }
  ```

  From `[dev-dependencies]` remove `resvg`, `usvg`, `tiny-skia` and `fontdb`. In the tests that import them, replace `usvg::` with `resvg::usvg::`, `tiny_skia::` with `resvg::tiny_skia::`, and `fontdb::` with `resvg::usvg::fontdb::`. Find them with `rg -l 'usvg::|tiny_skia::|fontdb::' tests`.

  Run: `cargo build && cargo test --no-run`. Expected: builds.

  Run: `cargo tree -e features -i resvg | rg 'system-fonts|memmap'`. Expected: no output.

  Run: `cargo deny check licenses advisories`. Expected: ok. If a new crate's licence is refused, stop and report it rather than widening `deny.toml`.

- [ ] **Step 2: Write the failing tests** at the bottom of `src/emit/png.rs` (module skeleton below, tests first):

  ```rust
  //! The PNG export (FR-55, FR-71, NFR-7, D-87): the wide document
  //! serialised with literal colours for one theme, rasterised by resvg
  //! through the vendored faces alone, over the theme's ground.

  #[cfg(test)]
  mod tests {
      use super::*;
      use crate::layout::arrange::{Arrangement, Blocks, arrange};
      use crate::layout::block::fonts;
      use crate::layout::test_support::{PRD_EXAMPLE, plotted};
      use crate::palette::{Palette, Theme};
      use crate::svg::{Group, Node, Point};
      use resvg::tiny_skia::Pixmap;

      fn prd() -> Document {
          let (catalogue, order) = plotted(PRD_EXAMPLE, None);
          let blocks = Blocks::new(&catalogue, &order, Some("Radar"), None, fonts());
          arrange(Arrangement::Wide, &blocks)
      }

      /// A document `width` by `height` with nothing in it.
      fn empty(width: f64, height: f64) -> Document {
          Document { width, height, root: Node::Group(Group::new(Point::ORIGIN, Vec::new())) }
      }

      #[test]
      fn the_default_is_the_documents_own_width() {
          let doc = empty(1025.077, 824.0);
          let size = pixel_size(&doc, None).expect("fits");
          assert_eq!((size.width, size.height, size.clamped), (1026, 825, None));
      }

      #[test]
      fn the_height_follows_a_requested_width() {
          let doc = empty(1000.0, 800.0);
          let size = pixel_size(&doc, Some(1920)).expect("fits");
          assert_eq!((size.width, size.height), (1920, 1536));
      }

      /// D-87: a default past FR-64's ceiling is drawn at it, and says so.
      #[test]
      fn a_default_past_the_ceiling_is_clamped() {
          let doc = empty(18_355.334, 2_000.0);
          let size = pixel_size(&doc, None).expect("fits");
          assert_eq!((size.width, size.clamped), (MAX_PNG_WIDTH, Some(18_356)));
          assert_eq!(size.height, 1090);
      }

      /// D-87: past 100 megapixels the run fails before allocating.
      #[test]
      fn pixel_size_refuses_past_the_area_cap() {
          let doc = empty(1_000.0, 20_000.0);
          let err = pixel_size(&doc, Some(10_000)).expect_err("200 megapixels");
          assert!(matches!(err, Fatal::ImageTooLarge { width: 10_000, height: 200_000 }), "{err:?}");
      }

      #[test]
      fn the_floor_width_renders() {
          let doc = prd();
          let size = pixel_size(&doc, Some(320)).expect("fits");
          assert!(size.height > 0);
          let png = rasterise(&doc, &Palette::built_in(Theme::Light), size).expect("renders");
          let pixmap = Pixmap::decode_png(&png).expect("a PNG");
          assert_eq!((pixmap.width(), pixmap.height()), (size.width, size.height));
      }

      /// Architecture §5: resvg renders an unresolved `var()` black.
      #[test]
      fn the_static_svg_has_no_custom_properties() {
          let svg = serialise(&prd(), Target::Static(&Palette::built_in(Theme::Dark)));
          assert!(!svg.contains("var("), "{svg}");
      }

      /// D-87: the corner pixel, where nothing is drawn, is the theme's
      /// Surface1, in both themes.
      #[test]
      fn the_ground_is_the_themes_surface() {
          let doc = prd();
          for theme in [Theme::Light, Theme::Dark] {
              let palette = Palette::built_in(theme);
              let size = pixel_size(&doc, Some(400)).expect("fits");
              let png = rasterise(&doc, &palette, size).expect("renders");
              let pixmap = Pixmap::decode_png(&png).expect("a PNG");
              let corner = pixmap.pixel(0, 0).expect("a pixel");
              let want = ground(&palette).to_color_u8();
              assert_eq!(
                  (corner.red(), corner.green(), corner.blue(), corner.alpha()),
                  (want.red(), want.green(), want.blue(), 255),
                  "{theme:?}"
              );
          }
      }

      /// A height that rounds to nothing cannot be rasterised, and says so.
      #[test]
      fn a_degenerate_size_is_a_rasterise_error() {
          let doc = empty(100.0, 100.0);
          let err = rasterise(&doc, &Palette::built_in(Theme::Light), PixelSize { width: 10, height: 0, clamped: None })
              .expect_err("no pixels");
          assert!(matches!(err, Fatal::Rasterise { .. }), "{err:?}");
      }
  }
  ```

  Add `pub mod png;` to `src/emit/mod.rs`.

- [ ] **Step 3:** Run `cargo test --lib emit::png`. Expected: FAIL, because `pixel_size`, `rasterise`, `PixelSize`, `ground`, `Fatal::Rasterise` and `Fatal::ImageTooLarge` are not defined.

- [ ] **Step 4: The diagnostics.** In `src/diagnostics.rs`'s `Fatal`, beside `Unwritable`:

  ```rust
  /// FR-55: resvg could not parse or rasterise the document, or encode
  /// the image. Nothing at `-o` has changed (NFR-14).
  #[error("the PNG cannot be drawn: {reason}")]
  Rasterise {
      /// What failed, in resvg's or the encoder's words.
      reason: String,
  },
  /// D-87: the image would pass `MAX_PNG_PIXELS`.
  #[error("a {width} by {height} pixel PNG is too large to draw; pass a smaller --width")]
  ImageTooLarge {
      /// The width in pixels.
      width: u32,
      /// The height in pixels.
      height: u32,
  },
  ```

  In `RenderKind`, after `NarrowWidened`:

  ```rust
  /// D-87: the document is wider than FR-64's ceiling and `--width` was
  /// not given, so the PNG is drawn at the ceiling.
  WidthClamped {
      /// The document's own width, rounded up.
      natural: u32,
  },
  ```

  And in `RenderKind::message`:

  ```rust
  Self::WidthClamped { natural } => format!(
      "the radar is {natural} pixels wide, past --width's {MAX_PNG_WIDTH}; the PNG is drawn {MAX_PNG_WIDTH} wide, or pass --width to choose"
  ),
  ```

  importing `crate::emit::png::MAX_PNG_WIDTH`. Add a case for each new variant to `tests/diagnostics.rs`'s message-table test, in the file's existing pattern.

- [ ] **Step 5: The module.**

  ```rust
  use std::sync::Arc;

  use resvg::tiny_skia::{Color, Pixmap, Transform};
  use resvg::usvg;

  use crate::diagnostics::Fatal;
  use crate::palette::{Neutral, Palette, Role};
  use crate::svg::{Document, Target, serialise};
  use crate::text::{FAMILY, REGULAR_TTF, SEMIBOLD_TTF};

  /// FR-64's ceiling: the widest PNG, in pixels.
  pub const MAX_PNG_WIDTH: u32 = 10_000;
  /// D-87: the largest image drawn, FR-64's ceiling squared.
  pub const MAX_PNG_PIXELS: u64 = 100_000_000;

  /// The pixel size an image is drawn at (FR-57, D-87).
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct PixelSize {
      /// The width in pixels.
      pub width: u32,
      /// The height, following the document's aspect.
      pub height: u32,
      /// The document's own width, when the default was clamped to
      /// `MAX_PNG_WIDTH` (D-87).
      pub clamped: Option<u32>,
  }

  /// FR-57: `requested`, or the document's own width rounded up and
  /// clamped to `MAX_PNG_WIDTH` (D-87); the height in proportion.
  ///
  /// # Errors
  ///
  /// [`Fatal::ImageTooLarge`] past `MAX_PNG_PIXELS` (D-87).
  pub fn pixel_size(document: &Document, requested: Option<u32>) -> Result<PixelSize, Fatal> {
      let natural = whole(document.width.ceil());
      let (width, clamped) = match requested {
          Some(width) => (width, None),
          None if natural > MAX_PNG_WIDTH => (MAX_PNG_WIDTH, Some(natural)),
          None => (natural, None),
      };
      let height = whole((f64::from(width) * document.height / document.width).round());
      if u64::from(width) * u64::from(height) > MAX_PNG_PIXELS {
          return Err(Fatal::ImageTooLarge { width, height });
      }
      Ok(PixelSize { width, height, clamped })
  }

  /// FR-55: `document` as PNG bytes, its colours `palette`'s literals
  /// (FR-56), over its `Surface1` (D-87), drawn through the vendored faces
  /// alone (NFR-7).
  ///
  /// # Errors
  ///
  /// [`Fatal::Rasterise`] when resvg cannot parse the document, the pixmap
  /// cannot be made, or the image cannot be encoded.
  pub fn rasterise(document: &Document, palette: &Palette, size: PixelSize) -> Result<Vec<u8>, Fatal> {
      let fail = |reason: String| Fatal::Rasterise { reason };
      let svg = serialise(document, Target::Static(palette));
      let tree = usvg::Tree::from_str(&svg, &usvg_options()).map_err(|e| fail(e.to_string()))?;
      let mut pixmap = Pixmap::new(size.width, size.height)
          .ok_or_else(|| fail(format!("{} by {} pixels cannot be allocated", size.width, size.height)))?;
      pixmap.fill(ground(palette));
      let scale = narrow(f64::from(size.width) / document.width);
      resvg::render(&tree, Transform::from_scale(scale, scale), &mut pixmap.as_mut());
      pixmap.encode_png().map_err(|e| fail(e.to_string()))
  }

  /// `usvg` options whose font database holds only the vendored faces,
  /// with Inter as the default family (NFR-7). The tests' oracles use the
  /// same, so they rasterise as the export does.
  #[must_use]
  pub fn usvg_options() -> usvg::Options<'static> {
      let mut db = usvg::fontdb::Database::new();
      db.load_font_data(REGULAR_TTF.to_vec());
      db.load_font_data(SEMIBOLD_TTF.to_vec());
      usvg::Options {
          fontdb: Arc::new(db),
          font_family: FAMILY.to_string(),
          ..usvg::Options::default()
      }
  }

  /// The theme's `Surface1`, the ground the HTML page paints (D-87).
  /// Palette values are `#RRGGBB`, validated when a palette is built.
  fn ground(palette: &Palette) -> Color {
      let hex = palette.value(Role::Neutral(Neutral::Surface1));
      let channel = |at: usize| {
          hex.get(at..at + 2)
              .and_then(|pair| u8::from_str_radix(pair, 16).ok())
              .unwrap_or_default()
      };
      Color::from_rgba8(channel(1), channel(3), channel(5), u8::MAX)
  }

  /// A rounded, non-negative pixel count as `u32`.
  #[expect(
      clippy::cast_possible_truncation,
      clippy::cast_sign_loss,
      clippy::as_conversions,
      reason = "f64 has no TryFrom to an integer; callers pass a rounded, non-negative value, and `as` saturates past u32::MAX, which the area cap then refuses"
  )]
  const fn whole(v: f64) -> u32 {
      v as u32
  }

  /// A scale factor as resvg's `f32`.
  #[expect(
      clippy::cast_possible_truncation,
      clippy::as_conversions,
      reason = "tiny-skia transforms are f32; a scale between 0.03 and 31 loses nothing that shows"
  )]
  const fn narrow(v: f64) -> f32 {
      v as f32
  }
  ```

  `string_slice` is denied, so `ground` uses `str::get`, as written. Replace the body of `tests/common/fonts.rs`'s `usvg_options` with `radargen::emit::png::usvg_options()`, keeping its doc comment.

- [ ] **Step 6:** Run `cargo test --lib emit::png && cargo test --test diagnostics`. Expected: PASS. Then run `cargo test`. Expected: PASS. The oracle tests now share the emitter's options.

- [ ] **Step 7:** Run `cargo clippy --all-targets -- -D warnings`. Expected: clean. Commit: `feat(png): rasterise a document over its theme's ground (FR-55, D-87)`.

---

### Task 2: The pipeline and the CLI

**Files:** `src/cli.rs`, `src/pipeline.rs`, `tests/cli.rs`.

**Interfaces:**
- Consumes: `png::{pixel_size, rasterise, PixelSize}`, `RenderKind::WidthClamped`.
- Produces: `Config { format: OutputFormat, theme: Theme, width: Option<u32>, .. }`.

- [ ] **Step 1: Failing CLI tests** in `tests/cli.rs`, in its existing style for `format`:

  ```rust
  #[test]
  fn png_is_a_format_and_svg_arrives_in_1_2() {
      assert_eq!(format(Path::new("r.png")), Ok(OutputFormat::Png));
      assert_eq!(format(Path::new("R.PNG")), Ok(OutputFormat::Png));
      let err = format(Path::new("r.svg")).expect_err("not yet");
      assert!(err.render().contains("arrives in radargen 1.2"), "{}", err.render());
  }
  ```

  Delete or rewrite any existing test asserting `.png` is `FormatNotInThisVersion`. Find it with `rg -n 'png' tests/cli.rs`.

- [ ] **Step 2:** Run `cargo test --test cli png_is_a_format`. Expected: FAIL.

- [ ] **Step 3: `cli.rs`.**
  - In `format`, `"png" => Ok(OutputFormat::Png)`, and `"svg" => Err(FormatNotInThisVersion { arrives_in: "1.2", .. })`.
  - The `#[error]` on `FormatNotInThisVersion` says "this version writes .html"; make it "this version writes .html and .png".
  - In the help text:
    - `about`: "Render a technology radar from a CSV file to a self-contained HTML page or a PNG image."
    - `-o`: ".html or .png, in any case (SVG arrives in version 1.2)".
    - `--theme` and `--width`: "for PNG and SVG (SVG in version 1.2)".
  - Update the `OutputFormat::Png` doc.
  - `tests/cli.rs` has a test keeping requirement numbers out of the help; keep it passing.

- [ ] **Step 4: `pipeline.rs`.**
  - `Config` gains `format: OutputFormat`, `theme: Theme` and `width: Option<u32>`.
  - `config()` keeps `format(output)?`'s value and copies `cli.theme` and `cli.width`.
  - `render` becomes:

  ```rust
  /// The order, the render warnings (FR-88, FR-89, FR-95), the blocks built
  /// once, and the output: the page around both arrangements (FR-68), or
  /// the wide arrangement as a PNG (FR-71).
  fn render(catalogue: &Catalogue, config: &Config, fonts: &Fonts, warnings: &mut Warnings) -> Result<Vec<u8>, Fatal> {
      let plotted = resolve(catalogue);
      let blocks = Blocks::new(catalogue, &plotted, config.title.as_deref(), config.subtitle.as_deref(), fonts);
      warn_crowding(catalogue, &plotted, blocks.radar.growth(), warnings);
      match config.format {
          OutputFormat::Html => {
              warn_narrow(&blocks, warnings);
              Ok(page(&Page {
                  wide: &wide_parts(&blocks),
                  narrow: &arrange(Arrangement::Narrow, &blocks),
                  catalogue,
                  plotted: &plotted,
                  title: config.title.as_deref(),
              })
              .into_bytes())
          }
          OutputFormat::Png | OutputFormat::Svg => {
              let wide = arrange(Arrangement::Wide, &blocks);
              let size = png::pixel_size(&wide, config.width)?;
              if let Some(natural) = size.clamped {
                  warnings.push(Warning::Render(RenderKind::WidthClamped { natural }));
              }
              png::rasterise(&wide, &Palette::built_in(config.theme), size)
          }
      }
  }
  ```

  `OutputFormat::Svg` never reaches here, because `format()` refuses it. Grouping it with `Png` keeps the match total without a panic. PR 17 splits it. Add a `//` comment saying so.

  `steps` becomes `let bytes = render(...)?; Ok(write_atomic(&config.output, &bytes)?)`.

  FR-95 (`warn_narrow`) concerns the narrow arrangement, which only HTML carries, so it moves inside the HTML arm.

- [ ] **Step 5:** Run `cargo test`. Expected: PASS. Commit: `feat(png): -o radar.png writes the wide arrangement; --theme and --width take effect`.

---

### Task 3: End to end

**Files:** `tests/run.rs`.

- [ ] **Step 1: Failing tests.** Add to `tests/run.rs`, using its `seeded`, `run_in`, `RENDER` and `PRD`:

  ```rust
  use resvg::tiny_skia::Pixmap;

  fn png(dir: &Path, name: &str) -> Pixmap {
      Pixmap::decode_png(&fs::read(dir.join(name)).expect("written")).expect("a PNG")
  }

  /// FR-55, FR-57, FR-71: a PNG of the wide arrangement, at --width, its
  /// height in proportion, with no output version (SC-25).
  #[test]
  fn renders_a_png_at_the_width_asked() {
      let dir = seeded(&[("radar.csv", PRD)]);
      let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png", "--width", "1920"]);
      assert!(ran.ok(), "{}", ran.stderr);
      let image = png(dir.path(), "radar.png");
      assert_eq!(image.width(), 1920);
      assert!(image.height() > 1000 && image.height() < 2000, "{}", image.height());
      let bytes = fs::read(dir.path().join("radar.png")).expect("written");
      assert!(!bytes.windows(8).any(|w| w == b"radargen"), "SC-25: no version in a PNG");
  }

  /// FR-56: the dark theme's ground, and the light default's.
  #[test]
  fn the_theme_sets_the_ground() {
      let dir = seeded(&[("radar.csv", PRD)]);
      for (theme, name) in [("light", "l.png"), ("Dark", "d.png")] {
          let ran = run_in(dir.path(), &["radar.csv", "-o", name, "--theme", theme]);
          assert!(ran.ok(), "{}", ran.stderr);
      }
      let (l, d) = (png(dir.path(), "l.png"), png(dir.path(), "d.png"));
      let corner = |p: &Pixmap| p.pixel(0, 0).expect("a pixel").red();
      assert!(corner(&l) > 200 && corner(&d) < 60, "{} {}", corner(&l), corner(&d));
  }

  /// FR-56, FR-57: HTML ignores both flags; the page is byte-identical.
  #[test]
  fn html_ignores_theme_and_width() {
      let dir = seeded(&[("radar.csv", PRD)]);
      assert!(run_in(dir.path(), &RENDER).ok());
      let plain = read(dir.path(), "radar.html");
      let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.html", "--theme", "dark", "--width", "800"]);
      assert!(ran.ok(), "{}", ran.stderr);
      assert_eq!(read(dir.path(), "radar.html"), plain);
  }

  /// FR-52: the extension is matched in any case.
  #[test]
  fn an_upper_case_png_extension_renders() {
      let dir = seeded(&[("radar.csv", PRD)]);
      let ran = run_in(dir.path(), &["radar.csv", "-o", "RADAR.PNG"]);
      assert!(ran.ok(), "{}", ran.stderr);
      let _ = png(dir.path(), "RADAR.PNG");
  }

  /// D-87, SC-18: an image past the area cap fails, writes nothing, and
  /// leaves the old file byte-identical.
  #[test]
  fn an_image_too_large_fails_and_writes_nothing() {
      // About 400 items make a legend long enough that 10,000 wide passes
      // 100 megapixels.
      let rows: String = (0..400).map(|i| format!("Item {i},Tools,Adopt,d\n")).collect();
      let dir = seeded(&[("radar.csv", &format!("{HEAD}{rows}")), ("radar.png", "old")]);
      let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png", "--width", "10000"]);
      assert!(!ran.ok());
      assert!(ran.stderr.contains("too large to draw"), "{}", ran.stderr);
      assert_eq!(read(dir.path(), "radar.png"), "old");
  }

  /// D-87: a default width past the ceiling is clamped, with a warning.
  #[test]
  fn a_default_width_past_the_ceiling_is_clamped() {
      let csv = format!("{HEAD}A,{},Adopt,d\nB,Tools,Adopt,d\n", "W".repeat(600));
      let dir = seeded(&[("radar.csv", &csv)]);
      let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png"]);
      assert!(ran.ok(), "{}", ran.stderr);
      assert!(ran.stderr.contains("the PNG is drawn 10000 wide"), "{}", ran.stderr);
      assert_eq!(png(dir.path(), "radar.png").width(), 10_000);
  }
  ```

  If 400 items do not pass 100 megapixels, find the count that does by computing the wide document's aspect in a scratch run (`radargen … -o x.html` and read its wide `viewBox`), and use the least round number past it. Record the figure in the test's comment.

- [ ] **Step 2:** Run `cargo test --test run`. Expected: every new test passes against Task 2's code, except where Tasks 1–2 left a gap. If one passes before Task 2 is applied (check with `git stash` of Task 2), it tests nothing; fix it.
- [ ] **Step 3:** Commit: `test(png): end to end, theme, width, clamp and the size cap`.

---

### Task 4: Goldens and determinism

**Files:** `tests/serialise/golden.rs`, `tests/golden/png/`, `tests/determinism.rs`.

- [ ] **Step 1: `golden_png`** in `tests/serialise/golden.rs`, beside `golden`:

  ```rust
  /// D-40: compares `got`'s decoded pixels with `tests/golden/png/{name}`'s.
  /// With `UPDATE_GOLDEN=1` (refused under CI) it writes the file and
  /// fails, so a blessing run is never green.
  pub fn golden_png(name: &str, got: &[u8]) {
      let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/png").join(name);
      if std::env::var_os("UPDATE_GOLDEN").is_some() {
          assert!(std::env::var_os("CI").is_none(), "UPDATE_GOLDEN is refused under CI");
          std::fs::create_dir_all(path.parent().expect("a parent")).expect("creates");
          std::fs::write(&path, got).expect("writes");
          panic!("blessed {}; unset UPDATE_GOLDEN and re-run", path.display());
      }
      let want = std::fs::read(&path).unwrap_or_else(|_| panic!("{} is missing: run with UPDATE_GOLDEN=1, then look at it", path.display()));
      let (want, got) = (decode(&want), decode(got));
      assert_eq!((want.width(), want.height()), (got.width(), got.height()), "{name}: size");
      let differing = want.data().chunks(4).zip(got.data().chunks(4)).filter(|(a, b)| a != b).count();
      assert_eq!(differing, 0, "{name}: {differing} pixels differ");
  }

  fn decode(png: &[u8]) -> resvg::tiny_skia::Pixmap {
      resvg::tiny_skia::Pixmap::decode_png(png).expect("a PNG")
  }
  ```

  And two tests:

  ```rust
  /// FR-55, FR-56: the PRD example, wide, in each theme, at its own width.
  #[test]
  fn prd_example_matches_its_png_goldens() {
      let doc = arrange(Arrangement::Wide, &prd().blocks);
      for (theme, name) in [(Theme::Light, "prd-light.png"), (Theme::Dark, "prd-dark.png")] {
          let size = radargen::emit::png::pixel_size(&doc, None).expect("fits");
          let png = radargen::emit::png::rasterise(&doc, &Palette::built_in(theme), size).expect("renders");
          golden_png(name, &png);
      }
  }
  ```

- [ ] **Step 2:** Run `UPDATE_GOLDEN=1 cargo test --test serialise png`, then `cargo test --test serialise png`. Expected: blessed, then PASS. **Open both PNGs and look at them**: the title, radar, key and legend are all present, the dark ground is dark, and no text is black on dark. Say what you saw in the commit message.

- [ ] **Step 3: Determinism across processes.** In `tests/determinism.rs`, following its existing two-process test, render the PRD example to `.png` in two processes and assert their decoded pixels are equal, using `Pixmap::decode_png` (NFR-4, D-40: pixels, not bytes).

- [ ] **Step 4:** Run `cargo test`. Expected: PASS. Commit: `test(png): goldens by decoded pixels, and determinism across processes`.

---

### Task 5: Docs, release

**Files:** `README.md`, `docs/design/architecture.md`, `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`.

- [ ] **Step 1:** In the README:
  - add PNG to the description and usage;
  - add a short "PNG" subsection: `-o radar.png`, `--theme light|dark`, `--width` from 320 to 10000 with the height following, and that it draws the wide arrangement;
  - mention the clamp and the size limit in one sentence each;
  - say SVG arrives in 1.2.
- [ ] **Step 2:** In architecture §3, mark the `src/emit/png.rs` row as present, not v1.1, and note that `usvg_options` is shared with the oracle tests.
- [ ] **Step 3:** Run `just ci`. Expected: green, browser checks included.
- [ ] **Step 4: Release** (assuming Ruling 4's 1.1.0):
  - set `version = "1.1.0"` and run `cargo update -p radargen`;
  - re-bless the HTML goldens. The diff must be only the generator line, `radargen 1.0` becoming `radargen 1.1`;
  - run `git-cliff --tag v1.1.0 --output CHANGELOG.md`;
  - `crumbs close rg-42m`;
  - commit: `chore(release): 1.1.0`.

---

## Self-review

- **Spec coverage:**
  - FR-52 (Task 2);
  - FR-55 and FR-71 (Tasks 1–2);
  - FR-56 and FR-57 (Tasks 1–3);
  - FR-64 (clap, already; clamp in Task 1);
  - NFR-4 and D-40 (Task 4);
  - NFR-7 (Task 1 features and `usvg_options`);
  - NFR-14 and SC-18 (Tasks 2–3);
  - SC-7 (one `Blocks`, `arrange(Wide)`, same as the HTML's parts);
  - SC-11 (ground, Task 1; looked at, Task 4);
  - SC-25 (Task 3).
- **No placeholders.** Task 3's item count is the one figure found by measurement; the step says how to find it.
- **Type consistency:** `PixelSize`, `pixel_size`, `rasterise`, `usvg_options`, `MAX_PNG_WIDTH` and `MAX_PNG_PIXELS` are defined in Task 1 and used in Tasks 2–4 with those names.
