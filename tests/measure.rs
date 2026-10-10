use radargen::text::{FontMetrics, REGULAR_TTF as FONT};

fn metrics() -> FontMetrics<'static> {
    FontMetrics::load(FONT).expect("font parses")
}

#[test]
fn empty_string_measures_zero() {
    assert_eq!(metrics().measure("", 12.5, 0.0), 0.0);
}

#[test]
fn single_glyph_has_positive_width() {
    assert!(metrics().measure("H", 12.5, 0.0) > 0.0);
}

#[test]
fn width_scales_linearly_with_size() {
    let m = metrics();
    let at12 = m.measure("Terraform", 12.0, 0.0);
    let at24 = m.measure("Terraform", 24.0, 0.0);
    assert!((at24 - at12 * 2.0).abs() < 1e-9, "{at24} != 2 x {at12}");
}

#[test]
fn repeated_glyph_is_additive() {
    let m = metrics();
    let one = m.measure("H", 12.5, 0.0);
    let two = m.measure("HH", 12.5, 0.0);
    assert!((two - one * 2.0).abs() < 1e-9, "{two} != 2 x {one}");
}

#[test]
fn letter_spacing_applies_between_glyphs_not_after() {
    let m = metrics();
    let plain = m.measure("Kubernetes", 12.5, 0.0);
    let spaced = m.measure("Kubernetes", 12.5, 0.75);
    // 10 glyphs -> 9 gaps
    assert!(
        (spaced - (plain + 9.0 * 0.75)).abs() < 1e-9,
        "{spaced} vs {plain}"
    );
}

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
    assert!(
        worst > 0.5,
        "shaping applied no kerning anywhere in the corpus: worst {worst}"
    );
}
