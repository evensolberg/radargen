---
tags:
  - plan
  - font
  - text
aliases: []
doc_id: RG-001
doc_name: pr-01-font-inter
crumb_id: rg-osr
document_title: "PR 01 — Swap the stand-in font for Inter: Implementation Plan"
created_date: 2026-10-09
synopsis: "Swap the stand-in Libertinus font for FR-85's Inter Regular and SemiBold, Latin-subset and vendored, with a weight parameter on text measurement."
status: draft
type: Plan
revision: "1.3"
review_date: 2026-10-09
reviewed_by:
  - rust-engineer
  - qa-expert
  - architect-reviewer
  - ui-designer
  - ux-researcher
completed_date:
comments:
---
# PR 01 — Swap the stand-in font for Inter: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Crumb:** `rg-osr` · **Phase:** v1.0 · **Blocks:** `rg-bkm` (PR 04), `rg-y1t` (PR 09), `rg-svr` (PR 10), `rg-67j` (PR 11)

**Goal:** Replace the spike's Libertinus Sans with FR-85's Inter (Regular 400 and SemiBold 600, Latin subset, TTF and WOFF2 from one source in one step), and give `text.rs` a weight parameter over both faces, loaded once.

**Architecture:** The fonts are vendored pre-subset under `assets/` and compiled in with `include_bytes!`. A checked-in script records how they were made, but nothing runs it at build time (FR-85: "subset once and vendored"). `text.rs` gains `Weight` and `Fonts`, which hold one `FontMetrics` per face. Measurement stays `rustybuzz`-shaped (FR-84). Every existing oracle test is re-pointed at Inter and extended to both weights.

**Tech Stack:** Rust 2024; `rustybuzz` 0.20, `ttf-parser` 0.25; dev: `resvg`/`usvg` 0.45, `fontdb` 0.23, `tiny-skia`; subsetting with `fonttools` `pyftsubset` run via `uvx`.

**Spec:** `docs/product/radargen-prd.md` v0.38 — FR-84, FR-85, NFR-5, NFR-7, R-7, D-43. Design: `docs/design/architecture.md` §3 (the `text.rs` and `assets/` rows plus the paragraphs below them) and §9 (the font swap lands before step 4).

## Global Constraints

- Inter, **two static instances**: Regular (400) and SemiBold (600). No variable font (D-43).
- **Latin subset, vendored pre-subset.** No build-time subsetting (FR-85, NFR-4).
- TTF and WOFF2 for each face come **from one source in one step** (architecture §3).
- OFL-1.1 licence text ships beside the font files (FR-85, NFR-18).
- Measurement goes through `rustybuzz`. Never sum `hmtx` advances (FR-84).
- Letter-spacing is added by us, as `n-1` gaps (FR-84).
- `cargo clippy --all-targets -- -D warnings` is clean. `Cargo.toml`'s `[lints]` denies `clippy::pedantic` and forbids `unsafe_code`, so that one command enforces both.
- No `#[allow]` without an inline reason.
- Commits: `type(scope): description`, ending with the `Co-Authored-By` trailer from the session. Run `git-mit es` first if `/opt/homebrew/bin/git-mit` exists.

## Review Focus

1. **A non-Latin character in a name** (R-7, A-4). The subset has no glyph for it, so it renders as tofu. Measuring it must still return a finite, positive width and must not panic. *Test: Task 2, `measuring_text_outside_the_subset_does_not_panic`.*
2. **The SemiBold face resolving to the wrong family in `fontdb`.** `pyftsubset` drops name IDs 16/17 by default. SemiBold would then register as family "Inter SemiBold", and `font-family="Inter" font-weight="600"` would silently fall back to Regular in `resvg`. *Test: Task 1, `both_faces_register_as_family_inter_with_their_weights`.*
3. **WOFF2 and TTF drifting apart.** If someone re-subsets one encoding but not the other, the HTML paints with a face the layout didn't measure. *Guard: Task 1's script writes all four files in one run. Task 1's test checks each of the four files still matches the SHA-256 that run recorded in `assets/FONTS.sha256`, so a hand-edited or separately regenerated asset fails.*
4. **Kerning silently lost by the subsetter.** If the `kern` feature or GPOS were dropped, measurement would still agree with `resvg` (both would lack it), but the face would differ from upstream Inter. *Test: Task 3 keeps the "kerned pairs are not the raw advance sum" regression guard, now against Inter.*
5. **Typographic punctuation the key and legend use.** The em dash `—`, `·` and curly quotes must be in the subset. *Test: Task 1, `subset_covers_printable_ascii_and_the_punctuation_we_emit`.*

## Decisions taken in this plan (flag on review)

- **Inter v4.1** from the upstream release zip, using the static TTFs under `extras/ttf/`. Task 1 Step 1 checks that the files are there.
- **Subset range:** Google Fonts' `latin` range, which is the conventional meaning of "Latin subset". It's listed verbatim in the script.
- **`--layout-features='*'`, `--name-IDs='*'`, `--no-hinting`.** Keeping every feature and name costs a few kB, and keeps kerning and the family name intact. Dropping hinting doesn't change advances: `rustybuzz` and `resvg` are unhinted.
- **No version bump.** The binary still exits 1 without rendering, so nothing a user runs changes.

---

### Task 1: Vendor the Inter subset

**Files:**
- Create: `scripts/subset-inter.sh`
- Create: `assets/Inter-Regular.ttf`, `assets/Inter-SemiBold.ttf`, `assets/Inter-Regular.woff2`, `assets/Inter-SemiBold.woff2` (generated)
- Create: `assets/FONTS.sha256` (generated)
- Replace: `assets/OFL.txt` with Inter's `LICENSE.txt`
- Test: `tests/font_assets.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: the four asset files at the paths above. Task 2 `include_bytes!`s the two TTFs, and Task 3 reads one WOFF2.

- [ ] **Step 1: Fetch the source and confirm its layout**

```bash
mkdir -p /tmp/inter && cd /tmp/inter
curl -fsSLO https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip
unzip -o -q Inter-4.1.zip
ls extras/ttf/Inter-Regular.ttf extras/ttf/Inter-SemiBold.ttf LICENSE.txt
uvx --from 'fonttools[woff]' python -c 'import fontTools, brotli; print(fontTools.version)'
```

Expected: all three paths list, and a fonttools version prints. Write that version into `FONTTOOLS_VERSION` in Step 3. If the zip's layout differs, stop and report rather than guessing at another file.

- [ ] **Step 2: Write the failing asset test**

`tests/font_assets.rs`:

```rust
//! FR-85: the vendored faces are Inter Regular and SemiBold, Latin-subset,
//! and each TTF/WOFF2 pair came out of one subsetting run.

const REGULAR: &[u8] = include_bytes!("../assets/Inter-Regular.ttf");
const SEMIBOLD: &[u8] = include_bytes!("../assets/Inter-SemiBold.ttf");

/// `pyftsubset` drops name IDs 16/17 unless told otherwise, and SemiBold then
/// registers as family "Inter SemiBold" — `font-weight="600"` on family
/// "Inter" would silently fall back to Regular in resvg.
#[test]
fn both_faces_register_as_family_inter_with_their_weights() {
    let mut db = fontdb::Database::new();
    db.load_font_data(REGULAR.to_vec());
    db.load_font_data(SEMIBOLD.to_vec());
    let mut seen: Vec<(String, u16)> = db
        .faces()
        .map(|f| (f.families[0].0.clone(), f.weight.0))
        .collect();
    seen.sort();
    assert_eq!(seen, vec![("Inter".to_string(), 400), ("Inter".to_string(), 600)]);
}

#[test]
fn subset_covers_printable_ascii_and_the_punctuation_we_emit() {
    for (name, data) in [("Regular", REGULAR), ("SemiBold", SEMIBOLD)] {
        let face = ttf_parser::Face::parse(data, 0).expect("face parses");
        let missing: Vec<char> = (' '..='~')
            .chain(['—', '–', '·', '‘', '’', '“', '”', '…', 'é', 'ø', 'ü'])
            .filter(|c| face.glyph_index(*c).is_none())
            .collect();
        assert!(missing.is_empty(), "{name} lacks {missing:?}");
    }
}

#[test]
fn subset_is_latin_only() {
    // A full Inter carries Cyrillic and Greek; the subset must not (FR-85, R-7).
    for (name, data) in [("Regular", REGULAR), ("SemiBold", SEMIBOLD)] {
        let face = ttf_parser::Face::parse(data, 0).expect("face parses");
        assert!(face.glyph_index('Ж').is_none(), "{name}: Cyrillic survived the subset");
        assert!(face.glyph_index('λ').is_none(), "{name}: Greek survived the subset");
    }
}

/// Architecture §3: the two encodings must stay one face. The script records
/// every output's digest in one run; a file edited or regenerated alone fails.
#[test]
fn assets_match_the_digests_the_subsetting_run_recorded() {
    let manifest = include_str!("../assets/FONTS.sha256");
    for line in manifest.lines().filter(|l| !l.trim().is_empty()) {
        let (digest, file) = line.split_once("  ").expect("sha256sum format");
        let path = format!("{}/assets/{file}", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(sha256_hex(&bytes), digest, "{file} changed outside the subsetting run");
    }
    let mut files: Vec<&str> = manifest
        .lines()
        .filter_map(|l| l.split_once("  ").map(|(_, f)| f))
        .collect();
    files.sort_unstable();
    assert_eq!(
        files,
        ["Inter-Regular.ttf", "Inter-Regular.woff2", "Inter-SemiBold.ttf", "Inter-SemiBold.woff2"]
    );
}

/// NFR-5 inlines both WOFF2 files, yet nothing else decodes either:
/// `resvg_ignores_an_embedded_font_face` passes on garbage bytes too.
#[test]
fn woff2_files_are_woff2() {
    for face in ["Regular", "SemiBold"] {
        let path = format!("{}/assets/Inter-{face}.woff2", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(bytes.get(..4), Some(&b"wOF2"[..]), "{path}");
    }
}

/// FR-85 / NFR-18: the licence beside the fonts is Inter's, not the stand-in's.
#[test]
fn the_shipped_licence_is_inters() {
    let ofl = include_str!("../assets/OFL.txt");
    assert!(ofl.contains("SIL OPEN FONT LICENSE Version 1.1"), "not OFL-1.1");
    assert!(ofl.contains("Inter Project Authors"), "OFL.txt is not Inter's licence");
    assert!(!ofl.contains("Libertinus"), "the stand-in's licence is still in place");
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    sha2::Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}
```

Add the dev-dependency: `cargo add --dev sha2`. It's MIT OR Apache-2.0, so §1's permissive-tree rule holds.

- [ ] **Step 3: Run it to see it fail**

Run: `cargo test --test font_assets`
Expected: a compile error, `couldn't read .../assets/Inter-Regular.ttf`.

- [ ] **Step 4: Write the subsetting script**

`scripts/subset-inter.sh`:

```bash
#!/usr/bin/env bash
# FR-85: subset Inter to Latin, once, and vendor the result. Not run by the
# build — a build step is one more thing that can differ between machines
# (NFR-4). Re-run only to change the subset, and commit all outputs together.
#
# Usage: scripts/subset-inter.sh /path/to/unzipped/Inter-4.1
set -euo pipefail

SRC="${1:?path to the unzipped Inter-4.1 release}"
OUT="$(cd "$(dirname "$0")/.." && pwd)/assets"
FONTTOOLS_VERSION="<version printed in Step 1>"   # replace before committing

# Google Fonts' "latin" range — the conventional meaning of a Latin subset. It
# lists U+FFFD, but Inter has no glyph for it, so pyftsubset keeps nothing
# there; PRD D-55 depends on that.
UNICODES="U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD"

subset() { # $1 = face, $2 = flavor ("" for TTF)
  local ext=ttf flavor=()
  if [[ -n "$2" ]]; then ext=$2; flavor=(--flavor="$2"); fi
  uvx --from "fonttools[woff]==${FONTTOOLS_VERSION}" pyftsubset \
    "$SRC/extras/ttf/Inter-$1.ttf" \
    --unicodes="$UNICODES" \
    --layout-features='*' \
    --name-IDs='*' \
    --no-hinting \
    ${flavor[@]+"${flavor[@]}"} \
    --output-file="$OUT/Inter-$1.$ext"
}

for face in Regular SemiBold; do
  subset "$face" ""
  subset "$face" woff2
done
cp "$SRC/LICENSE.txt" "$OUT/OFL.txt"

( cd "$OUT" && shasum -a 256 Inter-Regular.ttf Inter-SemiBold.ttf \
    Inter-Regular.woff2 Inter-SemiBold.woff2 > FONTS.sha256 )
ls -l "$OUT"/Inter-*
```

Before running it, replace the `FONTTOOLS_VERSION` value with the exact version Step 1 printed. The line must not be committed with the angle-bracket text.

- [ ] **Step 5: Run the script**

```bash
chmod +x scripts/subset-inter.sh
scripts/subset-inter.sh /tmp/inter
```

Expected: four `Inter-*` files listed. Note the four sizes; Task 4 records them (R-7).

- [ ] **Step 6: Run the asset test**

Run: `cargo test --test font_assets`
Expected: 6 passed. If `both_faces_register_as_family_inter_with_their_weights` fails with `"Inter SemiBold"`, the `--name-IDs='*'` flag didn't take. Fix the script; don't loosen the test.

- [ ] **Step 7: Commit**

```bash
git add scripts/subset-inter.sh assets/Inter-* assets/FONTS.sha256 assets/OFL.txt tests/font_assets.rs Cargo.toml Cargo.lock
git commit -m "feat(font): vendor Inter Regular and SemiBold, Latin subset (FR-85)"
```

Libertinus stays in place for now, because Task 3 still reads it. That keeps the commit green.

---

### Task 2: `Weight` and `Fonts` in `text.rs`

**Files:**
- Modify: `src/text.rs` (add items above `FontMetrics`; `FontMetrics` and `wrap` unchanged)
- Test: `tests/fonts.rs`

**Interfaces:**
- Consumes: `assets/Inter-Regular.ttf` and `assets/Inter-SemiBold.ttf` (Task 1).
- Produces, all in `radargen::text`:
  - `pub const FAMILY: &str = "Inter";`
  - `pub const REGULAR_TTF: &[u8]`, `pub const SEMIBOLD_TTF: &[u8]`
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Weight { Regular, SemiBold }`, plus `impl Weight { pub const fn css(self) -> u16 }`, which returns 400 or 600
  - `pub struct Fonts<'a>`, with `impl Fonts<'static> { pub fn embedded() -> Result<Self, ttf_parser::FaceParsingError> }` and `impl<'a> Fonts<'a> { pub fn face(&self, w: Weight) -> &FontMetrics<'a> }`

  PR 04's `style::TextStyle` holds a `Weight`. Layout blocks receive `&Fonts` and call `fonts.face(style.weight).measure(..)`.

- [ ] **Step 1: Write the failing tests**

`tests/fonts.rs`:

```rust
//! FR-85 / architecture §3: a measurement is (face, size, letter-spacing),
//! never (size, letter-spacing). Both faces load once, from the binary.

use radargen::text::{Fonts, Weight};

#[test]
fn both_embedded_faces_load() {
    Fonts::embedded().expect("embedded faces parse");
}

#[test]
fn semibold_is_wider_than_regular() {
    let f = Fonts::embedded().expect("faces");
    let regular = f.face(Weight::Regular).measure("Kubernetes", 12.5, 0.0);
    let semibold = f.face(Weight::SemiBold).measure("Kubernetes", 12.5, 0.0);
    assert!(semibold > regular, "SemiBold {semibold} should exceed Regular {regular}");
}

#[test]
fn css_weights_match_fr_85() {
    assert_eq!(Weight::Regular.css(), 400);
    assert_eq!(Weight::SemiBold.css(), 600);
}

/// R-7: a non-Latin name renders as tofu, which is accepted (A-4) — but it
/// must not take the measurement down with it.
#[test]
fn measuring_text_outside_the_subset_does_not_panic() {
    let f = Fonts::embedded().expect("faces");
    for w in [Weight::Regular, Weight::SemiBold] {
        let width = f.face(w).measure("Kubernetes 日本語 Ж", 12.5, 0.0);
        assert!(width.is_finite() && width > 0.0, "{w:?}: {width}");
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --test fonts`
Expected: a compile error, `unresolved imports radargen::text::Fonts, radargen::text::Weight`.

- [ ] **Step 3: Implement**

Add to `src/text.rs`, after the `use` line. Replace the module doc's first line with `//! Text measurement and FR-79 wrapping, over FR-85's two faces.` and keep the rest of the doc comment:

```rust
/// FR-85's family name, as `fontdb` registers both vendored faces.
pub const FAMILY: &str = "Inter";

/// Inter Regular, Latin subset (`scripts/subset-inter.sh`).
pub const REGULAR_TTF: &[u8] = include_bytes!("../assets/Inter-Regular.ttf");
/// Inter `SemiBold`, Latin subset, from the same run as [`REGULAR_TTF`].
pub const SEMIBOLD_TTF: &[u8] = include_bytes!("../assets/Inter-SemiBold.ttf");

/// FR-85 pins exactly two weights; FR-35 separates the legend's levels with
/// them plus size, letter-spacing and indentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    Regular,
    SemiBold,
}

impl Weight {
    /// The CSS `font-weight` the emitters write for this face.
    #[must_use]
    pub const fn css(self) -> u16 {
        match self {
            Self::Regular => 400,
            Self::SemiBold => 600,
        }
    }
}

/// Both faces, parsed once at startup and passed down — every `Block::size`
/// measures, and re-parsing per call would put a font parse in the inner loop
/// of layout (architecture §3).
pub struct Fonts<'a> {
    regular: FontMetrics<'a>,
    semibold: FontMetrics<'a>,
}

impl Fonts<'static> {
    /// The faces compiled into the binary. A parse failure is NFR-15's
    /// "font load failure" and is fatal to the caller.
    ///
    /// # Errors
    ///
    /// A vendored face that `ttf-parser` cannot parse.
    pub fn embedded() -> Result<Self, ttf_parser::FaceParsingError> {
        Ok(Self {
            regular: FontMetrics::load(REGULAR_TTF)?,
            semibold: FontMetrics::load(SEMIBOLD_TTF)?,
        })
    }
}

impl<'a> Fonts<'a> {
    #[must_use]
    pub fn face(&self, w: Weight) -> &FontMetrics<'a> {
        match w {
            Weight::Regular => &self.regular,
            Weight::SemiBold => &self.semibold,
        }
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --test fonts && cargo clippy --all-targets -- -D warnings`
Expected: 4 passed, clippy clean.

- [ ] **Step 5: Commit**

```bash
git add src/text.rs tests/fonts.rs
git commit -m "feat(text): Weight and Fonts — measure over both FR-85 faces"
```

---

### Task 3: Point the oracle tests at Inter, both weights; delete Libertinus

**Files:**
- Modify: `tests/measure.rs`, `tests/wrap.rs`, `tests/agrees_with_resvg.rs`, `tests/rasterises_within_measurement.rs`, `tests/outlines_match_text.rs`, `examples/report.rs`
- Delete: `assets/LibertinusSans-Regular.ttf`, `assets/LibertinusSans-Regular.woff2`

**Interfaces:**
- Consumes: `radargen::text::{FAMILY, REGULAR_TTF, SEMIBOLD_TTF, Fonts, Weight}` (Task 2).
- Produces: nothing new. The tests are the deliverable.

- [ ] **Step 1: Delete the stand-in so every stale reference fails**

```bash
git rm assets/LibertinusSans-Regular.ttf assets/LibertinusSans-Regular.woff2
cargo test --no-run 2>&1 | grep -c Libertinus
```

Expected: a non-zero count of `couldn't read ... LibertinusSans` errors. These are the failing tests this task fixes.

- [ ] **Step 2: Re-point `measure.rs` and `wrap.rs`**

In both files, and in `examples/report.rs` (which `cargo test` and `clippy --all-targets` also build), replace the `const FONT ... include_bytes!` line with:

```rust
use radargen::text::REGULAR_TTF as FONT;
```

In `measure.rs`, replace `kerned_pairs_are_not_the_raw_advance_sum` with the version below. The old one quoted a Libertinus-specific number for "Terraform", and Inter kerns different pairs:

```rust
/// Regression guard for the spike's central finding: measurement must go
/// through the shaper, because summing raw `hmtx` advances silently drops GPOS
/// kerning and diverges from what resvg rasterises (FR-84, D-42). Also guards
/// the subsetter: if GPOS kerning were dropped, this would stop diverging.
#[test]
fn kerned_pairs_are_not_the_raw_advance_sum() {
    let face = ttf_parser::Face::parse(FONT, 0).expect("face parses");
    let upem = f64::from(face.units_per_em());
    let raw_sum = |text: &str| -> f64 {
        text.chars()
            .filter_map(|c| face.glyph_index(c))
            .filter_map(|g| face.glyph_hor_advance(g))
            .map(f64::from)
            .sum::<f64>()
            * 12.5
            / upem
    };
    let m = metrics();
    let worst = ["Terraform", "AVATAR", "Today", "WAVE", "Tycho", "Yoga"]
        .iter()
        .map(|t| (m.measure(t, 12.5, 0.0) - raw_sum(t)).abs())
        .fold(0.0_f64, f64::max);
    assert!(worst > 0.5, "shaping applied no kerning anywhere in the corpus: worst {worst}");
}
```

- [ ] **Step 3: Extend `agrees_with_resvg.rs` to both weights**

Replace everything from the `use radargen::text::FontMetrics;` line through the end of `fn usvg_advance` with:

```rust
use radargen::text::{Fonts, Weight, FAMILY, REGULAR_TTF, SEMIBOLD_TTF};

const SIZE: f64 = 12.5;

fn options() -> usvg::Options<'static> {
    let mut db = fontdb::Database::new();
    db.load_font_data(REGULAR_TTF.to_vec());
    db.load_font_data(SEMIBOLD_TTF.to_vec());
    usvg::Options {
        fontdb: Arc::new(db),
        font_family: FAMILY.to_string(),
        ..usvg::Options::default()
    }
}

/// Right edge of the laid-out advance for `text`, laid out by usvg at x=0.
fn usvg_advance(text: &str, weight: Weight, opt: &usvg::Options) -> f64 {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="2000" height="200">
             <text x="0" y="100" font-family="{FAMILY}" font-weight="{}" font-size="{SIZE}">{text}</text>
           </svg>"#,
        weight.css()
    );
    let tree = usvg::Tree::from_str(&svg, opt).expect("usvg parses");
    f64::from(tree.root().abs_bounding_box().right())
}
```

Then replace the test with:

```rust
#[test]
fn our_advance_matches_usvg_layout_for_both_weights() {
    let opt = options();
    let fonts = Fonts::embedded().expect("faces");
    let mut worst = 0.0_f64;
    for weight in [Weight::Regular, Weight::SemiBold] {
        for word in [
            "Terraform", "Kubernetes", "Ansible", "Pulumi",
            "Trunk-Based", "Development", "Infrastructure",
            "Internationalisation", "WWWWWWWW", "illlliii",
        ] {
            let ours = fonts.face(weight).measure(word, SIZE, 0.0);
            let theirs = usvg_advance(word, weight, &opt);
            let delta = (ours - theirs).abs();
            worst = worst.max(delta);
            eprintln!("{weight:?} {word:22} ours {ours:8.4}  usvg {theirs:8.4}  delta {delta:+.4}");
        }
    }
    assert!(worst < 0.01, "worst delta {worst:.4} px");
}
```

This test also checks that `fontdb` resolves weight 600 to the SemiBold face. If it fell back to Regular, the SemiBold row would disagree by more than 0.01.

Then append:

```rust
/// R-7 / SC-7: a name outside the subset renders as tofu in the PNG (A-4),
/// but its wrap points still come from our measurement, so our width for it
/// must still be resvg's.
#[test]
fn tofu_measures_the_same_as_resvg_lays_it_out() {
    let opt = options();
    let fonts = Fonts::embedded().expect("faces");
    for weight in [Weight::Regular, Weight::SemiBold] {
        for word in ["日本語", "Kubernetes Ж"] {
            let ours = fonts.face(weight).measure(word, SIZE, 0.0);
            let theirs = usvg_advance(word, weight, &opt);
            assert!((ours - theirs).abs() < 0.01, "{weight:?} {word}: ours {ours} usvg {theirs}");
        }
    }
}
```

If it fails, report both widths rather than dropping the case: it means PNG wrap points for a non-Latin name disagree with what resvg draws.

- [ ] **Step 4: Re-point `rasterises_within_measurement.rs`**

Replace the `use radargen::text::FontMetrics;`, `const FONT` and `const FAMILY` lines with:

```rust
use radargen::text::{FontMetrics, FAMILY, REGULAR_TTF as FONT};
```

Remove the now-duplicate `const FAMILY`. The body is unchanged.

- [ ] **Step 5: Re-point `outlines_match_text.rs`**

Replace its `TTF`, `WOFF2` and `FAMILY` constants with:

```rust
use radargen::text::{FAMILY, REGULAR_TTF as TTF};
const WOFF2: &[u8] = include_bytes!("../assets/Inter-Regular.woff2");
```

In `resvg_ignores_an_embedded_font_face`, change the comment "fontdb holds Libertinus under its own name only" to "fontdb holds Inter under its own name only".

- [ ] **Step 6: Run the whole suite**

Run: `cargo test`, then `diff <(cargo run -q --example report) <(cargo run -q --example report)`. The diff should print nothing: the example is the existing cross-process determinism check (NFR-4, NFR-17).

Expected: everything passes except one test, which fails. `deduplicated_outlines_undercut_an_embedded_font` asserts `deduped < embedded`, and that measured an **unsubset** Libertinus WOFF2 (159 kB). A Latin-subset Inter WOFF2 is ~30.5 kB, so the comparison flips (measured during plan review: deduped 53,420 vs embedded 44,430 bytes). The spike already says "size is not what decided it… roughly a wash" (`docs/spikes/svg-font.md`).

Fix it: keep the `deduped * 5 < outlined` assertion, delete the `deduped < embedded` assertion, and keep the `eprintln!` that prints both sizes. Then rename the test `deduplicated_outlines_are_the_cheap_outlined_form` and change its doc comment's first sentence to "The size argument, which did not decide it — see the spike." Don't touch the spike document; it records what was measured then.

- [ ] **Step 7: Lint and commit**

```bash
cargo clippy --all-targets -- -D warnings
git add -A tests assets examples
git commit -m "test(text): run the oracle tests against Inter, both weights; drop Libertinus"
```

---

### Task 4: Documentation

**Files:**
- Modify: `README.md` (licence paragraph, about lines 34–35)
- Modify: `docs/design/architecture.md` (§3 `assets/` row, the "`assets/` currently holds Libertinus Sans" paragraph, and §12's R-7 row)

**Interfaces:** none.

- [ ] **Step 1: README**

Replace "The bundled font (Libertinus Sans) keeps its own OFL-1.1" with "The bundled font (Inter, Latin subset) keeps its own OFL-1.1". Keep the rest of the sentence.

- [ ] **Step 2: Architecture §3**

In the `assets/` table row, change the slice cell from `*stand-in*` to `*exists*`.

Replace the paragraph that begins "**`assets/` currently holds Libertinus Sans**" with the text below. Fill in the four byte counts from Task 1 Step 5, which are numbers you now have:

```markdown
**`assets/` holds Inter v4.1**, Regular and SemiBold, Latin-subset by
`scripts/subset-inter.sh` and vendored (FR-85). Measured sizes — TTF:
Regular N bytes, SemiBold N bytes; WOFF2: Regular N bytes, SemiBold N bytes.
The WOFF2 pair is what NFR-5 inlines into every HTML edition. R-7's licensing
question was answered earlier — Inter declares no Reserved Font Name, so the
subset keeps the name — and with the sizes measured, R-7 has nothing left
open but the script-coverage limit A-4 accepts.
```

- [ ] **Step 3: Architecture §12**

In the R-7 row, replace the response cell with: "Swapped to Inter before step 4 of §9 (PR 01). Subset sizes are in §3; no Reserved Font Name."

- [ ] **Step 4: Check the links and commit**

```bash
/opt/homebrew/bin/lychee --offline README.md docs/design/architecture.md
git add README.md docs/design/architecture.md
git commit -m "docs: record the Inter swap and its measured subset sizes (R-7)"
```

---

## Finishing the PR

Follow the house workflow in `~/.claude/CLAUDE.md`. For this PR:

1. Run `crumbs start rg-osr` before Task 1. Work on branch `feat/pr-01-inter`, created from `main`.
2. After Task 4: push, then open the PR with `gh pr create` and **no reviewer flag**.
3. Run the code-reviewer skill until clean, then do the Copilot loop until it reports "no new comments" on the head SHA. Then run simplify, then code-reviewer once more.
4. Close `rg-osr`, changing the status field only. **No version bump:** nothing a user can run changes.
5. Move this file to `docs/plans/done/` in the same PR.
6. Merge. There's no tag, since there's no bump.

## Revisions

| Revision | Date | Notes |
| --- | --- | --- |
| 1.0 | 2026-10-09 | Initial version |
| 1.1 | 2026-10-09 | Five-agent review (rust-engineer, qa-expert, architect-reviewer, ui-designer, ux-researcher) folded in |
| 1.2 | 2026-10-09 | PR 6 review: the subset script notes that Inter has no U+FFFD glyph (D-55) |
| 1.3 | 2026-10-09 | The pedantic and unsafe-code gate is now live in `Cargo.toml` (crumb `rg-q3w`) |
