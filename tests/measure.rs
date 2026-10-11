//! Pins `FontMetrics` widths for Latin text to the values the vendored Inter
//! subset produces, so a font or shaper change cannot move them unseen (NFR-4).

use radargen::text::{FontMetrics, REGULAR_TTF as FONT};

/// Latin text whose widths must not move: kerning pairs, Latin Extended
/// letters, and the extremes of advance width.
const LATIN_CORPUS: [&str; 13] = [
    "Terraform",
    "AVATAR",
    "Today",
    "WAVE",
    "Tycho",
    "Yoga",
    "office",
    "Dvořák",
    "Łukasiewicz",
    "İstanbul",
    "Trunk-Based Development",
    "WWWWWWWW",
    "illlliii",
];

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
    assert!(
        at12.mul_add(-2.0, at24).abs() < 1e-9,
        "{at24} != 2 x {at12}"
    );
}

#[test]
fn repeated_glyph_is_additive() {
    let m = metrics();
    let one = m.measure("H", 12.5, 0.0);
    let two = m.measure("HH", 12.5, 0.0);
    assert!(one.mul_add(-2.0, two).abs() < 1e-9, "{two} != 2 x {one}");
}

#[test]
fn letter_spacing_applies_between_glyphs_not_after() {
    let m = metrics();
    let plain = m.measure("Kubernetes", 12.5, 0.0);
    let spaced = m.measure("Kubernetes", 12.5, 0.75);
    // 10 glyphs -> 9 gaps
    assert!(
        (spaced - 9.0_f64.mul_add(0.75, plain)).abs() < 1e-9,
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

/// NFR-4, FR-79: Latin widths do not move when the subset or the shaper
/// changes. Pinned bit-exactly from the `rustybuzz` shaper and the Latin-only
/// subset, before either changed, so a later change that moves one is caught
/// here rather than as a moved wrap point.
#[test]
fn latin_measurements_are_pinned() {
    use radargen::text::Weight::{Regular, SemiBold};
    const PINNED: [(radargen::text::Weight, &str, u64); 26] = [
        (Regular, "Terraform", 0x404d_4e58_0000_0000),
        (Regular, "AVATAR", 0x4047_6380_0000_0000),
        (Regular, "Today", 0x4042_0d18_0000_0000),
        (Regular, "WAVE", 0x4041_c218_0000_0000),
        (Regular, "Tycho", 0x4042_0c50_0000_0000),
        (Regular, "Yoga", 0x403d_b4b0_0000_0000),
        (Regular, "office", 0x4040_dba0_0000_0000),
        (Regular, "Dvořák", 0x4044_f0f0_0000_0000),
        (Regular, "Łukasiewicz", 0x4052_20a0_0000_0000),
        (Regular, "İstanbul", 0x4047_4a80_0000_0000),
        (Regular, "Trunk-Based Development", 0x4063_a51a_0000_0000),
        (Regular, "WWWWWWWW", 0x4059_2af8_0000_0000),
        (Regular, "illlliii", 0x4038_3800_0000_0000),
        (SemiBold, "Terraform", 0x404e_2468_0000_0000),
        (SemiBold, "AVATAR", 0x4048_5bf0_0000_0000),
        (SemiBold, "Today", 0x4042_8180_0000_0000),
        (SemiBold, "WAVE", 0x4042_5d90_0000_0000),
        (SemiBold, "Tycho", 0x4042_91e8_0000_0000),
        (SemiBold, "Yoga", 0x403e_4ab0_0000_0000),
        (SemiBold, "office", 0x4041_6a98_0000_0000),
        (SemiBold, "Dvořák", 0x4045_7110_0000_0000),
        (SemiBold, "Łukasiewicz", 0x4052_acdc_0000_0000),
        (SemiBold, "İstanbul", 0x4048_2d10_0000_0000),
        (SemiBold, "Trunk-Based Development", 0x4064_1888_0000_0000),
        (SemiBold, "WWWWWWWW", 0x4059_fde8_0000_0000),
        (SemiBold, "illlliii", 0x403a_2c00_0000_0000),
    ];
    for text in LATIN_CORPUS {
        for weight in [Regular, SemiBold] {
            assert!(
                PINNED.iter().any(|&(w, t, _)| w == weight && t == text),
                "{weight:?} {text:?} is not pinned"
            );
        }
    }
    let fonts = radargen::text::Fonts::embedded().expect("faces");
    for (weight, text, bits) in PINNED {
        let width = fonts.face(weight).measure(text, 12.5, 0.0);
        assert_eq!(
            width.to_bits(),
            bits,
            "{weight:?} {text:?} measures {width}, pinned {}",
            f64::from_bits(bits)
        );
    }
}

/// FR-84: letter-spacing adds one gap per cluster but the last. Latin text
/// without combining marks has one glyph per cluster, so these widths must
/// not move when the count switches from glyphs to clusters.
#[test]
fn letter_spaced_latin_is_pinned() {
    use radargen::text::Weight::{Regular, SemiBold};
    const PINNED: [(radargen::text::Weight, &str, u64); 4] = [
        (Regular, "Terraform", 0x404f_39dd_1eb8_51ec),
        (Regular, "Trunk-Based Development", 0x4064_f705_851e_b852),
        (SemiBold, "Terraform", 0x4050_07f6_8f5c_28f6),
        (SemiBold, "Trunk-Based Development", 0x4065_6a73_851e_b852),
    ];
    let fonts = radargen::text::Fonts::embedded().expect("faces");
    for (weight, text, bits) in PINNED {
        let width = fonts.face(weight).measure(text, 12.5, 0.48);
        assert_eq!(
            width.to_bits(),
            bits,
            "{weight:?} {text:?} measures {width}"
        );
    }
}
