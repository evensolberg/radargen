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

use std::sync::Arc;
use radargen::text::FontMetrics;

const FONT: &[u8] = include_bytes!("../assets/LibertinusSans-Regular.ttf");
const SIZE: f64 = 12.5;
const FAMILY: &str = "Libertinus Sans";

fn options() -> usvg::Options<'static> {
    let mut db = fontdb::Database::new();
    db.load_font_data(FONT.to_vec());
    usvg::Options {
        fontdb: Arc::new(db),
        font_family: FAMILY.to_string(),
        ..usvg::Options::default()
    }
}

/// Right edge of the rendered ink for `text`, laid out by usvg at x=0.
fn right_edge(text: &str, opt: &usvg::Options) -> f64 {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="2000" height="200">
             <text x="0" y="100" font-family="{FAMILY}" font-size="{SIZE}">{text}</text>
           </svg>"#
    );
    let tree = usvg::Tree::from_str(&svg, opt).expect("usvg parses");
    let bb = tree.root().abs_bounding_box();
    f64::from(bb.right())
}

fn usvg_advance(word: &str, opt: &usvg::Options) -> f64 {
    right_edge(word, opt)
}

#[test]
fn our_advance_matches_usvg_layout() {
    let opt = options();
    let m = FontMetrics::load(FONT).expect("font parses");
    let mut worst = 0.0_f64;
    for word in [
        "Terraform", "Kubernetes", "Ansible", "Pulumi",
        "Trunk-Based", "Development", "Infrastructure",
        "Internationalisation", "WWWWWWWW", "illlliii",
    ] {
        let ours = m.measure(word, SIZE, 0.0);
        let theirs = usvg_advance(word, &opt);
        let delta = (ours - theirs).abs();
        worst = worst.max(delta);
        eprintln!("{word:22} ours {ours:8.4}  usvg {theirs:8.4}  delta {delta:+.4}");
    }
    eprintln!("worst delta across corpus: {worst:.6} px at {SIZE}px");
    assert!(worst < 0.01, "worst delta {worst:.4} px");
}
