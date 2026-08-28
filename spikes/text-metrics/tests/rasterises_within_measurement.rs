//! Closes the loop to PNG. The usvg oracle proves our advance matches resvg's
//! *layout*; this proves it matches what actually lands in the pixels, which is
//! what SC-7 asserts and what R-2 warns can diverge silently.
//!
//! Method: draw the string, rasterise, and find the rightmost inked column.
//! It must fall inside the width we computed — and, as a discrimination check,
//! must fall outside a box deliberately shrunk below it.

use std::sync::Arc;
use text_metrics_spike::FontMetrics;

const FONT: &[u8] = include_bytes!("../assets/LibertinusSans-Regular.ttf");
const FAMILY: &str = "Libertinus Sans";
const SIZE: f64 = 20.0;

fn rightmost_inked_column(text: &str) -> u32 {
    let mut db = fontdb::Database::new();
    db.load_font_data(FONT.to_vec());
    let opt = usvg::Options {
        fontdb: Arc::new(db),
        font_family: FAMILY.to_string(),
        ..usvg::Options::default()
    };
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="900" height="80">
             <text x="0" y="60" font-family="{FAMILY}" font-size="{SIZE}">{text}</text>
           </svg>"#
    );
    let tree = usvg::Tree::from_str(&svg, &opt).expect("usvg parses");
    let mut pixmap = tiny_skia::Pixmap::new(900, 80).expect("pixmap");
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());

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
    let m = FontMetrics::load(FONT).expect("font parses");
    for text in ["Terraform", "Trunk-Based Development", "WWWWWWWW", "Kubernetes"] {
        let measured = m.measure(text, SIZE, 0.0);
        let inked = f64::from(rightmost_inked_column(text));
        assert!(
            inked <= measured.ceil(),
            "{text}: ink reaches {inked} but we measured {measured:.4}"
        );
        // Discrimination: the check would catch an under-measurement of 20%.
        assert!(
            inked > measured * 0.8,
            "{text}: ink {inked} is far short of {measured:.4} — test is not discriminating"
        );
    }
}
