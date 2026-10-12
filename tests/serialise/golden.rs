//! A golden file per target (NFR-4, architecture §10).

use std::path::Path;

use radargen::layout::arrange::{Arrangement, arrange};
use radargen::palette::{Palette, Theme};
use radargen::svg::{Target, serialise};

use super::prd;

/// Byte-compares `got` with `tests/golden/serialiser/{name}`. With
/// `UPDATE_GOLDEN=1` (refused under CI) it writes the file and fails, so a
/// blessing run is never green.
fn golden(name: &str, got: &str) {
    assert!(
        !got.contains('\r') && !got.starts_with('\u{feff}'),
        "NFR-17: LF, no BOM"
    );
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/serialiser")
        .join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        assert!(
            std::env::var_os("CI").is_none(),
            "UPDATE_GOLDEN is refused under CI"
        );
        std::fs::write(&path, got).expect("writes");
        panic!("blessed {}; unset UPDATE_GOLDEN and re-run", path.display());
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "{} is missing: run with UPDATE_GOLDEN=1, then read it by eye",
            path.display()
        )
    });
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

/// The PRD example, wide, light, for the interactive or static target.
fn wide(interactive: bool) -> String {
    let palette = Palette::built_in(Theme::Light);
    let target = if interactive {
        Target::Interactive(&palette)
    } else {
        Target::Static(&palette)
    };
    serialise(&arrange(Arrangement::Wide, &prd().blocks), target)
}

/// The PRD example, wide, light: the static SVG and PNG source.
#[test]
fn prd_example_matches_its_static_golden() {
    golden("prd-wide-light.svg", &wide(false));
}

/// The same, as HTML inlines it: the widest attribute surface.
#[test]
fn prd_example_matches_its_interactive_golden() {
    golden("prd-wide-interactive.svg", &wide(true));
}
