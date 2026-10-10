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
    assert!(
        semibold > regular,
        "SemiBold {semibold} should exceed Regular {regular}"
    );
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
