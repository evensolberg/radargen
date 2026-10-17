use std::collections::BTreeMap;

use super::super::table::{PROSE_CLASS, TABLE_VIEW_CLASS};
use super::{
    ARIA_CONTROLS, ARIA_EXPANDED, ARRANGEMENT_CLASS, CLOSE_CLASS, DATA_FOCUSED, DATA_HOVERED,
    DATA_NARROW_QUERY, DATA_SELECTED, DATA_THEME, EMPTY_CLASS, HEADING_CLASS, JS_CLASS, PANEL_ID,
    SCRIPT, SKIP_CLASS, SKIP_OVER_PAGE, TO_DARK, TO_LIGHT, TOGGLE_CLASS, controls,
};
use super::{
    DATA_LEGEND, HIDE_LEGEND, LEGEND_HIDDEN, LEGEND_TOGGLE_CLASS, MOON_CLASS, SHOW_LEGEND,
    SKIP_TO_LEGEND, SKIP_TO_TABLE, SUN_CLASS,
};
use crate::emit::html::Legend;
use crate::svg::serialise::{
    DATA_HIT, DATA_LEGEND_ROW, DATA_MARKER, DATA_MARKER_INDICATOR, DATA_ROW,
};

/// The script's `const N = {…}` block as `name → value`, and the script
/// with the block taken out.
fn names() -> (BTreeMap<String, String>, String) {
    let (before, rest) = SCRIPT.split_once("const N = {").expect("a names block");
    let (block, after) = rest.split_once("};").expect("the block closes");
    let names = block
        .lines()
        .map(crate::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .map(|line| {
            let (name, value) = line
                .trim_end_matches(',')
                .split_once(": ")
                .unwrap_or_else(|| panic!("`name: 'value',`, not {line:?}"));
            let value = value
                .strip_prefix('\'')
                .and_then(|v| v.strip_suffix('\''))
                .unwrap_or_else(|| panic!("a quoted value: {line:?}"));
            (name.to_owned(), value.to_owned())
        })
        .collect();
    (names, [before, after].concat())
}

/// Every name the script uses is the one the page is written with, so a
/// renamed constant fails here even when no browser runs.
#[test]
fn the_scripts_names_are_the_pages() {
    let want: BTreeMap<String, String> = [
        ("row", DATA_ROW),
        ("marker", DATA_MARKER),
        ("hit", DATA_HIT),
        ("selected", DATA_SELECTED),
        ("hovered", DATA_HOVERED),
        ("expanded", ARIA_EXPANDED),
        ("controls", ARIA_CONTROLS),
        ("panel", PANEL_ID),
        ("heading", HEADING_CLASS),
        ("close", CLOSE_CLASS),
        ("empty", EMPTY_CLASS),
        ("prose", PROSE_CLASS),
        ("tableView", TABLE_VIEW_CLASS),
        ("legend", DATA_LEGEND_ROW),
        ("markerIndicator", DATA_MARKER_INDICATOR),
        ("focused", DATA_FOCUSED),
        ("narrowQuery", DATA_NARROW_QUERY),
        ("theme", DATA_THEME),
        ("toDark", TO_DARK),
        ("toLight", TO_LIGHT),
        ("js", JS_CLASS),
        ("toggle", TOGGLE_CLASS),
        ("legendToggle", LEGEND_TOGGLE_CLASS),
        ("legendState", DATA_LEGEND),
        ("legendHidden", LEGEND_HIDDEN),
        ("hideLegend", HIDE_LEGEND),
        ("showLegend", SHOW_LEGEND),
        ("skipLegend", SKIP_TO_LEGEND),
        ("skipTable", SKIP_TO_TABLE),
        (
            "words",
            super::super::css::VISUALLY_HIDDEN.trim_start_matches('.'),
        ),
        ("skip", SKIP_CLASS),
        ("arrangement", ARRANGEMENT_CLASS),
        ("legendScroll", super::super::css::WIDE_LEGEND),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect();
    let (got, rest) = names();
    assert_eq!(got, want, "the names block");
    for prefix in ["data-", "aria-"] {
        assert!(
            !rest.contains(prefix),
            "every {prefix} attribute comes from the names block"
        );
    }
}

/// Whether `word` occurs in `text` with no identifier character either
/// side, so `import` does not match `important`.
fn has_word(text: &str, word: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    text.match_indices(word).any(|(at, _)| {
        let before = text.get(..at).and_then(|s| s.chars().next_back());
        let after = text.get(at + word.len()..).and_then(|s| s.chars().next());
        !before.is_some_and(ident) && !after.is_some_and(ident)
    })
}

/// NFR-5: nothing that fetches, opens or loads. NFR-6: nothing that parses
/// markup or code from a string, so no CSV text can become either.
#[test]
fn the_script_fetches_nothing_and_parses_no_markup() {
    const BANNED: [&str; 26] = [
        "fetch",
        "XMLHttpRequest",
        "import",
        "WebSocket",
        "EventSource",
        "Worker",
        "sendBeacon",
        "navigator",
        "write",
        "Image",
        "src",
        "srcdoc",
        "innerHTML",
        "outerHTML",
        "insertAdjacentHTML",
        "DOMParser",
        "createContextualFragment",
        "eval",
        "Function",
        "setTimeout",
        "setInterval",
        "location",
        "href",
        "createElement",
        "insertRule",
        "cssText",
    ];
    for word in BANNED {
        assert!(!has_word(SCRIPT, word), "{word} in the script");
    }
    let lower = SCRIPT.to_lowercase();
    for text in ["<!--", "</script", "window.open"] {
        assert!(!lower.contains(text), "{text} in the script, in any case");
    }
}

#[test]
fn words_match_whole() {
    assert!(has_word("x.import(y)", "import"));
    assert!(!has_word("!important", "import"));
    assert!(!has_word("prefetch", "fetch"));
    assert!(has_word("write", "write"));
}

/// FR-38's panel is sticky at the viewport's bottom, scrolls inside itself,
/// and its close button and heading show focus (WCAG 2.4.7).
#[test]
fn the_panel_sticks_and_shows_focus() {
    let css = super::css(
        &super::super::css::Wide {
            title_height: 53.0,
            radar_width: 670.0,
            radar_height: 640.0,
            legend_width: 330.0,
        },
        true,
    );
    for rule in [
        ".detail{position:sticky;bottom:.75rem;max-height:min(45vh,28em);overflow:auto;",
        ".detail dd{margin:0 0 .6em;overflow-wrap:anywhere}",
        ".detail-close:focus-visible,.detail-heading:focus-visible{outline:2px solid var(--focus)",
        ".detail dd.prose{white-space:pre-line;max-width:70ch}",
    ] {
        assert!(css.contains(rule), "{rule} in {css}");
    }
}

/// The PRD example's wide parts, for the controls' rules.
const WIDE: super::super::css::Wide = super::super::css::Wide {
    title_height: 53.0,
    radar_width: 670.0,
    radar_height: 640.5,
    legend_width: 306.817,
};

/// §6.6: a page drawn with a `--palette` file has the skip link and no
/// toggle, and the built-in page's controls keep their bytes.
#[test]
fn a_palette_page_has_no_toggle() {
    let palette = controls(false, Legend::Shown);
    assert!(!palette.contains(TOGGLE_CLASS));
    assert!(
        !palette.contains(MOON_CLASS) && !palette.contains(SUN_CLASS),
        "no theme icons"
    );
    assert!(palette.contains(SKIP_CLASS));
    assert_eq!(
        controls(true, Legend::Shown),
        [
            "<div class=\"page-controls\"><button type=\"button\" class=\"skip\" hidden=\"\">Skip to the legend</button>",
            "<button type=\"button\" class=\"legend-toggle\" hidden=\"\">",
            "<svg class=\"legend-on\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M9 6h11M9 12h11M9 18h11M4 6h.01M4 12h.01M4 18h.01\"/></svg>",
            "<svg class=\"legend-off\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M9 6h11M9 12h3M9 18h3M4 6h.01M4 12h.01M4 18h.01M15 12l6 6M21 12l-6 6\"/></svg>",
            "<span class=\"visually-hidden\">Hide legend</span></button>",
            "<button type=\"button\" class=\"theme-toggle\" hidden=\"\">",
            "<svg class=\"theme-moon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M20 14.5A8.5 8.5 0 1 1 9.5 4a7.5 7.5 0 0 0 10.5 10.5z\"/></svg>",
            "<svg class=\"theme-sun\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"12\" r=\"4\"/><path d=\"M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4\"/></svg>",
            "<span class=\"visually-hidden\">Switch to dark theme</span></button></div>\n",
        ]
        .concat()
    );
}

/// rg-1ru: the toggle's words carry the class the script finds them by
/// (`N.words`), and each icon shows as the palette block showing says: the
/// moon under the light palette, the sun under either dark one.
#[test]
fn the_toggle_shows_the_icon_its_palette_picks() {
    let hidden = super::super::css::VISUALLY_HIDDEN.trim_start_matches('.');
    assert!(
        controls(true, Legend::Shown)
            .contains(&["<span class=\"", hidden, "\">", TO_DARK, "</span>"].concat())
    );
    let css = super::css(&WIDE, true);
    for rule in [
        ".page-controls .theme-toggle{",
        "padding:.4em}",
        ".theme-toggle svg{width:1.25em;height:1.25em;",
        "fill:none;stroke:currentColor;stroke-width:2;",
        ".theme-toggle .theme-moon{display:var(--theme-moon)}",
        ".theme-toggle .theme-sun{display:var(--theme-sun)}",
    ] {
        assert!(css.contains(rule), "{rule} in {css}");
    }
}

/// Plan RG-018A R11: the focused skip link, a text button a little taller
/// than the icon buttons, is drawn over the page on every page, so focusing
/// it never pushes the page down.
#[test]
fn the_skip_link_is_drawn_over_the_page() {
    for toggle in [true, false] {
        assert!(
            super::css(&WIDE, toggle).ends_with(SKIP_OVER_PAGE),
            "{toggle}"
        );
    }
    assert!(
        !super::css(&WIDE, false).contains(".theme-toggle svg"),
        "no icon rule"
    );
}

/// RG-102: the legend button is on every page, a --palette page too, named
/// by what a press does from where the page starts; its icons show the
/// state, swapped by the root attribute.
#[test]
fn the_legend_button_is_named_by_where_the_page_starts() {
    let name = |n: &str| ["<span class=\"visually-hidden\">", n, "</span>"].concat();
    for toggle in [true, false] {
        let (shown, hidden) = (
            controls(toggle, Legend::Shown),
            controls(toggle, Legend::Hidden),
        );
        assert!(shown.contains(&name(HIDE_LEGEND)) && !shown.contains(&name(SHOW_LEGEND)));
        assert!(hidden.contains(&name(SHOW_LEGEND)) && !hidden.contains(&name(HIDE_LEGEND)));
        assert!(shown.contains(LEGEND_TOGGLE_CLASS));
        let css = super::css(&WIDE, toggle);
        // Selector and declarations apart: a literal `{name:value}` reads to
        // clippy as a format argument.
        for rule in [
            [".page-controls .legend-toggle{", "padding:.4em}"],
            [".legend-toggle svg{", "width:1.25em;height:1.25em;"],
            [".legend-toggle{", "margin-inline-start:auto}"],
            [
                ".legend-toggle:not([hidden])~.theme-toggle{",
                "margin-inline-start:.5em}",
            ],
            [".legend-off{", "display:none}"],
            [":root[data-legend=\"hidden\"] .legend-on{", "display:none}"],
            [
                ":root[data-legend=\"hidden\"] .legend-off{",
                "display:block}",
            ],
        ] {
            let rule = rule.concat();
            assert!(css.contains(&rule), "{rule} in {css}");
        }
    }
}

/// rg-247: the radar's cap subtracts the page controls' height as
/// `3.05rem + 2px` (css.rs's `CAP_UNDER`), from four values here: the icon's
/// 1.25em, its .4em padding all round, the buttons' .5em block margin and
/// 1px border. A change to any of them must change `CAP_UNDER` too.
#[test]
fn the_controls_height_the_radar_cap_allows_for() {
    assert!(super::ICON_SVG.starts_with("width:1.25em;height:1.25em;"));
    assert_eq!(super::PAD, "padding:.4em");
    assert!(super::BUTTON_OPEN.contains("margin-block:.5em;"));
    assert!(super::BUTTON_OPEN.contains("border:1px solid var("));
    assert!(super::super::css::CAP_UNDER.contains(" - 3.05rem - 2px)"));
}
