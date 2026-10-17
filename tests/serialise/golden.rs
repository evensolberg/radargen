//! A golden file per target (NFR-4, architecture §10).

use std::path::Path;

use radargen::emit::png::{pixel_size, rasterise};
use radargen::layout::arrange::{Arrangement, arrange};
use radargen::palette::{Palette, Theme};
use radargen::svg::{Target, serialise};
use resvg::tiny_skia::{Pixmap, PremultipliedColorU8};

use super::prd;
use super::version::versionless;

/// Byte-compares `got`, its version replaced by a placeholder, with
/// `tests/golden/{dir}/{name}`. With `UPDATE_GOLDEN=1` (refused under CI)
/// it writes the file and fails, so a blessing run is never green.
pub fn golden(dir: &str, name: &str, got: &str) {
    assert!(
        !got.contains('\r') && !got.starts_with('\u{feff}'),
        "NFR-17: LF, no BOM"
    );
    let got = &*versionless(got);
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(dir)
        .join(name);
    let want = String::from_utf8(bless_or_read(&path, got.as_bytes())).expect("a golden is UTF-8");
    if want != got {
        let at = want
            .bytes()
            .zip(got.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        // Bytes, not str ranges: a non-ASCII character near the difference
        // must not blank the context.
        let around = |s: &str| {
            let bytes = s.as_bytes();
            let window = bytes.get(at.saturating_sub(40)..(at + 40).min(bytes.len()));
            String::from_utf8_lossy(window.unwrap_or_default()).into_owned()
        };
        panic!(
            "{name} differs at byte {at}:\n want …{}…\n  got …{}…",
            around(&want),
            around(got)
        );
    }
}

/// `path`'s bytes. With `UPDATE_GOLDEN=1` (refused under CI) it writes
/// `got` there instead and fails, so a blessing run is never green.
fn bless_or_read(path: &Path, got: &[u8]) -> Vec<u8> {
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        assert!(
            std::env::var_os("CI").is_none(),
            "UPDATE_GOLDEN is refused under CI"
        );
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("creates");
        std::fs::write(path, got).expect("writes");
        panic!("blessed {}; unset UPDATE_GOLDEN and re-run", path.display());
    }
    std::fs::read(path).unwrap_or_else(|_| {
        panic!(
            "{} is missing: run with UPDATE_GOLDEN=1, then check it by eye",
            path.display()
        )
    })
}

/// D-87: per channel, the most a pixel may differ across platforms.
const PNG_CHANNEL_TOLERANCE: u8 = 2;
/// D-87: the share of pixels that may differ at all, in thousandths.
const PNG_DIFFERING_PER_MILLE: usize = 1;

/// D-40, D-87: compares `got`'s decoded pixels with
/// `tests/golden/png/{name}`'s, within D-87's tolerance. With
/// `UPDATE_GOLDEN=1` (refused under CI) it writes the file and fails, so a
/// blessing run is never green. On a mismatch it writes the image and a
/// diff, magenta where a pixel differs past the tolerance, under
/// `target/png-golden/`.
pub fn golden_png(name: &str, got: &[u8]) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let want = bless_or_read(&root.join("tests/golden/png").join(name), got);
    let (want, image) = (decode(&want), decode(got));
    assert_eq!(
        (want.width(), want.height()),
        (image.width(), image.height()),
        "{name}: size"
    );
    let mut diff = image.clone();
    let mut differing = 0;
    let mut over = 0;
    let magenta = PremultipliedColorU8::from_rgba(255, 0, 255, 255).expect("opaque");
    for ((a, b), d) in want
        .pixels()
        .iter()
        .zip(image.pixels())
        .zip(diff.pixels_mut())
    {
        let (a, b) = (a.demultiply(), b.demultiply());
        let worst = [
            a.red().abs_diff(b.red()),
            a.green().abs_diff(b.green()),
            a.blue().abs_diff(b.blue()),
            a.alpha().abs_diff(b.alpha()),
        ]
        .into_iter()
        .max()
        .unwrap_or(0);
        differing += usize::from(worst > 0);
        if worst > PNG_CHANNEL_TOLERANCE {
            over += 1;
            *d = magenta;
        }
    }
    let allowed = want.pixels().len() * PNG_DIFFERING_PER_MILLE / 1000;
    if over > 0 || differing > allowed {
        let out = root.join("target/png-golden");
        std::fs::create_dir_all(&out).expect("creates");
        std::fs::write(out.join(name), got).expect("writes");
        diff.save_png(out.join(format!("diff-{name}")))
            .expect("writes");
        panic!(
            "{name}: {differing} pixels differ (at most {allowed}), {over} past {PNG_CHANNEL_TOLERANCE} per channel; see {}",
            out.display()
        );
    }
}

fn decode(png: &[u8]) -> Pixmap {
    Pixmap::decode_png(png).expect("a PNG")
}

/// `blocks`, wide, light, for the interactive or static target.
fn wide(blocks: &radargen::layout::arrange::Blocks, interactive: bool) -> String {
    let palette = Palette::built_in(Theme::Light);
    let target = if interactive {
        Target::Interactive(&palette)
    } else {
        Target::Static(&palette)
    };
    serialise(&arrange(Arrangement::Wide, blocks), target)
}

/// The PRD example, wide, light: the static SVG and PNG source.
#[test]
fn prd_example_matches_its_static_golden() {
    golden(
        "serialiser",
        "prd-wide-light.svg",
        &wide(&prd().blocks, false),
    );
}

/// The same, as HTML inlines it: the widest attribute surface.
#[test]
fn prd_example_matches_its_interactive_golden() {
    golden(
        "serialiser",
        "prd-wide-interactive.svg",
        &wide(&prd().blocks, true),
    );
}

/// D-86: a radar grown to fit a crowded innermost band, wide and static:
/// pins the growth chosen and that only geometry grew.
#[test]
fn a_grown_radar_matches_its_golden() {
    let grown = super::build(include_str!("../fixtures/grown.csv"), None, None);
    golden(
        "serialiser",
        "grown-wide-light.svg",
        &wide(&grown.blocks, false),
    );
}

/// The PRD example, wide, in `theme`, at its own width.
fn prd_png(theme: Theme) -> Vec<u8> {
    let doc = arrange(Arrangement::Wide, &prd().blocks);
    let size = pixel_size(&doc, None).expect("fits");
    rasterise(&doc, &Palette::built_in(theme), size).expect("renders")
}

/// FR-55: the PRD example as a light PNG. One test per golden, since a
/// blessing run stops at the first.
#[test]
fn prd_example_matches_its_light_png_golden() {
    golden_png("prd-light.png", &prd_png(Theme::Light));
}

/// FR-55, FR-56: the same, dark.
#[test]
fn prd_example_matches_its_dark_png_golden() {
    golden_png("prd-dark.png", &prd_png(Theme::Dark));
}

/// The PRD example, wide, exported in `theme` at its own width, titled
/// as `prd()` titles it.
pub fn prd_svg(theme: Theme) -> String {
    let built = prd();
    let doc = arrange(Arrangement::Wide, &built.blocks);
    let palette = Palette::built_in(theme);
    let fonts = radargen::text::Fonts::embedded().expect("loads");
    radargen::emit::svg::export(
        &doc,
        &radargen::emit::svg::Export {
            palette: &palette,
            fonts: &fonts,
            width: None,
            title: Some("Technology Radar"),
            desc: &built.blocks.radar.name(),
        },
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
