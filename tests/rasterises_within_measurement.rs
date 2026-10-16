//! Closes the loop to PNG. The usvg oracle proves our advance matches resvg's
//! *layout*; this proves it matches what actually lands in the pixels, which is
//! what SC-7 asserts and what R-2 warns can diverge silently — for both of
//! FR-85's weights, since the legend's headings are `SemiBold` (crumb rg-w1v).
//!
//! Method: draw the string, rasterise, and find the rightmost inked column.
//! It must fall inside the width we computed — and, as a discrimination check,
//! must fall outside a box deliberately shrunk below it.
//!
//! It does not check that weight 600 resolves to the `SemiBold` face — Regular
//! ink would fit inside `SemiBold`'s wider measurement; `tests/agrees_with_resvg.rs`
//! checks that.

use radargen::text::{FAMILY, Fonts, Weight};
use resvg::{tiny_skia, usvg};

#[path = "common/fonts.rs"]
mod fonts;

const SIZE: f64 = 20.0;

fn rightmost_inked_column(text: &str, weight: Weight) -> u32 {
    let opt = fonts::usvg_options();
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="900" height="80">
             <text x="0" y="60" font-family="{FAMILY}" font-weight="{}" font-size="{SIZE}">{text}</text>
           </svg>"#,
        weight.css()
    );
    let tree = usvg::Tree::from_str(&svg, &opt).expect("usvg parses");
    let mut pixmap = tiny_skia::Pixmap::new(900, 80).expect("pixmap");
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    let mut rightmost = 0;
    for y in 0..pixmap.height() {
        for x in 0..pixmap.width() {
            if pixmap.pixel(x, y).is_some_and(|p| p.alpha() > 0) {
                rightmost = rightmost.max(x);
            }
        }
    }
    rightmost
}

#[test]
fn rendered_ink_fits_inside_the_measured_advance() {
    let fonts = Fonts::embedded().expect("faces");
    for weight in [Weight::Regular, Weight::SemiBold] {
        for text in [
            "Terraform",
            "Trunk-Based Development",
            "WWWWWWWW",
            "Kubernetes",
            // D-68's scripts, which the subset now draws, including
            // strings ending in a stacked diacritic or a descender.
            "Ελληνικά",
            "Кириллица",
            "Tiếng Việt",
            "Ủy ban",
            "й",
            "ῷ",
        ] {
            let measured = fonts.face(weight).measure(text, SIZE, 0.0);
            let inked = f64::from(rightmost_inked_column(text, weight));
            assert!(
                inked <= measured.ceil(),
                "{weight:?} {text}: ink reaches {inked} but we measured {measured:.4}"
            );
            // Discrimination: the check would catch an under-measurement of 20%.
            assert!(
                inked > measured * 0.8,
                "{weight:?} {text}: ink {inked} is far short of {measured:.4} — test is not discriminating"
            );
        }
    }
}
