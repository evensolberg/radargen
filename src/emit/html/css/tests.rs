//! The stylesheet's contract (plan RG-014B, Task 1).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use super::*;
use crate::palette::{Palette, Theme};
use crate::text::{FAMILY, Weight};

/// The PRD example's wide and narrow widths.
const WIDE: f64 = 976.817;
const NARROW: f64 = 420.0;

/// `layout(WIDE, NARROW)` in full: the body, the SVGs, both caps, the
/// default and the one query (FR-51, FR-69).
const LAYOUT: &str = concat!(
    "body{margin:0;background:var(--surface-1);color:var(--text-primary)}\n",
    "svg{display:block;width:100%;height:auto}\n",
    ".wide{max-width:976.817px;margin:0 auto}\n",
    ".narrow{max-width:420px;margin:0 auto}\n",
    ".narrow{display:none}\n",
    "@media (max-width: 780px){.wide{display:none}.narrow{display:block}}\n",
);

/// The text between `start` and the next `end`, for each occurrence.
fn between<'a>(text: &'a str, start: &str, end: &str) -> Vec<&'a str> {
    text.split(start)
        .skip(1)
        .filter_map(|rest| rest.split_once(end).map(|(inside, _)| inside))
        .collect()
}

#[test]
fn the_font_faces_are_the_vendored_fonts() {
    let faces = font_faces();
    assert_eq!(faces.matches("@font-face").count(), 2, "{faces}");
    assert_eq!(
        faces.matches(&format!("font-family:\"{FAMILY}\"")).count(),
        2
    );
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let payloads = between(&faces, "base64,", ")");
    let weights = between(&faces, "font-weight:", ";");
    let expected = [
        (Weight::Regular, "Inter-Regular.woff2"),
        (Weight::SemiBold, "Inter-SemiBold.woff2"),
    ];
    assert_eq!(payloads.len(), expected.len());
    for ((payload, weight), (want_weight, file)) in payloads.iter().zip(&weights).zip(expected) {
        assert_eq!(*weight, want_weight.css().to_string(), "{file}");
        let decoded = STANDARD.decode(payload).expect("valid base64");
        let vendored = std::fs::read(assets.join(file)).expect("the asset exists");
        assert!(
            decoded == vendored,
            "{file}: the inlined font is the vendored one"
        );
    }
}

#[test]
fn palettes_override_in_order() {
    let declared = |theme| -> String {
        Palette::built_in(theme)
            .declarations()
            .into_iter()
            .map(|(role, value)| format!("{role}:{value};"))
            .collect::<Vec<_>>()
            .concat()
    };
    let (light, dark) = (declared(Theme::Light), declared(Theme::Dark));
    let want = [
        format!(":root{{color-scheme:light;{light}}}\n"),
        format!(
            "@media (prefers-color-scheme: dark){{:root:not([data-theme=\"light\"]){{color-scheme:dark;{dark}}}}}\n"
        ),
        format!(":root[data-theme=\"dark\"]{{color-scheme:dark;{dark}}}\n"),
    ]
    .concat();
    assert_eq!(palettes(), want);
    // The number inks are literals because the series agree (§5).
    let series = |theme| {
        Palette::built_in(theme)
            .declarations()
            .into_iter()
            .filter(|(role, _)| role.to_string().starts_with("--series-"))
            .map(|(_, value)| value.to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(series(Theme::Light), series(Theme::Dark));
}

#[test]
fn the_layout_is_exact() {
    assert_eq!(layout(WIDE, NARROW), LAYOUT);
    assert_eq!(narrow_query(WIDE), "(max-width: 780px)");
    assert_eq!(
        crate::style::breakpoint_px(WIDE),
        781,
        "the query is one below the breakpoint"
    );
}

/// FR-69: only the default and the query set `display` on an arrangement,
/// and nothing hides one any other way.
#[test]
fn only_the_breakpoint_rule_sets_display() {
    let css = layout(WIDE, NARROW);
    let displays: Vec<&str> = css
        .match_indices("display:")
        .filter_map(|(at, _)| css.get(..at))
        .filter_map(|before| before.rsplit(['{', '}', '\n']).nth(1))
        .collect();
    assert_eq!(displays, ["svg", ".narrow", ".wide", ".narrow"], "{css}");
    for hide in ["visibility", "opacity"] {
        assert!(!css.contains(hide), "{hide}");
    }
}

#[test]
fn one_rule_per_marker_key() {
    use crate::svg::ItemKey;
    let css = indicators(&[ItemKey(6), ItemKey(7), ItemKey(2)]);
    let supports = between(&css, "@supports selector(:has(*)){\n", "\n}\n");
    let [block] = supports.as_slice() else {
        panic!("one @supports block: {css}");
    };
    let rules: Vec<&str> = block.lines().skip(1).collect();
    assert_eq!(
        rules,
        [6, 7, 2].map(|k| format!(
            "svg:has([data-marker=\"{k}\"]:focus-visible) [data-marker-indicator=\"{k}\"]{{visibility:visible}}"
        ))
    );
    assert!(block.starts_with("[data-marker]:focus-visible{outline:2px solid transparent}"));

    let none = indicators(&[]);
    assert!(!none.contains("@supports"), "{none}");
    assert!(!none.contains("{}"), "{none}");
}

/// The browser's outline goes only where an indicator replaces it, so an
/// engine that drops one rule drops the other (WCAG 2.4.7).
#[test]
fn outlines_go_only_with_their_indicator() {
    use crate::svg::ItemKey;
    let css = indicators(&[ItemKey(6)]);
    let (outside, inside) = css
        .split_once("@supports selector(:has(*)){")
        .expect("a @supports block");
    for (part, wants) in [(outside, "[data-legend-row]"), (inside, "[data-marker]")] {
        let outlines: Vec<&str> = part
            .lines()
            .filter(|line| line.contains("outline"))
            .collect();
        assert_eq!(
            outlines,
            [format!(
                "{wants}:focus-visible{{outline:2px solid transparent}}"
            )],
            "{css}"
        );
    }
    assert!(css.contains(
        "[data-legend-row]:focus-visible>[data-row-indicator]>rect{stroke:var(--focus);stroke-width:2px}"
    ));
}

#[test]
fn the_stylesheet_requests_nothing_but_fonts() {
    use crate::svg::ItemKey;
    let css = [
        font_faces(),
        palettes(),
        layout(WIDE, NARROW),
        indicators(&[ItemKey(6)]),
        VISUALLY_HIDDEN.to_owned(),
    ]
    .concat();
    assert_eq!(css.matches("url(").count(), 2);
    assert_eq!(css.matches("url(data:font/woff2;base64,").count(), 2);
    assert_eq!(css.matches("src:").count(), 2);
    for banned in ["@import", "http", "<"] {
        assert!(!css.contains(banned), "{banned}");
    }
}
