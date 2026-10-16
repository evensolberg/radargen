//! The stylesheet's contract (plan RG-014B, Task 1).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use super::*;
use crate::palette::{Palette, Theme};
use crate::text::{FAMILY, Weight};

/// A wide arrangement's parts, the PRD example's widths: a 976.817-unit
/// document whose radar part is 670 wide, under a 53-unit title.
const WIDE: Wide = Wide {
    title_height: 53.0,
    radar_width: 670.0,
    radar_height: 640.5,
    legend_width: 306.817,
};
const NARROW: f64 = 420.0;

/// `layout(&WIDE, NARROW)` in full: the body, the SVGs, the wide grid and
/// its parts, the narrow cap, the default and the one query (FR-51, FR-69,
/// D-85).
const LAYOUT: &str = concat!(
    "body{margin:0;line-height:1.4;font-family:\"Inter\";background:var(--surface-1);color:var(--text-primary)}\n",
    "svg{display:block;width:100%;height:auto}\n",
    ".wide{display:grid;grid-template-columns:minmax(0,max(536px,calc((100vh - 53px)*670/640.5))) max-content;justify-content:center}\n",
    ".wide-title{grid-row:1;grid-column:1/-1;contain:inline-size}\n",
    ".wide-title svg{width:976.817px;max-width:100%}\n",
    ".wide-radar{grid-row:2;grid-column:1}\n",
    ".wide-key{grid-row:3;grid-column:1}\n",
    ".wide-key svg{width:670px;max-width:100%}\n",
    ".wide-legend{grid-row:2;grid-column:2;height:0;min-height:100%;box-sizing:border-box;overflow-y:auto;scrollbar-gutter:stable}\n",
    ".wide-legend svg{width:306.817px}\n",
    ".narrow{max-width:420px;margin:0 auto}\n",
    ".narrow{display:none}\n",
    "@media (max-width: 842px){.wide{display:none}.narrow{display:block}}\n",
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
    // The forced-colours block follows (forced_colours_map_the_neutrals_…).
    assert!(palettes().starts_with(&want), "{}", palettes());
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
    assert_eq!(layout(&WIDE, NARROW), LAYOUT);
    assert_eq!(narrow_query(&WIDE), "(max-width: 842px)");
    assert_eq!(
        WIDE.breakpoint(),
        843,
        "the query is one below the breakpoint: 976.817 − 0.2 × 670"
    );
}

/// D-85: the radar's column fills the window's width or the height under
/// the title, whichever runs out first, never below 0.8 of the radar's own
/// width, the scale the breakpoint allows;
/// page chrome is capped at that column and the legend beside it.
/// `the wide arrangement fills the window` in scripts/interaction.mjs
/// checks the rendered sizes.
#[test]
fn the_wide_cap_fills_the_window() {
    let radar = "max(536px,calc((100vh - 53px)*670/640.5))";
    assert_eq!(WIDE.radar_cap(), radar);
    assert_eq!(
        capped(".x{", &WIDE),
        format!(
            ".x{{max-width:calc({radar} + 306.817px);margin-inline:auto;padding-inline:20px;box-sizing:border-box}}\n"
        )
    );
}

/// FR-69: only the defaults and the query set `display` on an arrangement,
/// and nothing hides one any other way. The wide default is its grid
/// (D-85).
#[test]
fn only_the_breakpoint_rule_sets_display() {
    let css = layout(&WIDE, NARROW);
    let displays: Vec<&str> = css
        .match_indices("display:")
        .filter_map(|(at, _)| css.get(..at))
        .filter_map(|before| before.rsplit(['{', '}', '\n']).nth(1))
        .collect();
    assert_eq!(
        displays,
        ["svg", ".wide", ".narrow", ".wide", ".narrow"],
        "{css}"
    );
    for hide in ["visibility", "opacity"] {
        assert!(!css.contains(hide), "{hide}");
    }
}

/// The marker focus rules, whole.
const MARKER_FOCUS: [&str; 4] = [
    ".js [data-marker]:focus-visible{outline:2px solid transparent}\n",
    "[data-focus-ring]{visibility:hidden}\n",
    "[data-marker-indicator][data-focused]{visibility:visible}\n",
    "[data-marker-indicator][data-focused] [data-focus-ring]{visibility:visible}\n",
];

/// rg-jri: a marker's ring is shown by the attribute the script sets, with
/// fixed rules, so the stylesheet does not grow with the markers and no
/// `:has` reruns on every Tab. The browser's outline goes only when the
/// script runs to stand in for it (WCAG 2.4.7).
#[test]
fn marker_focus_is_by_attribute() {
    let css = indicators();
    for rule in MARKER_FOCUS {
        assert!(css.contains(rule), "{rule} in {css}");
    }
    for banned in [
        ":has(",
        "@supports",
        "[data-marker=\"",
        "[data-marker-indicator=\"",
    ] {
        assert!(!css.contains(banned), "{banned} in {css}");
    }
}

/// A legend row's ring is its own, so it needs neither script nor `:has`.
#[test]
fn legend_focus_is_unchanged() {
    let css = indicators();
    let outlines: Vec<&str> = css.lines().filter(|l| l.contains("outline")).collect();
    assert_eq!(
        outlines,
        [
            "[data-legend-row]:focus-visible{outline:2px solid transparent}",
            ".js [data-marker]:focus-visible{outline:2px solid transparent}",
        ],
        "{css}"
    );
    assert!(css.contains(
        "[data-legend-row]:focus-visible>[data-row-indicator]>rect{stroke:var(--focus);stroke-width:2px;stroke-dasharray:none}"
    ));
}

#[test]
fn the_stylesheet_requests_nothing_but_fonts() {
    let css = [
        font_faces(),
        palettes(),
        layout(&WIDE, NARROW),
        indicators(),
        [VISUALLY_HIDDEN, HIDDEN_BOX].concat(),
    ]
    .concat();
    assert_eq!(css.matches("url(").count(), 2);
    assert_eq!(css.matches("url(data:font/woff2;base64,").count(), 2);
    assert_eq!(css.matches("src:").count(), 2);
    for banned in ["@import", "http", "<"] {
        assert!(!css.contains(banned), "{banned}");
    }
}

/// rg-min: forced colours (Windows high contrast) remap neither SVG paint
/// nor the palette, so every neutral role takes its system colour there:
/// surfaces `Canvas`, ink and strokes `CanvasText`, focus `Highlight`. One block,
/// after the themes, at their specificity, so it wins in every theme.
#[test]
fn forced_colours_map_the_neutrals_to_system_colours() {
    let css = palettes();
    let forced = concat!(
        "@media (forced-colors: active){:root,:root[data-theme]{",
        "--surface-1:Canvas;--surface-2:Canvas;--text-primary:CanvasText;",
        "--text-muted:CanvasText;--stroke-ring:CanvasText;--stroke-divider:CanvasText;",
        "--marker-hollow:Canvas;--focus:Highlight;}}\n"
    );
    assert!(css.ends_with(forced), "{css}");
}

/// FR-37, FR-72, FR-73 on the radar and legend: hover, focus and selection
/// differ in shape. A marker's hover is a dashed muted square and its
/// selection a solid one; a legend row's hover is a dashed outline, its
/// focus a solid one, its selection a bar. Legend hover comes before legend
/// focus, at the same specificity, so focus wins on a row that is both.
const STATES: [&str; 9] = [
    "[data-legend-row][data-hovered]>[data-row-indicator]{visibility:visible}\n",
    "[data-legend-row][data-hovered]>[data-row-indicator]>rect{stroke:var(--text-muted);stroke-width:1px;stroke-dasharray:3 2;vector-effect:non-scaling-stroke}\n",
    "[data-legend-row][data-selected]>[data-selection]{visibility:visible}\n",
    "[data-legend-row]:focus-visible>[data-row-indicator]>rect{stroke:var(--focus);stroke-width:2px;stroke-dasharray:none}\n",
    "[data-marker-indicator][data-hovered],[data-marker-indicator][data-selected]{visibility:visible}\n",
    "[data-marker-indicator][data-hovered] [data-selection]{visibility:visible}\n",
    "[data-marker-indicator][data-hovered] [data-selection]>rect{stroke:var(--text-muted);stroke-dasharray:3 2;vector-effect:non-scaling-stroke}\n",
    "[data-marker-indicator][data-selected] [data-selection]{visibility:visible}\n",
    "[data-marker-indicator][data-selected] [data-selection]>rect{stroke:var(--text-primary);stroke-dasharray:none}\n",
];

/// Hit areas and controls look clickable.
const CURSOR: &str = "[data-hit],[role=button]{cursor:pointer}\n";

#[test]
fn hover_focus_and_selection_differ() {
    let css = indicators();
    let at = |rule: &str| css.find(rule).unwrap_or_else(|| panic!("{rule} in {css}"));
    for rule in STATES {
        at(rule);
    }
    let [hover, _, _, focus, _, _, hovered_square, _, selected_square] = STATES.map(at);
    assert!(hover < focus, "legend focus wins over hover");
    assert!(
        hovered_square < selected_square,
        "selection wins over hover"
    );
    assert!(css.contains(CURSOR));
}
