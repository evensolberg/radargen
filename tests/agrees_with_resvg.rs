//! R-1's real question: do our metrics match what resvg will rasterise?
//!
//! SC-7 requires the wrap points baked into the HTML to be the ones the PNG
//! shows. If our advance width and resvg's text layout disagree, the two
//! formats diverge silently — which is exactly what R-2 predicts and SC-7
//! would only catch late.
//!
//! Method: usvg's own bounding box for a text node is its *layout* box, whose
//! right edge is the shaped advance — verified against a single glyph, where it
//! equals the raw `hmtx` advance exactly. So usvg lays the string out and we
//! read the advance straight off it, with no sentinel and no bearing algebra.

use radargen::text::{FAMILY, Fonts, REGULAR_TTF, SEMIBOLD_TTF, Weight};
use std::sync::Arc;

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

/// Also checks that `fontdb` resolves weight 600 to the `SemiBold` face: had
/// it fallen back to Regular, the `SemiBold` rows would disagree by far more
/// than the tolerance.
#[test]
fn our_advance_matches_usvg_layout_for_both_weights() {
    let opt = options();
    let fonts = Fonts::embedded().expect("faces");
    let mut worst = 0.0_f64;
    for weight in [Weight::Regular, Weight::SemiBold] {
        for word in [
            "Terraform",
            "Kubernetes",
            "Ansible",
            "Pulumi",
            "Trunk-Based",
            "Development",
            "Infrastructure",
            "Internationalisation",
            "WWWWWWWW",
            "illlliii",
            // R-7 / SC-7: outside the subset these render as tofu in the PNG
            // (A-4), but their wrap points still come from our measurement.
            "日本語",
            "Kubernetes Ж",
        ] {
            let ours = fonts.face(weight).measure(word, SIZE, 0.0);
            let theirs = usvg_advance(word, weight, &opt);
            let delta = (ours - theirs).abs();
            // Per word, not via `worst`: `f64::max` would swallow a NaN.
            assert!(delta < 0.01, "{weight:?} {word}: ours {ours} usvg {theirs}");
            worst = worst.max(delta);
            eprintln!("{weight:?} {word:22} ours {ours:8.4}  usvg {theirs:8.4}  delta {delta:+.4}");
        }
    }
    eprintln!("worst delta across corpus: {worst:.6} px at {SIZE}px");
}
