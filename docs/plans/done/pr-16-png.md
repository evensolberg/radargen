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
synopsis: "-o radar.png writes the wide arrangement (FR-71) as a PNG: the document serialised with literal colours for --theme (FR-56) over the theme's own ground, which the static SVG now carries (D-87), rasterised by resvg through the vendored Inter faces alone (NFR-7), at --width pixels or the document's own width (FR-57), the height following its aspect. The default width is clamped to FR-64's 10,000 with a warning, text drawn below the legible size warns, and an image past 100 megapixels is refused before it is allocated. HTML still ignores both flags. .svg stays refused until PR 17 (1.2.0). Ships as 1.1.0."
status: complete
type: Plan
revision: "1.1"
review_date: 2026-10-16
reviewed_by:
  - "Rust (subagent)"
  - "QA (subagent)"
  - "Architecture (subagent)"
  - "Security (subagent)"
  - "Accessibility and visual design (subagent)"
completed_date: 2026-10-16
comments: "1.1 folds in the five plan reviews; every finding and its ruling is in the Review rulings table. The maintainer confirmed 1.1.0 and four PRD changes on 2026-10-16: the small-text warning, the ground in the static SVG, a lower area cap, and a golden tolerance. The lower cap is withdrawn (Ruling 3)."
---
# PR 16 — PNG export; --theme and --width take effect: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Crumb:** `rg-42m` · **Release:** 1.1.0 · **After:** 1.0.1 (#89) · **Then:** PR 17, SVG export, 1.2.0 (`rg-m2l`); PR 18, `--palette`, 1.3.0 (`rg-cdo`)

**Goal:** `radargen radar.csv -o radar.png --theme dark --width 1920` writes a PNG of the wide arrangement. It matches the HTML's wide arrangement item for item (SC-7) and stands alone on a slide (SC-11).

**Architecture:**

- **The static SVG gets a ground** (D-87). Under `Target::Static`, `serialise` writes a `Surface1` `<rect>` covering the viewBox before the root node. PNG inherits it now, and SVG in PR 17 inherits it for free. The interactive target is unchanged, since the page paints its own ground.
- **`src/emit/png.rs`** (architecture §3's planned file) is thin. It:
  - serialises the wide `Document` with `Target::Static(palette)`, which binds every colour to a literal (architecture §5: resvg renders an unresolved `var()` black);
  - parses that with `usvg`. The font database holds only `text::REGULAR_TTF` and `text::SEMIBOLD_TTF` (NFR-7), and the options read no file or URL (NFR-5);
  - frees the markup, then renders the tree into a `tiny_skia::Pixmap` scaled to the pixel width;
  - encodes PNG bytes.
- **The pipeline:**
  - It decides the format from `-o` (FR-52), as today. `Config` carries the format, the resolved `Palette`, and `--width`. The palette is resolved in `config()` (NFR-16 tier one), so PR 18's `--palette` file slots in there.
  - For `.png`, `render` arranges the wide arrangement only (FR-71) and returns the bytes for `write_atomic` (NFR-14).
  - Nothing is written unless rasterising succeeds, so SC-18 holds by construction.
  - `.svg` has its own arm, which refuses until PR 17.
- **The size checks.** `PixelSize`'s fields are private, and `pixel_size` is its only constructor. So the area cap and the height floor cannot be bypassed by a later caller.

**Tech stack:**

- Rust 2024.
- `resvg` 0.48 moves from dev-dependency to dependency, with default features off and `text` on. That means no system fonts, no memmap, no raster-image decoders and no svgz. It re-exports `usvg` (and through it `fontdb`) and `tiny_skia`. resvg pulls tiny-skia with its default features, so `png-format` is present.

**Spec:**

- `docs/product/radargen-prd.md` v0.71, amended to v0.72 by Task 0. The plan rests on: FR-52, FR-55, FR-56, FR-57, FR-64, FR-71, NFR-4, NFR-5, NFR-7, NFR-12, NFR-14, NFR-17, SC-7, SC-11, SC-18, SC-25, D-40, D-86, and the new D-87.
- Architecture §3 (`src/emit/png.rs`), §5 (literal colours for the static target), §7 (`Config`), §10 (PNG compared by decoded pixels).

## Global Constraints

- `--width` accepts **320–10000** inclusive, enforced by clap (FR-64). Both bounds become `style::MIN_EXPORT_WIDTH` and `style::MAX_EXPORT_WIDTH`, the single source for clap, the clamp and the messages.
- Height is **always** computed from the document's aspect ratio, never a flag (FR-57), and is never 0.
- PNG output is the **wide arrangement** only, drawn as one document (FR-71, architecture §2, §4), not D-85's split parts.
- Colours are **literal**, resolved for `--theme` at generation time. `--theme` is **ignored, not an error, on HTML** (FR-56), and so is `--width` (FR-57). Ignored means byte-identical output **and** identical stderr.
- Fonts are **pinned**: only the vendored faces, with no system font lookup (NFR-7). usvg reads **no file and no URL** (NFR-5).
- PNG is **pixel-identical** run to run on one platform, not byte-identical (NFR-4, D-40). Across platforms the PNG goldens allow D-87's tolerance (NFR-17 claims no PNG bit-exactness). It carries **no output version** (SC-25).
- A failure writes **nothing** and leaves an existing `-o` file byte-unchanged (NFR-8, NFR-14, SC-18).
- Strict clippy: pedantic, nursery and cargo. `as_conversions` and `string_slice` are denied, so every cast needs an `#[expect]` with a reason. No `unwrap`; `missing_docs`.
- NFR-4: plain `*` and `+`, never `mul_add`. `suboptimal_flops` is expected, with the project's standard reason.
- A test that writes a golden uses `UPDATE_GOLDEN=1`, which is refused under `CI`.

## Rulings

1. **The static SVG has a ground** (maintainer, 2026-10-16). The documents draw no background, and HTML paints the page's. A transparent PNG or SVG would put dark-theme ink (pale) on whatever slide it lands on, which fails SC-11. `serialise` under `Target::Static` writes a full-viewBox `Surface1` rect first, so PNG now and SVG in 1.2 share it.

   `Surface1` is the only correct colour. `MarkerHollow`, the text halos and the solid marker's separating stroke are all `Surface1`, and FR-82's contrast checks are made against it. Any other ground would show the halos as boxes, and changing it means re-verifying FR-82. (D-87)
2. **The default width is clamped to FR-64's ceiling.** FR-57's default is the document's own width, and since 1.0.1 (rg-tzz) a document can be wider than 10,000 units. A default past 10,000 is drawn at 10,000, with a warning naming the natural width and `--width`. Refusing would turn a layout limit into a fatal error on valid input, which D-71 rejects. An explicit `--width` never clamps and never warns about clamping. (D-87)
3. **An image past 100 megapixels is refused before it is allocated.** FR-64 bounds the width, but a long legend makes the document tall: 10,000 × 50,000 is a 2 GB pixmap that would kill the process rather than fail it.
   - **Why 100 megapixels:** it is 10,000 × 10,000, the square FR-64's ceiling implies.
   - **Peak memory at the cap:** about 1 GB. The pixmap is 400 MB, `encode_png` takes a demultiplied copy of another 400 MB, and the encoder adds its buffers. `rasterise` drops the markup string before allocating.
   - **What a refused run gets:** a structured error saying so and suggesting a smaller `--width`.
   - **The 64-megapixel cap is withdrawn.** It was agreed on 2026-10-16, but at 64 megapixels the PRD example itself (about 1025 × 824 units, an aspect most radars share) is refused at `--width 10000`: 10,000 × 8,039 is 80 megapixels. That would make FR-64's documented ceiling unusable on ordinary input. The real peak figure is stated in D-87 instead, which the security review offered as the alternative. (D-87)
4. **The release is 1.1.0** (maintainer, 2026-10-16). Every PR is a release, and a feature earns a minor bump. So PNG is 1.1.0, SVG (PR 17) is 1.2.0, and `--palette` (PR 18) is 1.3.0. The PRD's "v1.1" names the release line. The `.svg` refusal says "arrives in radargen 1.2".
5. **Text drawn below the legible size warns** (maintainer, 2026-10-16). The PNG scales the whole document by one factor. At `--width 320`, the PRD example's legend names draw at about 3.9 px. When `width / document.width < style::WIDE_MIN_SCALE`, where `LEGEND_NAME` text falls below `MIN_LEGIBLE_PX` (the threshold HTML already uses), the run warns. The warning names the least width that keeps text legible, or, when even 10,000 is not enough, says that fewer or shorter names would help. A warning, not an error, as D-71 treats every layout limit on valid input. (D-87)
6. **The PNG goldens allow a tolerance** (maintainer, 2026-10-16). CI runs macOS arm64, Linux x86_64 and Windows x86_64. tiny-skia's SIMD paths use different reciprocal approximations per architecture, so anti-aliased edges can differ by about 1 per channel.
   - **Golden tolerance:** a channel may differ by at most 2, and at most 0.1% of pixels may differ at all.
   - **Determinism test:** the same-process-pair test stays exact, since it is the real NFR-4 check on one platform.
   - **On a golden failure:** the test writes the rendered image and a diff image under `target/png-golden/`. (D-87)

## Review Focus

1. **A long legend at `--width 10000`**, or an image past 100 megapixels. Expected: a structured error, exit 1, nothing written, no allocation attempted. Tests:
   - `the_area_cap_is_inclusive` and `pixel_size_refuses_past_the_area_cap` (Task 2);
   - `an_image_too_large_fails_and_writes_nothing` (Task 3).

   This is a size refusal, not a rasterisation failure: `Fatal::Rasterise` has no end-to-end trigger, by design.
2. **`--theme dark`**, where any colour left as `var()`, or any default black, would render black on near-black. Expected: no `var(` reaches resvg, every paint is a palette literal, and no pixel of the dark PNG is darker than its ground by more than a small margin. Tests (Task 1 and Task 2):
   - `the_static_svg_paints_only_palette_values`;
   - `the_static_ground_is_the_themes_surface`;
   - `dark_ink_is_never_darker_than_its_ground`.
3. **`--width 320`**, the floor, on a wide document. Expected: a valid PNG 320 wide, its height in proportion, never 0, and the small-text warning. Tests:
   - `the_floor_width_renders`, `a_height_never_rounds_to_zero` and `a_narrow_width_reports_small_text` (Task 2);
   - `a_narrow_png_warns_about_small_text` (Task 3).
4. **`-o RADAR.PNG`, `-o r.Png`, and `-o` naming the input under a `.png` name.** Expected: the first two work (FR-52); the third is still D-74's error, with the input untouched. Tests:
   - `png_is_a_format_and_svg_arrives_in_1_2`;
   - `an_upper_case_png_extension_renders`;
   - `a_png_output_that_is_the_input_is_refused` (Task 3).
5. **A failed PNG write over an existing file.** Expected: exit 1, no stray temporary file, the old file byte-identical (SC-18). Tests: `an_image_too_large_fails_and_writes_nothing` and `a_png_into_a_missing_directory_fails` (Task 3), both comparing `snapshot(dir)` before and after.

---

## Review rulings

All five reviews, 2026-10-16. **Fixed** means the plan below now does it.

| # | Seat | Finding | Ruling |
| --- | --- | --- | --- |
| 1 | Rust (Critical) | `miniz_oxide` 0.8 and 0.9 trip `clippy::multiple_crate_versions` once resvg is a normal dependency | Fixed: `allowed-duplicate-crates = ["miniz_oxide"]` in `clippy.toml`, with the cause (png 0.18 directly and through flate2). Task 2 Step 1 |
| 2 | QA (Critical), Rust | PNG goldens compared exactly fail across architectures | Fixed: Ruling 6, D-87. Task 4 |
| 3 | QA (Critical) | Three existing tests pin `.png` as "arrives in 1.1" | Fixed: `tests/run.rs::bad_extensions_and_v1_1_formats`, `tests/cli.rs::png_and_svg_say_not_in_this_version_rather_than_unrecognised`, and `tests/diagnostics.rs`'s two rows are rewritten in Task 3 |
| 4 | Rust, QA | `UnrecognisedExtension` still says "must end in .html" | Fixed: ".html or .png", pinned in `tests/diagnostics.rs` and `tests/run.rs` |
| 5 | Architecture, Rust | `Png \| Svg` sharing one arm would write PNG bytes to `.svg` | Fixed: `Svg` has its own arm returning `FormatNotInThisVersion` |
| 6 | Architecture | The palette is built in `render`; §7 puts it in `Config` | Fixed: `Config.palette`, resolved in `config()`; no `theme` field |
| 7 | Architecture | The ground is PNG-only, so the SVG in 1.2 re-decides it | Fixed: Ruling 1; the ground is in the static serialiser. Task 1 |
| 8 | Architecture, Rust | The 10,000 ceiling is defined twice; `diagnostics` would import from `emit` | Fixed: `style::{MIN_EXPORT_WIDTH, MAX_EXPORT_WIDTH}`; clap uses them; `WidthClamped { natural, drawn }` carries its numbers |
| 9 | Security, Rust | The peak memory at the cap is understated | Fixed: Ruling 3 states about 1 GB; `rasterise` drops the markup before allocating |
| 10 | Security, Rust | usvg's default href resolver reads files | Fixed: `resources_dir: None` and resolvers returning `None`; `usvg_reads_no_file` test |
| 11 | Security | `rasterise` is public and trusts any `PixelSize` | Fixed: private fields, with `pixel_size` the only constructor |
| 12 | Accessibility | Nothing warns when text becomes illegible | Fixed: Ruling 5, `RenderKind::PngTextSmall` |
| 13 | Accessibility | Scripts Inter lacks render as boxes, unannounced | README says so. A warning when shaping yields glyph 0 is a new crumb (Task 5). `a_png_of_hostile_text_renders` pins that it does not fail |
| 14 | Accessibility | No alt-text guidance | Fixed: README, and a PNG section in `docs/a11y/manual-checks.md` (Task 5) |
| 15 | QA | `the_ground_is_the_themes_surface` compares `ground()` with itself | Fixed: the expected values are the palettes' literal hex, written in the test |
| 16 | QA | The `var(` test never exercises `rasterise` | Fixed: `dark_ink_is_never_darker_than_its_ground` renders; the string test stays as a cheap pin |
| 17 | QA | Task 3's tests come after Task 2's code; Task 1's tests fail only by not compiling | Fixed: end-to-end tests are written first in Task 3 and watched failing; Task 2 stubs with `todo!()` before its tests run |
| 18 | QA | `warn_narrow` moving and explicit `--width` not clamping have no test | Fixed: `a_png_gives_no_narrow_warning`, `an_explicit_width_never_clamps`; `html_ignores_theme_and_width` compares stderr |
| 19 | QA | No boundary test for the area cap | Fixed: `the_area_cap_is_inclusive` (10,000 × 10,000 accepted, 10,000 × 10,001 refused) |
| 20 | QA | D-74 is only tested with `.html` | Fixed: `a_png_output_that_is_the_input_is_refused` |
| 21 | QA | No PNG write-failure test | Fixed: `a_png_into_a_missing_directory_fails` |
| 22 | QA | Rasterising in debug builds is slow | Fixed: `[profile.dev.package.*]` at `opt-level = 3` for resvg, usvg and tiny-skia. The clamp test reuses rg-tzz's fixture, whose height stays small |
| 23 | QA | `usvg_options` becomes both code and oracle | Fixed: `the_font_database_holds_the_two_vendored_faces`; the feature check runs in `just ci` |
| 24 | QA | Glyph fallback | See 13 |
| 25 | QA | The determinism test checks little | Kept, and it also compares the static SVG strings from both processes |
| 26 | QA | SC-7's pin should be named | Fixed: Self-review cites `the_wide_parts_tile_the_wide_document` (`src/layout/arrange.rs`) |
| 27 | QA | `r.Png` and `--width abc`, `-5` | Fixed: Task 3 |
| 28 | QA, Rust | Grepping the bytes for "radargen" is close to vacuous | Fixed: `the_png_has_no_text_chunks` checks no `tEXt`, `iTXt` or `zTXt` chunk |
| 29 | QA | Looking by eye cannot be repeated | Both golden PNGs are attached to the PR body for the maintainer; the golden pins them after |
| 30 | Security | A height that rounds to 0 | Fixed: the height is at least 1 (`a_height_never_rounds_to_zero`) |
| 31 | Security | The area cap does not bound parse and shaping cost | Recorded as a known limit in D-87. HTML already pays the layout cost for the same input; a constant factor, not a new class. No crumb |
| 32 | Security, Rust | `ground()` parses hex again and fails silently | Fixed: `Palette::rgb(role) -> [u8; 3]`, shared with `luminance`, which already `expect`s valid hex |
| 33 | Security | `Rasterise`'s reason might quote input | resvg's and the encoder's messages carry positions and kinds, not text. A doc comment says so |
| 34 | Architecture | A format-agnostic intrinsic size for PR 17 | Not now (YAGNI). With the ground in the serialiser, PR 17 needs only FR-57's height rule. A note goes on `rg-m2l` (Task 5) |
| 35 | Architecture | Ruling 4 and the frontmatter still say "waits on the maintainer" | Fixed |
| 36 | Architecture | Stale "v1.1" mentions | Fixed: Task 0 and Task 5 list them. `Cargo.toml`'s description says "HTML page or PNG" until 1.2 |
| 37 | Architecture | Binary size is not measured | Fixed: before and after in the PR body (Task 5) |
| 38 | Architecture | `usvg_options` puts usvg in the public API | Fixed: a doc line says it is public for the test oracles |
| 39 | Architecture | Release stops before tag and publish | Fixed: Task 5 Step 6 |
| 40 | Rust | The new errors do not start `-o {path}:` | Declined: NFR-10's prefix is for errors about the path. These are about the image, and the path is fine. Threading the path into `emit::png` buys nothing |
| 41 | Accessibility | Interactive-only nodes in the whole static document | Fixed: `the_static_prd_document_has_no_interactive_nodes` |
| 42 | Accessibility | A long legend gives a tall image that shrinks on a slide | README says so, with `--width` and splitting the CSV as remedies |

---

## File map

| File | Change |
| --- | --- |
| `Cargo.toml` | `resvg` to `[dependencies]` (`default-features = false, features = ["text"]`); drop `resvg`, `tiny-skia`, `usvg` and `fontdb` from dev-dependencies; `[profile.dev.package.*]` for resvg, usvg and tiny-skia; `description` says "HTML page or PNG" |
| `clippy.toml` | `allowed-duplicate-crates = ["miniz_oxide"]` |
| `justfile` | `ci` runs the resvg feature check |
| `src/style.rs` | `MIN_EXPORT_WIDTH`, `MAX_EXPORT_WIDTH`, `MAX_PNG_PIXELS` |
| `src/palette.rs` | `Palette::rgb`; `luminance` uses it |
| `src/svg/serialise/mod.rs` | the static ground rect |
| `tests/golden/serialiser/*-light.svg` | re-blessed: one added `<rect>` line each |
| `src/emit/png.rs` | **new**: `PixelSize`, `pixel_size`, `rasterise`, `usvg_options` |
| `src/emit/mod.rs` | `pub mod png;` |
| `src/diagnostics.rs` | `Fatal::Rasterise`, `Fatal::ImageTooLarge`, `RenderKind::WidthClamped`, `RenderKind::PngTextSmall`; `UnrecognisedExtension` text |
| `src/cli.rs` | `format()` accepts `.png`; `.svg` arrives in 1.2; the width range from `style`; help text |
| `src/pipeline.rs` | `Config` gains `format`, `palette`, `width`; `render` returns bytes and may fail |
| `tests/common/fonts.rs` | `usvg_options` delegates to `radargen::emit::png::usvg_options` |
| `tests/cli.rs`, `tests/diagnostics.rs`, `tests/run.rs` | rewritten `.png` tests; PNG end to end |
| `tests/serialise/golden.rs` | `golden_png`, with a tolerance; two PNG goldens |
| `tests/golden/png/` | **new**: `prd-light.png`, `prd-dark.png` |
| `tests/determinism.rs` | PNG pixels and static SVG identical across processes |
| `README.md`, `docs/a11y/manual-checks.md`, `docs/product/radargen-prd.md`, `docs/design/architecture.md` | Task 0 and Task 5 |

---

### Task 0: Spec — D-87 and v0.72

**Files:** `docs/product/radargen-prd.md`.

- [x] **Step 1:** Add **D-87** after D-86, in the table's form:

  > **The static outputs have a ground; the PNG has a clamped default width, a small-text warning, an area cap and a golden tolerance** (FR-55, FR-57, FR-64, NFR-17, SC-11, SC-18). *Added by the PR 16 plan; confirmed by the maintainer 2026-10-16.*
  >
  > - **Ground.** The documents draw no background, so the static SVG, and the PNG drawn from it, open with a rect of the theme's `Surface1` covering the viewBox: a transparent dark export would put pale ink on whatever slide it lands on. `Surface1` is the only correct value, because `MarkerHollow`, the text halos and the solid marker's separating stroke are `Surface1`, and FR-82's contrast is measured against it. Changing it means re-verifying FR-82.
  > - **Default width.** FR-57's default is the document's own width, which since 1.0.1 can pass 10,000 units. Such a default is drawn at FR-64's 10,000, with a warning naming the natural width, since refusing valid input for a layout limit is what D-71 rejects. An explicit `--width` is never clamped.
  > - **Small text.** When the PNG's scale falls below `WIDE_MIN_SCALE`, where legend names draw under `MIN_LEGIBLE_PX`, the run warns and names the least width that keeps them legible.
  > - **Area cap.** FR-64 bounds the width, not the height, and a long legend makes the document tall. So an image past 100 megapixels (FR-64's 10,000 squared) is refused before it is allocated, with an error suggesting a smaller `--width`. Peak memory at the cap is about 1 GB: the pixmap, a demultiplied copy for encoding, and the encoder's buffers. The cap bounds pixels, not the cost of parsing and shaping a very large document, which HTML pays too.
  > - **Golden tolerance.** NFR-17 claims no PNG bit-exactness across platforms, and tiny-skia's SIMD paths differ by architecture. So the PNG goldens allow a difference of 2 per channel on at most 0.1% of pixels. Run to run on one platform, pixels are identical (NFR-4).
  > - **Rejected:**
  >   - a transparent export;
  >   - a ground drawn by the PNG emitter alone, which SVG would then re-decide;
  >   - refusing a default width past 10,000;
  >   - letting the allocator decide;
  >   - a 64-megapixel cap, which refuses the PRD example at `--width 10000`;
  >   - exact PNG goldens across platforms.

- [x] **Step 2:** In §14.1, and wherever "v1.1" names when PNG or SVG ships, say **1.1.0** for PNG and **1.2.0** for SVG, and say that "v1.1" names the release line. Find the places with `rg -n 'v1\.1' docs/product/radargen-prd.md`.
- [x] **Step 3:** Add a revision row, `| 0.72 | 2026-10-16 | **The static outputs' ground; the PNG's default width, small-text warning, size cap and golden tolerance** (D-87; rg-42m). PNG ships as 1.1.0, SVG as 1.2.0. |`, and set `Version: 0.72`.
- [x] **Step 4:** Commit: `docs(prd): D-87, the static ground and the PNG's limits`.

---

### Task 1: The static SVG's ground, and `Palette::rgb`

**Files:**
- Modify: `src/palette.rs`, `src/svg/serialise/mod.rs`, `src/svg/serialise/tests/mod.rs`, `tests/golden/serialiser/prd-wide-light.svg`, `tests/golden/serialiser/grown-wide-light.svg`

**Interfaces:**
- Produces: `Palette::rgb(&self, role: Role) -> [u8; 3]`, and static SVG whose first child is the ground rect.

- [x] **Step 1: Failing tests.**

  In `src/palette.rs`'s tests:

  ```rust
  /// D-87: the ground's channels, from the literal hex.
  #[test]
  fn rgb_reads_the_hex() {
      let light = Palette::built_in(Theme::Light);
      let dark = Palette::built_in(Theme::Dark);
      let surface = Role::Neutral(Neutral::Surface1);
      assert_eq!(light.rgb(surface), hex(light.value(surface)));
      assert_eq!(dark.rgb(surface), hex(dark.value(surface)));
      assert_ne!(light.rgb(surface), dark.rgb(surface));
  }

  /// An independent parse, so `rgb` is not its own oracle.
  fn hex(s: &str) -> [u8; 3] {
      let n = u32::from_str_radix(s.strip_prefix('#').expect("#RRGGBB"), 16).expect("hex");
      let [_, r, g, b] = n.to_be_bytes();
      [r, g, b]
  }
  ```

  In `src/svg/serialise/tests/mod.rs`:

  ```rust
  /// D-87: a static document opens with a Surface1 rect over the whole
  /// viewBox; an interactive one has none (the page paints its own).
  #[test]
  fn the_static_ground_is_the_themes_surface() {
      let doc = Document { width: 120.0, height: 80.0, root: Node::Group(Group::new(Point::ORIGIN, Vec::new())) };
      for theme in [Theme::Light, Theme::Dark] {
          let palette = Palette::built_in(theme);
          let svg = serialise(&doc, Target::Static(&palette));
          let first = svg.lines().nth(1).expect("a first child");
          let fill = palette.value(Role::Neutral(Neutral::Surface1));
          assert_eq!(first, format!(r#"<rect width="120" height="80" fill="{fill}"/>"#), "{theme:?}");
          let page = serialise(&doc, Target::Interactive(&palette));
          assert!(!page.contains("<rect"), "{page}");
      }
  }
  ```

  If the serialiser's attribute order or number format differs from the literal above (check `attr` and `fmt_coord`), write the expected line in its format. The test pins the line, not this spelling.

- [x] **Step 2:** Run `cargo test --lib palette::tests::rgb_reads_the_hex svg::serialise`. Expected: FAIL. `rgb` is not defined, and the static output has no rect.
- [x] **Step 3: Implement.**
  - **`Palette::rgb`:** the three channels, parsed with `str::get` (`string_slice` is denied) and the same `expect` message `luminance` uses: "palette values are #RRGGBB; PR 18 rejects anything else". `luminance` then takes `[u8; 3]` from `rgb`, not a hex string.
  - **The ground rect:** in `serialise`, after the `<svg …>` line and only when `!target.interactive()`, write the `<rect width height fill>` line. The fill is `palette.value(Role::Neutral(Neutral::Surface1))`, and the width and height go through `fmt_coord`. Add a doc line to `serialise` citing D-87.
- [x] **Step 4:** Run `cargo test`. Expected:
  - the two serialiser goldens `prd-wide-light.svg` and `grown-wide-light.svg` fail, each by the one added line;
  - `prd-wide-interactive.svg` passes;
  - everything else passes. The tail oracle compares two rasters that now share a ground.

  Re-bless with `UPDATE_GOLDEN=1 cargo test --test serialise golden`, then run `git diff --stat tests/golden`. Expected: two files, +1 line each.
- [x] **Step 5:** Run `cargo clippy --all-targets -- -D warnings`. Expected: clean. Commit: `feat(svg): the static target draws the theme's ground (D-87)`.

---

### Task 2: `emit/png.rs` — size and rasterise

**Files:**
- Create: `src/emit/png.rs`
- Modify: `src/emit/mod.rs`, `src/style.rs`, `src/diagnostics.rs`, `src/cli.rs` (the width range only), `Cargo.toml`, `clippy.toml`, `justfile`, `tests/common/fonts.rs`, `tests/diagnostics.rs`, and the tests that import `usvg`, `tiny_skia` or `fontdb`

**Interfaces:**
- Consumes: `Palette::rgb` is not needed here, since the ground is in the SVG (Task 1).
- Produces:
  - `style::MIN_EXPORT_WIDTH: u32 = 320`, `style::MAX_EXPORT_WIDTH: u32 = 10_000`, `style::MAX_PNG_PIXELS: u64 = 100_000_000`
  - `pub struct PixelSize` (private fields) with `width()`, `height()`, `clamped() -> Option<u32>` and `legible_from() -> Option<Option<u32>>`. The outer `None` means the text is legible. `Some(None)` means it is small at any width up to `MAX_EXPORT_WIDTH`. `Some(Some(w))` means `w` is the least width that keeps it legible.
  - `pub fn pixel_size(document: &Document, requested: Option<u32>) -> Result<PixelSize, Fatal>`
  - `pub fn rasterise(document: &Document, palette: &Palette, size: PixelSize) -> Result<Vec<u8>, Fatal>`
  - `pub fn usvg_options() -> usvg::Options<'static>`
  - `Fatal::Rasterise { reason: String }`, `Fatal::ImageTooLarge { width: u32, height: u32 }`
  - `RenderKind::WidthClamped { natural: u32, drawn: u32 }`, `RenderKind::PngTextSmall { width: u32, legible_from: Option<u32> }`

- [x] **Step 1: Dependencies and build settings.** In `Cargo.toml` `[dependencies]`:

  ```toml
  # PNG export (FR-55). Only `text`: no system fonts (NFR-7), no raster
  # image decoders, no svgz. usvg sees only the font database the emitter
  # builds.
  resvg = { version = "0.48", default-features = false, features = ["text"] }
  ```

  Then the rest of the build settings:
  - **Dev-dependencies:** from `[dev-dependencies]` remove `resvg`, `usvg`, `tiny-skia` and `fontdb`. If `resvg` stayed there with its defaults, feature unification would bring the system fonts back into test builds. In the tests that import them, replace `usvg::` with `resvg::usvg::`, `tiny_skia::` with `resvg::tiny_skia::`, and `fontdb::` with `resvg::usvg::fontdb::`. Find them with `rg -l 'usvg::|tiny_skia::|fontdb::' tests`.
  - **Faster test builds:** add

    ```toml
    # Rasterising in an unoptimised build is many times slower; the PNG
    # tests run on three OSes.
    [profile.dev.package.resvg]
    opt-level = 3
    [profile.dev.package.usvg]
    opt-level = 3
    [profile.dev.package.tiny-skia]
    opt-level = 3
    ```

  - **Duplicate crate:** in `clippy.toml`:

    ```toml
    # png 0.18 depends on miniz_oxide 0.8 directly and 0.9 through flate2;
    # neither is ours to unify (rg-42m).
    allowed-duplicate-crates = ["miniz_oxide"]
    ```

  - **Feature check:** in `justfile`, add a `deps-features` recipe and call it from `ci`:

    ```just
    # NFR-5, NFR-7: resvg must not pull system fonts, memmap, image decoders or svgz.
    deps-features:
        ! cargo tree -e features -i resvg | rg 'system-fonts|memmap|raster-images|svgz|fontconfig'
    ```

  Run each of these:
  - `cargo build && cargo test --no-run`. Expected: builds.
  - `just deps-features`. Expected: success, with no matches.
  - `cargo clippy --all-targets -- -D warnings`. Expected: clean.
  - `cargo deny check licenses advisories`. Expected: ok. If a crate's licence is refused, stop and report it rather than widening `deny.toml`.

  Record `ls -l target/release/radargen` after `cargo build --release`, before and after this step, for the PR body.

- [x] **Step 2: The constants.** In `src/style.rs`, near `MIN_LEGIBLE_PX`:

  ```rust
  /// FR-64: the narrowest `--width`, in pixels.
  pub const MIN_EXPORT_WIDTH: u32 = 320;
  /// FR-64: the widest `--width`, and the clamp for a default past it (D-87).
  pub const MAX_EXPORT_WIDTH: u32 = 10_000;
  /// D-87: the largest image drawn, `MAX_EXPORT_WIDTH` squared.
  pub const MAX_PNG_PIXELS: u64 = 100_000_000;
  ```

  In `src/cli.rs`, the `--width` parser becomes `clap::value_parser!(u32).range(i64::from(MIN_EXPORT_WIDTH)..=i64::from(MAX_EXPORT_WIDTH))`. `tests/cli.rs`'s width tests must still pass unchanged.

- [x] **Step 3: Stubs.** Create `src/emit/png.rs` with the module doc, the types, and `todo!()` bodies, so the tests below compile and fail on their own assertions:

  ```rust
  //! The PNG export (FR-55, FR-71, NFR-7, D-87): the wide document
  //! serialised with literal colours for one theme, over its ground,
  //! rasterised by resvg through the vendored faces alone.

  use std::sync::Arc;

  use resvg::tiny_skia::{Pixmap, Transform};
  use resvg::usvg;

  use crate::diagnostics::Fatal;
  use crate::palette::Palette;
  use crate::style::{MAX_EXPORT_WIDTH, MAX_PNG_PIXELS, WIDE_MIN_SCALE};
  use crate::svg::{Document, Target, serialise};
  use crate::text::{FAMILY, REGULAR_TTF, SEMIBOLD_TTF};

  /// The pixel size an image is drawn at (FR-57, D-87). Only
  /// [`pixel_size`] makes one, so every size has passed the area cap.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct PixelSize {
      width: u32,
      height: u32,
      clamped: Option<u32>,
      legible_from: Option<Option<u32>>,
  }

  impl PixelSize {
      /// The width in pixels.
      #[must_use]
      pub const fn width(self) -> u32 { self.width }
      /// The height, following the document's aspect, at least 1.
      #[must_use]
      pub const fn height(self) -> u32 { self.height }
      /// The document's own width, when the default was clamped to
      /// `MAX_EXPORT_WIDTH` (D-87).
      #[must_use]
      pub const fn clamped(self) -> Option<u32> { self.clamped }
      /// `None` when legend text draws at `MIN_LEGIBLE_PX` or more;
      /// otherwise the least width that keeps it so, or `None` inside
      /// when even `MAX_EXPORT_WIDTH` does not (D-87).
      #[must_use]
      pub const fn legible_from(self) -> Option<Option<u32>> { self.legible_from }
  }

  /// FR-57: `requested`, or the document's own width rounded up and
  /// clamped to `MAX_EXPORT_WIDTH` (D-87); the height in proportion.
  ///
  /// # Errors
  ///
  /// [`Fatal::ImageTooLarge`] past `MAX_PNG_PIXELS` (D-87).
  pub fn pixel_size(document: &Document, requested: Option<u32>) -> Result<PixelSize, Fatal> {
      todo!()
  }

  /// FR-55: `document` as PNG bytes, its colours `palette`'s literals
  /// (FR-56), drawn through the vendored faces alone (NFR-7).
  ///
  /// # Errors
  ///
  /// [`Fatal::Rasterise`] when resvg cannot parse the document, the pixmap
  /// cannot be made, or the image cannot be encoded.
  pub fn rasterise(document: &Document, palette: &Palette, size: PixelSize) -> Result<Vec<u8>, Fatal> {
      todo!()
  }

  /// `usvg` options whose font database holds only the vendored faces,
  /// with Inter as the default family (NFR-7), and which read no file or
  /// URL (NFR-5). Public so the test oracles rasterise exactly as the
  /// export does; not a stable API.
  #[must_use]
  pub fn usvg_options() -> usvg::Options<'static> {
      todo!()
  }
  ```

  Add `pub mod png;` to `src/emit/mod.rs`. Add the four diagnostics from Step 6 with their messages. Variants have no behaviour to stub, so they go in now, but their message tests come in Step 6.

- [x] **Step 4: Failing tests** at the bottom of `src/emit/png.rs`:

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use crate::layout::arrange::{Arrangement, Blocks, arrange};
      use crate::layout::block::fonts;
      use crate::layout::test_support::{PRD_EXAMPLE, plotted};
      use crate::palette::{Neutral, Role, Theme};
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

      fn decode(png: &[u8]) -> Pixmap {
          Pixmap::decode_png(png).expect("a PNG")
      }

      #[test]
      fn the_default_is_the_documents_own_width() {
          let size = pixel_size(&empty(1025.077, 824.0), None).expect("fits");
          assert_eq!((size.width(), size.height(), size.clamped()), (1026, 825, None));
          assert_eq!(size.legible_from(), None);
      }

      #[test]
      fn the_height_follows_a_requested_width() {
          let size = pixel_size(&empty(1000.0, 800.0), Some(1920)).expect("fits");
          assert_eq!((size.width(), size.height()), (1920, 1536));
      }

      /// D-87: a default past FR-64's ceiling is drawn at it, and says so.
      #[test]
      fn a_default_past_the_ceiling_is_clamped() {
          let size = pixel_size(&empty(18_355.334, 2_000.0), None).expect("fits");
          assert_eq!((size.width(), size.height(), size.clamped()), (MAX_EXPORT_WIDTH, 1090, Some(18_356)));
      }

      /// D-87: a width the user chose is never clamped, however wide the
      /// document.
      #[test]
      fn an_explicit_width_never_clamps() {
          let size = pixel_size(&empty(18_355.334, 2_000.0), Some(1920)).expect("fits");
          assert_eq!((size.width(), size.clamped()), (1920, None));
      }

      /// D-87: the cap is inclusive: exactly 100 megapixels is drawn.
      #[test]
      fn the_area_cap_is_inclusive() {
          assert!(pixel_size(&empty(1_000.0, 1_000.0), Some(10_000)).is_ok());
          let err = pixel_size(&empty(10_000.0, 10_001.0), Some(10_000)).expect_err("one row over");
          assert!(matches!(err, Fatal::ImageTooLarge { width: 10_000, height: 10_001 }), "{err:?}");
      }

      /// D-87: past 100 megapixels the run fails before allocating.
      #[test]
      fn pixel_size_refuses_past_the_area_cap() {
          let err = pixel_size(&empty(1_000.0, 20_000.0), Some(10_000)).expect_err("200 megapixels");
          assert!(matches!(err, Fatal::ImageTooLarge { width: 10_000, height: 200_000 }), "{err:?}");
      }

      /// FR-57: a very wide, very short document still has a row.
      #[test]
      fn a_height_never_rounds_to_zero() {
          let size = pixel_size(&empty(1_000_000.0, 10.0), Some(320)).expect("fits");
          assert_eq!(size.height(), 1);
      }

      /// D-87: at 320 the PRD example's legend draws far below 10 px; the
      /// least legible width is the document's width times `WIDE_MIN_SCALE`.
      #[test]
      fn a_narrow_width_reports_small_text() {
          let doc = empty(1025.077, 824.0);
          let size = pixel_size(&doc, Some(320)).expect("fits");
          assert_eq!(size.legible_from(), Some(Some(821)));
          let wide = empty(18_355.334, 2_000.0);
          assert_eq!(pixel_size(&wide, None).expect("fits").legible_from(), Some(None), "past the ceiling");
      }

      #[test]
      fn the_floor_width_renders() {
          let doc = prd();
          let size = pixel_size(&doc, Some(320)).expect("fits");
          let image = decode(&rasterise(&doc, &Palette::built_in(Theme::Light), size).expect("renders"));
          assert_eq!((image.width(), image.height()), (size.width(), size.height()));
      }

      /// D-87, Task 1: the corner, where nothing is drawn, is the theme's
      /// Surface1, written out here rather than read back from the palette.
      #[test]
      fn the_corner_is_the_ground() {
          let doc = prd();
          let surface = Role::Neutral(Neutral::Surface1);
          for theme in [Theme::Light, Theme::Dark] {
              let palette = Palette::built_in(theme);
              let want = u32::from_str_radix(&palette.value(surface)[1..], 16).expect("hex");
              let [_, r, g, b] = want.to_be_bytes();
              let size = pixel_size(&doc, Some(400)).expect("fits");
              let image = decode(&rasterise(&doc, &palette, size).expect("renders"));
              let corner = image.pixel(0, 0).expect("a pixel");
              assert_eq!((corner.red(), corner.green(), corner.blue(), corner.alpha()), (r, g, b, 255), "{theme:?}");
          }
      }

      /// Architecture §5: an unresolved `var()`, or a default black, would
      /// render black on the dark ground. No pixel is darker than it by
      /// more than anti-aliasing allows.
      #[test]
      fn dark_ink_is_never_darker_than_its_ground() {
          let doc = prd();
          let palette = Palette::built_in(Theme::Dark);
          let size = pixel_size(&doc, Some(400)).expect("fits");
          let image = decode(&rasterise(&doc, &palette, size).expect("renders"));
          let ground = image.pixel(0, 0).expect("a pixel");
          let luma = |p: resvg::tiny_skia::ColorU8| u32::from(p.red()) + u32::from(p.green()) + u32::from(p.blue());
          let floor = luma(ground).saturating_sub(12);
          let darkest = image.pixels().iter().map(|p| luma(p.demultiply())).min().expect("pixels");
          assert!(darkest >= floor, "a pixel at {darkest} under the ground's {}", luma(ground));
      }

      /// The cheap pin beside the render above.
      #[test]
      fn the_static_svg_has_no_custom_properties() {
          let svg = serialise(&prd(), Target::Static(&Palette::built_in(Theme::Dark)));
          assert!(!svg.contains("var("), "{svg}");
      }

      /// Every `fill` and `stroke` in the static PRD document is one of the
      /// theme's literals, or `none`: no stray `#000` or `currentColor`.
      #[test]
      fn the_static_svg_paints_only_palette_values() {
          for theme in [Theme::Light, Theme::Dark] {
              let palette = Palette::built_in(theme);
              let svg = serialise(&prd(), Target::Static(&palette));
              let allowed: Vec<&str> = palette.values().collect();
              for attr in ["fill=\"", "stroke=\""] {
                  for value in svg.split(attr).skip(1).filter_map(|rest| rest.split('"').next()) {
                      assert!(value == "none" || allowed.contains(&value), "{theme:?}: {attr}{value}\"");
                  }
              }
          }
      }

      /// The PNG drops every pointer and focus node at any depth.
      #[test]
      fn the_static_prd_document_has_no_interactive_nodes() {
          let svg = serialise(&prd(), Target::Static(&Palette::built_in(Theme::Light)));
          for name in ["data-hit", "data-marker-indicator", "data-row-indicator", "data-focus-ring", "data-selection"] {
              assert!(!svg.contains(name), "{name}");
          }
      }

      /// NFR-5: usvg, as the export configures it, reads no file.
      #[test]
      fn usvg_reads_no_file() {
          let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><image href="Cargo.toml" width="10" height="10"/></svg>"#;
          let tree = usvg::Tree::from_str(svg, &usvg_options()).expect("parses");
          assert!(!tree.root().has_children(), "the image was resolved");
      }

      /// NFR-7: exactly the two vendored faces.
      #[test]
      fn the_font_database_holds_the_two_vendored_faces() {
          let options = usvg_options();
          let mut weights: Vec<u16> = options.fontdb.faces().map(|f| {
              assert!(f.families.iter().any(|(name, _)| name == FAMILY), "{:?}", f.families);
              f.weight.0
          }).collect();
          weights.sort_unstable();
          assert_eq!(weights, [400, 600]);
      }
  }
  ```

  `palette.values()` is the palette's iterator over every literal. If it is named differently, use that. If none exists, add one in `src/palette.rs`, with a doc line. Check the exact names of the interactive-only attributes against `src/svg/serialise/` constants (`DATA_HIT`, `DATA_MARKER_INDICATOR`, …) and use the constants rather than string literals where they are public. Check `usvg::Group::has_children` against usvg 0.48's API: the assertion is that the tree holds no image node.

  `string_slice` is denied, including in tests. So in `the_corner_is_the_ground`, write `palette.value(surface).get(1..).expect("#RRGGBB")` in place of `[1..]`.

- [x] **Step 5:** Run `cargo test --lib emit::png`. Expected: every test FAILS, panicking at `not yet implemented`, except `the_static_svg_has_no_custom_properties`, `the_static_svg_paints_only_palette_values` and `the_static_prd_document_has_no_interactive_nodes`. Those three pin the serialiser (Task 1, and the existing static target) and pass already. Note that in the ledger. They are guards for this PR's guarantee, not tests of its new code.

- [x] **Step 6: The diagnostics.** In `src/diagnostics.rs`'s `Fatal`, beside `Unwritable`:

  ```rust
  /// FR-55: resvg could not parse or rasterise the document, or encode
  /// the image. Nothing at `-o` has changed (NFR-14). The reason is
  /// resvg's or the encoder's, which name positions and kinds, never the
  /// document's text.
  #[error("the PNG cannot be drawn: {reason}")]
  Rasterise {
      /// What failed.
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

  Change `UnrecognisedExtension`'s message to "the output file name must end in .html or .png".

  In `RenderKind`, after `NarrowWidened`:

  ```rust
  /// D-87: the document is wider than FR-64's ceiling and `--width` was
  /// not given, so the PNG is drawn at the ceiling.
  WidthClamped {
      /// The document's own width, rounded up.
      natural: u32,
      /// The width drawn.
      drawn: u32,
  },
  /// D-87: the PNG's legend text draws below `MIN_LEGIBLE_PX`.
  PngTextSmall {
      /// The width drawn.
      width: u32,
      /// The least width that keeps it legible, if one within FR-64 does.
      legible_from: Option<u32>,
  },
  ```

  And in `RenderKind::message`:

  ```rust
  Self::WidthClamped { natural, drawn } => format!(
      "the radar is {natural} pixels wide, past --width's limit; the PNG is drawn {drawn} wide, or pass --width to choose"
  ),
  Self::PngTextSmall { width, legible_from: Some(from) } => format!(
      "at {width} pixels wide the PNG's text is too small to read; --width {from} or more keeps it legible"
  ),
  Self::PngTextSmall { width, legible_from: None } => format!(
      "at {width} pixels wide the PNG's text is too small to read, at any --width; fewer or shorter names would help"
  ),
  ```

  In `tests/diagnostics.rs`'s message table:
  - add a row for each new variant and arm;
  - change the two `UnrecognisedExtension` rows to ".html or .png";
  - change the `FormatNotInThisVersion` row to `radar.svg`, `ext: "svg"`, `arrives_in: "1.2"`, with the text "this version writes .html and .png". Its `#[error]` changes accordingly in Task 3. Write the row now, so it fails until then.

- [x] **Step 7: The module.** Replace the stubs:

  ```rust
  pub fn pixel_size(document: &Document, requested: Option<u32>) -> Result<PixelSize, Fatal> {
      let natural = whole(document.width.ceil());
      let (width, clamped) = match requested {
          Some(width) => (width, None),
          None if natural > MAX_EXPORT_WIDTH => (MAX_EXPORT_WIDTH, Some(natural)),
          None => (natural, None),
      };
      let height = whole((f64::from(width) * document.height / document.width).round()).max(1);
      if u64::from(width) * u64::from(height) > MAX_PNG_PIXELS {
          return Err(Fatal::ImageTooLarge { width, height });
      }
      let legible_from = (f64::from(width) / document.width < WIDE_MIN_SCALE).then(|| {
          let from = whole((document.width * WIDE_MIN_SCALE).ceil());
          (from <= MAX_EXPORT_WIDTH).then_some(from)
      });
      Ok(PixelSize { width, height, clamped, legible_from })
  }

  pub fn rasterise(document: &Document, palette: &Palette, size: PixelSize) -> Result<Vec<u8>, Fatal> {
      let fail = |reason: String| Fatal::Rasterise { reason };
      let svg = serialise(document, Target::Static(palette));
      let tree = usvg::Tree::from_str(&svg, &usvg_options()).map_err(|e| fail(e.to_string()))?;
      // D-87: the markup is not needed past here, and the pixmap is the
      // largest allocation the run makes.
      drop(svg);
      let mut pixmap = Pixmap::new(size.width, size.height)
          .ok_or_else(|| fail(format!("{} by {} pixels cannot be allocated", size.width, size.height)))?;
      let scale = narrow(f64::from(size.width) / document.width);
      resvg::render(&tree, Transform::from_scale(scale, scale), &mut pixmap.as_mut());
      pixmap.encode_png().map_err(|e| fail(e.to_string()))
  }

  pub fn usvg_options() -> usvg::Options<'static> {
      let mut db = usvg::fontdb::Database::new();
      db.load_font_data(REGULAR_TTF.to_vec());
      db.load_font_data(SEMIBOLD_TTF.to_vec());
      usvg::Options {
          resources_dir: None,
          fontdb: Arc::new(db),
          font_family: FAMILY.to_string(),
          // NFR-5: no href is ever resolved, as data or as a path.
          image_href_resolver: usvg::ImageHrefResolver {
              resolve_data: Box::new(|_, _, _| None),
              resolve_string: Box::new(|_, _| None),
          },
          ..usvg::Options::default()
      }
  }

  /// A rounded, non-negative pixel count as `u32`.
  #[expect(
      clippy::cast_possible_truncation,
      clippy::cast_sign_loss,
      clippy::as_conversions,
      reason = "f64 has no TryFrom to an integer; callers pass a rounded, non-negative value; `as` saturates past u32::MAX and maps NaN to 0, which the area cap and the height floor then handle"
  )]
  const fn whole(v: f64) -> u32 {
      v as u32
  }

  /// A scale factor as resvg's `f32`.
  #[expect(
      clippy::cast_possible_truncation,
      clippy::as_conversions,
      reason = "tiny-skia transforms are f32; a scale between 0.0003 and 31 loses nothing that shows"
  )]
  const fn narrow(v: f64) -> f32 {
      v as f32
  }
  ```

  Check the closure signatures of `ImageHrefResolver`'s two fields against usvg 0.48's source (`~/.cargo/registry/src/*/usvg-0.48*/src/parser/image.rs`) and match them. Replace the body of `tests/common/fonts.rs`'s `usvg_options` with `radargen::emit::png::usvg_options()`, keeping its doc comment.

- [x] **Step 8:** Run `cargo test --lib emit::png && cargo test --test diagnostics`. Expected: PASS, except the `FormatNotInThisVersion` row, which waits for Task 3. Then run `cargo test`. Expected: PASS apart from that row. The oracle tests now share the emitter's options.

- [x] **Step 9:** Run `cargo clippy --all-targets -- -D warnings`. Expected: clean. Commit, `feat(png): size and rasterise a document (FR-55, FR-57, D-87)`, noting the pending diagnostics row.

---

### Task 3: The pipeline, the CLI, and end to end

**Files:** `src/cli.rs`, `src/pipeline.rs`, `tests/cli.rs`, `tests/run.rs`.

**Interfaces:**
- Consumes: `png::{pixel_size, rasterise}`, `PixelSize`'s accessors, `RenderKind::{WidthClamped, PngTextSmall}`.
- Produces: `Config { format: OutputFormat, palette: Palette, width: Option<u32>, .. }`.

- [x] **Step 1: Rewrite the tests that pin today's refusal.**

  In `tests/cli.rs`, replace `png_and_svg_say_not_in_this_version_rather_than_unrecognised` with:

  ```rust
  #[test]
  fn png_is_a_format_and_svg_arrives_in_1_2() {
      for path in ["r.png", "R.PNG", "r.Png"] {
          assert_eq!(format(Path::new(path)), Ok(OutputFormat::Png), "{path}");
      }
      // §14.1: SVG ships in 1.2.0; "unrecognised extension" would send the
      // reader hunting for a typo in a correct path.
      for path in ["e.svg", "e.SVG"] {
          match format(Path::new(path)) {
              Err(Fatal::FormatNotInThisVersion { path: named, arrives_in, .. }) => {
                  assert_eq!((named.as_str(), arrives_in), (path, "1.2"));
              }
              other => panic!("{path} gave {other:?}"),
          }
      }
  }
  ```

  Add `"abc"` and `"-5"` to the rejected values of the existing `--width` range test, in its own style.

  In `tests/run.rs`, rename `bad_extensions_and_v1_1_formats` to `bad_extensions_and_svg` and change it as follows. Keep its `snapshot()` check.
  - the `.txt` line expects "must end in .html or .png";
  - the second run uses `radar.svg` and expects "arrives in radargen 1.2".

- [x] **Step 2: Failing end-to-end tests** in `tests/run.rs`, using its `seeded`, `run_in`, `snapshot`, `read`, `RENDER`, `HEAD` and `PRD`:

  ```rust
  use resvg::tiny_skia::Pixmap;

  fn png(dir: &Path, name: &str) -> Pixmap {
      Pixmap::decode_png(&fs::read(dir.join(name)).expect("written")).expect("a PNG")
  }

  /// The chunk types of a PNG file, in order.
  fn chunks(bytes: &[u8]) -> Vec<[u8; 4]> {
      let mut at = 8;
      let mut out = Vec::new();
      while let Some(head) = bytes.get(at..at + 8) {
          let len = u32::from_be_bytes(head[..4].try_into().expect("4 bytes"));
          out.push(head[4..8].try_into().expect("4 bytes"));
          at += 12 + usize::try_from(len).expect("fits");
      }
      out
  }

  /// FR-55, FR-57, FR-71: a PNG of the wide arrangement at --width, its
  /// height in proportion.
  #[test]
  fn renders_a_png_at_the_width_asked() {
      let dir = seeded(&[("radar.csv", PRD)]);
      let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png", "--width", "1920"]);
      assert!(ran.ok(), "{}", ran.stderr);
      let image = png(dir.path(), "radar.png");
      assert_eq!(image.width(), 1920);
      assert!(image.height() > 1000 && image.height() < 2000, "{}", image.height());
  }

  /// SC-25: no output version, so no text chunk at all.
  #[test]
  fn the_png_has_no_text_chunks() {
      let dir = seeded(&[("radar.csv", PRD)]);
      assert!(run_in(dir.path(), &["radar.csv", "-o", "radar.png"]).ok());
      let kinds = chunks(&fs::read(dir.path().join("radar.png")).expect("written"));
      assert_eq!(kinds.first(), Some(b"IHDR"));
      for text in [b"tEXt", b"iTXt", b"zTXt"] {
          assert!(!kinds.contains(text), "{kinds:?}");
      }
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

  /// FR-56, FR-57: HTML ignores both flags: the same page, the same stderr.
  #[test]
  fn html_ignores_theme_and_width() {
      let dir = seeded(&[("radar.csv", PRD)]);
      let plain_run = run_in(dir.path(), &RENDER);
      assert!(plain_run.ok());
      let plain = read(dir.path(), "radar.html");
      let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.html", "--theme", "dark", "--width", "800"]);
      assert!(ran.ok(), "{}", ran.stderr);
      assert_eq!(read(dir.path(), "radar.html"), plain);
      assert_eq!(ran.stderr, plain_run.stderr);
  }

  /// FR-52: the extension is matched in any case.
  #[test]
  fn an_upper_case_png_extension_renders() {
      let dir = seeded(&[("radar.csv", PRD)]);
      let ran = run_in(dir.path(), &["radar.csv", "-o", "RADAR.PNG"]);
      assert!(ran.ok(), "{}", ran.stderr);
      let _ = png(dir.path(), "RADAR.PNG");
  }

  /// D-74 holds for PNG: the input under a .png name is refused, untouched.
  #[test]
  fn a_png_output_that_is_the_input_is_refused() {
      let dir = seeded(&[("data.png", PRD)]);
      let before = snapshot(dir.path());
      let ran = run_in(dir.path(), &["data.png", "-o", "data.png"]);
      assert_eq!(ran.status, 1);
      assert!(ran.stderr.contains("is the input"), "{}", ran.stderr);
      assert_eq!(snapshot(dir.path()), before);
  }

  /// NFR-14, SC-18: a PNG into a directory that is not there fails cleanly.
  #[test]
  fn a_png_into_a_missing_directory_fails() {
      let dir = seeded(&[("radar.csv", PRD)]);
      let before = snapshot(dir.path());
      let ran = run_in(dir.path(), &["radar.csv", "-o", "nowhere/radar.png"]);
      assert_eq!(ran.status, 1);
      assert_eq!(snapshot(dir.path()), before);
  }

  /// D-87, SC-18: an image past the area cap fails, writes nothing, and
  /// leaves the old file byte-identical.
  #[test]
  fn an_image_too_large_fails_and_writes_nothing() {
      // N items make the wide document at least as tall as it is wide,
      // so 10,000 wide passes 100 megapixels. N measured in Step 3.
      let rows: String = (0..N).map(|i| format!("Item {i},Tools,Adopt,d\n")).collect();
      let dir = seeded(&[("radar.csv", &format!("{HEAD}{rows}")), ("radar.png", "old")]);
      let before = snapshot(dir.path());
      let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png", "--width", "10000"]);
      assert_eq!(ran.status, 1);
      assert!(ran.stderr.contains("too large to draw"), "{}", ran.stderr);
      assert_eq!(snapshot(dir.path()), before);
  }

  /// D-87: a default width past the ceiling is clamped, with a warning,
  /// and the text-size warning with it. rg-tzz's fixture: its wide
  /// document passes 12,500 units, and it stays short.
  #[test]
  fn a_default_width_past_the_ceiling_is_clamped() {
      let csv = format!("{HEAD}A,{},Adopt,d\nB,Tools,Adopt,d\n", "W".repeat(600));
      let dir = seeded(&[("radar.csv", &csv)]);
      let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png"]);
      assert!(ran.ok(), "{}", ran.stderr);
      assert!(ran.stderr.contains("the PNG is drawn 10000 wide"), "{}", ran.stderr);
      assert!(ran.stderr.contains("too small to read, at any --width"), "{}", ran.stderr);
      assert_eq!(png(dir.path(), "radar.png").width(), 10_000);
  }

  /// D-87: at the floor the PRD example's text is too small, and the
  /// warning names a width that fixes it.
  #[test]
  fn a_narrow_png_warns_about_small_text() {
      let dir = seeded(&[("radar.csv", PRD)]);
      let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png", "--width", "320"]);
      assert!(ran.ok(), "{}", ran.stderr);
      assert!(ran.stderr.contains("keeps it legible"), "{}", ran.stderr);
      let at_own = run_in(dir.path(), &["radar.csv", "-o", "radar.png"]);
      assert!(!at_own.stderr.contains("too small"), "{}", at_own.stderr);
  }

  /// FR-95 concerns the narrow arrangement, which only HTML carries.
  #[test]
  fn a_png_gives_no_narrow_warning() {
      let csv = format!("{HEAD}A,{},Adopt,d\nB,Tools,Adopt,d\n", "W".repeat(600));
      let dir = seeded(&[("radar.csv", &csv)]);
      let html = run_in(dir.path(), &RENDER);
      assert!(html.stderr.contains("narrow arrangement is"), "{}", html.stderr);
      let png_run = run_in(dir.path(), &["radar.csv", "-o", "radar.png"]);
      assert!(!png_run.stderr.contains("narrow arrangement"), "{}", png_run.stderr);
  }

  /// NFR-7: names in scripts Inter lacks still render; the PNG is written.
  #[test]
  fn a_png_of_hostile_text_renders() {
      let csv = format!("{HEAD}Кафка,Tools,Adopt,d\n東京,Tools,Trial,d\n😀 &lt;b&gt;,Tools,Assess,d\n");
      let dir = seeded(&[("radar.csv", &csv)]);
      let ran = run_in(dir.path(), &["radar.csv", "-o", "radar.png"]);
      assert!(ran.ok(), "{}", ran.stderr);
      let _ = png(dir.path(), "radar.png");
  }
  ```

  Match `ran.status` and the D-74 message to `tests/run.rs`'s existing helpers and wording; read the existing D-74 test first. `chunks` uses slice indexing on bytes, which `string_slice` does not cover. If `clippy::indexing_slicing` is denied in tests, use `get` throughout.

- [x] **Step 3: Measure N.** Write a scratch CSV of 400 items. Run `cargo run -q -- scratch.csv -o x.html` and read the wide document's size from the radar part's `viewBox` plus the legend's. Take the least round count whose wide document is at least as tall as it is wide. Put it in the test as a `const N: usize`, with the measured aspect in its comment.

- [x] **Step 4:** Run `cargo test --test run --test cli`. Expected: FAIL. `.png` is still refused, so every PNG test fails at "arrives in radargen 1.1", and `png_is_a_format_and_svg_arrives_in_1_2` fails too. `html_ignores_theme_and_width` may already pass. If so, note in the ledger that it pins existing behaviour.

- [x] **Step 5: `cli.rs`.**
  - **`format`:** `"png" => Ok(OutputFormat::Png)`, and `"svg" => Err(FormatNotInThisVersion { arrives_in: "1.2", .. })`.
  - **The error text:** the `#[error]` on `FormatNotInThisVersion` says "this version writes .html and .png".
  - **The help text:**
    - `about`: "Render a technology radar from a CSV file to a self-contained HTML page or a PNG image."
    - `-o`: ".html or .png, in any case (SVG arrives in version 1.2)".
    - `--theme` and `--width`: "for PNG (and SVG, from version 1.2)".
  - **Docs:** update the `OutputFormat::Png` and `OutputFormat::Svg` docs and the `format()` rustdoc. Say 1.2, not v1.1.
  - **Keep passing:** `tests/cli.rs`'s test that keeps requirement numbers out of the help.

- [x] **Step 6: `pipeline.rs`.**
  - `Config` gains `format: OutputFormat`, `palette: Palette`, and `width: Option<u32>`. Its doc cites architecture §7: the palette is resolved here, so PR 18's `--palette` file is a tier-one error.
  - `config()` keeps `format(output)?`'s value, sets `palette: Palette::built_in(cli.theme)`, and copies `cli.width`.
  - `render` becomes:

  ```rust
  /// The order, the render warnings (FR-88, FR-89, FR-95, D-87), the blocks
  /// built once, and the output: the page around both arrangements
  /// (FR-68), or the wide arrangement as a PNG (FR-71).
  fn render(catalogue: &Catalogue, config: &Config, fonts: &Fonts, warnings: &mut Warnings) -> Result<Vec<u8>, Fatal> {
      let plotted = resolve(catalogue);
      let blocks = Blocks::new(catalogue, &plotted, config.title.as_deref(), config.subtitle.as_deref(), fonts);
      warn_crowding(catalogue, &plotted, blocks.radar.growth(), warnings);
      match config.format {
          OutputFormat::Html => {
              // FR-95 concerns the narrow arrangement, which only HTML carries.
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
          OutputFormat::Png => {
              let wide = arrange(Arrangement::Wide, &blocks);
              let size = png::pixel_size(&wide, config.width)?;
              if let Some(natural) = size.clamped() {
                  warnings.push(Warning::Render(RenderKind::WidthClamped { natural, drawn: size.width() }));
              }
              if let Some(legible_from) = size.legible_from() {
                  warnings.push(Warning::Render(RenderKind::PngTextSmall { width: size.width(), legible_from }));
              }
              png::rasterise(&wide, &config.palette, size)
          }
          // `format()` refuses .svg until PR 17, so this arm is not reached;
          // it fails loudly rather than write PNG bytes to a .svg file.
          OutputFormat::Svg => Err(Fatal::FormatNotInThisVersion {
              path: config.output.display().to_string(),
              ext: "svg".into(),
              arrives_in: "1.2",
          }),
      }
  }
  ```

  `steps` becomes `let bytes = render(...)?; Ok(write_atomic(&config.output, &bytes)?)`. Match `Warnings`' real push method and `Warning`'s real variant name, and read them first.

- [x] **Step 7:** Run `cargo test`. Expected: PASS, including Task 2's pending diagnostics row.
- [x] **Step 8:** Run `cargo clippy --all-targets -- -D warnings`. Expected: clean. Commit: `feat(png): -o radar.png writes the wide arrangement; --theme and --width take effect`.

---

### Task 4: Goldens and determinism

**Files:** `tests/serialise/golden.rs`, `tests/golden/png/`, `tests/determinism.rs`.

- [x] **Step 1: `golden_png`** in `tests/serialise/golden.rs`, beside `golden`:

  ```rust
  /// D-87: per channel, the most a pixel may differ across platforms.
  const PNG_CHANNEL_TOLERANCE: u8 = 2;
  /// D-87: the share of pixels that may differ at all, in thousandths.
  const PNG_DIFFERING_PER_MILLE: usize = 1;

  /// D-40, D-87: compares `got`'s decoded pixels with
  /// `tests/golden/png/{name}`'s, within D-87's tolerance. With
  /// `UPDATE_GOLDEN=1` (refused under CI) it writes the file and fails, so
  /// a blessing run is never green. On a mismatch it writes the image and
  /// a diff under `target/png-golden/`.
  pub fn golden_png(name: &str, got: &[u8]) {
      let root = Path::new(env!("CARGO_MANIFEST_DIR"));
      let path = root.join("tests/golden/png").join(name);
      if std::env::var_os("UPDATE_GOLDEN").is_some() {
          assert!(std::env::var_os("CI").is_none(), "UPDATE_GOLDEN is refused under CI");
          std::fs::create_dir_all(path.parent().expect("a parent")).expect("creates");
          std::fs::write(&path, got).expect("writes");
          panic!("blessed {}; unset UPDATE_GOLDEN and re-run", path.display());
      }
      let want = std::fs::read(&path)
          .unwrap_or_else(|_| panic!("{} is missing: run with UPDATE_GOLDEN=1, then look at it", path.display()));
      let (want, got_image) = (decode(&want), decode(got));
      assert_eq!((want.width(), want.height()), (got_image.width(), got_image.height()), "{name}: size");
      let mut diff = got_image.clone();
      let mut differing = 0;
      for ((a, b), d) in want.pixels().iter().zip(got_image.pixels()).zip(diff.pixels_mut()) {
          let (a, b) = (a.demultiply(), b.demultiply());
          let worst = [a.red().abs_diff(b.red()), a.green().abs_diff(b.green()), a.blue().abs_diff(b.blue()), a.alpha().abs_diff(b.alpha())]
              .into_iter().max().unwrap_or(0);
          let over = worst > PNG_CHANNEL_TOLERANCE;
          differing += usize::from(a != b);
          *d = if over { resvg::tiny_skia::PremultipliedColorU8::from_rgba(255, 0, 255, 255).expect("opaque") } else { *d };
          assert!(!over, "{name}: a pixel differs by {worst}; see target/png-golden/");
      }
      let allowed = want.pixels().len() * PNG_DIFFERING_PER_MILLE / 1000;
      if differing > allowed {
          let out = root.join("target/png-golden");
          std::fs::create_dir_all(&out).expect("creates");
          std::fs::write(out.join(name), got).expect("writes");
          diff.save_png(out.join(format!("diff-{name}"))).expect("writes");
          panic!("{name}: {differing} pixels differ, past {allowed}; see {}", out.display());
      }
  }

  fn decode(png: &[u8]) -> resvg::tiny_skia::Pixmap {
      resvg::tiny_skia::Pixmap::decode_png(png).expect("a PNG")
  }
  ```

  The early `assert!(!over…)` must write the files first, as the count branch does. Factor the "write `got` and `diff`, then panic" step into one helper both branches call. The loop above is the shape, not the final form.

  And the test:

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

- [x] **Step 2:** Run `UPDATE_GOLDEN=1 cargo test --test serialise png`, then `cargo test --test serialise png`. Expected: blessed, then PASS. **Open both PNGs and look at them.** Check that:
  - the title, radar, key and legend are all present (SC-11);
  - the dark ground is dark;
  - no text is black on dark;
  - there are no hover, focus or selection marks.

  Say what you saw in the commit message. Both files are attached to the PR body (Task 5).

- [x] **Step 3: Determinism across processes.** In `tests/determinism.rs`, following its existing two-process test, render the PRD example to `.png` in two processes. Assert that their decoded pixels are **exactly** equal, using `Pixmap::decode_png` (NFR-4, D-40: pixels, not bytes, and on one platform no tolerance). The test must also assert that the static SVG strings both processes rasterised are equal. Get them by serialising in-process from each run's inputs, or by having the test run both through the library in two child processes, whichever the file's existing pattern supports. When the pixels differ, that says whether the cause is upstream of resvg.

- [x] **Step 4:** Run `cargo test`. Expected: PASS. Commit: `test(png): goldens by decoded pixels within D-87's tolerance, and determinism across processes`.

---

### Task 5: Docs, release

**Files:** `README.md`, `docs/a11y/manual-checks.md`, `docs/design/architecture.md`, `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`.

- [x] **Step 1: README.**
  - **Description and usage:** add PNG to both.
  - **A "PNG" subsection** covering:
    - the flags: `-o radar.png`, `--theme light|dark`, and `--width` from 320 to 10000, with the height following;
    - what is drawn: the wide arrangement, title, radar, key and the full legend, on the theme's ground;
    - the limits: a default wider than 10,000 is clamped, a narrow `--width` warns when text gets too small, and an image past 100 megapixels is refused;
    - long legends: they make a tall image that shrinks on a slide; a larger `--width`, or splitting the CSV, helps;
    - **scripts:** only Latin, Greek and Cyrillic (what Inter covers) render, and other scripts and emoji appear as boxes. Check the claim against Inter's coverage before writing it;
    - **accessibility:** an image has no text alternative. Write alt text where the PNG is placed, and link the HTML page or the CSV as the long description.
  - **SVG:** say it arrives in 1.2.
- [x] **Step 2: `docs/a11y/manual-checks.md`.** Add a PNG section with these checks:
  - both themes;
  - title, key and legend present;
  - no hover, focus or selection marks;
  - no missing-glyph boxes for the fixture's names;
  - text readable at the default width.
- [x] **Step 3: Architecture.**
  - §3: mark the `src/emit/png.rs` row as present, not v1.1, and note that `usvg_options` is shared with the oracle tests. The `src/emit/svg.rs` row says 1.2.
  - §5: the static target draws the ground (D-87).
  - §9 step 7: say 1.2.
  - Find any other "v1.1" with `rg -n 'v1\.1' docs/design/architecture.md src`.
- [x] **Step 4: Crumbs.**
  - Create a new crumb: `crumbs create 'Warn when a PNG draws a glyph Inter lacks (tofu)' --type idea --tags png,a11y --phase v1.x`, with a message citing the accessibility review and FR-84's shaping.
  - Append a note to `rg-m2l`: `crumbs append rg-m2l 'PR 16: the ground is in the static serialiser (D-87); SVG needs only FR-57s height rule. Re-bless the HTML goldens for radargen 1.2.'`
- [x] **Step 5:** Run `just ci`. Expected: green, browser checks and `deps-features` included.
- [x] **Step 6: Release.**
  - **Version:** set `version = "1.1.0"` and `description` "… to a self-contained HTML page or a PNG image", then run `cargo update -p radargen`.
  - **Goldens:** re-bless the HTML goldens. The diff must be only the generator line, with `radargen 1.0` becoming `radargen 1.1`.
  - **Changelog:** run `git-cliff --tag v1.1.0 --output CHANGELOG.md`.
  - **Close the crumb:** `crumbs close rg-42m`.
  - **Commit:** `chore(release): 1.1.0`.
  - **PR body** includes:
    - the release binary's size before and after (Task 2 Step 1);
    - both golden PNGs attached;
    - GitHub Actions blocked by billing (`rg-3wv`);
    - Copilot out of credits, with the Opus whole-branch review filling that seat.
  - **After merge:**
    - check `git tag --list v1.1.0` is empty;
    - tag `v1.1.0` on the merge commit and push the tag;
    - run `just publish`;
    - run `just releasea` to install.

---

## Self-review

- **Spec coverage:**
  - FR-52: Task 3.
  - FR-55 and FR-71: Tasks 2–3.
  - FR-56 and FR-57: Tasks 1–3.
  - FR-64: Task 2, the single constants plus the clamp.
  - NFR-4 and D-40: Task 4.
  - NFR-5 and NFR-7: Task 2, the features, `usvg_options`, `usvg_reads_no_file` and the font database test.
  - NFR-14 and SC-18: Task 3.
  - SC-7: one `Blocks` and `arrange(Wide)`, pinned against the HTML's parts by `the_wide_parts_tile_the_wide_document` (`src/layout/arrange.rs`).
  - SC-11: the ground (Task 1), and the images looked at and attached (Task 4).
  - SC-25: `the_png_has_no_text_chunks`.
  - D-87: Tasks 0–4.
- **No placeholders.** Task 3's `N` is the one figure found by measurement, and Step 3 says how to find it. Three API names are to be checked against the source, and each step says where: `palette.values()`, the `ImageHrefResolver` closure signatures, and `Warnings`' push.
- **Type consistency:** these are defined once and used with the same names and types throughout:
  - `PixelSize` and its accessors, `pixel_size`, `rasterise`, `usvg_options`;
  - `MIN_EXPORT_WIDTH`, `MAX_EXPORT_WIDTH`, `MAX_PNG_PIXELS`;
  - `Palette::rgb`;
  - `RenderKind::{WidthClamped { natural, drawn }, PngTextSmall { width, legible_from }}`;
  - `Config { format, palette, width }`.
- **Review Focus:** every line names its pinning test, and each test is in the task that owns the code.
