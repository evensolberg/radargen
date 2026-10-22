//! The exported SVG's names, vocabulary and escaping (SC-9, SC-15, NFR-6,
//! D-88).

use radargen::emit::OUTPUT_VERSION;
use radargen::emit::svg::{Export, export};
use radargen::layout::arrange::{Arrangement, arrange};
use radargen::palette::{Palette, Theme};
use radargen::svg::{Document, Node, Semantics, TextPaint};
use radargen::text::Fonts;

use super::build;
use super::hostile::{RING, SECTOR, TITLE, csv};

/// The elements the export writes.
const ELEMENTS: [&str; 10] = [
    "svg", "title", "desc", "defs", "path", "use", "g", "rect", "circle", "line",
];

/// The attributes it writes, by local name: the static serialiser's
/// shapes, groups and controls, and the export's own. No `font-*`, no
/// `text-anchor`, `letter-spacing` or `style`.
const ATTRIBUTES: [&str; 26] = [
    "viewBox",
    "width",
    "height",
    "lang",
    "id",
    "href",
    "d",
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "cx",
    "cy",
    "r",
    "rx",
    "fill",
    "stroke",
    "stroke-width",
    "stroke-linejoin",
    "transform",
    "role",
    "aria-label",
    "aria-level",
    "aria-hidden",
];

/// The hostile CSV, wide, exported dark at 800.
fn hostile() -> (Document, String) {
    let built = build(&csv(), Some(&format!("{RING},Adopt")), Some(TITLE));
    let doc = arrange(Arrangement::Wide, &built.blocks);
    let palette = Palette::built_in(Theme::Dark);
    let fonts = Fonts::embedded().expect("loads");
    let svg = export(
        &doc,
        &Export {
            palette: &palette,
            fonts: &fonts,
            width: Some(800),
            title: Some(TITLE),
            desc: &built.blocks.radar.name(),
        },
    )
    .expect("exports");
    (doc, svg)
}

/// What the export must name, in document order: each filled text
/// outside a control by its words (and tail), each control by its
/// label, and the radar's group by its name. A wrapped heading carries
/// no label of its own: its outlined text already names it (rg-g0c).
fn names(node: &Node, in_control: bool, out: &mut Vec<String>) {
    match node {
        _ if node.is_interactive_only() => {}
        Node::Group(group) => {
            let inner = match &group.semantics {
                Semantics::Marker(c) | Semantics::LegendRow(c) => {
                    out.push(c.label.clone());
                    true
                }
                Semantics::Group(name) => {
                    out.push(name.clone());
                    in_control
                }
                _ => in_control,
            };
            for child in &group.children {
                names(child, inner, out);
            }
        }
        Node::Text {
            ink: TextPaint::Fill(_),
            lines,
            words,
            tail,
            ..
        } if !in_control && !lines.is_empty() => {
            let mut name = words.clone();
            if let Some(tail) = tail {
                name.push_str(&tail.text);
            }
            out.push(name);
        }
        _ => {}
    }
}

/// SC-9 through an XML parser: every name reads back exactly, once, in
/// order; the root's title and description read back; the only comment
/// is the version.
#[test]
fn the_export_names_every_text_exactly() {
    let (doc, svg) = hostile();
    let parsed = super::parse(&svg);
    let mut want = Vec::new();
    names(&doc.root, false, &mut want);
    let got: Vec<&str> = parsed
        .descendants()
        .filter_map(|n| n.attribute("aria-label"))
        .collect();
    assert_eq!(got, want);
    // The hostile strings reached the names at all.
    for needle in [SECTOR, RING] {
        assert!(
            got.iter().any(|l| l.contains(needle)),
            "{needle} in {got:?}"
        );
    }
    let root = parsed.root_element();
    let child = |tag: &str| {
        root.children()
            .find(|n| n.has_tag_name(tag))
            .and_then(|n| n.text())
    };
    assert_eq!(child("title"), Some(TITLE));
    assert_eq!(
        child("desc"),
        want.iter()
            .find(|n| n.starts_with("Technology radar: "))
            .map(String::as_str)
    );
    // The static tooltips read back as the document holds them.
    let tooltips: Vec<&str> = parsed
        .descendants()
        .filter(|n| n.has_tag_name("title") && n.parent() != Some(root))
        .filter_map(|n| n.text())
        .collect();
    assert_eq!(
        tooltips,
        super::hostile::Strings::of(&doc.root, false).tooltips
    );
    let comments: Vec<&str> = parsed
        .root()
        .children()
        .filter(roxmltree::Node::is_comment)
        .filter_map(|n| n.text())
        .collect();
    assert_eq!(comments, [format!(" {OUTPUT_VERSION} ")]);
}

/// SC-15, NFR-5: only the export's vocabulary; every id a glyph in
/// `<defs>`; every reference an `XLink` fragment naming one; every number
/// NFR-17's shape.
#[test]
fn the_export_uses_only_its_vocabulary() {
    const XLINK: &str = "http://www.w3.org/1999/xlink";
    for svg in [hostile().1, super::golden::prd_svg(Theme::Light)] {
        let parsed = super::parse(&svg);
        let ids: Vec<&str> = parsed
            .descendants()
            .filter_map(|n| n.attribute("id"))
            .collect();
        for node in parsed.descendants().filter(roxmltree::Node::is_element) {
            let name = node.tag_name().name();
            assert!(ELEMENTS.contains(&name), "<{name}>");
            for a in node.attributes() {
                assert!(ATTRIBUTES.contains(&a.name()), "<{name} {}>", a.name());
                assert!(
                    !a.value().contains("url(") && !a.value().contains("var("),
                    "<{name} {}={}>",
                    a.name(),
                    a.value()
                );
            }
            if let Some(id) = node.attribute("id") {
                assert!(
                    name == "path" && node.parent().is_some_and(|p| p.has_tag_name("defs")),
                    "{id}"
                );
                assert!(
                    id.strip_prefix('g')
                        .is_some_and(|n| n.chars().all(|c| c.is_ascii_digit())),
                    "{id}"
                );
            }
            if name == "use" {
                let href = node.attribute((XLINK, "href")).expect("an XLink href");
                assert!(
                    ids.contains(&href.strip_prefix('#').expect("a fragment")),
                    "{href}"
                );
            }
            if !["title", "desc"].contains(&name) {
                assert!(
                    node.children()
                        .filter(roxmltree::Node::is_text)
                        .filter_map(|t| t.text())
                        .all(|t| t.chars().all(char::is_whitespace)),
                    "text in <{name}>"
                );
            }
        }
        numbers_are_fmt_coord(&parsed);
    }
}

/// The attributes that hold numbers, written by `fmt_coord` alone.
const NUMERIC: [&str; 15] = [
    "width",
    "height",
    "viewBox",
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "cx",
    "cy",
    "r",
    "rx",
    "stroke-width",
    "d",
];

/// Every number in a numeric attribute, and every number in `d`, has
/// NFR-17's shape (vocabulary.rs's `well_formed`).
fn numbers_are_fmt_coord(parsed: &roxmltree::Document<'_>) {
    for node in parsed.descendants().filter(roxmltree::Node::is_element) {
        for a in node.attributes().filter(|a| NUMERIC.contains(&a.name())) {
            let numbers = a
                .value()
                .split(|c: char| c == ' ' || c.is_ascii_alphabetic())
                .filter(|token| !token.is_empty());
            for token in numbers {
                assert!(
                    super::vocabulary::well_formed(token),
                    "<{} {}>: {token}",
                    node.tag_name().name(),
                    a.name()
                );
            }
        }
    }
}
