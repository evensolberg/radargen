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

use radargen::text::{FAMILY, Fonts, Weight};

#[path = "common/fonts.rs"]
mod fonts;

const SIZE: f64 = 12.5;

/// Right edge of the laid-out advance for `text`, laid out by usvg at x=0.
fn usvg_advance(text: &str, weight: Weight, letter_spacing: f64, opt: &usvg::Options) -> f64 {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="2000" height="200">
             <text x="0" y="100" font-family="{FAMILY}" font-weight="{}" font-size="{SIZE}" letter-spacing="{letter_spacing}">{text}</text>
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
    let opt = fonts::usvg_options();
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
            // D-68: scripts the subset now draws, measured as resvg lays them out.
            "Ελληνικά",
            "Ελληνικός",
            "Кириллица",
            "Москва 2026",
            "Tiếng Việt",
            // The same text typed as base letters plus combining marks, some
            // outside the subset's ranges: the shaper must compose or position
            // them exactly as resvg does.
            "Tie\u{0302}\u{0301}ng Vie\u{0323}\u{0302}t",
            "α\u{0313}\u{0301}",
            "За\u{0301}мок",
            // Mixed scripts on one line.
            "Kubernetes Ж",
            // A-4: outside the subset this renders as tofu in the PNG, but its
            // wrap points still come from our measurement.
            "日本語",
        ] {
            let ours = fonts.face(weight).measure(word, SIZE, 0.0);
            let theirs = usvg_advance(word, weight, 0.0, &opt);
            let delta = (ours - theirs).abs();
            // Per word, not via `worst`: `f64::max` would swallow a NaN.
            assert!(delta < 0.01, "{weight:?} {word}: ours {ours} usvg {theirs}");
            worst = worst.max(delta);
            eprintln!("{weight:?} {word:22} ours {ours:8.4}  usvg {theirs:8.4}  delta {delta:+.4}");
        }
    }
    eprintln!("worst delta across corpus: {worst:.6} px at {SIZE}px");
}

/// FR-84: letter-spacing goes once between clusters, never after the last,
/// as resvg applies it. A combining mark that stays a separate glyph must
/// not earn a gap of its own.
#[test]
fn letter_spacing_matches_usvg_by_cluster() {
    let opt = fonts::usvg_options();
    let fonts = Fonts::embedded().expect("faces");
    for weight in [Weight::Regular, Weight::SemiBold] {
        for spacing in [0.48, 3.0] {
            for word in [
                "Terraform",
                "Trunk-Based Development",
                "Tiếng Việt",
                "За\u{0301}мок",
                "i\u{0307}x",
                "α\u{0313}\u{0301}",
                "Tie\u{0302}\u{0301}ng",
            ] {
                let ours = fonts.face(weight).measure(word, SIZE, spacing);
                let theirs = usvg_advance(word, weight, spacing, &opt);
                assert!(
                    (ours - theirs).abs() < 0.01,
                    "{weight:?} {word:?} at {spacing}: ours {ours} usvg {theirs}"
                );
            }
        }
    }
}
