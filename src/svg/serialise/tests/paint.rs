//! Late binding (architecture §5).

use super::*;

#[test]
fn paint_binds_late() {
    let palette = light();
    let (interactive, fixed) = (Target::Interactive(&palette), Target::Static(&palette));
    let series = swatch(Paint::Role(Role::Series(9)));
    assert_eq!(
        value_of(&one(&series, interactive), "fill"),
        "var(--series-2)"
    );
    assert_eq!(value_of(&one(&series, fixed), "fill"), "#C25400");
    for target in [interactive, fixed] {
        let ink = |i| value_of(&one(&swatch(Paint::NumberOn(i)), target), "fill").to_owned();
        assert_eq!(ink(9), ink(1));
        assert_eq!(ink(8), ink(0));
        assert!(ink(0).starts_with('#'), "a literal in every target");
        assert_eq!(value_of(&one(&swatch(Paint::None), target), "fill"), "none");
    }
}

/// Every static colour is `#RRGGBB`: nothing else may reach an attribute,
/// however a future palette is built (PR 18 validates `--palette`).
#[test]
fn static_paint_is_a_hex_literal() {
    let hex = |v: &str| {
        v.len() == 7 && v.starts_with('#') && v.chars().skip(1).all(|c| c.is_ascii_hexdigit())
    };
    for theme in [Theme::Light, Theme::Dark] {
        let palette = Palette::built_in(theme);
        let paints = palette
            .declarations()
            .into_iter()
            .map(|(role, _)| Paint::Role(role))
            .chain((0..16).map(Paint::NumberOn));
        for paint in paints {
            let markup = one(&swatch(paint), Target::Static(&palette));
            let fill = value_of(&markup, "fill");
            assert!(hex(fill), "{paint:?}: {fill}");
        }
    }
}
