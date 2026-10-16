use std::collections::BTreeMap;

use super::super::table::{PROSE_CLASS, TABLE_VIEW_CLASS};
use super::{
    ARIA_CONTROLS, ARIA_EXPANDED, ARRANGEMENT_CLASS, CLOSE_CLASS, DATA_FOCUSED, DATA_HOVERED,
    DATA_NARROW_QUERY, DATA_SELECTED, DATA_THEME, EMPTY_CLASS, HEADING_CLASS, JS_CLASS, PANEL_ID,
    SCRIPT, SKIP_CLASS, TO_DARK, TO_LIGHT, TOGGLE_CLASS,
};
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
    let css = super::css(&super::super::css::Wide {
        title_height: 53.0,
        radar_width: 670.0,
        radar_height: 640.0,
        legend_width: 330.0,
    });
    for rule in [
        ".detail{position:sticky;bottom:.75rem;max-height:min(45vh,28em);overflow:auto;",
        ".detail dd{margin:0 0 .6em;overflow-wrap:anywhere}",
        ".detail-close:focus-visible,.detail-heading:focus-visible{outline:2px solid var(--focus)",
        ".detail dd.prose{white-space:pre-line;max-width:70ch}",
    ] {
        assert!(css.contains(rule), "{rule} in {css}");
    }
}
